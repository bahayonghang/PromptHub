//! Settings_Service: persistence and retrieval of application settings
//! (Requirement 19.1, 19.2).
//!
//! Settings are stored as a single JSON document in the key/value `settings`
//! table under the [`SETTINGS_KEY`] key, mirroring the round-trip the
//! Storage_Engine property test already exercises (`storage::proptest_roundtrip`).
//! Modeling the whole [`Settings`] object as one JSON value keeps the schema
//! stable as new optional fields are added and lets a partial update merge
//! cleanly.
//!
//! ## Defaults (19.1)
//!
//! [`get`] returns the stored settings, or [`defaults`] when nothing has been
//! persisted yet. The defaults match the Frontend's expectations: the dark theme
//! (Requirement 22.5), English UI (Requirement 21.5), and auto-save enabled.
//!
//! ## Partial update (19.2)
//!
//! [`update`] takes a *partial* settings object (a JSON object carrying only the
//! fields to change), merges it over the currently stored settings at the
//! top-level field granularity, validates the merged shape, persists it, and
//! returns the result. Validation runs *before* the write, so a malformed update
//! never mutates stored data (Requirement 2.3).
//!
//! ## Testability / dependency injection
//!
//! Both functions take a borrowed `&rusqlite::Connection` rather than reaching
//! into [`crate::state::AppState`], so they are unit-testable with
//! `storage::create_memory_pool` + `init_schema`. The Command_Layer (task 17.1)
//! supplies a pooled connection.
#![allow(dead_code)]

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::error::AppError;
use crate::models::Settings;

/// Settings-table key under which the full [`Settings`] JSON document is stored.
const SETTINGS_KEY: &str = "app";

/// Default settings returned when nothing has been persisted yet (19.1).
///
/// The dark theme (22.5) and English UI (21.5) match the Frontend's startup
/// fallbacks; auto-save defaults on. All optional fields default to absent.
fn defaults() -> Settings {
    Settings {
        theme: "dark".to_string(),
        language: "en".to_string(),
        auto_save: true,
        ..Settings::default()
    }
}

/// Returns the stored application settings, or the defaults when none have been
/// persisted (Requirement 19.1).
pub fn get(conn: &Connection) -> Result<Settings, AppError> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![SETTINGS_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| AppError::internal(format!("failed to read settings: {e}")))?;

    match stored {
        Some(json) => serde_json::from_str(&json)
            .map_err(|e| AppError::internal(format!("stored settings are corrupt: {e}"))),
        None => Ok(defaults()),
    }
}

/// Merges a partial settings object over the stored settings, persists the
/// result, and returns it (Requirement 19.2).
///
/// `patch` must be a JSON object; each key it carries replaces the corresponding
/// top-level field, while fields it omits are left unchanged. The merged document
/// is validated by deserializing it back into [`Settings`] *before* the write, so
/// a patch that names an unknown-typed value is rejected with `VALIDATION` and
/// nothing is persisted (Requirement 2.3).
pub fn update(conn: &Connection, patch: &Value) -> Result<Settings, AppError> {
    let patch_obj = patch
        .as_object()
        .ok_or_else(|| AppError::validation("settings update must be a JSON object"))?;

    // Start from the stored settings (or defaults) as a JSON object.
    let current = get(conn)?;
    let mut merged = serde_json::to_value(&current)
        .map_err(|e| AppError::internal(format!("failed to encode current settings: {e}")))?;
    let merged_obj = merged
        .as_object_mut()
        .expect("Settings always serializes to a JSON object");

    // Top-level field merge: supplied fields overwrite, omitted fields persist.
    for (key, value) in patch_obj {
        merged_obj.insert(key.clone(), value.clone());
    }

    // Validate the merged shape before writing so a bad update never mutates
    // stored data (Req 2.3).
    let result: Settings = serde_json::from_value(merged)
        .map_err(|e| AppError::validation(format!("invalid settings update: {e}")))?;

    let json = serde_json::to_string(&result)
        .map_err(|e| AppError::internal(format!("failed to encode settings: {e}")))?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![SETTINGS_KEY, json],
    )
    .map_err(|e| AppError::internal(format!("failed to persist settings: {e}")))?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{create_memory_pool, init_schema};
    use serde_json::json;

    /// Builds an in-memory pooled connection with the schema initialized.
    fn conn() -> r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager> {
        let pool = create_memory_pool().unwrap();
        init_schema(&pool.get().unwrap()).unwrap();
        pool.get().unwrap()
    }

    #[test]
    fn get_returns_defaults_when_nothing_stored() {
        let conn = conn();
        let settings = get(&conn).unwrap();
        assert_eq!(settings.theme, "dark");
        assert_eq!(settings.language, "en");
        assert!(settings.auto_save);
        assert!(settings.sync.is_none());
    }

    #[test]
    fn update_persists_supplied_fields_and_returns_result() {
        let conn = conn();
        let result = update(&conn, &json!({ "theme": "light", "language": "ja" })).unwrap();
        assert_eq!(result.theme, "light");
        assert_eq!(result.language, "ja");
        // Unspecified required field keeps its default.
        assert!(result.auto_save);
    }

    #[test]
    fn update_leaves_unspecified_fields_unchanged() {
        let conn = conn();
        // First update sets theme + autoSave.
        update(&conn, &json!({ "theme": "light", "autoSave": false })).unwrap();
        // Second update touches only language.
        let result = update(&conn, &json!({ "language": "fr" })).unwrap();
        assert_eq!(result.language, "fr");
        // Previously-set fields are preserved.
        assert_eq!(result.theme, "light");
        assert!(!result.auto_save);
    }

    #[test]
    fn get_returns_stored_settings_after_update() {
        let conn = conn();
        update(&conn, &json!({ "theme": "system", "githubToken": "abc" })).unwrap();
        let stored = get(&conn).unwrap();
        assert_eq!(stored.theme, "system");
        assert_eq!(stored.github_token.as_deref(), Some("abc"));
    }

    #[test]
    fn update_merges_optional_nested_object() {
        let conn = conn();
        let result = update(
            &conn,
            &json!({ "sync": { "enabled": true, "provider": "webdav", "endpoint": "https://x" } }),
        )
        .unwrap();
        let sync = result.sync.expect("sync should be set");
        assert!(sync.enabled);
        assert_eq!(sync.provider, "webdav");
        assert_eq!(sync.endpoint.as_deref(), Some("https://x"));
    }

    #[test]
    fn update_rejects_non_object_patch() {
        let conn = conn();
        let err = update(&conn, &json!("not-an-object")).unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
    }

    #[test]
    fn update_rejects_wrong_typed_field_without_mutating() {
        let conn = conn();
        // Seed a known value.
        update(&conn, &json!({ "theme": "light" })).unwrap();
        // `theme` must be a string; a number is rejected.
        let err = update(&conn, &json!({ "theme": 42 })).unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
        // The earlier value is still stored (no mutation on error).
        assert_eq!(get(&conn).unwrap().theme, "light");
    }
}
