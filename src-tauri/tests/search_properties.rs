//! Property-based tests for the Search_Engine / Prompt_Service search path
//! (task 4.6).
//!
//! These run as an **integration test** against the public `prompthub_lib` API
//! (`services::prompt::*`, `storage::*`, `storage::fts::*`, `models::*`,
//! `error::*`), so they need no edits to any `mod.rs`. Each test builds a fresh
//! in-memory database and drives search through the public functions, exactly as
//! the Command_Layer (task 17.1) will.
//!
//! The Search_Engine's FTS5 virtual table (`prompts_fts`) is created by
//! [`init_fts`], which is **separate** from [`init_schema`]; every test setup
//! therefore calls *both* so the keyword path has a populated index.
//!
//! Properties implemented (design "Testing Strategy"):
//!   - Property 8:  Search read-after-write consistency
//!   - Property 9:  Keyword search is case-insensitive and exact-membership
//!   - Property 10: Filters combine with conjunctive (AND) logic
//!   - Property 11: Result ordering honors sort field and direction
//!   - Property 12: Pagination clamping and defaults
//!   - Property 13: Search keyword robustness
//!
//! **Validates: Requirements 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9**
//! (and Requirement 5.1 via Property 9's distribution of the keyword across all
//! indexed fields).

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::sample::select;
use rusqlite::{params, Connection};

use prompthub_lib::error::ErrorCode;
use prompthub_lib::models::{PromptListItem, SearchQuery, SortField, SortOrder};
use prompthub_lib::services::prompt::{self, PromptCreate};
use prompthub_lib::storage::fts::init_fts;
use prompthub_lib::storage::{create_memory_pool, init_schema, DbPool};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Builds an in-memory pool with **both** the base schema and the FTS index.
///
/// The Search_Engine table lives behind [`init_fts`], distinct from
/// [`init_schema`]; the keyword path requires both, so every test sets up both.
fn search_pool() -> DbPool {
    let pool = create_memory_pool().expect("memory pool");
    let conn = pool.get().expect("conn");
    init_schema(&conn).expect("schema");
    init_fts(&conn).expect("fts");
    pool
}

/// A minimal valid [`PromptCreate`] (non-empty title + user prompt) with every
/// other field defaulted; callers override fields as needed.
fn base_create(title: &str, user_prompt: &str) -> PromptCreate {
    PromptCreate {
        title: title.to_string(),
        user_prompt: user_prompt.to_string(),
        ..Default::default()
    }
}

/// The set of result ids (order-independent comparison).
fn id_set(prompts: &[PromptListItem]) -> BTreeSet<String> {
    prompts.iter().map(|p| p.id.clone()).collect()
}

/// The ordered list of result ids (order-sensitive comparison).
fn id_list(prompts: &[PromptListItem]) -> Vec<String> {
    prompts.iter().map(|p| p.id.clone()).collect()
}

/// Inserts a folder row directly so prompts can reference it under the ON
/// foreign-key constraint (the Folder_Service is out of scope for this task).
fn insert_folder(conn: &Connection, id: &str) {
    conn.execute(
        "INSERT INTO folders (id, name, created_at) VALUES (?1, ?1, 0)",
        params![id],
    )
    .expect("insert folder");
}

/// A keyword query with default sort/pagination and the given keyword.
fn keyword_query(keyword: &str) -> SearchQuery {
    SearchQuery {
        keyword: Some(keyword.to_string()),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Property 8: Search read-after-write consistency (Req 5.2)
// ---------------------------------------------------------------------------

/// A short "background" word (1..=5 letters) that can never equal the 6..=12
/// letter unique token, so background prompts never match the token search.
fn short_word() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-z]{1,5}").unwrap()
}

/// A bag of background user-prompt bodies, each a few short words.
fn background_bodies() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        prop::collection::vec(short_word(), 1..=4).prop_map(|w| w.join(" ")),
        0..=8,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// **Property 8: Search read-after-write consistency.**
    ///
    /// A prompt created with a unique token is immediately findable by that
    /// token (and is the *only* match, since the token is unique by
    /// construction); after the prompt is deleted, the same search no longer
    /// returns it. This exercises the FTS index being maintained inside the
    /// same transaction as the prompt insert/delete (Req 5.2).
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn search_reflects_create_then_delete(
        token in proptest::string::string_regex("[a-z]{6,12}").unwrap(),
        backgrounds in background_bodies(),
    ) {
        let pool = search_pool();
        let conn = pool.get().unwrap();

        // Background prompts: their words are <= 5 chars, so none equals the
        // >= 6 char token; they must never appear in a token search.
        for (i, body) in backgrounds.iter().enumerate() {
            prompt::create(&conn, base_create(&format!("bg{i}"), body)).unwrap();
        }

        // Create the target prompt carrying the unique token in its user prompt.
        let target = prompt::create(
            &conn,
            base_create("untitled", &format!("lead {token} tail")),
        )
        .unwrap();

        // Immediately findable, and the unique token matches only the target.
        let found = prompt::search(&conn, keyword_query(&token)).unwrap();
        prop_assert_eq!(id_list(&found), vec![target.id.clone()]);

        // After deletion the index no longer returns it.
        prompt::delete(&conn, &target.id).unwrap();
        let after = prompt::search(&conn, keyword_query(&token)).unwrap();
        prop_assert!(after.is_empty(), "deleted prompt still found for token {}", token);
    }
}

// ---------------------------------------------------------------------------
// Property 9: Keyword search is case-insensitive and exact-membership (Req 5.1, 5.3)
// ---------------------------------------------------------------------------

/// Distinct alphabetic vocabulary distributed across all five indexed fields
/// (title, description, system_prompt, user_prompt, tags). No word is a prefix
/// of another, and all are >= 4 chars, so each is a single FTS token that never
/// collides with the structural fillers below.
const WORDS9: &[&str] = &["alpha", "bravo", "charlie", "delta", "echo", "foxtrot"];

/// A guaranteed-absent token: not in [`WORDS9`] and not any filler.
const ABSENT9: &str = "zulu";

/// A random subset (0..=3) of indices into [`WORDS9`] for one field.
fn field_word_idxs() -> impl Strategy<Value = Vec<usize>> {
    prop::collection::vec(0usize..WORDS9.len(), 0..=3)
}

/// Per-prompt word assignment for the five indexed fields.
type Spec9 = (Vec<usize>, Vec<usize>, Vec<usize>, Vec<usize>, Vec<usize>);

/// A corpus of up to 12 prompt specs.
fn corpus9() -> impl Strategy<Value = Vec<Spec9>> {
    prop::collection::vec(
        (
            field_word_idxs(),
            field_word_idxs(),
            field_word_idxs(),
            field_word_idxs(),
            field_word_idxs(),
        ),
        0..=12,
    )
}

/// Joins the words named by `idxs` with spaces.
fn join_words(idxs: &[usize]) -> String {
    idxs.iter()
        .map(|&i| WORDS9[i])
        .collect::<Vec<_>>()
        .join(" ")
}

/// Re-cases `word` per `mode` (0 = lower, 1 = upper, 2 = capitalize-first) so the
/// query exercises case-insensitive matching against the lowercase corpus.
fn recase(word: &str, mode: u8) -> String {
    match mode % 3 {
        0 => word.to_lowercase(),
        1 => word.to_uppercase(),
        _ => {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// **Property 9: Keyword search is case-insensitive and exact-membership.**
    ///
    /// For a generated corpus that scatters a small vocabulary across all five
    /// indexed fields, a single-keyword search returns exactly those prompts
    /// whose union of indexed words contains the keyword — irrespective of the
    /// keyword's case — and returns nothing for a word absent from the corpus.
    /// Covers Req 5.1 (every field is indexed) and Req 5.3 (case-insensitive,
    /// empty result when nothing matches).
    ///
    /// **Validates: Requirements 5.1, 5.3**
    #[test]
    fn keyword_matches_exact_membership_case_insensitively(
        specs in corpus9(),
        q_idx in 0usize..=WORDS9.len(), // == len() selects the absent token
        case_mode in 0u8..3,
    ) {
        let pool = search_pool();
        let conn = pool.get().unwrap();

        // The query word: a vocabulary word, or the guaranteed-absent token.
        let query_word = if q_idx == WORDS9.len() { ABSENT9 } else { WORDS9[q_idx] };

        let mut expected: BTreeSet<String> = BTreeSet::new();

        for (i, (t, d, s, u, tags)) in specs.iter().enumerate() {
            // The prompt's matchable word set is the union of vocabulary words
            // placed in any indexed field.
            let mut wordset: BTreeSet<&str> = BTreeSet::new();
            for field in [t, d, s, u, tags] {
                for &idx in field {
                    wordset.insert(WORDS9[idx]);
                }
            }

            // Compose field values. Title and user_prompt must be non-empty, so
            // they carry a structural filler that is never a vocabulary word.
            let title = format!("t{i} {}", join_words(t));
            let description = if d.is_empty() { None } else { Some(join_words(d)) };
            let system_prompt = if s.is_empty() { None } else { Some(join_words(s)) };
            let user_prompt = if u.is_empty() {
                "filler9".to_string()
            } else {
                join_words(u)
            };
            let tag_values: Vec<String> = tags.iter().map(|&idx| WORDS9[idx].to_string()).collect();

            let created = prompt::create(
                &conn,
                PromptCreate {
                    title,
                    user_prompt,
                    description,
                    system_prompt,
                    tags: Some(tag_values),
                    ..Default::default()
                },
            )
            .unwrap();

            if wordset.contains(query_word) {
                expected.insert(created.id);
            }
        }

        // Query in a possibly-different case; the corpus is lowercase.
        let results = prompt::search(&conn, keyword_query(&recase(query_word, case_mode))).unwrap();
        prop_assert_eq!(id_set(&results), expected);
    }
}

// ---------------------------------------------------------------------------
// Property 10: Filters combine with conjunctive (AND) logic (Req 5.4)
// ---------------------------------------------------------------------------

/// Tag vocabulary for the filter corpus.
const TAGS10: &[&str] = &["a", "b", "c", "d"];
/// Folder ids referenced by the filter corpus (all inserted up front).
const FOLDERS10: &[&str] = &["f0", "f1", "f2"];

/// A deduplicated subset (0..=`max` elements) of tag indices.
fn tag_idx_subset(max: usize) -> impl Strategy<Value = Vec<usize>> {
    prop::collection::vec(0usize..TAGS10.len(), 0..=max).prop_map(|v| {
        v.into_iter()
            .collect::<BTreeSet<usize>>()
            .into_iter()
            .collect()
    })
}

/// One corpus prompt: a tag-index subset, an optional folder index, favorite flag.
type Pop10 = (Vec<usize>, Option<usize>, bool);

/// A filter corpus of up to 40 prompts (cap kept small for runtime).
fn corpus10() -> impl Strategy<Value = Vec<Pop10>> {
    prop::collection::vec(
        (
            tag_idx_subset(3),
            proptest::option::of(0usize..FOLDERS10.len()),
            any::<bool>(),
        ),
        0..=40,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// **Property 10: Filters combine with conjunctive (AND) logic.**
    ///
    /// For a random corpus (each prompt with random tags, folder, favorite) and a
    /// random combined `tags + folder + favorite` query, the search result equals
    /// exactly the set computed by an independent Rust-side conjunction of the
    /// same predicates: every result satisfies *all* supplied filters, and no
    /// qualifying prompt is omitted.
    ///
    /// **Validates: Requirements 5.4**
    #[test]
    fn filters_combine_conjunctively(
        corpus in corpus10(),
        q_tags in tag_idx_subset(2),
        q_folder in proptest::option::of(0usize..FOLDERS10.len()),
        q_fav in proptest::option::of(any::<bool>()),
    ) {
        let pool = search_pool();
        let conn = pool.get().unwrap();

        // Referenced folders must exist (foreign_keys=ON).
        for folder in FOLDERS10 {
            insert_folder(&conn, folder);
        }

        // Build the corpus and remember each prompt's filterable attributes.
        let mut records: Vec<(String, BTreeSet<usize>, Option<usize>, bool)> = Vec::new();
        for (i, (tags, folder, fav)) in corpus.iter().enumerate() {
            let tag_values: Vec<String> = tags.iter().map(|&t| TAGS10[t].to_string()).collect();
            let folder_id = folder.map(|f| FOLDERS10[f].to_string());
            let created = prompt::create(
                &conn,
                PromptCreate {
                    title: format!("p{i}"),
                    user_prompt: "body".to_string(),
                    tags: Some(tag_values),
                    folder_id,
                    is_favorite: Some(*fav),
                    ..Default::default()
                },
            )
            .unwrap();
            records.push((created.id, tags.iter().copied().collect(), *folder, *fav));
        }

        // Independent expected set: AND of all supplied predicates.
        let q_tag_set: BTreeSet<usize> = q_tags.iter().copied().collect();
        let expected: BTreeSet<String> = records
            .iter()
            .filter(|(_, tags, folder, fav)| {
                let tags_ok = q_tag_set.is_subset(tags);
                let folder_ok = q_folder.map_or(true, |qf| *folder == Some(qf));
                let fav_ok = q_fav.map_or(true, |qv| *fav == qv);
                tags_ok && folder_ok && fav_ok
            })
            .map(|(id, ..)| id.clone())
            .collect();

        let query = SearchQuery {
            tags: if q_tags.is_empty() {
                None
            } else {
                Some(q_tags.iter().map(|&t| TAGS10[t].to_string()).collect())
            },
            folder_id: q_folder.map(|f| FOLDERS10[f].to_string()),
            is_favorite: q_fav,
            // High enough that every match is returned (corpus <= 40).
            limit: Some(100),
            ..Default::default()
        };

        let results = prompt::search(&conn, query).unwrap();
        prop_assert_eq!(id_set(&results), expected);
    }
}

// ---------------------------------------------------------------------------
// Property 11: Result ordering honors sort field and direction (Req 5.5)
// ---------------------------------------------------------------------------

/// Per-prompt sort-key assignment: `(n, title_perm, created_perm, updated_perm,
/// usage_perm)`, where each permutation is a shuffle of `0..n`, giving every
/// prompt a *distinct* value in every sortable field (so the expected order is
/// unambiguous and the `id` tiebreak is never exercised).
fn sort_population(
) -> impl Strategy<Value = (usize, Vec<usize>, Vec<usize>, Vec<usize>, Vec<usize>)> {
    (1usize..=12).prop_flat_map(|n| {
        let perm = || Just((0..n).collect::<Vec<usize>>()).prop_shuffle();
        (Just(n), perm(), perm(), perm(), perm())
    })
}

/// Overwrites the four sortable fields of a prompt with deterministic values.
fn set_sort_fields(
    conn: &Connection,
    id: &str,
    title: &str,
    created: i64,
    updated: i64,
    usage: i64,
) {
    conn.execute(
        "UPDATE prompts SET title=?1, created_at=?2, updated_at=?3, usage_count=?4 WHERE id=?5",
        params![title, created, updated, usage, id],
    )
    .expect("set sort fields");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// **Property 11: Result ordering honors sort field and direction.**
    ///
    /// Builds a population whose prompts have distinct values in each of the four
    /// sortable fields, then for every `(field, direction)` combination asserts
    /// the returned id order equals the order an independent Rust sort produces.
    ///
    /// **Validates: Requirements 5.5**
    #[test]
    fn ordering_honors_field_and_direction(
        (n, title_perm, created_perm, updated_perm, usage_perm) in sort_population(),
    ) {
        let pool = search_pool();
        let conn = pool.get().unwrap();

        // record: (id, title_string, created, updated, usage)
        let mut records: Vec<(String, String, i64, i64, i64)> = Vec::with_capacity(n);
        for i in 0..n {
            let created = prompt::create(&conn, base_create("seed", "body")).unwrap();
            // Zero-padded numeric titles so BINARY (byte) ordering == numeric order,
            // matching SQLite's default collation and Rust's String ordering.
            let title = format!("title-{:03}", title_perm[i]);
            set_sort_fields(
                &conn,
                &created.id,
                &title,
                created_perm[i] as i64,
                updated_perm[i] as i64,
                usage_perm[i] as i64,
            );
            records.push((
                created.id,
                title,
                created_perm[i] as i64,
                updated_perm[i] as i64,
                usage_perm[i] as i64,
            ));
        }

        let fields = [
            SortField::Title,
            SortField::CreatedAt,
            SortField::UpdatedAt,
            SortField::UsageCount,
        ];
        let orders = [SortOrder::Asc, SortOrder::Desc];

        for &field in &fields {
            for &order in &orders {
                // Expected order: sort records by the chosen field's value.
                let mut sorted = records.clone();
                sorted.sort_by(|a, b| match field {
                    SortField::Title => a.1.cmp(&b.1),
                    SortField::CreatedAt => a.2.cmp(&b.2),
                    SortField::UpdatedAt => a.3.cmp(&b.3),
                    SortField::UsageCount => a.4.cmp(&b.4),
                });
                if order == SortOrder::Desc {
                    sorted.reverse();
                }
                let expected: Vec<String> = sorted.into_iter().map(|r| r.0).collect();

                let query = SearchQuery {
                    sort_by: Some(field),
                    sort_order: Some(order),
                    limit: Some(100),
                    ..Default::default()
                };
                let results = prompt::search(&conn, query).unwrap();
                prop_assert_eq!(id_list(&results), expected, "field={:?} order={:?}", field, order);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property 12: Pagination clamping and defaults (Req 5.6, 5.8, 5.9)
// ---------------------------------------------------------------------------

/// Sets only a prompt's `updated_at`, used to give the default sort a distinct,
/// deterministic ordering.
fn set_updated(conn: &Connection, id: &str, updated: i64) {
    conn.execute(
        "UPDATE prompts SET updated_at=?1 WHERE id=?2",
        params![updated, id],
    )
    .expect("set updated_at");
}

/// `(n, updated_perm, limit_input, offset_input)` for the pagination property.
/// `updated_perm` is a shuffle of `0..n` (distinct updated_at values);
/// `limit_input`/`offset_input` cover absent, zero, in-range, and out-of-range
/// values so the clamp and default rules are all exercised.
fn pagination_inputs() -> impl Strategy<Value = (usize, Vec<usize>, Option<u32>, Option<u32>)> {
    (1usize..=30).prop_flat_map(|n| {
        (
            Just(n),
            Just((0..n).collect::<Vec<usize>>()).prop_shuffle(),
            proptest::option::of(0u32..=120),
            proptest::option::of(0u32..=(n as u32 + 5)),
        )
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// **Property 12: Pagination clamping and defaults.**
    ///
    /// With a population sorted under the default order (updatedAt desc), the
    /// effective limit is `clamp(1..=100)` (50 when absent) and the effective
    /// offset is `>= 0` (0 when absent). The returned window equals the
    /// independently-computed slice `full[offset .. offset+limit]`, the result
    /// count never exceeds the effective limit, and an absent sort falls back to
    /// updatedAt descending.
    ///
    /// **Validates: Requirements 5.6, 5.8, 5.9**
    #[test]
    fn pagination_clamps_and_windows(
        (n, updated_perm, limit_input, offset_input) in pagination_inputs(),
    ) {
        let pool = search_pool();
        let conn = pool.get().unwrap();

        // record: (id, updated) with distinct updated values.
        let mut records: Vec<(String, i64)> = Vec::with_capacity(n);
        for &updated in updated_perm.iter().take(n) {
            let created = prompt::create(&conn, base_create("seed", "body")).unwrap();
            set_updated(&conn, &created.id, updated as i64);
            records.push((created.id, updated as i64));
        }

        // Independent ground-truth order: updatedAt descending (the default,
        // Req 5.8). updated values are distinct, so this is unambiguous.
        let mut full_sorted = records.clone();
        full_sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
        let full: Vec<String> = full_sorted.into_iter().map(|r| r.0).collect();

        // Default query (no sort, no pagination) returns the full default order.
        let default_results = prompt::search(&conn, SearchQuery::default()).unwrap();
        // Default limit is 50; with n <= 30 the whole corpus is returned.
        prop_assert_eq!(id_list(&default_results), full.clone());

        // Effective pagination per the documented rules.
        let eff_limit = limit_input.map(|l| l.clamp(1, 100)).unwrap_or(50) as usize;
        let eff_offset = offset_input.unwrap_or(0) as usize;
        let expected: Vec<String> = full
            .iter()
            .skip(eff_offset)
            .take(eff_limit)
            .cloned()
            .collect();

        let query = SearchQuery {
            limit: limit_input,
            offset: offset_input,
            ..Default::default()
        };
        let results = prompt::search(&conn, query).unwrap();

        prop_assert_eq!(id_list(&results), expected);
        prop_assert!(
            results.len() <= eff_limit,
            "result count {} exceeded effective limit {}",
            results.len(),
            eff_limit
        );
    }
}

// ---------------------------------------------------------------------------
// Property 13: Search keyword robustness (Req 5.7)
// ---------------------------------------------------------------------------

/// Keyword strategy biased toward FTS5 operators and special characters that a
/// naive query builder would choke on: quotes, `*`, `^`, `:`, parentheses,
/// `AND`/`OR`/`NOT`/`NEAR`, hyphens, braces, backslashes, plus arbitrary
/// printable and unicode runs.
fn hostile_keyword() -> impl Strategy<Value = String> {
    let fragments: Vec<&'static str> = vec![
        "\"", "*", "^", ":", "(", ")", "AND", "OR", "NOT", "NEAR", "-", "+", "{{", "}}", "\\",
        "foo", "bar", " ", "你好", "[", "]", "~", "/",
    ];
    prop_oneof![
        // Arbitrary printable-ASCII runs.
        proptest::string::string_regex("[ -~]{0,16}").unwrap(),
        // Composed entirely from operator/special fragments.
        prop::collection::vec(select(fragments), 0..=6).prop_map(|parts| parts.concat()),
        // Arbitrary characters incl. unicode/control.
        proptest::string::string_regex(".{0,12}").unwrap(),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// **Property 13: Search keyword robustness.**
    ///
    /// For any keyword — including FTS query operators and special characters —
    /// `search` returns either a result set (`Ok`) or a structured parse error
    /// (`Err` with code `PARSE`), and never panics or returns any other error
    /// code.
    ///
    /// **Validates: Requirements 5.7**
    #[test]
    fn keyword_search_never_panics(keyword in hostile_keyword()) {
        let pool = search_pool();
        let conn = pool.get().unwrap();

        // A little content so the index is non-empty.
        prompt::create(&conn, base_create("hello world", "the quick brown fox")).unwrap();
        prompt::create(&conn, base_create("another", "lazy dog jumps")).unwrap();

        match prompt::search(&conn, keyword_query(&keyword)) {
            Ok(_) => {}
            Err(err) => prop_assert_eq!(
                err.code,
                ErrorCode::Parse,
                "unexpected error code {:?} for keyword {:?}",
                err.code,
                keyword
            ),
        }
    }
}
