//! Folder_Service: hierarchical folder operations (Requirement 8).
//!
//! Folders form a tree via `folders.parent_id`. Each folder carries a
//! zero-based `sort_order` among its siblings (folders sharing the same parent,
//! or all root-level folders when `parent_id` is `NULL`).
//!
//! Every function takes a `&rusqlite::Connection` so it is unit-testable against
//! an in-memory pool ([`crate::storage::create_memory_pool`] +
//! [`crate::storage::init_schema`]); the Command_Layer (task 17.1) passes a
//! pooled connection. Timestamps come from [`crate::storage::time::now_millis`]
//! and rows are read back into [`crate::models::Folder`] via
//! [`crate::storage::mapping::folder_from_row`].
//!
//! Errors follow the shared taxonomy: a missing folder or parent id yields
//! [`AppError::not_found`] (Requirement 8.7); an invalid name or a cycle-inducing
//! parent yields [`AppError::validation`] (Requirements 8.8, 8.9).
//!
//! Some functions are only reachable once the Command_Layer is wired (task
//! 17.1), so the module allows currently-unused definitions.
#![allow(dead_code)]

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Deserializer, Serialize};

use crate::error::AppError;
use crate::models::Folder;
use crate::storage::mapping::folder_from_row;
use crate::storage::time::now_millis;

/// Maximum folder name length, in characters, after trimming (Requirement 8.1).
const MAX_NAME_LEN: usize = 255;

/// Input for [`create`] (Requirement 8.1).
///
/// `parentId` places the folder under an existing parent; when omitted the
/// folder is created at the root level. `icon` is optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderInput {
    /// Folder name; trimmed and validated to 1–255 characters.
    pub name: String,
    /// Optional parent folder id; `None` creates a root-level folder.
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Optional icon (e.g. an emoji).
    #[serde(default)]
    pub icon: Option<String>,
}

/// Input for [`update`] (Requirement 8.3).
///
/// Both fields are partial: an absent field is left unchanged. `parentId` is a
/// double option so the Frontend can distinguish "not supplied" (`None`, leave
/// unchanged) from "set to root" (`Some(None)`, an explicit `null`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFolderInput {
    /// New name, when supplied; trimmed and validated to 1–255 characters.
    #[serde(default)]
    pub name: Option<String>,
    /// New parent association, when supplied. `Some(None)` moves the folder to
    /// the root; `Some(Some(id))` reparents under `id`; `None` leaves it
    /// unchanged.
    #[serde(default, deserialize_with = "double_option")]
    pub parent_id: Option<Option<String>>,
}

/// Deserializes a present field (including an explicit JSON `null`) into
/// `Some(_)`, leaving an absent field as `None` via `#[serde(default)]`.
///
/// This is what lets [`UpdateFolderInput::parent_id`] tell "move to root"
/// (`Some(None)`) apart from "leave unchanged" (`None`).
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::deserialize(deserializer).map(Some)
}

/// Creates a folder (Requirements 8.1, 8.6, 8.7, 8.8).
///
/// Trims and validates the name (1–255 chars else `VALIDATION`). When a
/// `parentId` is supplied it must exist (else `NOT_FOUND`). The new folder's
/// `sortOrder` is one greater than the highest sort order among its siblings, or
/// `0` when it has no siblings. A fresh UUID id and `createdAt` are assigned.
pub fn create(conn: &Connection, input: CreateFolderInput) -> Result<Folder, AppError> {
    let name = validate_name(&input.name)?;

    if let Some(parent_id) = input.parent_id.as_deref() {
        ensure_exists(conn, parent_id)?;
    }

    let sort_order = next_sort_order(conn, input.parent_id.as_deref())?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();

    conn.execute(
        "INSERT INTO folders (id, name, icon, parent_id, sort_order, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)",
        params![id, name, input.icon, input.parent_id, sort_order, now],
    )
    .map_err(|e| AppError::internal(format!("failed to insert folder: {e}")))?;

    fetch(conn, &id)
}

/// Returns every folder, ordered by `sortOrder` then `createdAt` for a stable
/// result; returns an empty vec when none exist (Requirement 8.2).
pub fn list(conn: &Connection) -> Result<Vec<Folder>, AppError> {
    let mut stmt = conn
        .prepare("SELECT * FROM folders ORDER BY sort_order ASC, created_at ASC")
        .map_err(|e| AppError::internal(format!("failed to prepare folder list query: {e}")))?;
    let rows = stmt
        .query_map([], folder_from_row)
        .map_err(|e| AppError::internal(format!("failed to query folders: {e}")))?;

    let mut folders = Vec::new();
    for row in rows {
        folders
            .push(row.map_err(|e| AppError::internal(format!("failed to read folder row: {e}")))?);
    }
    Ok(folders)
}

/// Applies a partial patch to an existing folder (Requirements 8.3, 8.6, 8.7,
/// 8.8, 8.9).
///
/// Only supplied fields change. A supplied name is validated (`VALIDATION`). A
/// supplied parent must exist (`NOT_FOUND`) and must not make the folder its own
/// ancestor or descendant (`VALIDATION`); the cycle check walks the proposed
/// parent's ancestor chain with a recursive CTE and rejects if the folder itself
/// appears. An unknown folder id yields `NOT_FOUND`.
pub fn update(conn: &Connection, id: &str, input: UpdateFolderInput) -> Result<Folder, AppError> {
    let current = fetch(conn, id)?;

    let new_name = match input.name.as_deref() {
        Some(raw) => Some(validate_name(raw)?),
        None => None,
    };

    if let Some(parent_opt) = &input.parent_id {
        if let Some(parent_id) = parent_opt {
            ensure_exists(conn, parent_id)?;
            if would_create_cycle(conn, id, parent_id)? {
                return Err(AppError::validation(
                    "parent would make the folder its own ancestor or descendant",
                ));
            }
        }
    }

    let final_name = new_name.unwrap_or(current.name);
    let final_parent = match input.parent_id {
        Some(parent_opt) => parent_opt,
        None => current.parent_id,
    };
    let now = now_millis();

    conn.execute(
        "UPDATE folders SET name = ?1, parent_id = ?2, updated_at = ?3 WHERE id = ?4",
        params![final_name, final_parent, now, id],
    )
    .map_err(|e| AppError::internal(format!("failed to update folder: {e}")))?;

    fetch(conn, id)
}

/// Deletes a folder together with all of its descendants and clears the folder
/// association of every prompt that referenced any removed folder (Requirements
/// 8.4, 8.7, 4.3).
///
/// A recursive CTE gathers the folder plus every descendant, and a single DELETE
/// removes them all. `prompts.folder_id` has `ON DELETE SET NULL`, so prompts
/// pointing at a removed folder are left persisted with a null folder. An unknown
/// folder id yields `NOT_FOUND`.
pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    ensure_exists(conn, id)?;

    conn.execute(
        "WITH RECURSIVE descendants(fid) AS (\n\
           SELECT ?1\n\
           UNION ALL\n\
           SELECT f.id FROM folders f JOIN descendants d ON f.parent_id = d.fid\n\
         )\n\
         DELETE FROM folders WHERE id IN (SELECT fid FROM descendants)",
        [id],
    )
    .map_err(|e| AppError::internal(format!("failed to delete folder: {e}")))?;

    Ok(())
}

/// Assigns each listed folder a `sortOrder` equal to its zero-based position in
/// `ids` (Requirements 8.5, 8.7).
///
/// Runs inside a transaction: every id is validated first, so if any id is
/// unknown the call returns `NOT_FOUND` and the transaction rolls back, leaving
/// all folder records unchanged.
pub fn reorder(conn: &Connection, ids: &[String]) -> Result<(), AppError> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| AppError::internal(format!("failed to begin reorder transaction: {e}")))?;

    for id in ids {
        ensure_exists(&tx, id)?;
    }

    let now = now_millis();
    for (index, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE folders SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![index as i64, now, id],
        )
        .map_err(|e| AppError::internal(format!("failed to set folder sort order: {e}")))?;
    }

    tx.commit()
        .map_err(|e| AppError::internal(format!("failed to commit reorder transaction: {e}")))?;
    Ok(())
}

// --- internal helpers ------------------------------------------------------

/// Trims `raw` and validates its length is 1–255 characters, returning the
/// trimmed name. Rejects empty/whitespace-only or over-long names with
/// `VALIDATION` (Requirement 8.8).
fn validate_name(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    let len = trimmed.chars().count();
    if len < 1 || len > MAX_NAME_LEN {
        return Err(AppError::validation(format!(
            "folder name must be 1–{MAX_NAME_LEN} characters after trimming (got {len})"
        )));
    }
    Ok(trimmed.to_string())
}

/// Returns `NOT_FOUND` unless a folder with `id` exists.
fn ensure_exists(conn: &Connection, id: &str) -> Result<(), AppError> {
    if folder_exists(conn, id)? {
        Ok(())
    } else {
        Err(AppError::not_found(format!("folder `{id}` not found")))
    }
}

/// Reports whether a folder with `id` exists.
fn folder_exists(conn: &Connection, id: &str) -> Result<bool, AppError> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM folders WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .map_err(|e| AppError::internal(format!("failed to look up folder: {e}")))?;
    Ok(count > 0)
}

/// Computes the sort order for a new folder under `parent_id`: max sibling sort
/// order + 1, or 0 when there are no siblings (Requirement 8.1).
fn next_sort_order(conn: &Connection, parent_id: Option<&str>) -> Result<i64, AppError> {
    let max: Option<i64> = match parent_id {
        Some(pid) => conn.query_row(
            "SELECT MAX(sort_order) FROM folders WHERE parent_id = ?1",
            [pid],
            |row| row.get(0),
        ),
        None => conn.query_row(
            "SELECT MAX(sort_order) FROM folders WHERE parent_id IS NULL",
            [],
            |row| row.get(0),
        ),
    }
    .map_err(|e| AppError::internal(format!("failed to compute sibling sort order: {e}")))?;

    Ok(max.map(|m| m + 1).unwrap_or(0))
}

/// Reports whether reparenting `folder_id` under `proposed_parent` would create
/// a cycle, i.e. whether `folder_id` is `proposed_parent` itself or any of its
/// ancestors (Requirement 8.9).
fn would_create_cycle(
    conn: &Connection,
    folder_id: &str,
    proposed_parent: &str,
) -> Result<bool, AppError> {
    let count: i64 = conn
        .query_row(
            "WITH RECURSIVE ancestors(fid) AS (\n\
               SELECT ?1\n\
               UNION ALL\n\
               SELECT f.parent_id FROM folders f JOIN ancestors a ON f.id = a.fid\n\
               WHERE f.parent_id IS NOT NULL\n\
             )\n\
             SELECT COUNT(*) FROM ancestors WHERE fid = ?2",
            params![proposed_parent, folder_id],
            |row| row.get(0),
        )
        .map_err(|e| AppError::internal(format!("failed to evaluate folder cycle check: {e}")))?;
    Ok(count > 0)
}

/// Reads a folder by id, returning `NOT_FOUND` when it does not exist.
fn fetch(conn: &Connection, id: &str) -> Result<Folder, AppError> {
    conn.query_row("SELECT * FROM folders WHERE id = ?1", [id], folder_from_row)
        .optional()
        .map_err(|e| AppError::internal(format!("failed to read folder: {e}")))?
        .ok_or_else(|| AppError::not_found(format!("folder `{id}` not found")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::storage::{create_memory_pool, init_schema, DbPool};
    use rusqlite::params;

    /// Builds an in-memory pool with the schema initialized.
    fn schema_pool() -> DbPool {
        let pool = create_memory_pool().unwrap();
        init_schema(&pool.get().unwrap()).unwrap();
        pool
    }

    fn create_named(conn: &Connection, name: &str, parent: Option<&str>) -> Folder {
        create(
            conn,
            CreateFolderInput {
                name: name.to_string(),
                parent_id: parent.map(str::to_string),
                icon: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn create_assigns_sort_order_among_siblings() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        // Root siblings get 0, 1, 2 in creation order.
        let a = create_named(&conn, "A", None);
        let b = create_named(&conn, "B", None);
        let c = create_named(&conn, "C", None);
        assert_eq!(a.sort_order, 0);
        assert_eq!(b.sort_order, 1);
        assert_eq!(c.sort_order, 2);

        // Children of `a` form an independent sibling group, restarting at 0.
        let a1 = create_named(&conn, "A1", Some(&a.id));
        let a2 = create_named(&conn, "A2", Some(&a.id));
        assert_eq!(a1.sort_order, 0);
        assert_eq!(a2.sort_order, 1);

        // A child of `b` also starts at 0 (separate parent).
        let b1 = create_named(&conn, "B1", Some(&b.id));
        assert_eq!(b1.sort_order, 0);
    }

    #[test]
    fn create_sets_parent_and_no_updated_at() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let root = create_named(&conn, "Root", None);
        assert_eq!(root.parent_id, None);
        assert!(root.updated_at.is_none(), "fresh folder has no updatedAt");
        assert!(!root.created_at.is_empty());

        let child = create_named(&conn, "Child", Some(&root.id));
        assert_eq!(child.parent_id.as_deref(), Some(root.id.as_str()));
    }

    #[test]
    fn create_trims_name() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let folder = create_named(&conn, "  Spaced  ", None);
        assert_eq!(folder.name, "Spaced");
    }

    #[test]
    fn create_rejects_empty_and_whitespace_names() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        for bad in ["", "   ", "\t\n"] {
            let err = create(
                &conn,
                CreateFolderInput {
                    name: bad.to_string(),
                    parent_id: None,
                    icon: None,
                },
            )
            .unwrap_err();
            assert_eq!(
                err.code,
                ErrorCode::Validation,
                "name {bad:?} should be invalid"
            );
        }

        // Nothing was created.
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn create_enforces_255_char_boundary() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        // 255 chars: accepted.
        let ok_name = "x".repeat(255);
        let folder = create_named(&conn, &ok_name, None);
        assert_eq!(folder.name.chars().count(), 255);

        // 256 chars: rejected with VALIDATION.
        let too_long = "x".repeat(256);
        let err = create(
            &conn,
            CreateFolderInput {
                name: too_long,
                parent_id: None,
                icon: None,
            },
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn create_with_missing_parent_is_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let err = create(
            &conn,
            CreateFolderInput {
                name: "Orphan".into(),
                parent_id: Some("does-not-exist".into()),
                icon: None,
            },
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
        assert!(
            list(&conn).unwrap().is_empty(),
            "no folder created on bad parent"
        );
    }

    #[test]
    fn list_returns_empty_then_all() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        assert!(list(&conn).unwrap().is_empty());

        create_named(&conn, "A", None);
        create_named(&conn, "B", None);
        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn update_partial_patch_preserves_unsupplied_fields() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let parent = create_named(&conn, "Parent", None);
        let folder = create(
            &conn,
            CreateFolderInput {
                name: "Original".into(),
                parent_id: Some(parent.id.clone()),
                icon: Some("📁".into()),
            },
        )
        .unwrap();

        // Patch only the name; parent and icon must be preserved.
        let updated = update(
            &conn,
            &folder.id,
            UpdateFolderInput {
                name: Some("Renamed".into()),
                parent_id: None,
            },
        )
        .unwrap();
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.parent_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(updated.icon.as_deref(), Some("📁"));
        assert!(updated.updated_at.is_some(), "update sets updatedAt");
    }

    #[test]
    fn update_can_move_folder_to_root() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let parent = create_named(&conn, "Parent", None);
        let child = create_named(&conn, "Child", Some(&parent.id));
        assert_eq!(child.parent_id.as_deref(), Some(parent.id.as_str()));

        // Explicit Some(None) moves it to root.
        let moved = update(
            &conn,
            &child.id,
            UpdateFolderInput {
                name: None,
                parent_id: Some(None),
            },
        )
        .unwrap();
        assert_eq!(moved.parent_id, None);
    }

    #[test]
    fn update_rejects_invalid_name() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let folder = create_named(&conn, "Keep", None);
        let err = update(
            &conn,
            &folder.id,
            UpdateFolderInput {
                name: Some("   ".into()),
                parent_id: None,
            },
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);

        // Name unchanged.
        assert_eq!(fetch(&conn, &folder.id).unwrap().name, "Keep");
    }

    #[test]
    fn update_rejects_self_parent_cycle() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let folder = create_named(&conn, "Self", None);
        let err = update(
            &conn,
            &folder.id,
            UpdateFolderInput {
                name: None,
                parent_id: Some(Some(folder.id.clone())),
            },
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert_eq!(fetch(&conn, &folder.id).unwrap().parent_id, None);
    }

    #[test]
    fn update_rejects_descendant_parent_cycle() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        // grandparent -> parent -> child
        let grandparent = create_named(&conn, "GP", None);
        let parent = create_named(&conn, "P", Some(&grandparent.id));
        let child = create_named(&conn, "C", Some(&parent.id));

        // Making the grandparent a child of its own descendant is a cycle.
        let err = update(
            &conn,
            &grandparent.id,
            UpdateFolderInput {
                name: None,
                parent_id: Some(Some(child.id.clone())),
            },
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);

        // Tree unchanged.
        assert_eq!(fetch(&conn, &grandparent.id).unwrap().parent_id, None);
        assert_eq!(
            fetch(&conn, &parent.id).unwrap().parent_id.as_deref(),
            Some(grandparent.id.as_str())
        );
    }

    #[test]
    fn update_allows_valid_reparent() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let a = create_named(&conn, "A", None);
        let b = create_named(&conn, "B", None);
        // Move b under a (not a cycle).
        let moved = update(
            &conn,
            &b.id,
            UpdateFolderInput {
                name: None,
                parent_id: Some(Some(a.id.clone())),
            },
        )
        .unwrap();
        assert_eq!(moved.parent_id.as_deref(), Some(a.id.as_str()));
    }

    #[test]
    fn update_missing_folder_is_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = update(&conn, "nope", UpdateFolderInput::default()).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn update_missing_parent_is_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let folder = create_named(&conn, "F", None);
        let err = update(
            &conn,
            &folder.id,
            UpdateFolderInput {
                name: None,
                parent_id: Some(Some("ghost".into())),
            },
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn delete_removes_descendants_and_clears_prompt_folder_id() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        // root -> child -> grandchild
        let root = create_named(&conn, "Root", None);
        let child = create_named(&conn, "Child", Some(&root.id));
        let grandchild = create_named(&conn, "Grandchild", Some(&child.id));
        // A sibling that should survive.
        let other = create_named(&conn, "Other", None);

        // Prompts referencing folders inside and outside the deleted subtree.
        conn.execute(
            "INSERT INTO prompts (id,title,user_prompt,folder_id,created_at,updated_at) \
             VALUES ('p_child','T','U',?1,0,0)",
            params![child.id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO prompts (id,title,user_prompt,folder_id,created_at,updated_at) \
             VALUES ('p_grand','T','U',?1,0,0)",
            params![grandchild.id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO prompts (id,title,user_prompt,folder_id,created_at,updated_at) \
             VALUES ('p_other','T','U',?1,0,0)",
            params![other.id],
        )
        .unwrap();

        delete(&conn, &root.id).unwrap();

        // Root and all descendants are gone; the unrelated folder remains.
        let remaining = list(&conn).unwrap();
        let ids: Vec<&str> = remaining.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, vec![other.id.as_str()]);

        // Prompts inside the deleted subtree have a null folder; the survivor keeps its folder.
        let folder_of = |pid: &str| -> Option<String> {
            conn.query_row(
                "SELECT folder_id FROM prompts WHERE id = ?1",
                [pid],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(folder_of("p_child"), None);
        assert_eq!(folder_of("p_grand"), None);
        assert_eq!(folder_of("p_other").as_deref(), Some(other.id.as_str()));
    }

    #[test]
    fn delete_missing_folder_is_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = delete(&conn, "missing").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn reorder_assigns_positional_sort_orders() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let a = create_named(&conn, "A", None);
        let b = create_named(&conn, "B", None);
        let c = create_named(&conn, "C", None);
        // Initial sort orders: a=0, b=1, c=2.

        // Reorder to [c, a, b].
        reorder(&conn, &[c.id.clone(), a.id.clone(), b.id.clone()]).unwrap();

        let sort_of = |id: &str| -> i64 {
            conn.query_row("SELECT sort_order FROM folders WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .unwrap()
        };
        assert_eq!(sort_of(&c.id), 0);
        assert_eq!(sort_of(&a.id), 1);
        assert_eq!(sort_of(&b.id), 2);
    }

    #[test]
    fn reorder_with_missing_id_is_not_found_and_changes_nothing() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let a = create_named(&conn, "A", None);
        let b = create_named(&conn, "B", None);
        // a=0, b=1 initially.

        let err = reorder(&conn, &[b.id.clone(), "ghost".into(), a.id.clone()]).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);

        // Transaction rolled back: original sort orders preserved.
        let sort_of = |id: &str| -> i64 {
            conn.query_row("SELECT sort_order FROM folders WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .unwrap()
        };
        assert_eq!(sort_of(&a.id), 0);
        assert_eq!(sort_of(&b.id), 1);
    }

    #[test]
    fn double_option_distinguishes_absent_null_and_value() {
        // Absent field -> None (leave unchanged).
        let absent: UpdateFolderInput = serde_json::from_str("{}").unwrap();
        assert!(absent.parent_id.is_none());

        // Explicit null -> Some(None) (move to root).
        let null: UpdateFolderInput = serde_json::from_str(r#"{"parentId": null}"#).unwrap();
        assert_eq!(null.parent_id, Some(None));

        // Value -> Some(Some(id)) (reparent).
        let value: UpdateFolderInput = serde_json::from_str(r#"{"parentId": "abc"}"#).unwrap();
        assert_eq!(value.parent_id, Some(Some("abc".to_string())));
    }
}
