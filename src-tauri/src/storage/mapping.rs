//! Row-to-domain mapping for the Storage_Engine (Requirements 2.5, 4.9).
//!
//! Each function maps a `rusqlite::Row` (a `SELECT * FROM <table>` row, or any
//! row exposing the table's columns by name) into its domain DTO from
//! [`crate::models`]. The mapping is the single place that:
//!
//! - converts integer epoch-millisecond timestamp columns into ISO_8601 strings
//!   via [`crate::storage::time::millis_to_iso8601`] (Requirement 4.9);
//! - parses JSON TEXT columns (`variables`, `tags`, `images`, `videos`,
//!   `files_snapshot`, `safety_report`) into the domain `Vec`/`Value` types,
//!   treating NULL/empty as an empty collection or `None`;
//! - decodes enum TEXT columns (`prompt_type`, `safety_level`) through serde so
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

use crate::models::{Folder, Prompt, PromptVersion, Skill, SkillFileSnapshot, SkillVersion};
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

/// Parses an optional JSON TEXT column, treating NULL/empty as `None`.
fn parse_json_opt<T: DeserializeOwned>(raw: Option<&str>) -> Result<Option<T>, serde_json::Error> {
    match raw {
        None => Ok(None),
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => serde_json::from_str(s).map(Some),
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

/// Reads an optional JSON TEXT column into `Option<T>` (NULL/empty -> `None`).
fn get_json_opt<T: DeserializeOwned>(row: &Row<'_>, col: &str) -> rusqlite::Result<Option<T>> {
    let raw: Option<String> = row.get(col)?;
    parse_json_opt(raw.as_deref()).map_err(conversion_err)
}

/// Reads an enum TEXT column.
fn get_enum<T: DeserializeOwned>(row: &Row<'_>, col: &str) -> rusqlite::Result<T> {
    let s: String = row.get(col)?;
    parse_enum(&s).map_err(conversion_err)
}

/// Reads an optional enum TEXT column (NULL -> `None`).
fn get_enum_opt<T: DeserializeOwned>(row: &Row<'_>, col: &str) -> rusqlite::Result<Option<T>> {
    let s: Option<String> = row.get(col)?;
    match s {
        None => Ok(None),
        Some(s) => parse_enum(&s).map(Some).map_err(conversion_err),
    }
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
        system_prompt: row.get("system_prompt")?,
        user_prompt: row.get("user_prompt")?,
        variables: get_json_array(row, "variables")?,
        tags: get_json_array(row, "tags")?,
        folder_id: row.get("folder_id")?,
        images: get_json_array(row, "images")?,
        videos: get_json_array(row, "videos")?,
        is_favorite: row.get("is_favorite")?,
        is_pinned: row.get("is_pinned")?,
        current_version: row.get("current_version")?,
        usage_count: row.get("usage_count")?,
        source: row.get("source")?,
        notes: row.get("notes")?,
        last_ai_response: row.get("last_ai_response")?,
        created_at: get_iso(row, "created_at")?,
        updated_at: get_iso(row, "updated_at")?,
    })
}

/// Maps a `prompt_versions` row into a [`PromptVersion`].
pub fn prompt_version_from_row(row: &Row<'_>) -> rusqlite::Result<PromptVersion> {
    Ok(PromptVersion {
        id: row.get("id")?,
        prompt_id: row.get("prompt_id")?,
        version: row.get("version")?,
        system_prompt: row.get("system_prompt")?,
        user_prompt: row.get("user_prompt")?,
        variables: get_json_array(row, "variables")?,
        note: row.get("note")?,
        ai_response: row.get("ai_response")?,
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

/// Maps a `skills` row into a [`Skill`].
pub fn skill_from_row(row: &Row<'_>) -> rusqlite::Result<Skill> {
    Ok(Skill {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        content: row.get("content")?,
        protocol_type: row.get("protocol_type")?,
        version: row.get("version")?,
        author: row.get("author")?,
        tags: get_json_array(row, "tags")?,
        is_favorite: row.get("is_favorite")?,
        source_url: row.get("source_url")?,
        source_id: row.get("source_id")?,
        source_label: row.get("source_label")?,
        source_branch: row.get("source_branch")?,
        source_directory: row.get("source_directory")?,
        canonical_skill_path: row.get("canonical_skill_path")?,
        local_repo_path: row.get("local_repo_path")?,
        directory_fingerprint: row.get("directory_fingerprint")?,
        icon_url: row.get("icon_url")?,
        icon_emoji: row.get("icon_emoji")?,
        icon_background: row.get("icon_background")?,
        category: row.get("category")?,
        is_builtin: row.get("is_builtin")?,
        registry_slug: row.get("registry_slug")?,
        content_url: row.get("content_url")?,
        safety_level: get_enum_opt(row, "safety_level")?,
        safety_score: row.get("safety_score")?,
        safety_report: get_json_opt(row, "safety_report")?,
        safety_scanned_at: get_iso_opt(row, "safety_scanned_at")?,
        current_version: row.get("current_version")?,
        version_tracking_enabled: row.get("version_tracking_enabled")?,
        created_at: get_iso(row, "created_at")?,
        updated_at: get_iso(row, "updated_at")?,
    })
}

/// Maps a `skill_versions` row into a [`SkillVersion`].
pub fn skill_version_from_row(row: &Row<'_>) -> rusqlite::Result<SkillVersion> {
    Ok(SkillVersion {
        id: row.get("id")?,
        skill_id: row.get("skill_id")?,
        version: row.get("version")?,
        content: row.get("content")?,
        files_snapshot: get_json_opt::<Vec<SkillFileSnapshot>>(row, "files_snapshot")?,
        note: row.get("note")?,
        created_at: get_iso(row, "created_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PromptType, SafetyLevel, Variable};
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

    #[test]
    fn skill_row_maps_enum_json_report_and_timestamps() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        conn.execute(
            "INSERT INTO skills \
             (id,name,description,content,tags,is_favorite,category,is_builtin,\
              safety_level,safety_score,safety_report,safety_scanned_at,\
              current_version,version_tracking_enabled,created_at,updated_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                "s1",
                "My Skill",
                "does things",
                "# Body",
                r#"["util","fun"]"#,
                1_i64,
                "general",
                0_i64,
                "high-risk",
                42_i64,
                r#"{"findings":[{"severity":"high"}]}"#,
                1_700_000_000_000_i64,
                1_i64,
                1_i64,
                1_700_000_000_000_i64,
                1_700_000_000_123_i64,
            ],
        )
        .unwrap();

        let skill = conn
            .query_row("SELECT * FROM skills WHERE id = ?1", ["s1"], |row| {
                skill_from_row(row)
            })
            .unwrap();

        assert_eq!(skill.name, "My Skill");
        assert_eq!(skill.content.as_deref(), Some("# Body"));
        assert_eq!(skill.tags, vec!["util".to_string(), "fun".to_string()]);
        assert!(skill.is_favorite);
        // schema defaults for the unspecified protocol_type column.
        assert_eq!(skill.protocol_type, "skill");
        assert_eq!(skill.safety_level, Some(SafetyLevel::HighRisk));
        assert_eq!(skill.safety_score, Some(42));
        assert_eq!(
            skill.safety_report,
            Some(serde_json::json!({ "findings": [{ "severity": "high" }] }))
        );
        assert_eq!(
            skill.safety_scanned_at.as_deref(),
            Some("2023-11-14T22:13:20.000Z")
        );
        assert!(skill.version_tracking_enabled);
        assert_eq!(skill.created_at, "2023-11-14T22:13:20.000Z");
        assert_eq!(skill.updated_at, "2023-11-14T22:13:20.123Z");
    }

    #[test]
    fn skill_row_treats_missing_safety_fields_as_none() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        conn.execute(
            "INSERT INTO skills (id,name,created_at,updated_at) \
             VALUES ('s2','Bare',0,0)",
            [],
        )
        .unwrap();

        let skill = conn
            .query_row("SELECT * FROM skills WHERE id = ?1", ["s2"], |row| {
                skill_from_row(row)
            })
            .unwrap();

        assert_eq!(skill.safety_level, None);
        assert_eq!(skill.safety_report, None);
        assert_eq!(skill.safety_scanned_at, None);
        assert!(skill.tags.is_empty());
        assert_eq!(skill.category, "general");
    }

    #[test]
    fn skill_version_row_maps_files_snapshot() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        conn.execute(
            "INSERT INTO skills (id,name,created_at,updated_at) VALUES ('s1','S',0,0)",
            [],
        )
        .unwrap();

        let files = vec![SkillFileSnapshot {
            relative_path: "SKILL.md".into(),
            content: "# Body".into(),
        }];
        let files_json = serde_json::to_string(&files).unwrap();

        conn.execute(
            "INSERT INTO skill_versions (id,skill_id,version,content,files_snapshot,note,created_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                "sv1",
                "s1",
                1_i64,
                "# Body",
                files_json,
                None::<String>,
                1_700_000_000_000_i64,
            ],
        )
        .unwrap();

        let version = conn
            .query_row(
                "SELECT * FROM skill_versions WHERE id = ?1",
                ["sv1"],
                skill_version_from_row,
            )
            .unwrap();

        assert_eq!(version.skill_id, "s1");
        assert_eq!(version.version, 1);
        assert_eq!(version.files_snapshot, Some(files));
        assert_eq!(version.note, None);
        assert_eq!(version.created_at, "2023-11-14T22:13:20.000Z");
    }

    #[test]
    fn skill_version_row_treats_null_files_snapshot_as_none() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        conn.execute(
            "INSERT INTO skills (id,name,created_at,updated_at) VALUES ('s1','S',0,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO skill_versions (id,skill_id,version,content,created_at) \
             VALUES ('sv1','s1',1,'c',0)",
            [],
        )
        .unwrap();

        let version = conn
            .query_row(
                "SELECT * FROM skill_versions WHERE id = ?1",
                ["sv1"],
                skill_version_from_row,
            )
            .unwrap();

        assert_eq!(version.files_snapshot, None);
    }
}
