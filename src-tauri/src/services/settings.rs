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

use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::error::AppError;
use crate::models::Settings;
use crate::state::EncryptionState;

/// Settings-table key under which the full [`Settings`] JSON document is stored.
const SETTINGS_KEY: &str = "app";

const MAX_INTERFACE_FONT_FAMILIES: usize = 4;
const MAX_FONT_FAMILY_LENGTH: usize = 128;

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
///
/// Secret fields (`githubToken`, `sync.password`) are never returned. Existence
/// is reported as `hasGithubToken` / `hasSyncPassword`.
pub fn get(conn: &Connection) -> Result<Settings, AppError> {
    Ok(redact_secrets(load_stored(conn)?))
}

fn load_stored(conn: &Connection) -> Result<Settings, AppError> {
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

fn secret_present(value: Option<&str>) -> bool {
    value.map(|value| !value.is_empty()).unwrap_or(false)
}

fn redact_secrets(mut settings: Settings) -> Settings {
    settings.has_github_token = Some(secret_present(settings.github_token.as_deref()));
    settings.github_token = None;
    settings.has_sync_password = Some(secret_present(
        settings
            .sync
            .as_ref()
            .and_then(|sync| sync.password.as_deref()),
    ));
    if let Some(sync) = settings.sync.as_mut() {
        sync.password = None;
    }
    settings
}

fn nonempty_secret(value: Option<&Value>) -> bool {
    match value {
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Null) | None => false,
        Some(_) => true,
    }
}

fn secret_write_requested(patch: &Value) -> bool {
    nonempty_secret(patch.get("githubToken")) || nonempty_secret(patch.pointer("/sync/password"))
}

/// Keeps persisted secrets when a redacted DTO or a nested `sync` object is
/// written back without a replacement value. Top-level merge replaces the whole
/// `sync` object, so an omitted `password` would otherwise wipe `ENC::` bytes.
fn restore_omitted_secrets(result: &mut Settings, current: &Settings, patch: &Value) {
    if !nonempty_secret(patch.get("githubToken")) {
        result.github_token = current.github_token.clone();
    }
    if !nonempty_secret(patch.pointer("/sync/password")) {
        let stored = current.sync.as_ref().and_then(|sync| sync.password.clone());
        if let Some(sync) = result.sync.as_mut() {
            sync.password = stored;
        }
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
pub fn update(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    patch: &Value,
) -> Result<Settings, AppError> {
    let patch_obj = patch
        .as_object()
        .ok_or_else(|| AppError::validation("settings update must be a JSON object"))?;

    let writing_secrets = secret_write_requested(patch);
    let seal_key = if writing_secrets {
        if !crate::services::security::has_master_password(conn)? {
            return Err(AppError::validation(
                "githubToken and sync.password require a master password",
            ));
        }
        Some(
            crate::services::security::unlocked_key(encryption)?.ok_or_else(|| {
                AppError::locked("unlock the library before saving githubToken or sync.password")
            })?,
        )
    } else {
        None
    };

    // Merge over the stored document (including ENC:: secrets) so a redacted
    // DTO cannot wipe persisted tokens. hasGithubToken / hasSyncPassword on a
    // patch are DTO-only and must not be written back.
    let current = load_stored(conn)?;
    let mut merged = serde_json::to_value(&current)
        .map_err(|e| AppError::internal(format!("failed to encode current settings: {e}")))?;
    let merged_obj = merged
        .as_object_mut()
        .expect("Settings always serializes to a JSON object");

    for (key, value) in patch_obj {
        if key == "hasGithubToken" || key == "hasSyncPassword" {
            continue;
        }
        merged_obj.insert(key.clone(), value.clone());
    }
    merged_obj.remove("hasGithubToken");
    merged_obj.remove("hasSyncPassword");

    // Validate the merged shape before writing so a bad update never mutates
    // stored data (Req 2.3).
    let mut result: Settings = serde_json::from_value(merged)
        .map_err(|e| AppError::validation(format!("invalid settings update: {e}")))?;
    validate(&result)?;
    restore_omitted_secrets(&mut result, &current, patch);

    if let Some(key) = seal_key.as_deref() {
        if let Some(token) = result.github_token.as_mut() {
            if !token.is_empty() && !crate::services::security::is_encrypted_value(token) {
                *token = crate::services::security::encrypt(token, key)?;
            }
        }
        if let Some(password) = result.sync.as_mut().and_then(|sync| sync.password.as_mut()) {
            if !password.is_empty() && !crate::services::security::is_encrypted_value(password) {
                *password = crate::services::security::encrypt(password, key)?;
            }
        }
    }

    result.has_github_token = None;
    result.has_sync_password = None;

    let json = serde_json::to_string(&result)
        .map_err(|e| AppError::internal(format!("failed to encode settings: {e}")))?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![SETTINGS_KEY, json],
    )
    .map_err(|e| AppError::internal(format!("failed to persist settings: {e}")))?;

    Ok(redact_secrets(result))
}

fn validate(settings: &Settings) -> Result<(), AppError> {
    let Some(families) = settings.interface_font_stack.as_ref() else {
        return Ok(());
    };

    if families.is_empty() || families.len() > MAX_INTERFACE_FONT_FAMILIES {
        return Err(AppError::validation(format!(
            "interfaceFontStack must contain 1 to {MAX_INTERFACE_FONT_FAMILIES} families"
        )));
    }

    if families.iter().any(|family| {
        let trimmed = family.trim();
        trimmed.is_empty()
            || trimmed.chars().count() > MAX_FONT_FAMILY_LENGTH
            || trimmed.chars().any(char::is_control)
    }) {
        return Err(AppError::validation(
            "interfaceFontStack contains an invalid font family",
        ));
    }

    Ok(())
}

/// Enumerates the OS-installed font family names, sorted and de-duplicated
/// (settings-appearance-redesign). Best-effort: returns an empty list when no
/// system fonts can be loaded. Each face contributes its primary (English US)
/// family name, so variants like "Arial Bold" collapse onto "Arial".
pub fn list_system_fonts() -> Vec<String> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    let mut names: Vec<String> = db
        .faces()
        .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
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

    fn enc() -> Mutex<EncryptionState> {
        Mutex::new(EncryptionState::default())
    }

    #[test]
    fn list_system_fonts_is_sorted_and_deduplicated() {
        let fonts = list_system_fonts();
        // Invariant holds regardless of how many fonts the host has: the result
        // is strictly ascending (sorted with no adjacent duplicates).
        for pair in fonts.windows(2) {
            assert!(pair[0] < pair[1], "fonts not sorted/unique: {pair:?}");
        }
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
        let result = update(
            &conn,
            &enc(),
            &json!({ "theme": "light", "language": "ja" }),
        )
        .unwrap();
        assert_eq!(result.theme, "light");
        assert_eq!(result.language, "ja");
        // Unspecified required field keeps its default.
        assert!(result.auto_save);
    }

    #[test]
    fn update_leaves_unspecified_fields_unchanged() {
        let conn = conn();
        // First update sets theme + autoSave.
        update(
            &conn,
            &enc(),
            &json!({ "theme": "light", "autoSave": false }),
        )
        .unwrap();
        // Second update touches only language.
        let result = update(&conn, &enc(), &json!({ "language": "fr" })).unwrap();
        assert_eq!(result.language, "fr");
        // Previously-set fields are preserved.
        assert_eq!(result.theme, "light");
        assert!(!result.auto_save);
    }

    #[test]
    fn get_returns_stored_settings_after_update() {
        let conn = conn();
        update(
            &conn,
            &enc(),
            &json!({ "theme": "system", "defaultFolderId": "f1" }),
        )
        .unwrap();
        let stored = get(&conn).unwrap();
        assert_eq!(stored.theme, "system");
        assert_eq!(stored.default_folder_id.as_deref(), Some("f1"));
        assert_eq!(stored.has_github_token, Some(false));
        assert_eq!(stored.has_sync_password, Some(false));
        assert!(stored.github_token.is_none());
        assert_eq!(stored.allow_private_network, None);
    }

    #[test]
    fn update_persists_allow_private_network() {
        let conn = conn();
        assert_eq!(get(&conn).unwrap().allow_private_network, None);
        let result = update(&conn, &enc(), &json!({ "allowPrivateNetwork": true })).unwrap();
        assert_eq!(result.allow_private_network, Some(true));
        assert_eq!(get(&conn).unwrap().allow_private_network, Some(true));
        let cleared = update(&conn, &enc(), &json!({ "allowPrivateNetwork": false })).unwrap();
        assert_eq!(cleared.allow_private_network, Some(false));
    }

    #[test]
    fn update_merges_optional_nested_object() {
        let conn = conn();
        let result = update(
            &conn,
            &enc(),
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
        let err = update(&conn, &enc(), &json!("not-an-object")).unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
    }

    #[test]
    fn update_rejects_wrong_typed_field_without_mutating() {
        let conn = conn();
        // Seed a known value.
        update(&conn, &enc(), &json!({ "theme": "light" })).unwrap();
        // `theme` must be a string; a number is rejected.
        let err = update(&conn, &enc(), &json!({ "theme": 42 })).unwrap_err();
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
            update(&conn, &enc(), &Value::Object(prior_obj)).unwrap();

            // Apply an arbitrary subset of appearance fields as a patch.
            let mut patch_obj = serde_json::Map::new();
            for (i, field) in patch.iter().enumerate() {
                if let Some(v) = field {
                    patch_obj.insert(APPEARANCE_KEYS[i].to_string(), Value::String(v.clone()));
                }
            }
            let result = update(&conn, &enc(), &Value::Object(patch_obj)).unwrap();

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
            update(&conn, &enc(), &patch).unwrap();

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
            &enc(),
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
        update(&conn, &enc(), &json!({ "flavor": "Latte" })).unwrap();
        // `flavor` must be a string; a number is rejected by validate-before-write.
        let err = update(&conn, &enc(), &json!({ "flavor": 42 })).unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
        // The previously stored value is unchanged (no mutation on error).
        assert_eq!(get(&conn).unwrap().flavor.as_deref(), Some("Latte"));
    }

    #[test]
    fn appearance_preferences_round_trip_and_preserve_unrelated_fields() {
        let conn = conn();
        update(
            &conn,
            &enc(),
            &json!({
                "language": "ja",
                "themeFamily": "catppuccin",
                "catppuccinDarkVariant": "macchiato",
                "interfaceFontStack": ["Inter", "Yu Gothic UI"]
            }),
        )
        .unwrap();

        let result = update(&conn, &enc(), &json!({ "themeFamily": "claude" })).unwrap();
        assert_eq!(result.language, "ja");
        assert_eq!(result.theme_family.as_deref(), Some("claude"));
        assert_eq!(result.catppuccin_dark_variant.as_deref(), Some("macchiato"));
        assert_eq!(
            result.interface_font_stack.as_deref(),
            Some(["Inter".to_string(), "Yu Gothic UI".to_string()].as_slice())
        );
        assert_eq!(get(&conn).unwrap(), result);
    }

    #[test]
    fn invalid_font_stack_is_rejected_before_write() {
        let conn = conn();
        update(&conn, &enc(), &json!({ "interfaceFontStack": ["Inter"] })).unwrap();

        for invalid in [
            json!([]),
            json!(["A", "B", "C", "D", "E"]),
            json!(["  "]),
            json!(["bad\nfont"]),
        ] {
            let err = update(&conn, &enc(), &json!({ "interfaceFontStack": invalid })).unwrap_err();
            assert_eq!(err.code_str(), "VALIDATION");
            assert_eq!(
                get(&conn).unwrap().interface_font_stack,
                Some(vec!["Inter".to_string()])
            );
        }

        let err = update(&conn, &enc(), &json!({ "interfaceFontStack": "Inter" })).unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
        assert_eq!(
            get(&conn).unwrap().interface_font_stack,
            Some(vec!["Inter".to_string()])
        );
    }

    #[test]
    fn legacy_settings_json_remains_readable() {
        let conn = conn();
        let legacy = json!({
            "theme": "dark",
            "language": "zh",
            "autoSave": false,
            "flavor": "Mocha",
            "displayFont": "Space Grotesk",
            "bodyFont": "Inter"
        });
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![SETTINGS_KEY, legacy.to_string()],
        )
        .unwrap();

        let settings = get(&conn).unwrap();
        assert_eq!(settings.flavor.as_deref(), Some("Mocha"));
        assert_eq!(settings.body_font.as_deref(), Some("Inter"));
        assert!(settings.theme_family.is_none());
        assert!(settings.catppuccin_dark_variant.is_none());
        assert!(settings.interface_font_stack.is_none());
    }

    #[test]
    fn settings_get_redacts_secrets_and_requires_master_password_to_write_them() {
        let conn = conn();
        let enc = enc();
        let err = update(&conn, &enc, &json!({ "githubToken": "ghp_plaintext" })).unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
        assert!(load_stored(&conn).unwrap().github_token.is_none());

        let err = update(
            &conn,
            &enc,
            &json!({ "sync": { "enabled": true, "provider": "webdav", "password": "sync-secret" } }),
        )
        .unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");

        crate::services::security::set_master_password(&conn, &enc, "password123").unwrap();
        let dto = update(
            &conn,
            &enc,
            &json!({
                "githubToken": "ghp_plaintext",
                "sync": { "enabled": true, "provider": "webdav", "password": "sync-secret" }
            }),
        )
        .unwrap();
        assert_eq!(dto.has_github_token, Some(true));
        assert_eq!(dto.has_sync_password, Some(true));
        assert!(dto.github_token.is_none());
        assert!(dto
            .sync
            .as_ref()
            .and_then(|sync| sync.password.as_ref())
            .is_none());

        let raw: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![SETTINGS_KEY],
                |row| row.get(0),
            )
            .unwrap();
        assert!(raw.contains("ENC::"));
        assert!(!raw.contains("ghp_plaintext"));
        assert!(!raw.contains("sync-secret"));

        let got = get(&conn).unwrap();
        assert_eq!(got.has_github_token, Some(true));
        assert_eq!(got.has_sync_password, Some(true));
        assert!(got.github_token.is_none());
        assert!(serde_json::to_string(&got)
            .unwrap()
            .contains("hasGithubToken"));
        assert!(!serde_json::to_string(&got)
            .unwrap()
            .contains("ghp_plaintext"));
        assert!(!serde_json::to_string(&got).unwrap().contains("sync-secret"));
    }

    #[test]
    fn existing_plaintext_secrets_are_sealed_on_first_master_password() {
        let conn = conn();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![
                SETTINGS_KEY,
                json!({
                    "theme": "dark",
                    "language": "en",
                    "autoSave": true,
                    "githubToken": "ghp_legacy",
                    "sync": { "enabled": true, "provider": "webdav", "password": "legacy-sync" }
                })
                .to_string()
            ],
        )
        .unwrap();
        let enc = enc();
        crate::services::security::set_master_password(&conn, &enc, "password123").unwrap();
        let raw: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![SETTINGS_KEY],
                |row| row.get(0),
            )
            .unwrap();
        assert!(raw.contains("ENC::"));
        assert!(!raw.contains("ghp_legacy"));
        assert!(!raw.contains("legacy-sync"));
        let dto = get(&conn).unwrap();
        assert_eq!(dto.has_github_token, Some(true));
        assert_eq!(dto.has_sync_password, Some(true));
    }

    #[test]
    fn writing_secrets_while_locked_is_locked_and_writes_nothing() {
        let conn = conn();
        let enc = enc();
        crate::services::security::set_master_password(&conn, &enc, "password123").unwrap();
        crate::services::security::lock(&enc).unwrap();
        let err = update(&conn, &enc, &json!({ "githubToken": "ghp_new" })).unwrap_err();
        assert_eq!(err.code_str(), "LOCKED");
        let raw: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![SETTINGS_KEY],
                |row| row.get(0),
            )
            .optional()
            .unwrap()
            .unwrap_or_default();
        assert!(!raw.contains("ghp_new"));
    }

    #[test]
    fn change_master_password_rekeys_settings_secrets() {
        let conn = conn();
        let enc = enc();
        crate::services::security::set_master_password(&conn, &enc, "oldpassword").unwrap();
        update(
            &conn,
            &enc,
            &json!({ "githubToken": "ghp_keep", "sync": { "enabled": true, "provider": "webdav", "password": "sync-keep" } }),
        )
        .unwrap();
        let old_key = crate::services::security::unlocked_key(&enc)
            .unwrap()
            .unwrap();
        crate::services::security::change_master_password(
            &conn,
            &enc,
            "oldpassword",
            "newpassword",
        )
        .unwrap();
        let new_key = crate::services::security::unlocked_key(&enc)
            .unwrap()
            .unwrap();
        let raw: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![SETTINGS_KEY],
                |row| row.get(0),
            )
            .unwrap();
        let stored: Settings = serde_json::from_str(&raw).unwrap();
        let token = stored.github_token.unwrap();
        let password = stored.sync.unwrap().password.unwrap();
        assert!(token.starts_with("ENC::"));
        assert!(crate::services::security::decrypt(&token, &old_key).is_err());
        assert_eq!(
            crate::services::security::decrypt(&token, &new_key).unwrap(),
            "ghp_keep"
        );
        assert_eq!(
            crate::services::security::decrypt(&password, &new_key).unwrap(),
            "sync-keep"
        );
    }

    #[test]
    fn redacted_or_nested_patch_does_not_wipe_persisted_secrets() {
        let conn = conn();
        let enc = enc();
        crate::services::security::set_master_password(&conn, &enc, "password123").unwrap();
        update(
            &conn,
            &enc,
            &json!({
                "githubToken": "ghp_keep",
                "sync": { "enabled": true, "provider": "webdav", "password": "sync-keep" }
            }),
        )
        .unwrap();

        let dto = get(&conn).unwrap();
        update(&conn, &enc, &serde_json::to_value(&dto).unwrap()).unwrap();
        update(
            &conn,
            &enc,
            &json!({
                "sync": { "enabled": true, "provider": "webdav", "endpoint": "https://dav.example" }
            }),
        )
        .unwrap();

        let raw: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![SETTINGS_KEY],
                |row| row.get(0),
            )
            .unwrap();
        let stored: Settings = serde_json::from_str(&raw).unwrap();
        let key = crate::services::security::unlocked_key(&enc)
            .unwrap()
            .unwrap();
        assert_eq!(
            crate::services::security::decrypt(stored.github_token.as_deref().unwrap(), &key)
                .unwrap(),
            "ghp_keep"
        );
        assert_eq!(
            crate::services::security::decrypt(
                stored.sync.unwrap().password.as_deref().unwrap(),
                &key
            )
            .unwrap(),
            "sync-keep"
        );
    }
}
