//! Row-to-domain mapping for the Storage_Engine (Requirements 2.5, 4.9).
//!
//! Each function maps a `rusqlite::Row` (a `SELECT * FROM <table>` row, or any
//! row exposing the table's columns by name) into its domain DTO from
//! [`crate::models`]. The mapping is the single place that:
//!
//! - converts integer epoch-millisecond timestamp columns into ISO_8601 strings
//!   via [`crate::storage::time::millis_to_iso8601`] (Requirement 4.9);
//! - parses JSON TEXT columns (`variables`, `tags`, `images`, `videos`) into
//!   domain collections, treating NULL/empty as an empty collection;
//! - decodes enum TEXT columns (`prompt_type`) through serde so
//!   the wire spellings stay authoritative;
//! - maps integer `0`/`1` columns to `bool`.
//!
//! Mapping functions return `rusqlite::Result<T>` so they compose directly with
//! `query_row`/`query_map`. A malformed JSON/enum column surfaces as a
//! `FromSqlConversionFailure` rather than a panic.
//!
//! The INSERT/UPDATE side and the services that consume these readers land in
//! later tasks, so the module is allowed to carry currently-unused functions.
#![allow(dead_code)]

use rusqlite::types::Type;
use rusqlite::{Error as SqlError, Row};
use serde::de::DeserializeOwned;

use crate::models::{
    Folder, Prompt, PromptListItem, PromptTypeDefinition, PromptTypeSnapshot, PromptVersion,
};
use crate::storage::time::millis_to_iso8601;

/// Wraps a JSON/enum decode failure as a rusqlite column-conversion error.
fn conversion_err(e: serde_json::Error) -> SqlError {
    SqlError::FromSqlConversionFailure(0, Type::Text, Box::new(e))
}

/// Parses a JSON TEXT column into a `Vec`, treating NULL/empty as an empty vec.
fn parse_json_array<T: DeserializeOwned>(raw: Option<&str>) -> Result<Vec<T>, serde_json::Error> {
    match raw {
        None => Ok(Vec::new()),
        Some(s) if s.trim().is_empty() => Ok(Vec::new()),
        Some(s) => serde_json::from_str(s),
    }
}

/// Decodes an enum value stored as its wire-spelling TEXT.
fn parse_enum<T: DeserializeOwned>(s: &str) -> Result<T, serde_json::Error> {
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
}

/// Reads a JSON TEXT column into a `Vec` (NULL/empty -> empty vec).
fn get_json_array<T: DeserializeOwned>(row: &Row<'_>, col: &str) -> rusqlite::Result<Vec<T>> {
    let raw: Option<String> = row.get(col)?;
    parse_json_array(raw.as_deref()).map_err(conversion_err)
}

/// Reads an enum TEXT column.
fn get_enum<T: DeserializeOwned>(row: &Row<'_>, col: &str) -> rusqlite::Result<T> {
    let s: String = row.get(col)?;
    parse_enum(&s).map_err(conversion_err)
}

/// Reads an epoch-millisecond timestamp column as an ISO_8601 string.
fn get_iso(row: &Row<'_>, col: &str) -> rusqlite::Result<String> {
    let millis: i64 = row.get(col)?;
    Ok(millis_to_iso8601(millis))
}

/// Reads an optional epoch-millisecond timestamp column as `Option<String>`.
fn get_iso_opt(row: &Row<'_>, col: &str) -> rusqlite::Result<Option<String>> {
    let millis: Option<i64> = row.get(col)?;
    Ok(millis.map(millis_to_iso8601))
}

/// Maps a `prompts` row into a [`Prompt`].
pub fn prompt_from_row(row: &Row<'_>) -> rusqlite::Result<Prompt> {
    Ok(Prompt {
        id: row.get("id")?,
        title: row.get("title")?,
        description: row.get("description")?,
        prompt_type: get_enum(row, "prompt_type")?,
        type_definition_id: row.get("type_definition_id")?,
        system_prompt: row.get("system_prompt")?,
        user_prompt: row.get("user_prompt")?,
        messages: get_json_array(row, "messages")?,
        variables: get_json_array(row, "variables")?,
        tags: get_json_array(row, "tags")?,
        folder_id: row.get("folder_id")?,
        images: get_json_array(row, "images")?,
        videos: get_json_array(row, "videos")?,
        is_favorite: row.get("is_favorite")?,
        is_pinned: row.get("is_pinned")?,
        is_private: row.get("is_private")?,
        is_locked: false,
        current_version: row.get("current_version")?,
        usage_count: row.get("usage_count")?,
        source: row.get("source")?,
        notes: row.get("notes")?,
        last_ai_response: row.get("last_ai_response")?,
        created_at: get_iso(row, "created_at")?,
        updated_at: get_iso(row, "updated_at")?,
    })
}

/// Maps a prompt list/search row into a [`PromptListItem`] without bodies.
pub fn prompt_list_item_from_row(row: &Row<'_>) -> rusqlite::Result<PromptListItem> {
    Ok(PromptListItem {
        id: row.get("id")?,
        title: row.get("title")?,
        description: row.get("description")?,
        prompt_type: get_enum(row, "prompt_type")?,
        type_definition_id: row.get("type_definition_id")?,
        tags: get_json_array(row, "tags")?,
        folder_id: row.get("folder_id")?,
        is_favorite: row.get("is_favorite")?,
        is_pinned: row.get("is_pinned")?,
        is_private: row.get("is_private")?,
        is_locked: false,
        current_version: row.get("current_version")?,
        usage_count: row.get("usage_count")?,
        created_at: get_iso(row, "created_at")?,
        updated_at: get_iso(row, "updated_at")?,
    })
}

/// Maps a `prompt_versions` row into a [`PromptVersion`].
pub fn prompt_version_from_row(row: &Row<'_>) -> rusqlite::Result<PromptVersion> {
    let type_definition_id: Option<String> = row.get("type_definition_id")?;
    let type_definition_name: Option<String> = row.get("type_definition_name")?;
    let type_definition_base_kind = row
        .get::<_, Option<String>>("type_definition_base_kind")?
        .map(|value| parse_enum(&value).map_err(conversion_err))
        .transpose()?;
    Ok(PromptVersion {
        id: row.get("id")?,
        prompt_id: row.get("prompt_id")?,
        version: row.get("version")?,
        system_prompt: row.get("system_prompt")?,
        user_prompt: row.get("user_prompt")?,
        messages: get_json_array(row, "messages")?,
        variables: get_json_array(row, "variables")?,
        title: row.get("title")?,
        description: row.get("description")?,
        prompt_type: get_enum(row, "prompt_type")?,
        type_definition_id: type_definition_id.clone(),
        type_definition: match (
            type_definition_id,
            type_definition_name,
            type_definition_base_kind,
        ) {
            (Some(id), Some(name), Some(base_kind)) => Some(PromptTypeSnapshot {
                id,
                name,
                base_kind,
            }),
            _ => None,
        },
        tags: get_json_array(row, "tags")?,
        folder_id: row.get("folder_id")?,
        images: get_json_array(row, "images")?,
        videos: get_json_array(row, "videos")?,
        is_favorite: row.get("is_favorite")?,
        is_pinned: row.get("is_pinned")?,
        is_private: row.get("is_private")?,
        source: row.get("source")?,
        notes: row.get("notes")?,
        note: row.get("note")?,
        ai_response: row.get("ai_response")?,
        source_action: get_enum(row, "source_action")?,
        parent_revision_id: row.get("parent_revision_id")?,
        created_at: get_iso(row, "created_at")?,
    })
}

pub fn prompt_type_definition_from_row(row: &Row<'_>) -> rusqlite::Result<PromptTypeDefinition> {
    Ok(PromptTypeDefinition {
        id: row.get("id")?,
        name: row.get("name")?,
        base_kind: get_enum(row, "base_kind")?,
        created_at: get_iso(row, "created_at")?,
    })
}

/// Maps a `folders` row into a [`Folder`].
pub fn folder_from_row(row: &Row<'_>) -> rusqlite::Result<Folder> {
    Ok(Folder {
        id: row.get("id")?,
        name: row.get("name")?,
        icon: row.get("icon")?,
        parent_id: row.get("parent_id")?,
        sort_order: row.get("sort_order")?,
        created_at: get_iso(row, "created_at")?,
        updated_at: get_iso_opt(row, "updated_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PromptType, Variable};
    use crate::storage::{create_memory_pool, init_schema, DbPool};
    use rusqlite::params;
    /// Builds an in-memory pool with the schema initialized.
    fn schema_pool() -> DbPool {
        let pool = create_memory_pool().unwrap();
        init_schema(&pool.get().unwrap()).unwrap();
        pool
    }

    fn sample_variables() -> Vec<Variable> {
        vec![Variable {
            name: "name".into(),
            r#type: "text".into(),
            label: Some("Name".into()),
            default_value: None,
            options: None,
            required: true,
        }]
    }

    #[test]
    fn prompt_row_maps_json_enums_bools_and_timestamps() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let variables = sample_variables();
        let variables_json = serde_json::to_string(&variables).unwrap();

        conn.execute(
            "INSERT INTO prompts \
             (id,title,description,prompt_type,system_prompt,user_prompt,variables,tags,folder_id,\
              images,videos,is_favorite,is_pinned,current_version,usage_count,source,notes,\
              last_ai_response,created_at,updated_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
            params![
                "p1",
                "Title",
                "desc",
                "image",
                "sys",
                "Hello {{name}}",
                variables_json,
                r#"["a","b"]"#,
                None::<String>,
                r#"["img1.png"]"#,
                "[]",
                1_i64,
                0_i64,
                2_i64,
                5_i64,
                "src",
                None::<String>,
                None::<String>,
                1_700_000_000_000_i64,
                1_700_000_000_123_i64,
            ],
        )
        .unwrap();

        let prompt = conn
            .query_row("SELECT * FROM prompts WHERE id = ?1", ["p1"], |row| {
                prompt_from_row(row)
            })
            .unwrap();

        assert_eq!(prompt.id, "p1");
        assert_eq!(prompt.prompt_type, PromptType::Image);
        assert_eq!(prompt.description.as_deref(), Some("desc"));
        assert_eq!(prompt.variables, variables);
        assert_eq!(prompt.tags, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(prompt.images, vec!["img1.png".to_string()]);
        assert!(prompt.videos.is_empty());
        assert_eq!(prompt.folder_id, None);
        assert!(prompt.is_favorite);
        assert!(!prompt.is_pinned);
        assert_eq!(prompt.current_version, 2);
        assert_eq!(prompt.usage_count, 5);
        assert_eq!(prompt.created_at, "2023-11-14T22:13:20.000Z");
        assert_eq!(prompt.updated_at, "2023-11-14T22:13:20.123Z");
    }

    #[test]
    fn prompt_row_treats_default_json_columns_as_empty() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        // Insert only the required columns; the JSON columns fall back to their
        // schema default of '[]' and prompt_type defaults to 'text'.
        conn.execute(
            "INSERT INTO prompts (id,title,user_prompt,created_at,updated_at) \
             VALUES ('p2','T','U',0,0)",
            [],
        )
        .unwrap();

        let prompt = conn
            .query_row("SELECT * FROM prompts WHERE id = ?1", ["p2"], |row| {
                prompt_from_row(row)
            })
            .unwrap();

        assert_eq!(prompt.prompt_type, PromptType::Text);
        assert!(prompt.variables.is_empty());
        assert!(prompt.tags.is_empty());
        assert!(prompt.images.is_empty());
        assert!(prompt.videos.is_empty());
        assert_eq!(prompt.created_at, "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn prompt_version_row_maps_variables_and_timestamp() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        conn.execute(
            "INSERT INTO prompts (id,title,user_prompt,created_at,updated_at) \
             VALUES ('p1','T','U',0,0)",
            [],
        )
        .unwrap();

        let variables = sample_variables();
        let variables_json = serde_json::to_string(&variables).unwrap();
        conn.execute(
            "INSERT INTO prompt_versions \
             (id,prompt_id,version,system_prompt,user_prompt,variables,note,ai_response,created_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                "v1",
                "p1",
                1_i64,
                "sys",
                "U",
                variables_json,
                "a note",
                None::<String>,
                1_700_000_000_000_i64,
            ],
        )
        .unwrap();

        let version = conn
            .query_row(
                "SELECT * FROM prompt_versions WHERE id = ?1",
                ["v1"],
                prompt_version_from_row,
            )
            .unwrap();

        assert_eq!(version.prompt_id, "p1");
        assert_eq!(version.version, 1);
        assert_eq!(version.variables, variables);
        assert_eq!(version.note.as_deref(), Some("a note"));
        assert_eq!(version.ai_response, None);
        assert_eq!(version.created_at, "2023-11-14T22:13:20.000Z");
    }

    #[test]
    fn folder_row_maps_nullable_updated_at() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        // updated_at left NULL.
        conn.execute(
            "INSERT INTO folders (id,name,icon,parent_id,sort_order,created_at) \
             VALUES ('f1','Root','📁',NULL,3,0)",
            [],
        )
        .unwrap();
        // updated_at present.
        conn.execute(
            "INSERT INTO folders (id,name,parent_id,sort_order,created_at,updated_at) \
             VALUES ('f2','Child','f1',0,0,1_700_000_000_000)",
            [],
        )
        .unwrap();

        let root = conn
            .query_row("SELECT * FROM folders WHERE id = ?1", ["f1"], |row| {
                folder_from_row(row)
            })
            .unwrap();
        assert_eq!(root.name, "Root");
        assert_eq!(root.icon.as_deref(), Some("📁"));
        assert_eq!(root.parent_id, None);
        assert_eq!(root.sort_order, 3);
        assert_eq!(root.created_at, "1970-01-01T00:00:00.000Z");
        assert_eq!(root.updated_at, None);

        let child = conn
            .query_row("SELECT * FROM folders WHERE id = ?1", ["f2"], |row| {
                folder_from_row(row)
            })
            .unwrap();
        assert_eq!(child.parent_id.as_deref(), Some("f1"));
        assert_eq!(
            child.updated_at.as_deref(),
            Some("2023-11-14T22:13:20.000Z")
        );
    }
}
