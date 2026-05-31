//! Version_Service: prompt version-history operations (Requirement 7).
//!
//! This module owns the list/create/rollback/delete business rules for a
//! prompt's version history. Like the Prompt_Service, every function is written
//! against a borrowed [`rusqlite::Connection`] rather than reaching into global
//! [`crate::state::AppState`], so the rules are directly unit-testable with an
//! in-memory pool (`storage::create_memory_pool` + `storage::init_schema`) and
//! the Command_Layer can hand them a pooled connection.
//!
//! ## Existence semantics (`NOT_FOUND`)
//!
//! Every operation is scoped to an owning prompt. [`list`] and [`create`] return
//! `NOT_FOUND` when the prompt itself does not exist; [`rollback`] returns
//! `NOT_FOUND` when either the prompt or the requested version is missing; and
//! [`delete`] returns `NOT_FOUND` when no matching `(prompt_id, version)` row
//! exists (Req 7.5, 7.6, 7.7). A prompt that exists but has no versions yet is
//! *not* an error — [`list`] returns an empty vector for it (Req 7.1).
//!
//! ## Validation before mutation (Req 2.3)
//!
//! [`create`] validates the optional note length (≤1000 characters, Req 7.8)
//! *before* any database write, so a rejected request never creates a version.
//!
//! ## Versioning (Req 7.2)
//!
//! [`create`] snapshots the prompt's *current* `systemPrompt`, `userPrompt`, and
//! `variables`, and assigns a version number equal to the prompt's highest
//! existing version plus 1 (starting at 1 for the first version). The two writes
//! (the snapshot insert plus bumping `prompts.current_version` to the new
//! version) run inside a single transaction so a mid-operation failure rolls back
//! fully (Req 4.8). Bumping `current_version` keeps the prompt's
//! highest-version pointer consistent with the inserted snapshot; the snapshot
//! does not otherwise touch the prompt's content or `updatedAt`.
//!
//! Timestamps are stored as epoch milliseconds and read back as ISO_8601 strings
//! through [`crate::storage::mapping`] (Requirement 4.9).
#![allow(dead_code)]

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::AppError;
use crate::models::{Prompt, PromptVersion};
use crate::storage::mapping::{prompt_from_row, prompt_version_from_row};
use crate::storage::time::now_millis;

/// Maximum allowed length, in characters, of a version note (Req 7.8).
const MAX_NOTE_CHARS: usize = 1000;

/// Maps a raw rusqlite error into an `INTERNAL` [`AppError`].
fn db_err(context: &str, e: rusqlite::Error) -> AppError {
    AppError::internal(format!("{context}: {e}"))
}

/// Serializes a slice to a JSON array TEXT column value (`[]` on failure/empty).
fn variables_json(variables: &[crate::models::Variable]) -> String {
    serde_json::to_string(variables).unwrap_or_else(|_| "[]".to_string())
}

/// Fetches the owning prompt by identifier, returning `NOT_FOUND` when absent.
///
/// Used by every operation to enforce that version history is always scoped to an
/// existing prompt (Req 7.5, 7.6) and to read the current snapshot fields.
fn get_prompt(conn: &Connection, prompt_id: &str) -> Result<Prompt, AppError> {
    conn.query_row(
        "SELECT * FROM prompts WHERE id = ?1",
        [prompt_id],
        prompt_from_row,
    )
    .optional()
    .map_err(|e| db_err("failed to read prompt", e))?
    .ok_or_else(|| AppError::not_found(format!("prompt `{prompt_id}` not found")))
}

/// Returns a prompt's versions ordered by version number ascending (Req 7.1).
///
/// Returns `NOT_FOUND` when the prompt does not exist; returns an empty vector
/// when the prompt exists but has no versions yet.
pub fn list(conn: &Connection, prompt_id: &str) -> Result<Vec<PromptVersion>, AppError> {
    // Distinguish "no such prompt" (NOT_FOUND) from "prompt has no versions"
    // (empty list) by checking the prompt exists first.
    get_prompt(conn, prompt_id)?;

    let mut stmt = conn
        .prepare("SELECT * FROM prompt_versions WHERE prompt_id = ?1 ORDER BY version ASC")
        .map_err(|e| db_err("failed to prepare version list", e))?;
    let rows = stmt
        .query_map([prompt_id], prompt_version_from_row)
        .map_err(|e| db_err("failed to query versions", e))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| db_err("failed to map version rows", e))
}

/// Creates a version snapshot of the prompt's current state (Req 7.2, 7.6, 7.8).
///
/// Validates the optional `note` length (≤1000 characters) before any write
/// (Req 7.8). Returns `NOT_FOUND` when the prompt does not exist (Req 7.6). The
/// new version number is the prompt's highest existing version plus 1, starting
/// at 1 for the first version. The snapshot captures the prompt's current
/// `systemPrompt`, `userPrompt`, and `variables`; `aiResponse` is not part of the
/// snapshot. The snapshot insert and the `prompts.current_version` bump run in a
/// single transaction (Req 4.8).
pub fn create(
    conn: &Connection,
    prompt_id: &str,
    note: Option<String>,
) -> Result<PromptVersion, AppError> {
    // Validate before touching the database so a rejected request never creates a
    // version (Req 2.3, 7.8). Count Unicode scalar values, not bytes.
    if let Some(note) = note.as_deref() {
        if note.chars().count() > MAX_NOTE_CHARS {
            return Err(AppError::validation(format!(
                "note exceeds the {MAX_NOTE_CHARS}-character limit"
            )));
        }
    }

    // NOT_FOUND when the prompt does not exist; also the source of the snapshot.
    let prompt = get_prompt(conn, prompt_id)?;

    // version = highest existing version for this prompt + 1, starting at 1.
    let max_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM prompt_versions WHERE prompt_id = ?1",
            [prompt_id],
            |row| row.get(0),
        )
        .map_err(|e| db_err("failed to compute next version", e))?;
    let next_version = max_version + 1;

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();
    let variables = variables_json(&prompt.variables);

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| db_err("failed to begin create-version transaction", e))?;
    tx.execute(
        "INSERT INTO prompt_versions \
         (id, prompt_id, version, system_prompt, user_prompt, variables, note, ai_response, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            id,
            prompt_id,
            next_version,
            prompt.system_prompt,
            prompt.user_prompt,
            variables,
            note,
            None::<String>,
            now,
        ],
    )
    .map_err(|e| db_err("failed to insert version", e))?;
    // Keep the prompt's highest-version pointer consistent with the new snapshot.
    tx.execute(
        "UPDATE prompts SET current_version = ?1 WHERE id = ?2",
        params![next_version, prompt_id],
    )
    .map_err(|e| db_err("failed to bump current_version", e))?;
    tx.commit()
        .map_err(|e| db_err("failed to commit create-version transaction", e))?;

    conn.query_row(
        "SELECT * FROM prompt_versions WHERE id = ?1",
        [id],
        prompt_version_from_row,
    )
    .map_err(|e| db_err("failed to read created version", e))
}

/// Restores a prompt to a previous version's snapshot (Req 7.3, 7.5).
///
/// Returns `NOT_FOUND`, leaving the prompt unchanged, when either the prompt or
/// the requested version does not exist (Req 7.5). On success, the prompt's
/// `systemPrompt`, `userPrompt`, and `variables` are set to the version's
/// snapshot and `updatedAt` is refreshed to the current time; the updated prompt
/// is returned.
pub fn rollback(conn: &Connection, prompt_id: &str, version: i64) -> Result<Prompt, AppError> {
    // NOT_FOUND when the prompt does not exist (prompt left unchanged).
    get_prompt(conn, prompt_id)?;

    // NOT_FOUND when the prompt has no such version (prompt left unchanged).
    let snapshot = conn
        .query_row(
            "SELECT * FROM prompt_versions WHERE prompt_id = ?1 AND version = ?2",
            params![prompt_id, version],
            prompt_version_from_row,
        )
        .optional()
        .map_err(|e| db_err("failed to read version", e))?
        .ok_or_else(|| {
            AppError::not_found(format!(
                "version `{version}` not found for prompt `{prompt_id}`"
            ))
        })?;

    let now = now_millis();
    conn.execute(
        "UPDATE prompts SET system_prompt = ?1, user_prompt = ?2, variables = ?3, updated_at = ?4 \
         WHERE id = ?5",
        params![
            snapshot.system_prompt,
            snapshot.user_prompt,
            variables_json(&snapshot.variables),
            now,
            prompt_id,
        ],
    )
    .map_err(|e| db_err("failed to roll back prompt", e))?;

    get_prompt(conn, prompt_id)
}

/// Removes a single version from a prompt's history (Req 7.4, 7.7).
///
/// Returns `NOT_FOUND` when no matching `(prompt_id, version)` row exists,
/// leaving the version history unchanged (Req 7.7). All other versions are left
/// intact. The prompt's `current_version` pointer is intentionally left untouched
/// so deleting an arbitrary version does not rewrite the prompt record.
pub fn delete(conn: &Connection, prompt_id: &str, version: i64) -> Result<(), AppError> {
    let affected = conn
        .execute(
            "DELETE FROM prompt_versions WHERE prompt_id = ?1 AND version = ?2",
            params![prompt_id, version],
        )
        .map_err(|e| db_err("failed to delete version", e))?;
    if affected == 0 {
        return Err(AppError::not_found(format!(
            "version `{version}` not found for prompt `{prompt_id}`"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::models::Variable;
    use crate::services::prompt::{self, PromptCreate, PromptUpdate};
    use crate::storage::{create_memory_pool, init_schema, DbPool};
    use rusqlite::params;

    /// Builds an in-memory pool with the schema initialized.
    fn schema_pool() -> DbPool {
        let pool = create_memory_pool().expect("memory pool");
        init_schema(&pool.get().expect("conn")).expect("schema");
        pool
    }

    fn var(name: &str) -> Variable {
        Variable {
            name: name.into(),
            r#type: "text".into(),
            label: None,
            default_value: None,
            options: None,
            required: false,
        }
    }

    /// Creates a prompt with known snapshot fields and returns its id.
    fn seed_prompt(conn: &Connection) -> String {
        let created = prompt::create(
            conn,
            PromptCreate {
                title: "Versioned".into(),
                user_prompt: "user v1".into(),
                system_prompt: Some("sys v1".into()),
                variables: Some(vec![var("a")]),
                ..Default::default()
            },
        )
        .expect("create prompt");
        created.id
    }

    #[test]
    fn list_returns_empty_when_prompt_has_no_versions() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = seed_prompt(&conn);
        assert!(list(&conn, &id).unwrap().is_empty());
    }

    #[test]
    fn list_missing_prompt_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = list(&conn, "nope").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn list_orders_versions_ascending() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = seed_prompt(&conn);

        create(&conn, &id, None).unwrap();
        create(&conn, &id, None).unwrap();
        create(&conn, &id, None).unwrap();

        let versions = list(&conn, &id).unwrap();
        let numbers: Vec<i64> = versions.iter().map(|v| v.version).collect();
        assert_eq!(numbers, vec![1, 2, 3]);
    }

    #[test]
    fn create_assigns_monotonic_versions_starting_at_one() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = seed_prompt(&conn);

        let first = create(&conn, &id, None).unwrap();
        let second = create(&conn, &id, None).unwrap();
        assert_eq!(first.version, 1);
        assert_eq!(second.version, 2);

        // The prompt's current_version pointer tracks the latest snapshot.
        assert_eq!(prompt::get(&conn, &id).unwrap().current_version, 2);
    }

    #[test]
    fn create_snapshots_current_prompt_fields() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = seed_prompt(&conn);

        // Version 1 snapshots the original fields.
        let v1 = create(&conn, &id, Some("first".into())).unwrap();
        assert_eq!(v1.system_prompt.as_deref(), Some("sys v1"));
        assert_eq!(v1.user_prompt, "user v1");
        assert_eq!(v1.variables, vec![var("a")]);
        assert_eq!(v1.note.as_deref(), Some("first"));
        assert_eq!(v1.prompt_id, id);

        // Mutate the prompt, then version 2 must capture the *current* fields.
        prompt::update(
            &conn,
            &id,
            PromptUpdate {
                system_prompt: Some("sys v2".into()),
                user_prompt: Some("user v2".into()),
                variables: Some(vec![var("b"), var("c")]),
                ..Default::default()
            },
        )
        .unwrap();

        let v2 = create(&conn, &id, None).unwrap();
        assert_eq!(v2.system_prompt.as_deref(), Some("sys v2"));
        assert_eq!(v2.user_prompt, "user v2");
        assert_eq!(v2.variables, vec![var("b"), var("c")]);
        assert_eq!(v2.note, None);
    }

    #[test]
    fn create_rejects_note_over_limit_without_creating_version() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = seed_prompt(&conn);

        let too_long = "x".repeat(MAX_NOTE_CHARS + 1);
        let err = create(&conn, &id, Some(too_long)).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);

        // No version was created (Req 7.8), and the pointer is untouched.
        assert!(list(&conn, &id).unwrap().is_empty());
        assert_eq!(prompt::get(&conn, &id).unwrap().current_version, 0);
    }

    #[test]
    fn create_accepts_note_at_exactly_the_limit() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = seed_prompt(&conn);

        let at_limit = "y".repeat(MAX_NOTE_CHARS);
        let version = create(&conn, &id, Some(at_limit.clone())).unwrap();
        assert_eq!(version.note, Some(at_limit));
    }

    #[test]
    fn create_missing_prompt_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = create(&conn, "nope", None).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn rollback_restores_snapshot_and_bumps_updated_at() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = seed_prompt(&conn);

        // Snapshot the original state as version 1.
        create(&conn, &id, None).unwrap();

        // Mutate the prompt away from the snapshot.
        prompt::update(
            &conn,
            &id,
            PromptUpdate {
                system_prompt: Some("sys v2".into()),
                user_prompt: Some("user v2".into()),
                variables: Some(vec![var("b")]),
                ..Default::default()
            },
        )
        .unwrap();

        // Force a strictly earlier updated_at so the rollback's refresh is visible.
        conn.execute(
            "UPDATE prompts SET updated_at = updated_at - 1000 WHERE id = ?1",
            params![id],
        )
        .unwrap();
        let baseline = prompt::get(&conn, &id).unwrap();

        let restored = rollback(&conn, &id, 1).unwrap();
        assert_eq!(restored.system_prompt.as_deref(), Some("sys v1"));
        assert_eq!(restored.user_prompt, "user v1");
        assert_eq!(restored.variables, vec![var("a")]);
        assert!(
            restored.updated_at > baseline.updated_at,
            "updatedAt should advance: {} !> {}",
            restored.updated_at,
            baseline.updated_at
        );
    }

    #[test]
    fn rollback_unknown_version_returns_not_found_and_leaves_prompt_unchanged() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = seed_prompt(&conn);
        let before = prompt::get(&conn, &id).unwrap();

        let err = rollback(&conn, &id, 99).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);

        // The prompt is untouched (Req 7.5).
        assert_eq!(prompt::get(&conn, &id).unwrap(), before);
    }

    #[test]
    fn rollback_missing_prompt_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = rollback(&conn, "nope", 1).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn delete_removes_one_version_leaving_others_intact() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = seed_prompt(&conn);

        create(&conn, &id, None).unwrap(); // version 1
        create(&conn, &id, None).unwrap(); // version 2
        create(&conn, &id, None).unwrap(); // version 3

        delete(&conn, &id, 2).unwrap();

        let remaining: Vec<i64> = list(&conn, &id)
            .unwrap()
            .iter()
            .map(|v| v.version)
            .collect();
        assert_eq!(remaining, vec![1, 3]);
    }

    #[test]
    fn delete_unknown_version_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = seed_prompt(&conn);
        create(&conn, &id, None).unwrap();

        let err = delete(&conn, &id, 99).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);

        // The existing version is untouched (Req 7.7).
        let remaining: Vec<i64> = list(&conn, &id)
            .unwrap()
            .iter()
            .map(|v| v.version)
            .collect();
        assert_eq!(remaining, vec![1]);
    }
}
