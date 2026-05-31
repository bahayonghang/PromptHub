//! Skill_Service: skill CRUD and version-history operations (Requirement 9).
//!
//! This module owns the create/read/list/update/delete business rules for
//! skills plus their version history (list/create/rollback/delete). Like the
//! Prompt_Service and Version_Service, every function is written against a
//! borrowed [`rusqlite::Connection`] rather than reaching into global
//! [`crate::state::AppState`], so the rules are directly unit-testable with an
//! in-memory pool (`storage::create_memory_pool` + `storage::init_schema`) and
//! the Command_Layer (task 17.1) can hand them a pooled connection.
//!
//! ## Validation (no mutation on error — Req 2.3)
//!
//! [`create`] and [`update`] validate a non-empty `name` (trimmed) *before* any
//! database write, so a rejected request never mutates persistent data
//! (Req 9.11). When `protocol_type`/`category` are omitted on create they fall
//! back to the schema defaults `skill`/`general`.
//!
//! ## Partial update strategy (Req 9.4)
//!
//! [`update`] uses a typed [`SkillUpdate`] patch where each field is an
//! `Option<T>`: `Some` replaces the field, `None` leaves it unchanged. The
//! implementation reads the existing skill (returning `NOT_FOUND` when absent),
//! overlays the supplied fields, refreshes `updatedAt`, and writes the full row
//! back. Because it starts from the stored values, unsupplied fields are
//! preserved by construction. Nullable text fields cannot be reset to NULL
//! through this patch shape; that is an accepted limitation for this task's
//! scope (mirroring the Prompt_Service patch shape).
//!
//! ## Versioning (Req 9.6–9.10)
//!
//! [`version_create`] snapshots the skill's *current* `content` and file set
//! (`files_snapshot`) and assigns a version number equal to the skill's highest
//! existing version plus 1 (starting at 1). The skill's multi-file local
//! repository is wired by a later task (8.3); until then there is no file set on
//! the skill record, so the snapshot captures `content` and stores
//! `files_snapshot` as NULL. The snapshot insert and the
//! `skills.current_version` bump run inside a single transaction (Req 4.8).
//! [`version_rollback`] restores the skill's `content` from a snapshot and
//! refreshes `updatedAt`; restoring the file set into the local repository is
//! part of the later local-repo sync path, so this task restores the persisted
//! `content` field.
//!
//! Timestamps are stored as epoch milliseconds and read back as ISO_8601 strings
//! through [`crate::storage::mapping`] (Requirement 4.9).
#![allow(dead_code)]

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::{Skill, SkillVersion};
use crate::storage::mapping::{skill_from_row, skill_version_from_row};
use crate::storage::time::now_millis;

/// Arguments for creating a skill (Req 9.1).
///
/// `name` is required (non-empty after trimming); every other field is optional.
/// `protocol_type` defaults to `skill` and `category` defaults to `general` when
/// omitted, matching the schema defaults. The safety fields (`safetyLevel`,
/// `safetyScore`, ...) are owned by the safety-scanning path (task 8.5) and are
/// not accepted here, so they start NULL.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SkillCreate {
    /// Skill name (required, non-empty after trimming).
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// SKILL.md content / instructions.
    pub content: Option<String>,
    /// Protocol type; defaults to `skill` when omitted.
    pub protocol_type: Option<String>,
    /// Optional skill version label.
    pub version: Option<String>,
    /// Optional author.
    pub author: Option<String>,
    /// Free-form tags.
    pub tags: Option<Vec<String>>,
    /// Favorite flag (default `false`).
    pub is_favorite: Option<bool>,
    /// Source URL when imported.
    pub source_url: Option<String>,
    /// Stable source identity for same-name variants.
    pub source_id: Option<String>,
    /// Human-readable source label.
    pub source_label: Option<String>,
    /// Source branch when imported from a git-like store.
    pub source_branch: Option<String>,
    /// Source directory when imported from a nested store.
    pub source_directory: Option<String>,
    /// Canonical skill path inside the source.
    pub canonical_skill_path: Option<String>,
    /// Absolute path to the local repository directory.
    pub local_repo_path: Option<String>,
    /// Stable fingerprint of the full skill directory.
    pub directory_fingerprint: Option<String>,
    /// Skill icon URL.
    pub icon_url: Option<String>,
    /// Emoji icon fallback.
    pub icon_emoji: Option<String>,
    /// Icon background color.
    pub icon_background: Option<String>,
    /// Skill category; defaults to `general` when omitted.
    pub category: Option<String>,
    /// Whether this is a built-in skill (default `false`).
    pub is_builtin: Option<bool>,
    /// Unique slug in the registry.
    pub registry_slug: Option<String>,
    /// Remote SKILL.md URL.
    pub content_url: Option<String>,
    /// Whether version tracking is enabled (default `false`).
    pub version_tracking_enabled: Option<bool>,
}

/// Partial-update patch for a skill (Req 9.4).
///
/// Each `Some` field replaces the stored value; each `None` field is left
/// unchanged. A supplied `name` is validated (non-empty after trimming) before
/// any write (Req 9.11).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SkillUpdate {
    /// Replacement name (validated non-empty when supplied).
    pub name: Option<String>,
    /// Replacement description.
    pub description: Option<String>,
    /// Replacement content.
    pub content: Option<String>,
    /// Replacement protocol type.
    pub protocol_type: Option<String>,
    /// Replacement version label.
    pub version: Option<String>,
    /// Replacement author.
    pub author: Option<String>,
    /// Replacement tags.
    pub tags: Option<Vec<String>>,
    /// Replacement favorite flag.
    pub is_favorite: Option<bool>,
    /// Replacement source URL.
    pub source_url: Option<String>,
    /// Replacement source identity.
    pub source_id: Option<String>,
    /// Replacement source label.
    pub source_label: Option<String>,
    /// Replacement source branch.
    pub source_branch: Option<String>,
    /// Replacement source directory.
    pub source_directory: Option<String>,
    /// Replacement canonical skill path.
    pub canonical_skill_path: Option<String>,
    /// Replacement local repository path.
    pub local_repo_path: Option<String>,
    /// Replacement directory fingerprint.
    pub directory_fingerprint: Option<String>,
    /// Replacement icon URL.
    pub icon_url: Option<String>,
    /// Replacement emoji icon.
    pub icon_emoji: Option<String>,
    /// Replacement icon background.
    pub icon_background: Option<String>,
    /// Replacement category.
    pub category: Option<String>,
    /// Replacement built-in flag.
    pub is_builtin: Option<bool>,
    /// Replacement registry slug.
    pub registry_slug: Option<String>,
    /// Replacement remote content URL.
    pub content_url: Option<String>,
    /// Replacement version-tracking flag.
    pub version_tracking_enabled: Option<bool>,
}

/// Maps a raw rusqlite error into an `INTERNAL` [`AppError`].
fn db_err(context: &str, e: rusqlite::Error) -> AppError {
    AppError::internal(format!("{context}: {e}"))
}

/// Serializes a slice to a JSON array TEXT column value (`[]` when empty).
fn json_array<T: Serialize>(items: &[T]) -> String {
    serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string())
}

/// Creates a skill and returns the stored record (Req 9.1).
///
/// Validates a non-empty `name` (Req 9.11) before writing. Generates a UUID
/// identifier, sets `createdAt` equal to `updatedAt` at creation (Req 9.1),
/// applies the schema defaults `skill`/`general` for `protocolType`/`category`
/// when omitted, and persists the supplied metadata fields.
pub fn create(conn: &Connection, input: SkillCreate) -> Result<Skill, AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("name is required"));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();
    let tags = json_array(&input.tags.unwrap_or_default());
    let protocol_type = input.protocol_type.unwrap_or_else(|| "skill".to_string());
    let category = input.category.unwrap_or_else(|| "general".to_string());

    conn.execute(
        "INSERT INTO skills \
         (id, name, description, content, protocol_type, version, author, tags, is_favorite, \
          source_url, source_id, source_label, source_branch, source_directory, \
          canonical_skill_path, local_repo_path, directory_fingerprint, icon_url, icon_emoji, \
          icon_background, category, is_builtin, registry_slug, content_url, current_version, \
          version_tracking_enabled, created_at, updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,\
                 ?23,?24,?25,?26,?27,?28)",
        params![
            id,
            input.name,
            input.description,
            input.content,
            protocol_type,
            input.version,
            input.author,
            tags,
            input.is_favorite.unwrap_or(false),
            input.source_url,
            input.source_id,
            input.source_label,
            input.source_branch,
            input.source_directory,
            input.canonical_skill_path,
            input.local_repo_path,
            input.directory_fingerprint,
            input.icon_url,
            input.icon_emoji,
            input.icon_background,
            category,
            input.is_builtin.unwrap_or(false),
            input.registry_slug,
            input.content_url,
            0_i64,
            input.version_tracking_enabled.unwrap_or(false),
            now,
            now,
        ],
    )
    .map_err(|e| db_err("failed to insert skill", e))?;

    get(conn, &id)
}

/// Fetches a skill by identifier (Req 9.2), returning `NOT_FOUND` when absent
/// (Req 9.10).
pub fn get(conn: &Connection, id: &str) -> Result<Skill, AppError> {
    conn.query_row("SELECT * FROM skills WHERE id = ?1", [id], skill_from_row)
        .optional()
        .map_err(|e| db_err("failed to read skill", e))?
        .ok_or_else(|| AppError::not_found(format!("skill `{id}` not found")))
}

/// Returns all stored skills (Req 9.3), or an empty vector when none exist.
///
/// Ordered by creation time ascending for a stable, intuitive listing.
pub fn list(conn: &Connection) -> Result<Vec<Skill>, AppError> {
    let mut stmt = conn
        .prepare("SELECT * FROM skills ORDER BY created_at ASC, id ASC")
        .map_err(|e| db_err("failed to prepare skill list", e))?;
    let rows = stmt
        .query_map([], skill_from_row)
        .map_err(|e| db_err("failed to query skills", e))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| db_err("failed to map skill rows", e))
}

/// Applies a partial update to an existing skill and returns it (Req 9.4).
///
/// Supplied fields replace their stored values; unsupplied fields are preserved.
/// `updatedAt` is always refreshed to the current time. Returns `NOT_FOUND` when
/// the skill does not exist (Req 9.10); a supplied empty `name` is rejected with
/// `VALIDATION` before any write (Req 9.11).
pub fn update(conn: &Connection, id: &str, patch: SkillUpdate) -> Result<Skill, AppError> {
    // Validate the optional name before touching the database so a rejected
    // request never mutates stored data (Req 2.3, 9.11).
    if let Some(name) = patch.name.as_deref() {
        if name.trim().is_empty() {
            return Err(AppError::validation("name is required"));
        }
    }

    // NOT_FOUND when the skill does not exist; also the base for preserved fields.
    let existing = get(conn, id)?;

    let name = patch.name.unwrap_or(existing.name);
    let description = patch.description.or(existing.description);
    let content = patch.content.or(existing.content);
    let protocol_type = patch.protocol_type.unwrap_or(existing.protocol_type);
    let version = patch.version.or(existing.version);
    let author = patch.author.or(existing.author);
    let tags = json_array(&patch.tags.unwrap_or(existing.tags));
    let is_favorite = patch.is_favorite.unwrap_or(existing.is_favorite);
    let source_url = patch.source_url.or(existing.source_url);
    let source_id = patch.source_id.or(existing.source_id);
    let source_label = patch.source_label.or(existing.source_label);
    let source_branch = patch.source_branch.or(existing.source_branch);
    let source_directory = patch.source_directory.or(existing.source_directory);
    let canonical_skill_path = patch.canonical_skill_path.or(existing.canonical_skill_path);
    let local_repo_path = patch.local_repo_path.or(existing.local_repo_path);
    let directory_fingerprint = patch
        .directory_fingerprint
        .or(existing.directory_fingerprint);
    let icon_url = patch.icon_url.or(existing.icon_url);
    let icon_emoji = patch.icon_emoji.or(existing.icon_emoji);
    let icon_background = patch.icon_background.or(existing.icon_background);
    let category = patch.category.unwrap_or(existing.category);
    let is_builtin = patch.is_builtin.unwrap_or(existing.is_builtin);
    let registry_slug = patch.registry_slug.or(existing.registry_slug);
    let content_url = patch.content_url.or(existing.content_url);
    let version_tracking_enabled = patch
        .version_tracking_enabled
        .unwrap_or(existing.version_tracking_enabled);
    let now = now_millis();

    conn.execute(
        "UPDATE skills SET \
         name=?1, description=?2, content=?3, protocol_type=?4, version=?5, author=?6, tags=?7, \
         is_favorite=?8, source_url=?9, source_id=?10, source_label=?11, source_branch=?12, \
         source_directory=?13, canonical_skill_path=?14, local_repo_path=?15, \
         directory_fingerprint=?16, icon_url=?17, icon_emoji=?18, icon_background=?19, \
         category=?20, is_builtin=?21, registry_slug=?22, content_url=?23, \
         version_tracking_enabled=?24, updated_at=?25 \
         WHERE id=?26",
        params![
            name,
            description,
            content,
            protocol_type,
            version,
            author,
            tags,
            is_favorite,
            source_url,
            source_id,
            source_label,
            source_branch,
            source_directory,
            canonical_skill_path,
            local_repo_path,
            directory_fingerprint,
            icon_url,
            icon_emoji,
            icon_background,
            category,
            is_builtin,
            registry_slug,
            content_url,
            version_tracking_enabled,
            now,
            id,
        ],
    )
    .map_err(|e| db_err("failed to update skill", e))?;

    get(conn, id)
}

/// Deletes a skill by identifier (Req 9.5).
///
/// The `ON DELETE CASCADE` foreign key on `skill_versions` removes the skill's
/// version history as part of the same delete (Req 4.5). Returns `NOT_FOUND`
/// when the skill does not exist (Req 9.10).
pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    let affected = conn
        .execute("DELETE FROM skills WHERE id = ?1", [id])
        .map_err(|e| db_err("failed to delete skill", e))?;
    if affected == 0 {
        return Err(AppError::not_found(format!("skill `{id}` not found")));
    }
    Ok(())
}

// --- version history (Req 9.6–9.10) ----------------------------------------

/// Fetches the owning skill by identifier, returning `NOT_FOUND` when absent.
///
/// Used by the version operations to enforce that version history is always
/// scoped to an existing skill (Req 9.10).
fn get_skill(conn: &Connection, skill_id: &str) -> Result<Skill, AppError> {
    get(conn, skill_id)
}

/// Returns a skill's versions ordered by version number ascending (Req 9.6).
///
/// Returns `NOT_FOUND` when the skill does not exist (Req 9.10, consistent with
/// the Version_Service); returns an empty vector when the skill exists but has
/// no versions yet.
pub fn version_list(conn: &Connection, skill_id: &str) -> Result<Vec<SkillVersion>, AppError> {
    // Distinguish "no such skill" (NOT_FOUND) from "skill has no versions"
    // (empty list) by checking the skill exists first.
    get_skill(conn, skill_id)?;

    let mut stmt = conn
        .prepare("SELECT * FROM skill_versions WHERE skill_id = ?1 ORDER BY version ASC")
        .map_err(|e| db_err("failed to prepare skill version list", e))?;
    let rows = stmt
        .query_map([skill_id], skill_version_from_row)
        .map_err(|e| db_err("failed to query skill versions", e))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| db_err("failed to map skill version rows", e))
}

/// Creates a version snapshot of the skill's current state (Req 9.7, 9.10).
///
/// Returns `NOT_FOUND` when the skill does not exist (Req 9.10). The new version
/// number is the skill's highest existing version plus 1, starting at 1. The
/// snapshot captures the skill's current `content`; the multi-file `files_snapshot`
/// is left NULL until the local-repo sync path (task 8.3) populates a file set on
/// the skill record. The snapshot insert and the `skills.current_version` bump run
/// in a single transaction (Req 4.8).
pub fn version_create(
    conn: &Connection,
    skill_id: &str,
    note: Option<String>,
) -> Result<SkillVersion, AppError> {
    // NOT_FOUND when the skill does not exist; also the source of the snapshot.
    let skill = get_skill(conn, skill_id)?;

    // version = highest existing version for this skill + 1, starting at 1.
    let max_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM skill_versions WHERE skill_id = ?1",
            [skill_id],
            |row| row.get(0),
        )
        .map_err(|e| db_err("failed to compute next skill version", e))?;
    let next_version = max_version + 1;

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| db_err("failed to begin create-skill-version transaction", e))?;
    tx.execute(
        "INSERT INTO skill_versions (id, skill_id, version, content, files_snapshot, note, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            id,
            skill_id,
            next_version,
            skill.content,
            // No file set is available on the skill record yet (task 8.3), so the
            // multi-file snapshot is stored as NULL.
            None::<String>,
            note,
            now,
        ],
    )
    .map_err(|e| db_err("failed to insert skill version", e))?;
    // Keep the skill's highest-version pointer consistent with the new snapshot.
    tx.execute(
        "UPDATE skills SET current_version = ?1 WHERE id = ?2",
        params![next_version, skill_id],
    )
    .map_err(|e| db_err("failed to bump skill current_version", e))?;
    tx.commit()
        .map_err(|e| db_err("failed to commit create-skill-version transaction", e))?;

    conn.query_row(
        "SELECT * FROM skill_versions WHERE id = ?1",
        [id],
        skill_version_from_row,
    )
    .map_err(|e| db_err("failed to read created skill version", e))
}

/// Restores a skill to a previous version's snapshot (Req 9.8, 9.10).
///
/// Returns `NOT_FOUND`, leaving the skill unchanged, when either the skill or the
/// requested version does not exist (Req 9.10). On success, the skill's `content`
/// is set to the version's snapshot and `updatedAt` is refreshed to the current
/// time; the updated skill is returned. The version's `files_snapshot` is the
/// source of truth for the file set restored into the local repository by the
/// later local-repo sync path (task 8.3); the persisted skill field restored here
/// is `content`.
pub fn version_rollback(
    conn: &Connection,
    skill_id: &str,
    version: i64,
) -> Result<Skill, AppError> {
    // NOT_FOUND when the skill does not exist (skill left unchanged).
    get_skill(conn, skill_id)?;

    // NOT_FOUND when the skill has no such version (skill left unchanged).
    let snapshot = conn
        .query_row(
            "SELECT * FROM skill_versions WHERE skill_id = ?1 AND version = ?2",
            params![skill_id, version],
            skill_version_from_row,
        )
        .optional()
        .map_err(|e| db_err("failed to read skill version", e))?
        .ok_or_else(|| {
            AppError::not_found(format!(
                "version `{version}` not found for skill `{skill_id}`"
            ))
        })?;

    let now = now_millis();
    conn.execute(
        "UPDATE skills SET content = ?1, updated_at = ?2 WHERE id = ?3",
        params![snapshot.content, now, skill_id],
    )
    .map_err(|e| db_err("failed to roll back skill", e))?;

    get_skill(conn, skill_id)
}

/// Removes a single version from a skill's history (Req 9.9, 9.10).
///
/// Returns `NOT_FOUND` when no matching `(skill_id, version)` row exists, leaving
/// the version history unchanged (Req 9.10). All other versions are left intact.
/// The skill's `current_version` pointer is intentionally left untouched so
/// deleting an arbitrary version does not rewrite the skill record (mirroring the
/// Version_Service).
pub fn version_delete(conn: &Connection, skill_id: &str, version: i64) -> Result<(), AppError> {
    let affected = conn
        .execute(
            "DELETE FROM skill_versions WHERE skill_id = ?1 AND version = ?2",
            params![skill_id, version],
        )
        .map_err(|e| db_err("failed to delete skill version", e))?;
    if affected == 0 {
        return Err(AppError::not_found(format!(
            "version `{version}` not found for skill `{skill_id}`"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::storage::{create_memory_pool, init_schema, DbPool};

    /// Builds an in-memory pool with the schema initialized.
    fn schema_pool() -> DbPool {
        let pool = create_memory_pool().expect("memory pool");
        init_schema(&pool.get().expect("conn")).expect("schema");
        pool
    }

    fn sample_create() -> SkillCreate {
        SkillCreate {
            name: "My Skill".into(),
            description: Some("does things".into()),
            content: Some("# Body".into()),
            tags: Some(vec!["a".into(), "b".into()]),
            author: Some("alice".into()),
            is_favorite: Some(true),
            ..Default::default()
        }
    }

    // --- create + get (Req 9.1, 9.2, 9.11) ---------------------------------

    #[test]
    fn create_then_get_round_trips_fields_and_defaults() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let created = create(&conn, sample_create()).unwrap();

        // Generated id is non-empty; timestamps equal at creation (Req 9.1).
        assert!(!created.id.is_empty());
        assert_eq!(created.created_at, created.updated_at);
        assert!(created.created_at.ends_with('Z'));

        // Supplied fields persisted.
        assert_eq!(created.name, "My Skill");
        assert_eq!(created.description.as_deref(), Some("does things"));
        assert_eq!(created.content.as_deref(), Some("# Body"));
        assert_eq!(created.tags, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(created.author.as_deref(), Some("alice"));
        assert!(created.is_favorite);

        // Schema defaults applied when omitted (Req 9.1).
        assert_eq!(created.protocol_type, "skill");
        assert_eq!(created.category, "general");
        assert_eq!(created.current_version, 0);
        assert!(!created.is_builtin);
        assert!(!created.version_tracking_enabled);

        // get returns the same record.
        let fetched = get(&conn, &created.id).unwrap();
        assert_eq!(fetched, created);
    }

    #[test]
    fn create_applies_supplied_protocol_type_and_category() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let created = create(
            &conn,
            SkillCreate {
                name: "Custom".into(),
                protocol_type: Some("agent".into()),
                category: Some("coding".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(created.protocol_type, "agent");
        assert_eq!(created.category, "coding");
    }

    #[test]
    fn create_rejects_empty_name_without_mutating() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let err = create(
            &conn,
            SkillCreate {
                name: "   ".into(),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);

        // No skill was created (Req 9.11).
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn get_missing_skill_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = get(&conn, "nope").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    // --- list (Req 9.3) ----------------------------------------------------

    #[test]
    fn list_returns_empty_then_all() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        assert!(list(&conn).unwrap().is_empty());

        let a = create(
            &conn,
            SkillCreate {
                name: "A".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let b = create(
            &conn,
            SkillCreate {
                name: "B".into(),
                ..Default::default()
            },
        )
        .unwrap();

        let ids: Vec<String> = list(&conn).unwrap().into_iter().map(|s| s.id).collect();
        assert!(ids.contains(&a.id));
        assert!(ids.contains(&b.id));
        assert_eq!(ids.len(), 2);
    }

    // --- update (Req 9.4, 9.10, 9.11) --------------------------------------

    #[test]
    fn update_partial_patch_preserves_fields_and_bumps_updated_at() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();

        // Force a strictly earlier updated_at so the refresh is observable.
        conn.execute(
            "UPDATE skills SET updated_at = updated_at - 1000 WHERE id = ?1",
            params![created.id],
        )
        .unwrap();
        let baseline = get(&conn, &created.id).unwrap();

        let updated = update(
            &conn,
            &created.id,
            SkillUpdate {
                content: Some("# New Body".into()),
                ..Default::default()
            },
        )
        .unwrap();

        // Supplied field changed.
        assert_eq!(updated.content.as_deref(), Some("# New Body"));
        // Unsupplied fields preserved (Req 9.4).
        assert_eq!(updated.name, baseline.name);
        assert_eq!(updated.description, baseline.description);
        assert_eq!(updated.tags, baseline.tags);
        assert_eq!(updated.author, baseline.author);
        assert_eq!(updated.is_favorite, baseline.is_favorite);
        // createdAt unchanged; updatedAt advanced.
        assert_eq!(updated.created_at, baseline.created_at);
        assert!(
            updated.updated_at > baseline.updated_at,
            "updatedAt should advance: {} !> {}",
            updated.updated_at,
            baseline.updated_at
        );
    }

    #[test]
    fn update_rejects_empty_name_without_mutating() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();
        let before = get(&conn, &created.id).unwrap();

        let err = update(
            &conn,
            &created.id,
            SkillUpdate {
                name: Some("  ".into()),
                content: Some("should not persist".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);

        // The skill is untouched (Req 9.11 / 2.3).
        assert_eq!(get(&conn, &created.id).unwrap(), before);
    }

    #[test]
    fn update_missing_skill_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = update(
            &conn,
            "nope",
            SkillUpdate {
                content: Some("x".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    // --- delete + cascade (Req 9.5, 9.10, 4.5) -----------------------------

    #[test]
    fn delete_removes_skill_and_cascades_versions() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();

        version_create(&conn, &created.id, None).unwrap();
        version_create(&conn, &created.id, None).unwrap();

        // Sanity: versions exist before delete.
        let count_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_versions WHERE skill_id = ?1",
                [&created.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_before, 2);

        delete(&conn, &created.id).unwrap();

        // Skill gone.
        assert_eq!(
            get(&conn, &created.id).unwrap_err().code,
            ErrorCode::NotFound
        );
        // Versions cascade-deleted (Req 4.5).
        let count_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_versions WHERE skill_id = ?1",
                [&created.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_after, 0);
    }

    #[test]
    fn delete_missing_skill_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = delete(&conn, "nope").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    // --- version_list (Req 9.6, 9.10) --------------------------------------

    #[test]
    fn version_list_empty_when_skill_has_no_versions() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();
        assert!(version_list(&conn, &created.id).unwrap().is_empty());
    }

    #[test]
    fn version_list_missing_skill_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = version_list(&conn, "nope").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn version_list_orders_ascending() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();

        version_create(&conn, &created.id, None).unwrap();
        version_create(&conn, &created.id, None).unwrap();
        version_create(&conn, &created.id, None).unwrap();

        let numbers: Vec<i64> = version_list(&conn, &created.id)
            .unwrap()
            .iter()
            .map(|v| v.version)
            .collect();
        assert_eq!(numbers, vec![1, 2, 3]);
    }

    // --- version_create (Req 9.7, 9.10) ------------------------------------

    #[test]
    fn version_create_assigns_monotonic_versions_and_snapshots_content() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();

        let first = version_create(&conn, &created.id, Some("first".into())).unwrap();
        assert_eq!(first.version, 1);
        assert_eq!(first.content.as_deref(), Some("# Body"));
        assert_eq!(first.note.as_deref(), Some("first"));
        assert_eq!(first.skill_id, created.id);
        // No file set is wired yet, so files_snapshot is NULL/None.
        assert_eq!(first.files_snapshot, None);

        // Mutate content, then version 2 captures the *current* content.
        update(
            &conn,
            &created.id,
            SkillUpdate {
                content: Some("# Body v2".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let second = version_create(&conn, &created.id, None).unwrap();
        assert_eq!(second.version, 2);
        assert_eq!(second.content.as_deref(), Some("# Body v2"));

        // The skill's current_version pointer tracks the latest snapshot.
        assert_eq!(get(&conn, &created.id).unwrap().current_version, 2);
    }

    #[test]
    fn version_create_missing_skill_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = version_create(&conn, "nope", None).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    // --- version_rollback (Req 9.8, 9.10) ----------------------------------

    #[test]
    fn version_rollback_restores_snapshot_and_bumps_updated_at() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();

        // Snapshot original content as version 1.
        version_create(&conn, &created.id, None).unwrap();

        // Mutate the skill away from the snapshot.
        update(
            &conn,
            &created.id,
            SkillUpdate {
                content: Some("# Body v2".into()),
                ..Default::default()
            },
        )
        .unwrap();

        // Force a strictly earlier updated_at so the rollback's refresh is visible.
        conn.execute(
            "UPDATE skills SET updated_at = updated_at - 1000 WHERE id = ?1",
            params![created.id],
        )
        .unwrap();
        let baseline = get(&conn, &created.id).unwrap();

        let restored = version_rollback(&conn, &created.id, 1).unwrap();
        assert_eq!(restored.content.as_deref(), Some("# Body"));
        assert!(
            restored.updated_at > baseline.updated_at,
            "updatedAt should advance: {} !> {}",
            restored.updated_at,
            baseline.updated_at
        );
    }

    #[test]
    fn version_rollback_unknown_version_returns_not_found_and_leaves_skill_unchanged() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();
        let before = get(&conn, &created.id).unwrap();

        let err = version_rollback(&conn, &created.id, 99).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);

        // The skill is untouched (Req 9.10).
        assert_eq!(get(&conn, &created.id).unwrap(), before);
    }

    #[test]
    fn version_rollback_missing_skill_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = version_rollback(&conn, "nope", 1).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    // --- version_delete (Req 9.9, 9.10) ------------------------------------

    #[test]
    fn version_delete_removes_one_leaving_others_intact() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();

        version_create(&conn, &created.id, None).unwrap(); // version 1
        version_create(&conn, &created.id, None).unwrap(); // version 2
        version_create(&conn, &created.id, None).unwrap(); // version 3

        version_delete(&conn, &created.id, 2).unwrap();

        let remaining: Vec<i64> = version_list(&conn, &created.id)
            .unwrap()
            .iter()
            .map(|v| v.version)
            .collect();
        assert_eq!(remaining, vec![1, 3]);
    }

    #[test]
    fn version_delete_unknown_version_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();
        version_create(&conn, &created.id, None).unwrap();

        let err = version_delete(&conn, &created.id, 99).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);

        // The existing version is untouched (Req 9.10).
        let remaining: Vec<i64> = version_list(&conn, &created.id)
            .unwrap()
            .iter()
            .map(|v| v.version)
            .collect();
        assert_eq!(remaining, vec![1]);
    }
}
