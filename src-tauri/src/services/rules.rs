//! Rules_Service: managed platform rule files and their version history
//! (Requirement 14).
//!
//! Like the other services, every function is written against a borrowed
//! [`rusqlite::Connection`] (plus a base directory `&Path` where snapshot files
//! are written) rather than reaching into global [`crate::state::AppState`], so
//! the rules are directly unit-testable with an in-memory pool
//! (`storage::create_memory_pool` + `storage::init_schema`) and a `tempfile`
//! directory. The Command_Layer (task 17.1) hands these functions a pooled
//! connection and the resolved rules directory.
//!
//! ## Storage approach (documented decision)
//!
//! The `rules` table stores the rule **descriptor** (platform metadata, managed
//! and target paths, `sync_status`, `content_hash`, `current_version`). The
//! `rule_versions` table stores, per the schema, a `file_path` for each snapshot
//! rather than inline content. This module therefore uses **file-backed
//! snapshots**: each version's content is written to a file under a
//! caller-provided base directory and the absolute path is recorded in
//! `rule_versions.file_path`. The DTO [`RuleVersionSnapshot`] (which carries
//! inline `content`) is reconstructed by reading that file back. This keeps the
//! on-disk shape faithful to the reference design (managed copy + per-version
//! files) while remaining fully self-contained and testable with a tempdir — no
//! OS paths are hard-coded.
//!
//! - The rule's **current content** lives at its `managed_path` file. [`read`]
//!   and [`list`] read it back (falling back to the target file, then to an
//!   empty string when neither exists, mirroring the reference).
//! - **Snapshot files** are written under the caller-supplied `versions_dir` as
//!   `{versions_dir}/{rule-id}/{version:04}.md`.
//!
//! ## Hashing (documented decision)
//!
//! `content_hash` is the lowercase hex SHA-256 of the content bytes (`sha2`).
//! [`scan`] compares the hash of the **target** file's bytes to the stored
//! managed `content_hash` to classify `sync_status` (14.2): equal -> `synced`,
//! target absent -> `target-missing`, different -> `out-of-sync`, unreadable ->
//! `sync-error`.
//!
//! ## Version cap and ordering (14.7, 14.9 — Property 30)
//!
//! Snapshots are ordered **most-recent first** (`ORDER BY version DESC`; `version`
//! is unique and monotonically increasing per rule). [`save`] retains at most the
//! 20 most-recent snapshots, deleting the oldest rows (and their files) once the
//! count would exceed 20. [`delete_version`] removes a single snapshot and returns
//! the remaining snapshots, still most-recent first.
//!
//! Unknown rule id on [`read`]/[`save`] returns `NOT_FOUND` and makes no mutation
//! (14.8). Timestamps are stored as epoch milliseconds and read back as ISO_8601
//! strings (Requirement 4.9).
#![allow(dead_code)]

use std::fs;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::models::{RuleFileContent, RuleVersionSnapshot, SyncStatus};
use crate::storage::time::{millis_to_iso8601, now_millis};

/// Maximum number of version snapshots retained per rule (14.9).
const RULE_VERSION_LIMIT: i64 = 20;

/// Allowed `rule_versions.source` values (matches the schema CHECK constraint).
const ALLOWED_SOURCES: [&str; 3] = ["manual-save", "ai-rewrite", "create"];

/// Arguments for registering a project-scoped rule (14.5).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AddProjectInput {
    /// Human-readable project name (required, non-empty after trimming).
    pub name: String,
    /// Absolute path to the project root directory (required, non-empty).
    pub project_root_path: String,
    /// Optional rule description.
    pub description: Option<String>,
    /// Optional initial content to seed the managed file with.
    pub initial_content: Option<String>,
    /// Optional caller-supplied id; a UUID is generated when omitted.
    pub id: Option<String>,
}

/// Raw `rules` row, read locally (no rules mapper exists in `storage::mapping`).
struct RuleRow {
    id: String,
    platform_id: String,
    platform_name: String,
    platform_icon: String,
    platform_description: String,
    canonical_file_name: String,
    description: String,
    managed_path: String,
    target_path: String,
    project_root_path: Option<String>,
    sync_status: SyncStatus,
    content_hash: String,
}

/// Maps a raw rusqlite error into an `INTERNAL` [`AppError`].
fn db_err(context: &str, e: rusqlite::Error) -> AppError {
    AppError::internal(format!("{context}: {e}"))
}

/// Maps a raw filesystem error into an `IO` [`AppError`].
fn io_err(context: &str, e: std::io::Error) -> AppError {
    AppError::io(format!("{context}: {e}"))
}

/// Lowercase hex SHA-256 of the given bytes.
fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Lowercase hex SHA-256 of the given content string.
fn hash_content(content: &str) -> String {
    hash_bytes(content.as_bytes())
}

/// Stable wire spelling for a [`SyncStatus`] (matches the schema CHECK values).
fn sync_status_str(status: SyncStatus) -> &'static str {
    match status {
        SyncStatus::Synced => "synced",
        SyncStatus::TargetMissing => "target-missing",
        SyncStatus::OutOfSync => "out-of-sync",
        SyncStatus::SyncError => "sync-error",
    }
}

/// Decodes a stored `sync_status` TEXT value (defaults to `sync-error` on an
/// unrecognized value, which the schema CHECK constraint already prevents).
fn parse_sync_status(raw: &str) -> SyncStatus {
    match raw {
        "synced" => SyncStatus::Synced,
        "target-missing" => SyncStatus::TargetMissing,
        "out-of-sync" => SyncStatus::OutOfSync,
        _ => SyncStatus::SyncError,
    }
}

/// Replaces path-unsafe characters (e.g. the `:` in `project:<uuid>` ids) so a
/// rule id can be used as a directory segment on every target platform.
fn sanitize_segment(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Lowercases and dash-joins a display name for use in a directory name.
fn slugify(value: &str) -> String {
    let slug: String = value
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "rule".to_string()
    } else {
        trimmed
    }
}

/// Writes `content` to `path`, creating parent directories as needed.
fn write_file(path: &Path, content: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| io_err("failed to create directory", e))?;
    }
    fs::write(path, content).map_err(|e| io_err("failed to write file", e))
}

/// Reads the rule's current content: the managed file if present, else the
/// target file if present, else an empty string (mirrors the reference).
fn read_current_content(row: &RuleRow) -> Result<String, AppError> {
    if Path::new(&row.managed_path).exists() {
        fs::read_to_string(&row.managed_path).map_err(|e| io_err("failed to read managed rule", e))
    } else if Path::new(&row.target_path).exists() {
        fs::read_to_string(&row.target_path).map_err(|e| io_err("failed to read target rule", e))
    } else {
        Ok(String::new())
    }
}

/// Classifies a rule's sync status by comparing the target file's hash to the
/// managed `content_hash` (14.2).
fn compute_sync_status(target_path: &str, content_hash: &str) -> SyncStatus {
    let path = Path::new(target_path);
    if !path.exists() {
        return SyncStatus::TargetMissing;
    }
    match fs::read(path) {
        Ok(bytes) => {
            if hash_bytes(&bytes) == content_hash {
                SyncStatus::Synced
            } else {
                SyncStatus::OutOfSync
            }
        }
        Err(_) => SyncStatus::SyncError,
    }
}

/// Reads a single `rules` row by id, returning `NOT_FOUND` when absent (14.8).
fn get_rule_row(conn: &Connection, id: &str) -> Result<RuleRow, AppError> {
    conn.query_row(
        "SELECT id, platform_id, platform_name, platform_icon, platform_description, \
                canonical_file_name, description, managed_path, target_path, project_root_path, \
                sync_status, content_hash \
         FROM rules WHERE id = ?1",
        [id],
        |row| {
            let sync_raw: String = row.get(10)?;
            Ok(RuleRow {
                id: row.get(0)?,
                platform_id: row.get(1)?,
                platform_name: row.get(2)?,
                platform_icon: row.get(3)?,
                platform_description: row.get(4)?,
                canonical_file_name: row.get(5)?,
                description: row.get(6)?,
                managed_path: row.get(7)?,
                target_path: row.get(8)?,
                project_root_path: row.get(9)?,
                sync_status: parse_sync_status(&sync_raw),
                content_hash: row.get(11)?,
            })
        },
    )
    .optional()
    .map_err(|e| db_err("failed to read rule", e))?
    .ok_or_else(|| AppError::not_found(format!("rule `{id}` not found")))
}

/// Loads all rule rows ordered by creation time (stable listing).
fn all_rule_ids(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut stmt = conn
        .prepare("SELECT id FROM rules ORDER BY created_at ASC, id ASC")
        .map_err(|e| db_err("failed to prepare rule list", e))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| db_err("failed to query rules", e))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| db_err("failed to map rule ids", e))
}

/// Loads a rule's version snapshots ordered most-recent first (14.7, 14.9),
/// reading each snapshot's content back from its `file_path`.
fn load_versions(conn: &Connection, rule_id: &str) -> Result<Vec<RuleVersionSnapshot>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, file_path, source, created_at FROM rule_versions \
             WHERE rule_id = ?1 ORDER BY version DESC",
        )
        .map_err(|e| db_err("failed to prepare rule version list", e))?;
    let rows = stmt
        .query_map([rule_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|e| db_err("failed to query rule versions", e))?;

    let mut snapshots = Vec::new();
    for row in rows {
        let (id, file_path, source, created_at) =
            row.map_err(|e| db_err("failed to map rule version row", e))?;
        // Missing snapshot files degrade to empty content rather than failing the
        // whole read; the row remains the source of truth for ordering/metadata.
        let content = fs::read_to_string(&file_path).unwrap_or_default();
        snapshots.push(RuleVersionSnapshot {
            id,
            saved_at: millis_to_iso8601(created_at),
            content,
            source,
        });
    }
    Ok(snapshots)
}

/// Builds the full [`RuleFileContent`] DTO for a rule row.
fn to_file_content(conn: &Connection, row: &RuleRow) -> Result<RuleFileContent, AppError> {
    let content = read_current_content(row)?;
    let versions = load_versions(conn, &row.id)?;
    let exists = Path::new(&row.target_path).exists();
    Ok(RuleFileContent {
        id: row.id.clone(),
        platform_id: row.platform_id.clone(),
        platform_name: row.platform_name.clone(),
        platform_icon: row.platform_icon.clone(),
        platform_description: row.platform_description.clone(),
        name: row.canonical_file_name.clone(),
        description: row.description.clone(),
        path: row.target_path.clone(),
        exists,
        managed_path: Some(row.managed_path.clone()),
        target_path: Some(row.target_path.clone()),
        project_root_path: row.project_root_path.clone(),
        sync_status: Some(row.sync_status),
        content,
        versions,
    })
}

/// Returns all managed rule descriptors, empty when none are managed (14.1).
pub fn list(conn: &Connection) -> Result<Vec<RuleFileContent>, AppError> {
    let ids = all_rule_ids(conn)?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let row = get_rule_row(conn, &id)?;
        out.push(to_file_content(conn, &row)?);
    }
    Ok(out)
}

/// Detects rule files and returns their descriptors, recomputing and persisting
/// each `sync_status` against its target file (14.2).
///
/// Each returned descriptor's `sync_status` is exactly one of `synced`,
/// `out-of-sync`, `target-missing`, or `sync-error`.
pub fn scan(conn: &Connection) -> Result<Vec<RuleFileContent>, AppError> {
    let ids = all_rule_ids(conn)?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let mut row = get_rule_row(conn, &id)?;
        let status = compute_sync_status(&row.target_path, &row.content_hash);
        if status != row.sync_status {
            conn.execute(
                "UPDATE rules SET sync_status = ?1 WHERE id = ?2",
                params![sync_status_str(status), id],
            )
            .map_err(|e| db_err("failed to update rule sync_status", e))?;
            row.sync_status = status;
        }
        out.push(to_file_content(conn, &row)?);
    }
    Ok(out)
}

/// Returns a rule's file content by id, `NOT_FOUND` when unknown (14.3, 14.8).
pub fn read(conn: &Connection, id: &str) -> Result<RuleFileContent, AppError> {
    let row = get_rule_row(conn, id)?;
    to_file_content(conn, &row)
}

/// Persists rule content, creates a version snapshot, and returns the updated
/// rule content (14.4). `NOT_FOUND` (no mutation) for an unknown id (14.8).
///
/// `source` defaults to `manual-save`; the snapshot file is written under
/// `versions_dir`. Retains at most the 20 most-recent snapshots (14.9).
pub fn save(
    conn: &Connection,
    id: &str,
    content: &str,
    source: Option<&str>,
    versions_dir: &Path,
) -> Result<RuleFileContent, AppError> {
    // NOT_FOUND before any write (14.8).
    let row = get_rule_row(conn, id)?;

    let source = source.unwrap_or("manual-save");
    if !ALLOWED_SOURCES.contains(&source) {
        return Err(AppError::validation(format!(
            "invalid rule version source `{source}`"
        )));
    }

    // Persist the managed content and recompute the descriptor hash/status.
    write_file(Path::new(&row.managed_path), content)?;
    let content_hash = hash_content(content);
    let sync_status = compute_sync_status(&row.target_path, &content_hash);

    // Next version number, monotonic per rule (starts at 1).
    let max_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM rule_versions WHERE rule_id = ?1",
            [id],
            |r| r.get(0),
        )
        .map_err(|e| db_err("failed to compute next rule version", e))?;
    let next_version = max_version + 1;

    // Write the snapshot file under the caller-provided base directory.
    let snapshot_path = versions_dir
        .join(sanitize_segment(id))
        .join(format!("{next_version:04}.md"));
    write_file(&snapshot_path, content)?;
    let snapshot_path_str = snapshot_path.to_string_lossy().to_string();
    let version_id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();

    // Collect the snapshot files that the cap will discard so they can be removed
    // from disk after the transaction commits.
    let stale_files = stale_version_files(conn, id)?;

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| db_err("failed to begin save-rule transaction", e))?;
    tx.execute(
        "INSERT INTO rule_versions (id, rule_id, version, file_path, source, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![version_id, id, next_version, snapshot_path_str, source, now],
    )
    .map_err(|e| db_err("failed to insert rule version", e))?;
    tx.execute(
        "UPDATE rules SET content_hash = ?1, current_version = ?2, sync_status = ?3, updated_at = ?4 \
         WHERE id = ?5",
        params![content_hash, next_version, sync_status_str(sync_status), now, id],
    )
    .map_err(|e| db_err("failed to update rule after save", e))?;
    // Enforce the 20-snapshot cap, discarding the oldest (lowest version) rows.
    tx.execute(
        "DELETE FROM rule_versions WHERE rule_id = ?1 AND id IN (\
            SELECT id FROM rule_versions WHERE rule_id = ?1 \
            ORDER BY version DESC LIMIT -1 OFFSET ?2)",
        params![id, RULE_VERSION_LIMIT],
    )
    .map_err(|e| db_err("failed to enforce rule version cap", e))?;
    tx.commit()
        .map_err(|e| db_err("failed to commit save-rule transaction", e))?;

    // Best-effort removal of discarded snapshot files (DB is the source of truth).
    for path in stale_files {
        let _ = fs::remove_file(path);
    }

    read(conn, id)
}

/// Returns the `file_path`s of snapshots that the version cap would discard
/// (those beyond the 20 most-recent), used to clean up files after a save.
fn stale_version_files(conn: &Connection, rule_id: &str) -> Result<Vec<String>, AppError> {
    // After inserting one new snapshot the retained set is the 20 most-recent, so
    // the rows at offset (LIMIT-1) and beyond in the current set become stale.
    let offset = RULE_VERSION_LIMIT - 1;
    let mut stmt = conn
        .prepare(
            "SELECT file_path FROM rule_versions WHERE rule_id = ?1 \
             ORDER BY version DESC LIMIT -1 OFFSET ?2",
        )
        .map_err(|e| db_err("failed to prepare stale version query", e))?;
    let rows = stmt
        .query_map(params![rule_id, offset], |row| row.get::<_, String>(0))
        .map_err(|e| db_err("failed to query stale versions", e))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| db_err("failed to map stale version rows", e))
}

/// Registers a project-scoped rule target and returns its descriptor (14.5).
///
/// The managed file is written under `managed_dir`; an initial `create` snapshot
/// is recorded under `versions_dir` when the seeded content is non-empty.
pub fn add_project(
    conn: &Connection,
    input: AddProjectInput,
    managed_dir: &Path,
    versions_dir: &Path,
) -> Result<RuleFileContent, AppError> {
    let name = input.name.trim().to_string();
    let root = input.project_root_path.trim().to_string();
    if name.is_empty() {
        return Err(AppError::validation("project name is required"));
    }
    if root.is_empty() {
        return Err(AppError::validation("project root path is required"));
    }

    // Reject a second registration for the same project root (14.5).
    let duplicate: Option<String> = conn
        .query_row(
            "SELECT id FROM rules WHERE scope = 'project' AND LOWER(project_root_path) = LOWER(?1) \
             LIMIT 1",
            [&root],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| db_err("failed to check for duplicate project rule", e))?;
    if duplicate.is_some() {
        return Err(AppError::conflict(format!(
            "a project rule is already registered for `{root}`"
        )));
    }

    let project_id = input.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let rule_id = format!("project:{project_id}");
    let canonical_file_name = "AGENTS.md";
    let managed_path = managed_dir
        .join(format!(
            "{}__{}",
            slugify(&name),
            sanitize_segment(&project_id)
        ))
        .join(canonical_file_name);
    let target_path = Path::new(&root).join(canonical_file_name);
    let target_path_str = target_path.to_string_lossy().to_string();

    // Seed the managed content from an existing target file when present,
    // otherwise from the supplied initial content (mirrors the reference).
    let initial_content = if target_path.exists() {
        fs::read_to_string(&target_path).map_err(|e| io_err("failed to read target rule", e))?
    } else {
        input.initial_content.unwrap_or_default()
    };
    write_file(&managed_path, &initial_content)?;
    let content_hash = hash_content(&initial_content);
    let sync_status = compute_sync_status(&target_path_str, &content_hash);

    let description = input
        .description
        .unwrap_or_else(|| "Project rule file loaded from a user-managed directory.".to_string());
    let now = now_millis();

    // An initial snapshot is created only when there is seed content.
    let create_initial_version = !initial_content.trim().is_empty();
    let current_version = if create_initial_version { 1 } else { 0 };

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| db_err("failed to begin add-project transaction", e))?;
    tx.execute(
        "INSERT INTO rules \
         (id, scope, platform_id, platform_name, platform_icon, platform_description, \
          canonical_file_name, description, managed_path, target_path, project_root_path, \
          sync_status, current_version, content_hash, created_at, updated_at) \
         VALUES (?1,'project',?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        params![
            rule_id,
            "workspace",
            name,
            "FolderRoot",
            format!("Project rules from {root}"),
            canonical_file_name,
            description,
            managed_path.to_string_lossy().to_string(),
            target_path_str,
            root,
            sync_status_str(sync_status),
            current_version,
            content_hash,
            now,
            now,
        ],
    )
    .map_err(|e| db_err("failed to insert project rule", e))?;

    if create_initial_version {
        let snapshot_path = versions_dir
            .join(sanitize_segment(&rule_id))
            .join("0001.md");
        write_file(&snapshot_path, &initial_content)?;
        tx.execute(
            "INSERT INTO rule_versions (id, rule_id, version, file_path, source, created_at) \
             VALUES (?1,?2,?3,?4,'create',?5)",
            params![
                uuid::Uuid::new_v4().to_string(),
                rule_id,
                1_i64,
                snapshot_path.to_string_lossy().to_string(),
                now,
            ],
        )
        .map_err(|e| db_err("failed to insert initial rule version", e))?;
    }

    tx.commit()
        .map_err(|e| db_err("failed to commit add-project transaction", e))?;

    read(conn, &rule_id)
}

/// Removes a project rule registration (14.6).
///
/// The `ON DELETE CASCADE` foreign key removes the rule's version rows. Returns
/// `NOT_FOUND` when no project rule with the id exists.
pub fn remove_project(conn: &Connection, id: &str) -> Result<(), AppError> {
    let affected = conn
        .execute(
            "DELETE FROM rules WHERE id = ?1 AND scope = 'project'",
            [id],
        )
        .map_err(|e| db_err("failed to remove project rule", e))?;
    if affected == 0 {
        return Err(AppError::not_found(format!(
            "project rule `{id}` not found"
        )));
    }
    Ok(())
}

/// Deletes a single rule version and returns the remaining snapshots ordered
/// most-recent first (14.7). Returns `NOT_FOUND` for an unknown rule (14.8).
///
/// Deleting an absent version is a no-op that returns the current snapshots,
/// matching the reference behavior.
pub fn delete_version(
    conn: &Connection,
    rule_id: &str,
    version_id: &str,
) -> Result<Vec<RuleVersionSnapshot>, AppError> {
    // NOT_FOUND when the rule does not exist (14.8).
    get_rule_row(conn, rule_id)?;

    let file_path: Option<String> = conn
        .query_row(
            "SELECT file_path FROM rule_versions WHERE rule_id = ?1 AND id = ?2",
            params![rule_id, version_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| db_err("failed to read rule version", e))?;

    if let Some(path) = file_path {
        conn.execute(
            "DELETE FROM rule_versions WHERE rule_id = ?1 AND id = ?2",
            params![rule_id, version_id],
        )
        .map_err(|e| db_err("failed to delete rule version", e))?;
        let _ = fs::remove_file(path);
    }

    load_versions(conn, rule_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::storage::{create_memory_pool, init_schema, DbPool};
    use rusqlite::Connection;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Builds an in-memory pool with the schema initialized.
    fn schema_pool() -> DbPool {
        let pool = create_memory_pool().expect("memory pool");
        init_schema(&pool.get().expect("conn")).expect("schema");
        pool
    }

    /// Inserts a global rule row whose managed/target paths point into `dir`.
    ///
    /// `content_hash` is the hash of the managed content the caller intends; the
    /// managed/target files are created separately by each test as needed.
    fn seed_rule(
        conn: &Connection,
        id: &str,
        managed_path: &Path,
        target_path: &Path,
        content_hash: &str,
        sync_status: SyncStatus,
    ) {
        conn.execute(
            "INSERT INTO rules \
             (id, scope, platform_id, platform_name, platform_icon, platform_description, \
              canonical_file_name, description, managed_path, target_path, project_root_path, \
              sync_status, current_version, content_hash, created_at, updated_at) \
             VALUES (?1,'global','claude','Claude','icon','desc','CLAUDE.md','a rule',\
                     ?2,?3,NULL,?4,0,?5,0,0)",
            params![
                id,
                managed_path.to_string_lossy().to_string(),
                target_path.to_string_lossy().to_string(),
                sync_status_str(sync_status),
                content_hash,
            ],
        )
        .unwrap();
    }

    struct Dirs {
        _tmp: TempDir,
        managed: PathBuf,
        target: PathBuf,
        versions: PathBuf,
    }

    fn dirs() -> Dirs {
        let tmp = tempfile::tempdir().unwrap();
        let managed = tmp.path().join("managed");
        let target = tmp.path().join("target");
        let versions = tmp.path().join("versions");
        Dirs {
            _tmp: tmp,
            managed,
            target,
            versions,
        }
    }

    // --- list (14.1) -------------------------------------------------------

    #[test]
    fn list_returns_empty_then_descriptors() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();

        assert!(list(&conn).unwrap().is_empty());

        let managed = d.managed.join("r1.md");
        let target = d.target.join("CLAUDE.md");
        write_file(&managed, "hello").unwrap();
        seed_rule(
            &conn,
            "r1",
            &managed,
            &target,
            &hash_content("hello"),
            SyncStatus::TargetMissing,
        );

        let listed = list(&conn).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "r1");
        assert_eq!(listed[0].content, "hello");
        assert_eq!(listed[0].name, "CLAUDE.md");
        assert!(listed[0].versions.is_empty());
    }

    // --- scan (14.2) -------------------------------------------------------

    #[test]
    fn scan_computes_synced_when_target_matches() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();

        let managed = d.managed.join("r1.md");
        let target = d.target.join("CLAUDE.md");
        write_file(&managed, "X").unwrap();
        write_file(&target, "X").unwrap();
        seed_rule(
            &conn,
            "r1",
            &managed,
            &target,
            &hash_content("X"),
            SyncStatus::OutOfSync,
        );

        let scanned = scan(&conn).unwrap();
        assert_eq!(scanned[0].sync_status, Some(SyncStatus::Synced));
        // Persisted.
        assert_eq!(
            read(&conn, "r1").unwrap().sync_status,
            Some(SyncStatus::Synced)
        );
    }

    #[test]
    fn scan_computes_target_missing_when_no_target_file() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();

        let managed = d.managed.join("r1.md");
        let target = d.target.join("CLAUDE.md"); // never created
        write_file(&managed, "X").unwrap();
        seed_rule(
            &conn,
            "r1",
            &managed,
            &target,
            &hash_content("X"),
            SyncStatus::Synced,
        );

        let scanned = scan(&conn).unwrap();
        assert_eq!(scanned[0].sync_status, Some(SyncStatus::TargetMissing));
        assert!(!scanned[0].exists);
    }

    #[test]
    fn scan_computes_out_of_sync_when_target_differs() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();

        let managed = d.managed.join("r1.md");
        let target = d.target.join("CLAUDE.md");
        write_file(&managed, "X").unwrap();
        write_file(&target, "Y").unwrap();
        seed_rule(
            &conn,
            "r1",
            &managed,
            &target,
            &hash_content("X"),
            SyncStatus::Synced,
        );

        let scanned = scan(&conn).unwrap();
        assert_eq!(scanned[0].sync_status, Some(SyncStatus::OutOfSync));
    }

    // --- read (14.3, 14.8) -------------------------------------------------

    #[test]
    fn read_unknown_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = read(&conn, "nope").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn read_returns_content_and_versions() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();

        let managed = d.managed.join("r1.md");
        let target = d.target.join("CLAUDE.md");
        write_file(&managed, "current").unwrap();
        seed_rule(
            &conn,
            "r1",
            &managed,
            &target,
            &hash_content("current"),
            SyncStatus::TargetMissing,
        );

        let got = read(&conn, "r1").unwrap();
        assert_eq!(got.content, "current");
        assert_eq!(got.id, "r1");
    }

    // --- save (14.4, 14.8) -------------------------------------------------

    #[test]
    fn save_persists_creates_version_and_returns_content() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();

        let managed = d.managed.join("r1.md");
        let target = d.target.join("CLAUDE.md");
        seed_rule(
            &conn,
            "r1",
            &managed,
            &target,
            &hash_content(""),
            SyncStatus::TargetMissing,
        );

        let saved = save(&conn, "r1", "new body", None, &d.versions).unwrap();

        // Returned content reflects the save and carries one snapshot.
        assert_eq!(saved.content, "new body");
        assert_eq!(saved.versions.len(), 1);
        assert_eq!(saved.versions[0].content, "new body");
        assert_eq!(saved.versions[0].source, "manual-save");

        // Managed file persisted on disk.
        assert_eq!(fs::read_to_string(&managed).unwrap(), "new body");

        // content_hash + current_version updated in the row.
        let (hash, ver): (String, i64) = conn
            .query_row(
                "SELECT content_hash, current_version FROM rules WHERE id = 'r1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(hash, hash_content("new body"));
        assert_eq!(ver, 1);
    }

    #[test]
    fn save_unknown_returns_not_found_without_mutation() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();
        let err = save(&conn, "nope", "x", None, &d.versions).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM rule_versions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn save_rejects_invalid_source() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();
        let managed = d.managed.join("r1.md");
        let target = d.target.join("CLAUDE.md");
        seed_rule(
            &conn,
            "r1",
            &managed,
            &target,
            &hash_content(""),
            SyncStatus::TargetMissing,
        );

        let err = save(&conn, "r1", "x", Some("bogus"), &d.versions).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn save_enforces_twenty_version_cap_discarding_oldest() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();

        let managed = d.managed.join("r1.md");
        let target = d.target.join("CLAUDE.md");
        seed_rule(
            &conn,
            "r1",
            &managed,
            &target,
            &hash_content(""),
            SyncStatus::TargetMissing,
        );

        // 25 saves -> versions 1..=25, only the 20 most-recent retained.
        let mut last = RuleFileContent {
            id: String::new(),
            platform_id: String::new(),
            platform_name: String::new(),
            platform_icon: String::new(),
            platform_description: String::new(),
            name: String::new(),
            description: String::new(),
            path: String::new(),
            exists: false,
            managed_path: None,
            target_path: None,
            project_root_path: None,
            sync_status: None,
            content: String::new(),
            versions: Vec::new(),
        };
        for i in 1..=25 {
            last = save(&conn, "r1", &format!("body {i}"), None, &d.versions).unwrap();
        }

        // Exactly 20 retained.
        assert_eq!(last.versions.len(), 20);
        // Most-recent first: newest content is "body 25".
        assert_eq!(last.versions[0].content, "body 25");
        assert_eq!(last.versions[19].content, "body 6");

        // The discarded oldest version files were removed from disk.
        let dropped = d.versions.join("r1").join("0001.md");
        assert!(
            !dropped.exists(),
            "oldest snapshot file should be discarded"
        );
        // A retained snapshot file still exists.
        let kept = d.versions.join("r1").join("0025.md");
        assert!(kept.exists());

        // DB also holds exactly 20 rows.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM rule_versions WHERE rule_id = 'r1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 20);
    }

    // --- delete_version (14.7, 14.9) ---------------------------------------

    #[test]
    fn delete_version_removes_and_returns_remaining_most_recent_first() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();

        let managed = d.managed.join("r1.md");
        let target = d.target.join("CLAUDE.md");
        seed_rule(
            &conn,
            "r1",
            &managed,
            &target,
            &hash_content(""),
            SyncStatus::TargetMissing,
        );

        for i in 1..=3 {
            save(&conn, "r1", &format!("body {i}"), None, &d.versions).unwrap();
        }
        let before = read(&conn, "r1").unwrap().versions;
        assert_eq!(before.len(), 3);
        // Most-recent first.
        assert_eq!(before[0].content, "body 3");

        // Delete the middle snapshot (content "body 2").
        let middle_id = before[1].id.clone();
        let remaining = delete_version(&conn, "r1", &middle_id).unwrap();

        assert_eq!(remaining.len(), 2);
        // Still most-recent first, and the deleted one is gone.
        assert_eq!(remaining[0].content, "body 3");
        assert_eq!(remaining[1].content, "body 1");
        assert!(!remaining.iter().any(|v| v.id == middle_id));
    }

    #[test]
    fn delete_version_unknown_rule_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = delete_version(&conn, "nope", "v1").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn delete_version_absent_version_is_noop() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();
        let managed = d.managed.join("r1.md");
        let target = d.target.join("CLAUDE.md");
        seed_rule(
            &conn,
            "r1",
            &managed,
            &target,
            &hash_content(""),
            SyncStatus::TargetMissing,
        );
        save(&conn, "r1", "body 1", None, &d.versions).unwrap();

        let remaining = delete_version(&conn, "r1", "does-not-exist").unwrap();
        assert_eq!(remaining.len(), 1);
    }

    // --- add_project / remove_project (14.5, 14.6) -------------------------

    #[test]
    fn add_project_registers_and_returns_descriptor() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();

        let root = d._tmp.path().join("my-project");
        let input = AddProjectInput {
            name: "My Project".into(),
            project_root_path: root.to_string_lossy().to_string(),
            initial_content: Some("# Project rules".into()),
            ..Default::default()
        };

        let created = add_project(&conn, input, &d.managed, &d.versions).unwrap();
        assert!(created.id.starts_with("project:"));
        assert_eq!(created.platform_id, "workspace");
        assert_eq!(created.content, "# Project rules");
        // An initial `create` snapshot was recorded.
        assert_eq!(created.versions.len(), 1);
        assert_eq!(created.versions[0].source, "create");

        // It now appears in the list.
        assert_eq!(list(&conn).unwrap().len(), 1);
    }

    #[test]
    fn add_project_rejects_empty_name_and_root() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();

        let err = add_project(
            &conn,
            AddProjectInput {
                name: "  ".into(),
                project_root_path: "/some/path".into(),
                ..Default::default()
            },
            &d.managed,
            &d.versions,
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);

        let err = add_project(
            &conn,
            AddProjectInput {
                name: "Name".into(),
                project_root_path: "   ".into(),
                ..Default::default()
            },
            &d.managed,
            &d.versions,
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn add_project_rejects_duplicate_root() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();
        let root = d._tmp.path().join("dup");

        let mk = || AddProjectInput {
            name: "P".into(),
            project_root_path: root.to_string_lossy().to_string(),
            ..Default::default()
        };
        add_project(&conn, mk(), &d.managed, &d.versions).unwrap();
        let err = add_project(&conn, mk(), &d.managed, &d.versions).unwrap_err();
        assert_eq!(err.code, ErrorCode::Conflict);
    }

    #[test]
    fn remove_project_removes_then_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();
        let root = d._tmp.path().join("proj");

        let created = add_project(
            &conn,
            AddProjectInput {
                name: "P".into(),
                project_root_path: root.to_string_lossy().to_string(),
                initial_content: Some("seed".into()),
                ..Default::default()
            },
            &d.managed,
            &d.versions,
        )
        .unwrap();

        remove_project(&conn, &created.id).unwrap();
        // Rule and its versions are gone (cascade).
        assert!(list(&conn).unwrap().is_empty());
        let vcount: i64 = conn
            .query_row("SELECT COUNT(*) FROM rule_versions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(vcount, 0);

        // Removing again is NOT_FOUND.
        let err = remove_project(&conn, &created.id).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }
}
