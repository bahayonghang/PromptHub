//! Property 1: Persistence / serialization round-trip (proptest).
//!
//! *For any* valid domain object (Prompt, Folder, PromptVersion, Settings),
//! persisting it and then reading it back SHALL return
//! an object whose fields are each equal to the stored object, with all
//! collection fields preserved and `createdAt`/`updatedAt` timestamps returned as
//! valid ISO_8601 strings that re-parse to the persisted UTC instant.
//!
//! For the table-backed entities the row is INSERTed with timestamps as epoch
//! milliseconds (the storage representation) and read back through the
//! [`crate::storage::mapping`] row readers; equality is then asserted against the
//! generated struct. Generated timestamps are derived from a random millisecond
//! value via [`millis_to_iso8601`], so the insert (`iso8601_to_millis`) and the
//! readback (`millis_to_iso8601`) compose to the identity on the canonical form.
//!
//! For [`Settings`] (stored as a single JSON value in the key/value `settings`
//! table) the round-trip is JSON serialize -> store -> read -> deserialize.
//!
//! **Validates: Requirements 2.5, 4.9, 6.2, 6.7, 8.2, 8.6, 9.2, 9.3**

use proptest::prelude::*;
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::models::{
    Folder, Prompt, PromptType, PromptVersion, SecuritySettings, Settings, SyncSettings, Variable,
};
use crate::storage::mapping;
use crate::storage::time::{iso8601_to_millis, millis_to_iso8601};
use crate::storage::{create_memory_pool, init_schema, DbPool};

// --------------------------------------------------------------------------
// Shared helpers
// --------------------------------------------------------------------------

/// Builds an in-memory pool with the schema initialized.
fn schema_pool() -> DbPool {
    let pool = create_memory_pool().expect("memory pool");
    init_schema(&pool.get().expect("conn")).expect("schema");
    pool
}

/// Encodes an enum to the wire spelling it is stored as (e.g. `high-risk`).
fn enum_wire<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value).expect("enum serializes") {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    }
}

/// A random millisecond timestamp within a comfortably representable range
/// (≈1900–2100), covering both negative (pre-epoch) and positive instants.
fn ms() -> impl Strategy<Value = i64> {
    -2_208_988_800_000i64..=4_102_444_800_000i64
}

/// A canonical ISO_8601 timestamp string derived from a random millis value.
fn iso() -> impl Strategy<Value = String> {
    ms().prop_map(millis_to_iso8601)
}

/// Arbitrary text including spaces, punctuation, and a little unicode; excludes
/// control characters so it stores cleanly as SQLite TEXT.
fn text() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9 _你好{}.,!#@-]{0,20}").expect("valid regex")
}

/// Non-empty variant of [`text`].
fn nonempty_text() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9 _你好]{1,30}").expect("valid regex")
}

/// Optional [`text`].
fn opt_text() -> impl Strategy<Value = Option<String>> {
    proptest::option::of(text())
}

/// A relative media file path component.
fn file_ref() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9_./-]{1,12}").expect("valid regex")
}

/// A generated identifier.
fn id() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9_-]{1,16}").expect("valid regex")
}

/// A single template variable.
fn variable() -> impl Strategy<Value = Variable> {
    (
        nonempty_text(),
        prop_oneof![
            Just("text"),
            Just("textarea"),
            Just("number"),
            Just("select"),
        ],
        opt_text(),
        opt_text(),
        proptest::option::of(prop::collection::vec(text(), 0..4)),
        any::<bool>(),
    )
        .prop_map(
            |(name, ty, label, default_value, options, required)| Variable {
                name,
                r#type: ty.to_string(),
                label,
                default_value,
                options,
                required,
            },
        )
}

/// A list of template variables.
fn variables() -> impl Strategy<Value = Vec<Variable>> {
    prop::collection::vec(variable(), 0..4)
}

/// A list of free-form tags.
fn tags() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(text(), 0..4)
}

/// A list of media file references.
fn refs() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(file_ref(), 0..4)
}

// --------------------------------------------------------------------------
// Entity strategies
// --------------------------------------------------------------------------

/// Strategy producing an arbitrary valid [`Prompt`].
fn prompt_strategy() -> impl Strategy<Value = Prompt> {
    let core = (
        id(),
        nonempty_text(),
        opt_text(),
        prop_oneof![
            Just(PromptType::Text),
            Just(PromptType::Image),
            Just(PromptType::Video),
        ],
        opt_text(),
        nonempty_text(),
    );
    let collections = (
        variables(),
        tags(),
        proptest::option::of(id()),
        refs(),
        refs(),
    );
    let flags = (any::<bool>(), any::<bool>(), 0i64..10_000, 0i64..10_000);
    let opt = (opt_text(), opt_text(), opt_text());
    let ts = (iso(), iso());

    (core, collections, flags, opt, ts).prop_map(
        |(
            (id, title, description, prompt_type, system_prompt, user_prompt),
            (variables, tags, folder_id, images, videos),
            (is_favorite, is_pinned, current_version, usage_count),
            (source, notes, last_ai_response),
            (created_at, updated_at),
        )| Prompt {
            id,
            title,
            description,
            prompt_type,
            system_prompt,
            user_prompt,
            variables,
            tags,
            folder_id,
            images,
            videos,
            is_favorite,
            is_pinned,
            current_version,
            usage_count,
            source,
            notes,
            last_ai_response,
            created_at,
            updated_at,
        },
    )
}

/// Strategy producing an arbitrary valid [`PromptVersion`].
fn prompt_version_strategy() -> impl Strategy<Value = PromptVersion> {
    (
        id(),
        id(),
        1i64..10_000,
        opt_text(),
        nonempty_text(),
        variables(),
        opt_text(),
        opt_text(),
        iso(),
    )
        .prop_map(
            |(
                id,
                prompt_id,
                version,
                system_prompt,
                user_prompt,
                variables,
                note,
                ai_response,
                created_at,
            )| PromptVersion {
                id,
                prompt_id,
                version,
                system_prompt,
                user_prompt,
                variables,
                note,
                ai_response,
                created_at,
            },
        )
}

/// Strategy producing an arbitrary valid [`Folder`].
///
/// Half the time the folder references a parent; the parent id is prefixed so it
/// can never collide with the child's own (prefixed) id when both are inserted.
fn folder_strategy() -> impl Strategy<Value = Folder> {
    (
        id(),
        nonempty_text(),
        opt_text(),
        proptest::option::of(id()),
        0i64..10_000,
        iso(),
        proptest::option::of(iso()),
    )
        .prop_map(
            |(seed, name, icon, parent_seed, sort_order, created_at, updated_at)| Folder {
                id: format!("child-{seed}"),
                name,
                icon,
                parent_id: parent_seed.map(|p| format!("parent-{p}")),
                sort_order,
                created_at,
                updated_at,
            },
        )
}

/// Strategy producing arbitrary [`SyncSettings`].
fn sync_settings() -> impl Strategy<Value = SyncSettings> {
    (
        any::<bool>(),
        prop_oneof![
            Just("manual"),
            Just("webdav"),
            Just("self-hosted"),
            Just("s3"),
        ],
        opt_text(),
        opt_text(),
        opt_text(),
        opt_text(),
        proptest::option::of(any::<bool>()),
        proptest::option::of(iso()),
    )
        .prop_map(
            |(
                enabled,
                provider,
                endpoint,
                username,
                password,
                remote_path,
                auto_sync,
                last_sync_at,
            )| {
                SyncSettings {
                    enabled,
                    provider: provider.to_string(),
                    endpoint,
                    username,
                    password,
                    remote_path,
                    auto_sync,
                    last_sync_at,
                }
            },
        )
}

/// Strategy producing arbitrary [`SecuritySettings`].
fn security_settings() -> impl Strategy<Value = SecuritySettings> {
    (any::<bool>(), any::<bool>()).prop_map(|(master_password_configured, unlocked)| {
        SecuritySettings {
            master_password_configured,
            unlocked,
        }
    })
}

/// Strategy producing arbitrary valid [`Settings`].
fn settings_strategy() -> impl Strategy<Value = Settings> {
    let g1 = (
        prop_oneof![Just("light"), Just("dark"), Just("system")],
        prop_oneof![
            Just("en"),
            Just("zh"),
            Just("zh-TW"),
            Just("ja"),
            Just("fr"),
            Just("de"),
            Just("es"),
        ],
        any::<bool>(),
        proptest::option::of(prop_oneof![Just("single"), Just("multi")]),
        proptest::option::of(prop::collection::vec(text(), 0..4)),
        opt_text(),
    );
    // Opacity (0.00–1.00) and blur radius (0–25px in 0.5 steps) are generated as
    // quantized values from a UI slider rather than arbitrary `f64`. Arbitrary
    // full-precision doubles can lose 1 ULP through serde_json's shortest-decimal
    // float formatting, which is a JSON-numeral limitation unrelated to the
    // persistence property under test.
    let opacity = proptest::option::of((0u32..=100).prop_map(|n| f64::from(n) / 100.0));
    let blur = proptest::option::of((0u32..=50).prop_map(|n| f64::from(n) / 2.0));
    let g2 = (
        opt_text(),
        opacity,
        blur,
        proptest::option::of(iso()),
        opt_text(),
    );
    let g3 = (
        proptest::option::of(sync_settings()),
        proptest::option::of(prop_oneof![Just("stable"), Just("preview")]),
        proptest::option::of(any::<bool>()),
        proptest::option::of(any::<bool>()),
        opt_text(),
        proptest::option::of(security_settings()),
    );

    (g1, g2, g3).prop_map(
        |(
            (theme, language, auto_save, tag_filter_mode, prompt_tag_catalog, default_folder_id),
            (
                background_image_file_name,
                background_image_opacity,
                background_image_blur,
                last_manual_backup_at,
                last_manual_backup_version,
            ),
            (sync, update_channel, launch_at_startup, minimize_on_launch, github_token, security),
        )| Settings {
            theme: theme.to_string(),
            language: language.to_string(),
            auto_save,
            tag_filter_mode: tag_filter_mode.map(String::from),
            prompt_tag_catalog,
            default_folder_id,
            background_image_file_name,
            background_image_opacity,
            background_image_blur,
            last_manual_backup_at,
            last_manual_backup_version,
            sync,
            update_channel: update_channel.map(String::from),
            launch_at_startup,
            minimize_on_launch,
            github_token,
            security,
            // Appearance fields are not exercised by this round-trip; default them.
            ..Default::default()
        },
    )
}

// --------------------------------------------------------------------------
// Insert helpers (domain struct -> storage row)
// --------------------------------------------------------------------------

/// Converts an ISO_8601 string back into the epoch-millisecond storage form.
fn to_millis(iso: &str) -> i64 {
    iso8601_to_millis(iso).expect("generated timestamp is valid ISO_8601")
}

/// Inserts a minimal folder row so a foreign-key reference resolves.
fn insert_min_folder(conn: &Connection, fid: &str) {
    conn.execute(
        "INSERT OR IGNORE INTO folders (id,name,created_at) VALUES (?1,'parent',0)",
        params![fid],
    )
    .expect("insert minimal folder");
}

/// Inserts a minimal prompt row so a foreign-key reference resolves.
fn insert_min_prompt(conn: &Connection, pid: &str) {
    conn.execute(
        "INSERT OR IGNORE INTO prompts (id,title,user_prompt,created_at,updated_at) \
         VALUES (?1,'T','U',0,0)",
        params![pid],
    )
    .expect("insert minimal prompt");
}

fn insert_prompt(conn: &Connection, p: &Prompt) {
    if let Some(fid) = &p.folder_id {
        insert_min_folder(conn, fid);
    }
    conn.execute(
        "INSERT INTO prompts \
         (id,title,description,prompt_type,system_prompt,user_prompt,variables,tags,folder_id,\
          images,videos,is_favorite,is_pinned,current_version,usage_count,source,notes,\
          last_ai_response,created_at,updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
        params![
            p.id,
            p.title,
            p.description,
            enum_wire(&p.prompt_type),
            p.system_prompt,
            p.user_prompt,
            serde_json::to_string(&p.variables).unwrap(),
            serde_json::to_string(&p.tags).unwrap(),
            p.folder_id,
            serde_json::to_string(&p.images).unwrap(),
            serde_json::to_string(&p.videos).unwrap(),
            p.is_favorite,
            p.is_pinned,
            p.current_version,
            p.usage_count,
            p.source,
            p.notes,
            p.last_ai_response,
            to_millis(&p.created_at),
            to_millis(&p.updated_at),
        ],
    )
    .expect("insert prompt");
}

fn insert_prompt_version(conn: &Connection, v: &PromptVersion) {
    insert_min_prompt(conn, &v.prompt_id);
    conn.execute(
        "INSERT INTO prompt_versions \
         (id,prompt_id,version,system_prompt,user_prompt,variables,note,ai_response,created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            v.id,
            v.prompt_id,
            v.version,
            v.system_prompt,
            v.user_prompt,
            serde_json::to_string(&v.variables).unwrap(),
            v.note,
            v.ai_response,
            to_millis(&v.created_at),
        ],
    )
    .expect("insert prompt version");
}

fn insert_folder(conn: &Connection, f: &Folder) {
    if let Some(pid) = &f.parent_id {
        insert_min_folder(conn, pid);
    }
    conn.execute(
        "INSERT INTO folders (id,name,icon,parent_id,sort_order,created_at,updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            f.id,
            f.name,
            f.icon,
            f.parent_id,
            f.sort_order,
            to_millis(&f.created_at),
            f.updated_at.as_deref().map(to_millis),
        ],
    )
    .expect("insert folder");
}

/// Asserts the timestamp strings produced by the mapping are valid ISO_8601 that
/// re-parse to the same UTC instant they encode (Property 1 timestamp clause).
fn assert_iso_roundtrips(iso: &str) {
    let millis = iso8601_to_millis(iso).expect("mapped timestamp is valid ISO_8601");
    assert_eq!(
        millis_to_iso8601(millis),
        iso,
        "mapped timestamp must be the canonical ISO_8601 form"
    );
}

// --------------------------------------------------------------------------
// Property 1: persistence / serialization round-trip
// --------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// **Validates: Requirements 2.5, 4.9, 6.2, 6.7**
    #[test]
    fn prompt_persistence_round_trip(prompt in prompt_strategy()) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        insert_prompt(&conn, &prompt);

        let read = conn
            .query_row("SELECT * FROM prompts WHERE id = ?1", [&prompt.id], |row| {
                mapping::prompt_from_row(row)
            })
            .unwrap();

        prop_assert_eq!(&read, &prompt);
        assert_iso_roundtrips(&read.created_at);
        assert_iso_roundtrips(&read.updated_at);
    }

    /// **Validates: Requirements 2.5, 4.9**
    #[test]
    fn prompt_version_persistence_round_trip(version in prompt_version_strategy()) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        insert_prompt_version(&conn, &version);

        let read = conn
            .query_row(
                "SELECT * FROM prompt_versions WHERE id = ?1",
                [&version.id],
                mapping::prompt_version_from_row,
            )
            .unwrap();

        prop_assert_eq!(&read, &version);
        assert_iso_roundtrips(&read.created_at);
    }

    /// **Validates: Requirements 2.5, 4.9, 8.2, 8.6**
    #[test]
    fn folder_persistence_round_trip(folder in folder_strategy()) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        insert_folder(&conn, &folder);

        let read = conn
            .query_row("SELECT * FROM folders WHERE id = ?1", [&folder.id], |row| {
                mapping::folder_from_row(row)
            })
            .unwrap();

        prop_assert_eq!(&read, &folder);
        assert_iso_roundtrips(&read.created_at);
        if let Some(updated) = &read.updated_at {
            assert_iso_roundtrips(updated);
        }
    }

    /// Settings persist as a single JSON value in the key/value `settings` table;
    /// the round-trip is JSON serialize -> store -> read -> deserialize.
    ///
    /// **Validates: Requirements 2.5**
    #[test]
    fn settings_persistence_round_trip(settings in settings_strategy()) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let json = serde_json::to_string(&settings).unwrap();
        conn.execute(
            "INSERT INTO settings (key,value) VALUES ('app',?1)",
            params![json],
        )
        .unwrap();

        let stored: String = conn
            .query_row("SELECT value FROM settings WHERE key = 'app'", [], |row| {
                row.get(0)
            })
            .unwrap();
        let read: Settings = serde_json::from_str(&stored).unwrap();

        prop_assert_eq!(read, settings);
    }
}
