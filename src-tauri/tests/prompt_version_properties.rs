//! Property-based tests for the Prompt_Service and Version_Service (task 4.5).
//!
//! These run as an **integration test** against the public `prompthub_lib` API
//! (`services::prompt::*`, `services::version::*`, `storage::*`, `models::*`,
//! `error::*`), so they need no edits to any `mod.rs`. Each test builds a fresh
//! in-memory database ([`create_memory_pool`] + [`init_schema`]) and drives the
//! services through their public functions, exactly as the Command_Layer
//! (task 17.1) will.
//!
//! Properties implemented (design "Testing Strategy"):
//!   - Property 4:  Mutating commands never mutate on error
//!   - Property 5:  Version number monotonicity
//!   - Property 6:  Rollback restores the snapshot
//!   - Property 7:  Partial update preserves unsupplied fields
//!   - Property 14: Copy substitutes matched placeholders only
//!   - Property 15: Tag rename is non-duplicating and idempotent
//!   - Property 16: Tag delete removes the tag everywhere
//!   - Property 17: Distinct tag aggregation
//!   - Property 18: Required-field create validation
//!   - Property 19: promptType domain validation and default
//!   - Property 20: Version note length validation
//!
//! **Validates: Requirements 2.3, 2.7, 6.4, 6.8, 6.9, 6.10, 6.11, 6.13, 6.14,
//! 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8**

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use proptest::prelude::*;
use proptest::sample::select;
use rusqlite::params;

use prompthub_lib::error::ErrorCode;
use prompthub_lib::models::{PromptType, Variable};
use prompthub_lib::services::prompt::{self, PromptCreate, PromptUpdate};
use prompthub_lib::services::version;
use prompthub_lib::storage::{create_memory_pool, init_schema, DbPool};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// An identifier that can never collide with a generated v4 UUID, used to drive
/// the `NOT_FOUND` failure paths.
const MISSING_ID: &str = "nonexistent-prompt-id";

/// Maximum note length accepted by [`version::create`] (Req 7.8).
const MAX_NOTE_CHARS: usize = 1000;

/// Builds an in-memory pool with the schema initialized.
fn schema_pool() -> DbPool {
    let pool = create_memory_pool().expect("memory pool");
    init_schema(&pool.get().expect("conn")).expect("schema");
    pool
}

/// A minimal valid [`PromptCreate`] with the given title/user prompt and all
/// other fields defaulted.
fn mk_create(title: &str, user_prompt: &str) -> PromptCreate {
    PromptCreate {
        title: title.to_string(),
        user_prompt: user_prompt.to_string(),
        ..Default::default()
    }
}

/// The wire spelling a [`PromptType`] is stored/serialized as.
fn type_wire(t: PromptType) -> &'static str {
    match t {
        PromptType::Text => "text",
        PromptType::Image => "image",
        PromptType::Video => "video",
    }
}

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Small text values (may be empty) used for nullable string fields.
fn small_text() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9 ]{0,12}").unwrap()
}

/// A non-blank text value (non-empty after trimming).
fn nonblank_text() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9]{1,12}").unwrap()
}

/// A `Variable` with all field variants exercised.
fn variable_strat() -> impl Strategy<Value = Variable> {
    (
        proptest::string::string_regex("[a-z]{1,6}").unwrap(),
        select(vec!["text", "textarea", "number", "select"]),
        proptest::option::of(small_text()),
        proptest::option::of(small_text()),
        proptest::option::of(prop::collection::vec(
            proptest::string::string_regex("[a-z]{1,4}").unwrap(),
            0..=3,
        )),
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

/// A bounded list of variables.
fn variables_strat() -> impl Strategy<Value = Vec<Variable>> {
    prop::collection::vec(variable_strat(), 0..=3)
}

/// Tags drawn from a small alphabet (so renames/deletes hit real collisions),
/// deduplicated while preserving first-seen order.
fn tag_set() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(select(vec!["a", "b", "c", "d", "e"]), 0..=5).prop_map(|tags| {
        let mut seen = HashSet::new();
        tags.into_iter()
            .map(String::from)
            .filter(|t| seen.insert(t.clone()))
            .collect()
    })
}

/// Title/user-prompt candidate biased toward the create-validation boundary:
/// empty, whitespace-only, or a non-blank value.
fn maybe_blank() -> impl Strategy<Value = String> {
    prop_oneof![
        1 => Just(String::new()),
        1 => proptest::string::string_regex("[ \\t\\n\\r]{1,4}").unwrap(),
        3 => nonblank_text(),
    ]
}

// ---------------------------------------------------------------------------
// Snapshot helpers for Property 4 (no mutation on error)
// ---------------------------------------------------------------------------

/// A complete, comparable snapshot of all persisted prompt + version state.
///
/// Maps are sorted (`BTreeMap`) so equality is order-independent, capturing
/// every field the mutating commands could touch.
#[derive(Debug, Clone, PartialEq)]
struct DbSnapshot {
    prompts: BTreeMap<String, prompthub_lib::models::Prompt>,
    versions: BTreeMap<String, Vec<i64>>,
}

/// Captures the full prompt + version state via the public list/query APIs.
fn snapshot(conn: &rusqlite::Connection) -> DbSnapshot {
    let prompts: BTreeMap<String, prompthub_lib::models::Prompt> = prompt::list(conn)
        .unwrap()
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect();
    let versions = prompts
        .keys()
        .map(|id| {
            let nums: Vec<i64> = version::list(conn, id)
                .unwrap()
                .into_iter()
                .map(|v| v.version)
                .collect();
            (id.clone(), nums)
        })
        .collect();
    DbSnapshot { prompts, versions }
}

/// Seeds a handful of prompts (some with versions) so a snapshot is non-trivial.
/// Returns the id of the first prompt for use as a valid-but-target row.
fn seed_corpus(conn: &rusqlite::Connection) -> String {
    let a = prompt::create(conn, mk_create("Alpha", "body a")).unwrap();
    let b = prompt::create(conn, mk_create("Beta", "body b")).unwrap();
    let _c = prompt::create(conn, mk_create("Gamma", "body c")).unwrap();
    version::create(conn, &a.id, Some("v1".into())).unwrap();
    version::create(conn, &a.id, None).unwrap();
    version::create(conn, &b.id, None).unwrap();
    a.id
}

// ---------------------------------------------------------------------------
// Property 18: Required-field create validation
// Property 19: promptType domain validation and default
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// **Property 18: Required-field create validation.**
    ///
    /// `create` succeeds iff both `title` and `userPrompt` are non-empty after
    /// trimming; otherwise it returns `VALIDATION` and creates no record.
    ///
    /// **Validates: Requirements 6.13**
    #[test]
    fn create_requires_nonblank_title_and_user_prompt(
        title in maybe_blank(),
        user_prompt in maybe_blank(),
    ) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let valid = !title.trim().is_empty() && !user_prompt.trim().is_empty();

        match prompt::create(&conn, mk_create(&title, &user_prompt)) {
            Ok(p) => {
                prop_assert!(valid, "accepted blank title/userPrompt");
                prop_assert_eq!(prompt::list(&conn).unwrap().len(), 1);
                // Stored verbatim (no trimming of the body on create).
                prop_assert_eq!(p.title, title);
                prop_assert_eq!(p.user_prompt, user_prompt);
            }
            Err(err) => {
                prop_assert!(!valid, "rejected a valid create");
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert!(prompt::list(&conn).unwrap().is_empty(), "no record on reject");
            }
        }
    }

    /// **Property 19: promptType domain validation and default.**
    ///
    /// A missing `promptType` defaults to `text`; a value in {text,image,video}
    /// is accepted and persisted; any other value is rejected with `VALIDATION`,
    /// leaving stored data unchanged.
    ///
    /// **Validates: Requirements 6.6, 6.14**
    #[test]
    fn prompt_type_domain_and_default(
        // None => omitted (defaults to text); Some => an arbitrary string that
        // may or may not be in the domain.
        raw in proptest::option::of(prop_oneof![
            select(vec!["text", "image", "video"]).prop_map(String::from),
            proptest::string::string_regex("[a-zA-Z]{1,8}").unwrap(),
        ]),
    ) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let expected_type = match raw.as_deref() {
            None => Some(PromptType::Text),
            Some("text") => Some(PromptType::Text),
            Some("image") => Some(PromptType::Image),
            Some("video") => Some(PromptType::Video),
            Some(_) => None, // out of domain
        };

        let input = PromptCreate {
            title: "T".into(),
            user_prompt: "U".into(),
            prompt_type: raw.clone(),
            ..Default::default()
        };

        match prompt::create(&conn, input) {
            Ok(p) => {
                let want = expected_type.expect("accepted an out-of-domain promptType");
                prop_assert_eq!(p.prompt_type, want);
                // Persisted: re-read returns the same type.
                let got = prompt::get(&conn, &p.id).unwrap();
                prop_assert_eq!(got.prompt_type, want);
                prop_assert_eq!(type_wire(got.prompt_type), type_wire(want));
            }
            Err(err) => {
                prop_assert!(expected_type.is_none(), "rejected an in-domain promptType");
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert!(prompt::list(&conn).unwrap().is_empty(), "no record on reject");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property 4: Mutating commands never mutate on error
// ---------------------------------------------------------------------------

/// The space of failing mutations exercised by Property 4. Each variant names a
/// mutating command invoked with arguments that must fail (validation or
/// not-found), and therefore must leave the store byte-for-byte unchanged.
#[derive(Debug, Clone)]
enum FailingOp {
    /// create with a blank title (VALIDATION).
    CreateBlankTitle,
    /// create with a blank userPrompt (VALIDATION).
    CreateBlankUserPrompt,
    /// create with an out-of-domain promptType (VALIDATION).
    CreateBadType(String),
    /// update a missing prompt (NOT_FOUND).
    UpdateMissing,
    /// update an existing prompt with a bad promptType (VALIDATION).
    UpdateBadType(String),
    /// delete a missing prompt (NOT_FOUND).
    DeleteMissing,
    /// create a version for a missing prompt (NOT_FOUND).
    VersionCreateMissing,
    /// create a version with an over-length note (VALIDATION).
    VersionCreateLongNote,
    /// rollback a missing prompt (NOT_FOUND).
    RollbackMissingPrompt,
    /// rollback to a missing version on an existing prompt (NOT_FOUND).
    RollbackMissingVersion,
}

fn failing_op_strat() -> impl Strategy<Value = FailingOp> {
    prop_oneof![
        Just(FailingOp::CreateBlankTitle),
        Just(FailingOp::CreateBlankUserPrompt),
        proptest::string::string_regex("[a-z]{1,6}")
            .unwrap()
            .prop_map(FailingOp::CreateBadType),
        Just(FailingOp::UpdateMissing),
        proptest::string::string_regex("[a-z]{1,6}")
            .unwrap()
            .prop_map(FailingOp::UpdateBadType),
        Just(FailingOp::DeleteMissing),
        Just(FailingOp::VersionCreateMissing),
        Just(FailingOp::VersionCreateLongNote),
        Just(FailingOp::RollbackMissingPrompt),
        Just(FailingOp::RollbackMissingVersion),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// **Property 4: Mutating commands never mutate on error.**
    ///
    /// For any failing mutating command, the call returns a structured error
    /// (stable code + message) and the full prompt + version snapshot is
    /// identical to its pre-invocation state.
    ///
    /// **Validates: Requirements 2.3, 2.7**
    #[test]
    fn failing_mutations_leave_store_unchanged(op in failing_op_strat()) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let existing_id = seed_corpus(&conn);

        let before = snapshot(&conn);

        // For the bad-type variant we need a value guaranteed out of domain.
        let out_of_domain = |s: &str| !matches!(s, "text" | "image" | "video");

        let err = match &op {
            FailingOp::CreateBlankTitle => {
                prompt::create(&conn, mk_create("   ", "body")).unwrap_err()
            }
            FailingOp::CreateBlankUserPrompt => {
                prompt::create(&conn, mk_create("title", "   ")).unwrap_err()
            }
            FailingOp::CreateBadType(s) => {
                prop_assume!(out_of_domain(s));
                let input = PromptCreate {
                    title: "T".into(),
                    user_prompt: "U".into(),
                    prompt_type: Some(s.clone()),
                    ..Default::default()
                };
                prompt::create(&conn, input).unwrap_err()
            }
            FailingOp::UpdateMissing => {
                prompt::update(&conn, MISSING_ID, PromptUpdate {
                    title: Some("new".into()),
                    ..Default::default()
                }).unwrap_err()
            }
            FailingOp::UpdateBadType(s) => {
                prop_assume!(out_of_domain(s));
                prompt::update(&conn, &existing_id, PromptUpdate {
                    prompt_type: Some(s.clone()),
                    ..Default::default()
                }).unwrap_err()
            }
            FailingOp::DeleteMissing => {
                prompt::delete(&conn, MISSING_ID).unwrap_err()
            }
            FailingOp::VersionCreateMissing => {
                version::create(&conn, MISSING_ID, None).unwrap_err()
            }
            FailingOp::VersionCreateLongNote => {
                let long = "x".repeat(MAX_NOTE_CHARS + 1);
                version::create(&conn, &existing_id, Some(long)).unwrap_err()
            }
            FailingOp::RollbackMissingPrompt => {
                version::rollback(&conn, MISSING_ID, 1).unwrap_err()
            }
            FailingOp::RollbackMissingVersion => {
                version::rollback(&conn, &existing_id, 9999).unwrap_err()
            }
        };

        // Structured error: a stable code and a non-empty message.
        prop_assert!(matches!(
            err.code,
            ErrorCode::Validation | ErrorCode::NotFound
        ), "unexpected error code {:?} for {:?}", err.code, op);
        prop_assert!(!err.message.is_empty(), "error must carry a message");

        // The persistent store is unchanged from its pre-invocation snapshot.
        prop_assert_eq!(snapshot(&conn), before, "store mutated on error for {:?}", op);
    }
}

// ---------------------------------------------------------------------------
// Property 5: Version number monotonicity
// Property 20: Version note length validation
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// **Property 5: Version number monotonicity.**
    ///
    /// A sequence of `version::create` calls assigns numbers 1, 2, 3, ... with no
    /// gaps or duplicates, and `version::list` reflects the same ascending
    /// sequence; the prompt's `currentVersion` tracks the latest.
    ///
    /// **Validates: Requirements 7.2**
    #[test]
    fn version_numbers_are_strictly_monotonic(n in 1usize..=12) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = prompt::create(&conn, mk_create("T", "U")).unwrap().id;

        let assigned: Vec<i64> = (0..n)
            .map(|_| version::create(&conn, &id, None).unwrap().version)
            .collect();

        let assigned_expected: Vec<i64> = (2..=n as i64 + 1).collect();
        prop_assert_eq!(&assigned, &assigned_expected, "create returned non-monotonic versions");

        let listed: Vec<i64> = version::list(&conn, &id)
            .unwrap()
            .into_iter()
            .map(|v| v.version)
            .collect();
        let listed_expected: Vec<i64> = (1..=n as i64 + 1).collect();
        prop_assert_eq!(&listed, &listed_expected, "list disagrees with assigned versions");

        prop_assert_eq!(prompt::get(&conn, &id).unwrap().current_version, n as i64 + 1);
    }

    /// **Property 20: Version note length validation.**
    ///
    /// A note of at most 1000 characters is accepted; a note exceeding 1000
    /// characters is rejected with `VALIDATION`, creating no version.
    ///
    /// **Validates: Requirements 7.8**
    #[test]
    fn version_note_length_validation(len in 0usize..=1003) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let id = prompt::create(&conn, mk_create("T", "U")).unwrap().id;

        // Use a multi-byte character to confirm the limit counts characters, not
        // bytes.
        let note = "é".repeat(len);
        let valid = len <= MAX_NOTE_CHARS;

        match version::create(&conn, &id, Some(note.clone())) {
            Ok(v) => {
                prop_assert!(valid, "accepted a note of {len} chars");
                prop_assert_eq!(v.note.as_deref(), Some(note.as_str()));
                prop_assert_eq!(version::list(&conn, &id).unwrap().len(), 2);
            }
            Err(err) => {
                prop_assert!(!valid, "rejected a note of {len} chars");
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert_eq!(version::list(&conn, &id).unwrap().len(), 1, "no additional version on reject");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property 6: Rollback restores the snapshot
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// **Property 6: Rollback restores the snapshot.**
    ///
    /// Capturing a version snapshot, performing arbitrary intermediate mutations,
    /// and rolling back to that version restores the snapshotted `systemPrompt`,
    /// `userPrompt`, and `variables`, and refreshes `updatedAt` to a value no
    /// earlier than before.
    ///
    /// **Validates: Requirements 7.3**
    #[test]
    fn rollback_restores_snapshotted_fields(
        sys0 in proptest::option::of(small_text()),
        user0 in nonblank_text(),
        vars0 in variables_strat(),
        sys1 in proptest::option::of(small_text()),
        user1 in nonblank_text(),
        vars1 in variables_strat(),
    ) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        // Create a prompt in state 0.
        let created = prompt::create(&conn, PromptCreate {
            title: "T".into(),
            user_prompt: user0.clone(),
            system_prompt: sys0.clone(),
            variables: Some(vars0.clone()),
            ..Default::default()
        }).unwrap();
        let id = created.id;

        // Snapshot state 0 as version 1.
        let snap = version::create(&conn, &id, None).unwrap();
        prop_assert_eq!(snap.system_prompt.clone(), sys0.clone());
        prop_assert_eq!(&snap.user_prompt, &user0);
        prop_assert_eq!(&snap.variables, &vars0);

        // Mutate to state 1.
        prompt::update(&conn, &id, PromptUpdate {
            user_prompt: Some(user1.clone()),
            system_prompt: sys1.clone(),
            variables: Some(vars1.clone()),
            ..Default::default()
        }).unwrap();

        // Force a strictly earlier updated_at so the rollback refresh is visible
        // even when the wall clock has not advanced a millisecond.
        conn.execute(
            "UPDATE prompts SET updated_at = updated_at - 1000 WHERE id = ?1",
            params![id],
        ).unwrap();
        let baseline = prompt::get(&conn, &id).unwrap();

        // Roll back to version 1.
        let restored = version::rollback(&conn, &id, 1).unwrap();

        // Snapshotted fields restored exactly.
        prop_assert_eq!(restored.system_prompt.clone(), sys0.clone());
        prop_assert_eq!(&restored.user_prompt, &user0);
        prop_assert_eq!(&restored.variables, &vars0);

        // updatedAt advanced (no earlier than the prior value). Timestamps are
        // canonical millisecond ISO_8601, so lexical >= equals chronological >=.
        prop_assert!(
            restored.updated_at >= baseline.updated_at,
            "updatedAt regressed: {} < {}",
            restored.updated_at,
            baseline.updated_at
        );
        prop_assert!(restored.updated_at > baseline.updated_at,
            "updatedAt should strictly advance after the forced backdate");
    }
}

// ---------------------------------------------------------------------------
// Property 7: Partial update preserves unsupplied fields
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// **Property 7: Partial update preserves unsupplied fields.**
    ///
    /// Applying a partial patch sets exactly the supplied fields to their values,
    /// leaves every unsupplied field unchanged, keeps `createdAt` fixed, and sets
    /// `updatedAt` to a value no earlier than before. Each `Some` patch field is
    /// checked against the supplied value; each `None` against the original.
    ///
    /// **Validates: Requirements 6.4**
    #[test]
    fn partial_update_preserves_unsupplied_fields(
        p_title in proptest::option::of(nonblank_text()),
        p_user in proptest::option::of(nonblank_text()),
        p_desc in proptest::option::of(small_text()),
        p_type in proptest::option::of(select(vec!["text", "image", "video"]).prop_map(String::from)),
        p_sys in proptest::option::of(small_text()),
        p_vars in proptest::option::of(variables_strat()),
        p_tags in proptest::option::of(tag_set()),
        p_fav in proptest::option::of(any::<bool>()),
        p_pin in proptest::option::of(any::<bool>()),
        p_usage in proptest::option::of(0i64..1000),
        p_source in proptest::option::of(small_text()),
        p_notes in proptest::option::of(small_text()),
    ) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        // Seed with known, non-default values so "preserved" is observable.
        let original = prompt::create(&conn, PromptCreate {
            title: "orig-title".into(),
            user_prompt: "orig-user".into(),
            description: Some("orig-desc".into()),
            prompt_type: Some("image".into()),
            system_prompt: Some("orig-sys".into()),
            variables: Some(vec![Variable {
                name: "orig".into(),
                r#type: "text".into(),
                label: None,
                default_value: None,
                options: None,
                required: true,
            }]),
            tags: Some(vec!["x".into(), "y".into()]),
            images: Some(vec!["i.png".into()]),
            videos: Some(vec!["v.mp4".into()]),
            is_favorite: Some(true),
            is_pinned: Some(true),
            usage_count: Some(7),
            source: Some("orig-src".into()),
            notes: Some("orig-notes".into()),
            ..Default::default()
        }).unwrap();
        let id = original.id.clone();

        // Backdate so an advance in updatedAt is detectable.
        conn.execute(
            "UPDATE prompts SET updated_at = updated_at - 1000 WHERE id = ?1",
            params![id],
        ).unwrap();
        let before = prompt::get(&conn, &id).unwrap();

        let patch = PromptUpdate {
            type_definition_id: None,
            title: p_title.clone(),
            user_prompt: p_user.clone(),
            description: p_desc.clone(),
            prompt_type: p_type.clone(),
            system_prompt: p_sys.clone(),
            messages: None,
            variables: p_vars.clone(),
            tags: p_tags.clone(),
            folder_id: None,
            images: None,
            videos: None,
            is_favorite: p_fav,
            is_pinned: p_pin,
            is_private: None,
            usage_count: p_usage,
            source: p_source.clone(),
            notes: p_notes.clone(),
            last_ai_response: None,
        };

        let updated = prompt::update(&conn, &id, patch).unwrap();

        // Each supplied field equals the supplied value; each unsupplied field
        // equals the original.
        match &p_title {
            Some(v) => prop_assert_eq!(&updated.title, v),
            None => prop_assert_eq!(&updated.title, &before.title),
        }
        match &p_user {
            Some(v) => prop_assert_eq!(&updated.user_prompt, v),
            None => prop_assert_eq!(&updated.user_prompt, &before.user_prompt),
        }
        match &p_desc {
            Some(v) => prop_assert_eq!(updated.description.as_deref(), Some(v.as_str())),
            None => prop_assert_eq!(&updated.description, &before.description),
        }
        match &p_type {
            Some(v) => {
                let want = match v.as_str() {
                    "text" => PromptType::Text,
                    "image" => PromptType::Image,
                    _ => PromptType::Video,
                };
                prop_assert_eq!(updated.prompt_type, want);
            }
            None => prop_assert_eq!(updated.prompt_type, before.prompt_type),
        }
        match &p_sys {
            Some(v) => prop_assert_eq!(updated.system_prompt.as_deref(), Some(v.as_str())),
            None => prop_assert_eq!(&updated.system_prompt, &before.system_prompt),
        }
        match &p_vars {
            Some(v) => prop_assert_eq!(&updated.variables, v),
            None => prop_assert_eq!(&updated.variables, &before.variables),
        }
        match &p_tags {
            Some(v) => prop_assert_eq!(&updated.tags, v),
            None => prop_assert_eq!(&updated.tags, &before.tags),
        }
        match p_fav {
            Some(v) => prop_assert_eq!(updated.is_favorite, v),
            None => prop_assert_eq!(updated.is_favorite, before.is_favorite),
        }
        match p_pin {
            Some(v) => prop_assert_eq!(updated.is_pinned, v),
            None => prop_assert_eq!(updated.is_pinned, before.is_pinned),
        }
        match p_usage {
            Some(v) => prop_assert_eq!(updated.usage_count, v),
            None => prop_assert_eq!(updated.usage_count, before.usage_count),
        }
        match &p_source {
            Some(v) => prop_assert_eq!(updated.source.as_deref(), Some(v.as_str())),
            None => prop_assert_eq!(&updated.source, &before.source),
        }
        match &p_notes {
            Some(v) => prop_assert_eq!(updated.notes.as_deref(), Some(v.as_str())),
            None => prop_assert_eq!(&updated.notes, &before.notes),
        }

        // Fields never supplied through this patch are always preserved.
        prop_assert_eq!(&updated.images, &before.images);
        prop_assert_eq!(&updated.videos, &before.videos);
        prop_assert_eq!(&updated.folder_id, &before.folder_id);

        // createdAt fixed; updatedAt no earlier than before (here strictly later
        // due to the backdate).
        prop_assert_eq!(&updated.created_at, &before.created_at);
        prop_assert!(
            updated.updated_at >= before.updated_at,
            "updatedAt regressed: {} < {}",
            updated.updated_at,
            before.updated_at
        );
    }
}

// ---------------------------------------------------------------------------
// Property 14: Copy substitutes matched placeholders only
// ---------------------------------------------------------------------------

/// A fragment of prompt text: either literal text (no braces) or a `{{name}}`
/// placeholder drawn from a small name pool.
#[derive(Debug, Clone)]
enum Frag {
    Lit(String),
    Ph(String),
}

/// The fixed placeholder-name pool used by Property 14, so the value map can be
/// generated independently of the token stream.
const NAME_POOL: [&str; 4] = ["alpha", "beta", "gamma", "delta"];

fn frag_strat() -> impl Strategy<Value = Frag> {
    prop_oneof![
        // Literals carry no braces, so they can never form a placeholder.
        proptest::string::string_regex("[a-zA-Z0-9 ]{0,6}")
            .unwrap()
            .prop_map(Frag::Lit),
        select(NAME_POOL.to_vec()).prop_map(|n| Frag::Ph(n.to_string())),
    ]
}

/// Builds the prompt text from fragments (`{{name}}` for placeholders).
fn build_text(frags: &[Frag]) -> String {
    let mut s = String::new();
    for f in frags {
        match f {
            Frag::Lit(t) => s.push_str(t),
            Frag::Ph(n) => {
                s.push_str("{{");
                s.push_str(n);
                s.push_str("}}");
            }
        }
    }
    s
}

/// Expected substitution: matched placeholders replaced, unmatched left verbatim.
fn expected_text(frags: &[Frag], values: &HashMap<String, String>) -> String {
    let mut s = String::new();
    for f in frags {
        match f {
            Frag::Lit(t) => s.push_str(t),
            Frag::Ph(n) => match values.get(n) {
                Some(v) => s.push_str(v),
                None => {
                    s.push_str("{{");
                    s.push_str(n);
                    s.push_str("}}");
                }
            },
        }
    }
    s
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// **Property 14: Copy substitutes matched placeholders only.**
    ///
    /// `copy` replaces each `{{name}}` whose name has a supplied value with that
    /// value and leaves verbatim every placeholder whose name is absent from the
    /// value map, in both the system and user prompt. It is read-only.
    ///
    /// **Validates: Requirements 6.11**
    #[test]
    fn copy_substitutes_only_matched_placeholders(
        frags in prop::collection::vec(frag_strat(), 0..=10),
        // Independently decide a value (or absence) for each pool name.
        a in proptest::option::of(small_text()),
        b in proptest::option::of(small_text()),
        c in proptest::option::of(small_text()),
        d in proptest::option::of(small_text()),
    ) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let mut values: HashMap<String, String> = HashMap::new();
        for (name, val) in NAME_POOL.iter().zip([a, b, c, d]) {
            if let Some(v) = val {
                values.insert(name.to_string(), v);
            }
        }

        let body = build_text(&frags);
        // Prefixes carry no braces and keep user_prompt non-empty for create.
        let user = format!("u:{body}");
        let sys = format!("s:{body}");

        let created = prompt::create(&conn, PromptCreate {
            title: "T".into(),
            user_prompt: user.clone(),
            system_prompt: Some(sys.clone()),
            ..Default::default()
        }).unwrap();
        let id = created.id.clone();

        let expected_body = expected_text(&frags, &values);
        let want_user = format!("u:{expected_body}");
        let want_sys = format!("s:{expected_body}");

        let result = prompt::copy(&conn, &id, &values).unwrap();
        prop_assert_eq!(&result.user_prompt, &want_user);
        prop_assert_eq!(result.system_prompt.as_deref(), Some(want_sys.as_str()));

        // copy is read-only: the stored prompt is untouched.
        let stored = prompt::get(&conn, &id).unwrap();
        prop_assert_eq!(&stored.user_prompt, &user);
        prop_assert_eq!(stored.system_prompt.as_deref(), Some(sys.as_str()));
    }
}

// ---------------------------------------------------------------------------
// Tag corpus helpers (Properties 15, 16, 17)
// ---------------------------------------------------------------------------

/// Creates one prompt per supplied tag list and returns their ids in order.
fn seed_tagged(conn: &rusqlite::Connection, corpus: &[Vec<String>]) -> Vec<String> {
    corpus
        .iter()
        .enumerate()
        .map(|(i, tags)| {
            prompt::create(
                conn,
                PromptCreate {
                    title: format!("p{i}"),
                    user_prompt: "body".into(),
                    tags: Some(tags.clone()),
                    ..Default::default()
                },
            )
            .unwrap()
            .id
        })
        .collect()
}

/// Reads the `(id -> tags)` map via the public `get`.
fn tags_by_id(conn: &rusqlite::Connection, ids: &[String]) -> BTreeMap<String, Vec<String>> {
    ids.iter()
        .map(|id| (id.clone(), prompt::get(conn, id).unwrap().tags))
        .collect()
}

/// `1..=8` prompts, each with a (deduplicated) tag list from the small alphabet.
fn tag_corpus() -> impl Strategy<Value = Vec<Vec<String>>> {
    prop::collection::vec(tag_set(), 1..=8)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// **Property 15: Tag rename is non-duplicating and idempotent.**
    ///
    /// Renaming `old` to `new` leaves no prompt carrying `old`, gives every prompt
    /// that carried `old` exactly one `new`, leaves all other tags unchanged
    /// (order preserved, deduplicated), and is idempotent on a second call.
    ///
    /// **Validates: Requirements 6.9**
    #[test]
    fn tag_rename_non_duplicating_and_idempotent(
        corpus in tag_corpus(),
        old in select(vec!["a", "b", "c", "d", "e"]),
        new in select(vec!["a", "b", "c", "d", "e"]),
    ) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let ids = seed_tagged(&conn, &corpus);

        let before = tags_by_id(&conn, &ids);
        prompt::tag_rename(&conn, old, new).unwrap();
        let after = tags_by_id(&conn, &ids);

        // Expected per-prompt: map old->new, dedup preserving first-seen order.
        // (When old == new this is the identity, so the corpus is unchanged.)
        for (id, original) in &before {
            let mut seen = HashSet::new();
            let expected: Vec<String> = original
                .iter()
                .map(|t| if t == old { new.to_string() } else { t.clone() })
                .filter(|t| seen.insert(t.clone()))
                .collect();
            prop_assert_eq!(after.get(id).unwrap(), &expected);
        }

        if old != new {
            // No prompt carries `old`; prompts that carried `old` carry exactly
            // one `new`.
            for (id, original) in &before {
                let tags = after.get(id).unwrap();
                prop_assert!(!tags.iter().any(|t| t == old), "old tag survived rename");
                if original.iter().any(|t| t == old) {
                    let count = tags.iter().filter(|t| t.as_str() == new).count();
                    prop_assert_eq!(count, 1, "expected exactly one `new` tag");
                }
            }
        }

        // Idempotent: a second rename changes nothing.
        prompt::tag_rename(&conn, old, new).unwrap();
        prop_assert_eq!(tags_by_id(&conn, &ids), after);
    }

    /// **Property 16: Tag delete removes the tag everywhere.**
    ///
    /// Deleting a tag removes it from every prompt that carried it and leaves all
    /// other tags (order preserved) unchanged.
    ///
    /// **Validates: Requirements 6.10**
    #[test]
    fn tag_delete_removes_everywhere(
        corpus in tag_corpus(),
        target in select(vec!["a", "b", "c", "d", "e"]),
    ) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let ids = seed_tagged(&conn, &corpus);

        let before = tags_by_id(&conn, &ids);
        prompt::tag_delete(&conn, target).unwrap();
        let after = tags_by_id(&conn, &ids);

        for (id, original) in &before {
            let expected: Vec<String> =
                original.iter().filter(|t| t.as_str() != target).cloned().collect();
            prop_assert_eq!(after.get(id).unwrap(), &expected);
            prop_assert!(!after.get(id).unwrap().iter().any(|t| t == target));
        }
    }

    /// **Property 17: Distinct tag aggregation.**
    ///
    /// `tag_list` equals the sorted, deduplicated union of all tags across all
    /// prompts, and is empty when no prompt has a tag.
    ///
    /// **Validates: Requirements 6.8**
    #[test]
    fn tag_list_is_sorted_distinct_union(corpus in tag_corpus()) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        seed_tagged(&conn, &corpus);

        let expected: Vec<String> = corpus
            .iter()
            .flatten()
            .cloned()
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect();

        prop_assert_eq!(prompt::tag_list(&conn).unwrap(), expected);
    }

    /// `tag_list` is empty when no prompt carries any tag.
    ///
    /// **Validates: Requirements 6.8**
    #[test]
    fn tag_list_empty_when_no_tags(n in 0usize..=5) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        for i in 0..n {
            prompt::create(&conn, mk_create(&format!("p{i}"), "body")).unwrap();
        }
        prop_assert!(prompt::tag_list(&conn).unwrap().is_empty());
    }
}
