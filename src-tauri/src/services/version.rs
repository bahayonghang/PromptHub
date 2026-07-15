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
use crate::models::{Prompt, PromptRevisionSource, PromptVersion};
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

fn json_array<T: serde::Serialize>(items: &[T]) -> String {
    serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string())
}

fn prompt_type_wire(prompt_type: crate::models::PromptType) -> &'static str {
    match prompt_type {
        crate::models::PromptType::Text => "text",
        crate::models::PromptType::Image => "image",
        crate::models::PromptType::Video => "video",
    }
}

fn source_wire(source: PromptRevisionSource) -> &'static str {
    match source {
        PromptRevisionSource::Create => "create",
        PromptRevisionSource::Save => "save",
        PromptRevisionSource::Manual => "manual",
        PromptRevisionSource::Rollback => "rollback",
        PromptRevisionSource::Import => "import",
        PromptRevisionSource::Replace => "replace",
    }
}

/// Appends a complete immutable snapshot. The caller owns the surrounding
/// transaction when this is part of a prompt mutation.
pub(crate) fn append_snapshot(
    conn: &Connection,
    prompt: &Prompt,
    note: Option<String>,
    source_action: PromptRevisionSource,
    parent_override: Option<&str>,
) -> Result<PromptVersion, AppError> {
    let latest: Option<(i64, String)> = conn
        .query_row(
            "SELECT version, id FROM prompt_versions WHERE prompt_id = ?1 ORDER BY version DESC LIMIT 1",
            [&prompt.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| db_err("failed to read latest revision", e))?;
    let next_version = latest.as_ref().map_or(1, |(version, _)| version + 1);
    let parent_revision_id = if source_action == PromptRevisionSource::Create {
        None
    } else {
        parent_override
            .map(str::to_owned)
            .or_else(|| latest.map(|(_, id)| id))
    };
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();
    let type_definition = prompt
        .type_definition_id
        .as_deref()
        .map(|id| crate::services::prompt_type::get(conn, id))
        .transpose()?;

    conn.execute(
        "INSERT INTO prompt_versions \
         (id,prompt_id,version,system_prompt,user_prompt,messages,variables,title,description,prompt_type,\
          type_definition_id,type_definition_name,type_definition_base_kind,tags,folder_id,images,videos,\
          is_favorite,is_pinned,is_private,source,notes,note,ai_response,source_action,parent_revision_id,created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27)",
        params![
            id,
            prompt.id,
            next_version,
            prompt.system_prompt,
            prompt.user_prompt,
            json_array(&prompt.messages),
            variables_json(&prompt.variables),
            prompt.title,
            prompt.description,
            prompt_type_wire(prompt.prompt_type),
            prompt.type_definition_id,
            type_definition.as_ref().map(|definition| &definition.name),
            type_definition
                .as_ref()
                .map(|definition| prompt_type_wire(definition.base_kind)),
            json_array(&prompt.tags),
            prompt.folder_id,
            json_array(&prompt.images),
            json_array(&prompt.videos),
            prompt.is_favorite,
            prompt.is_pinned,
            prompt.is_private,
            prompt.source,
            prompt.notes,
            note,
            prompt.last_ai_response,
            source_wire(source_action),
            parent_revision_id,
            now,
        ],
    )
    .map_err(|e| db_err("failed to insert revision", e))?;
    conn.execute(
        "UPDATE prompts SET current_version = ?1 WHERE id = ?2",
        params![next_version, prompt.id],
    )
    .map_err(|e| db_err("failed to update current revision", e))?;

    conn.query_row(
        "SELECT * FROM prompt_versions WHERE id = ?1",
        [id],
        prompt_version_from_row,
    )
    .map_err(|e| db_err("failed to read created revision", e))
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

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| db_err("failed to begin create-version transaction", e))?;
    let prompt = get_prompt(&tx, prompt_id)?;
    let revision = append_snapshot(&tx, &prompt, note, PromptRevisionSource::Manual, None)?;
    tx.commit()
        .map_err(|e| db_err("failed to commit create-version transaction", e))?;
    Ok(revision)
}

/// Restores a prompt to a previous version's snapshot (Req 7.3, 7.5).
///
/// Returns `NOT_FOUND`, leaving the prompt unchanged, when either the prompt or
/// the requested version does not exist (Req 7.5). On success, the prompt's
/// `systemPrompt`, `userPrompt`, and `variables` are set to the version's
/// snapshot and `updatedAt` is refreshed to the current time; the updated prompt
/// is returned.
pub fn rollback(conn: &Connection, prompt_id: &str, version: i64) -> Result<Prompt, AppError> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| db_err("failed to begin rollback transaction", e))?;
    get_prompt(&tx, prompt_id)?;
    let snapshot = tx
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
    tx.execute(
        "UPDATE prompts SET title=?1,description=?2,prompt_type=?3,type_definition_id=?4,system_prompt=?5,user_prompt=?6,messages=?7,\
         variables=?8,tags=?9,folder_id=?10,images=?11,videos=?12,is_favorite=?13,is_pinned=?14,\
         is_private=?15,source=?16,notes=?17,last_ai_response=?18,updated_at=?19 WHERE id=?20",
        params![
            snapshot.title,
            snapshot.description,
            prompt_type_wire(snapshot.prompt_type),
            snapshot.type_definition_id,
            snapshot.system_prompt,
            snapshot.user_prompt,
            json_array(&snapshot.messages),
            variables_json(&snapshot.variables),
            json_array(&snapshot.tags),
            snapshot.folder_id,
            json_array(&snapshot.images),
            json_array(&snapshot.videos),
            snapshot.is_favorite,
            snapshot.is_pinned,
            snapshot.is_private,
            snapshot.source,
            snapshot.notes,
            snapshot.ai_response,
            now,
            prompt_id,
        ],
    )
    .map_err(|e| db_err("failed to roll back prompt", e))?;
    let restored = get_prompt(&tx, prompt_id)?;
    append_snapshot(
        &tx,
        &restored,
        Some(format!("Rollback to v{version}")),
        PromptRevisionSource::Rollback,
        Some(&snapshot.id),
    )?;
    tx.commit()
        .map_err(|e| db_err("failed to commit rollback transaction", e))?;
    get_prompt(conn, prompt_id)
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
    fn prompt_creation_records_initial_revision() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = seed_prompt(&conn);
        let revisions = list(&conn, &id).unwrap();
        assert_eq!(revisions.len(), 1);
        assert_eq!(revisions[0].source_action, PromptRevisionSource::Create);
        assert_eq!(revisions[0].title, "Versioned");
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
        assert_eq!(numbers, vec![1, 2, 3, 4]);
    }

    #[test]
    fn create_assigns_monotonic_versions_starting_at_one() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = seed_prompt(&conn);

        let first = create(&conn, &id, None).unwrap();
        let second = create(&conn, &id, None).unwrap();
        assert_eq!(first.version, 2);
        assert_eq!(second.version, 3);

        // The prompt's current_version pointer tracks the latest snapshot.
        assert_eq!(prompt::get(&conn, &id).unwrap().current_version, 3);
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

        // No additional version was created, and the initial pointer is untouched.
        assert_eq!(list(&conn, &id).unwrap().len(), 1);
        assert_eq!(prompt::get(&conn, &id).unwrap().current_version, 1);
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
    fn meaningful_save_snapshots_all_fields_noop_does_not_and_rollback_appends() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = prompt::create(
            &conn,
            PromptCreate {
                title: "Original".into(),
                description: Some("old description".into()),
                prompt_type: Some("image".into()),
                system_prompt: Some("old system".into()),
                user_prompt: "old user".into(),
                variables: Some(vec![var("old")]),
                tags: Some(vec!["old-tag".into()]),
                images: Some(vec!["old.png".into()]),
                videos: Some(vec!["old.mp4".into()]),
                is_favorite: Some(true),
                is_pinned: Some(true),
                source: Some("old source".into()),
                notes: Some("old notes".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let original = list(&conn, &created.id).unwrap()[0].clone();

        let changed = prompt::update(
            &conn,
            &created.id,
            PromptUpdate {
                title: Some("Changed".into()),
                description: Some("new description".into()),
                prompt_type: Some("video".into()),
                system_prompt: Some("new system".into()),
                user_prompt: Some("new user".into()),
                variables: Some(vec![var("new")]),
                tags: Some(vec!["new-tag".into()]),
                images: Some(vec!["new.png".into()]),
                videos: Some(vec!["new.mp4".into()]),
                is_favorite: Some(false),
                is_pinned: Some(false),
                source: Some("new source".into()),
                notes: Some("new notes".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let revisions = list(&conn, &created.id).unwrap();
        assert_eq!(revisions.len(), 2);
        let saved = &revisions[1];
        assert_eq!(saved.source_action, PromptRevisionSource::Save);
        assert_eq!(saved.title, changed.title);
        assert_eq!(saved.description, changed.description);
        assert_eq!(saved.prompt_type, changed.prompt_type);
        assert_eq!(saved.tags, changed.tags);
        assert_eq!(saved.images, changed.images);
        assert_eq!(saved.videos, changed.videos);
        assert_eq!(saved.is_favorite, changed.is_favorite);
        assert_eq!(saved.is_pinned, changed.is_pinned);
        assert_eq!(saved.source, changed.source);
        assert_eq!(saved.notes, changed.notes);

        prompt::update(
            &conn,
            &created.id,
            PromptUpdate {
                title: Some(changed.title.clone()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(list(&conn, &created.id).unwrap().len(), 2);

        let restored = rollback(&conn, &created.id, original.version).unwrap();
        assert_eq!(restored.title, original.title);
        assert_eq!(restored.description, original.description);
        assert_eq!(restored.prompt_type, original.prompt_type);
        assert_eq!(restored.tags, original.tags);
        assert_eq!(restored.images, original.images);
        assert_eq!(restored.videos, original.videos);
        assert_eq!(restored.is_favorite, original.is_favorite);
        assert_eq!(restored.is_pinned, original.is_pinned);
        assert_eq!(restored.source, original.source);
        assert_eq!(restored.notes, original.notes);

        let rollback_revision = list(&conn, &created.id).unwrap().pop().unwrap();
        assert_eq!(
            rollback_revision.source_action,
            PromptRevisionSource::Rollback
        );
        assert_eq!(rollback_revision.parent_revision_id, Some(original.id));
    }
}
