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
    use proptest::prelude::*;
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

    // ---- Appearance settings (settings-appearance-redesign) --------------

    /// camelCase wire names for the six appearance fields.
    const APPEARANCE_KEYS: [&str; 6] = [
        "flavor",
        "accentColor",
        "displayFont",
        "bodyFont",
        "fontScale",
        "density",
    ];

    /// Valid value catalog for the appearance field at index `i`.
    fn catalog(i: usize) -> &'static [&'static str] {
        match i {
            0 => &[
                "Latte",
                "Frappé",
                "Macchiato",
                "Mocha",
                "Claude Light",
                "Claude Dark",
            ],
            1 => &[
                "Rosewater",
                "Flamingo",
                "Pink",
                "Mauve",
                "Red",
                "Maroon",
                "Peach",
                "Yellow",
                "Green",
                "Teal",
                "Sky",
                "Sapphire",
                "Blue",
                "Lavender",
            ],
            2 | 3 => &["System", "Inter", "Space Grotesk", "JetBrains Mono"],
            4 => &["Small", "Default", "Large", "Extra Large"],
            _ => &["Compact", "Default", "Comfortable"],
        }
    }

    /// Strategy yielding one valid value from the field's catalog.
    fn value(i: usize) -> impl Strategy<Value = String> {
        let owned: Vec<String> = catalog(i).iter().map(|s| (*s).to_string()).collect();
        proptest::sample::select(owned)
    }

    /// Strategy yielding an arbitrary subset of the six appearance fields, each
    /// either absent or set to a valid catalog value.
    fn appearance_subset() -> impl Strategy<Value = [Option<String>; 6]> {
        (
            proptest::option::of(value(0)),
            proptest::option::of(value(1)),
            proptest::option::of(value(2)),
            proptest::option::of(value(3)),
            proptest::option::of(value(4)),
            proptest::option::of(value(5)),
        )
            .prop_map(|(a, b, c, d, e, f)| [a, b, c, d, e, f])
    }

    /// Extracts the six appearance fields in `APPEARANCE_KEYS` order.
    fn appearance_of(s: &Settings) -> [Option<String>; 6] {
        [
            s.flavor.clone(),
            s.accent_color.clone(),
            s.display_font.clone(),
            s.body_font.clone(),
            s.font_scale.clone(),
            s.density.clone(),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: settings-appearance-redesign, Property 13: Partial settings updates are isolated
        ///
        /// **Validates: Requirements 10.3**
        #[test]
        fn partial_settings_updates_are_isolated(
            theme in proptest::sample::select(vec!["light", "dark", "system"]),
            language in proptest::sample::select(vec!["en", "zh", "zh-TW", "ja", "fr", "de", "es"]),
            auto_save in any::<bool>(),
            prior in appearance_subset(),
            patch in appearance_subset(),
        ) {
            let conn = conn();

            // Persist arbitrary prior settings (non-appearance + a subset of appearance).
            let mut prior_obj = serde_json::Map::new();
            prior_obj.insert("theme".to_string(), Value::String(theme.to_string()));
            prior_obj.insert("language".to_string(), Value::String(language.to_string()));
            prior_obj.insert("autoSave".to_string(), Value::Bool(auto_save));
            for (i, field) in prior.iter().enumerate() {
                if let Some(v) = field {
                    prior_obj.insert(APPEARANCE_KEYS[i].to_string(), Value::String(v.clone()));
                }
            }
            update(&conn, &Value::Object(prior_obj)).unwrap();

            // Apply an arbitrary subset of appearance fields as a patch.
            let mut patch_obj = serde_json::Map::new();
            for (i, field) in patch.iter().enumerate() {
                if let Some(v) = field {
                    patch_obj.insert(APPEARANCE_KEYS[i].to_string(), Value::String(v.clone()));
                }
            }
            let result = update(&conn, &Value::Object(patch_obj)).unwrap();

            // Non-appearance fields are untouched by the appearance patch.
            prop_assert_eq!(result.theme.as_str(), theme);
            prop_assert_eq!(result.language.as_str(), language);
            prop_assert_eq!(result.auto_save, auto_save);

            // Exactly the supplied fields changed; every other field is preserved.
            let got = appearance_of(&result);
            for (i, got_field) in got.iter().enumerate() {
                let expected = match &patch[i] {
                    Some(v) => Some(v.clone()),
                    None => prior[i].clone(),
                };
                prop_assert_eq!(got_field, &expected);
            }

            // The returned settings equal the stored settings.
            prop_assert_eq!(appearance_of(&get(&conn).unwrap()), got);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: settings-appearance-redesign, Property 14: Appearance persistence round-trips
        ///
        /// **Validates: Requirements 10.5**
        #[test]
        fn appearance_persistence_round_trips(
            flavor in value(0),
            accent in value(1),
            display in value(2),
            body in value(3),
            scale in value(4),
            density in value(5),
        ) {
            let conn = conn();
            let patch = json!({
                "flavor": flavor,
                "accentColor": accent,
                "displayFont": display,
                "bodyFont": body,
                "fontScale": scale,
                "density": density,
            });
            update(&conn, &patch).unwrap();

            // Read back with no intervening update.
            let stored = get(&conn).unwrap();
            prop_assert_eq!(stored.flavor.as_deref(), Some(flavor.as_str()));
            prop_assert_eq!(stored.accent_color.as_deref(), Some(accent.as_str()));
            prop_assert_eq!(stored.display_font.as_deref(), Some(display.as_str()));
            prop_assert_eq!(stored.body_font.as_deref(), Some(body.as_str()));
            prop_assert_eq!(stored.font_scale.as_deref(), Some(scale.as_str()));
            prop_assert_eq!(stored.density.as_deref(), Some(density.as_str()));
        }
    }

    #[test]
    fn appearance_patch_round_trips_through_camel_case() {
        let conn = conn();
        let result = update(
            &conn,
            &json!({
                "flavor": "Mocha",
                "accentColor": "Blue",
                "displayFont": "Inter",
                "bodyFont": "System",
                "fontScale": "Large",
                "density": "Compact",
            }),
        )
        .unwrap();

        // The camelCase patch deserializes into the typed snake_case fields.
        assert_eq!(result.flavor.as_deref(), Some("Mocha"));
        assert_eq!(result.accent_color.as_deref(), Some("Blue"));
        assert_eq!(result.display_font.as_deref(), Some("Inter"));
        assert_eq!(result.body_font.as_deref(), Some("System"));
        assert_eq!(result.font_scale.as_deref(), Some("Large"));
        assert_eq!(result.density.as_deref(), Some("Compact"));

        // And re-serializes back to the same camelCase wire names.
        let value = serde_json::to_value(&result).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.get("flavor").and_then(Value::as_str), Some("Mocha"));
        assert_eq!(obj.get("accentColor").and_then(Value::as_str), Some("Blue"));
        assert_eq!(
            obj.get("displayFont").and_then(Value::as_str),
            Some("Inter")
        );
        assert_eq!(obj.get("bodyFont").and_then(Value::as_str), Some("System"));
        assert_eq!(obj.get("fontScale").and_then(Value::as_str), Some("Large"));
        assert_eq!(obj.get("density").and_then(Value::as_str), Some("Compact"));
    }

    #[test]
    fn wrong_typed_appearance_field_is_rejected_without_mutating() {
        let conn = conn();
        // Seed a known appearance value.
        update(&conn, &json!({ "flavor": "Latte" })).unwrap();
        // `flavor` must be a string; a number is rejected by validate-before-write.
        let err = update(&conn, &json!({ "flavor": 42 })).unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
        // The previously stored value is unchanged (no mutation on error).
        assert_eq!(get(&conn).unwrap().flavor.as_deref(), Some("Latte"));
    }
}
