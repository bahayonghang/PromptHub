//! Property-based tests for the Rules_Service (task 9.2).
//!
//! These run as an **integration test** against the public `prompthub_lib` API
//! (`services::rules::*`, `storage::*`, `models::*`, `error::*`), so they need no
//! edits to any `mod.rs`. Each case builds a fresh in-memory database
//! ([`create_memory_pool`] + [`init_schema`]) and a per-case `tempfile`
//! directory, then drives the service through its public functions exactly as
//! the Command_Layer (task 17.1) will. Snapshots are file-backed, so every
//! `save` performs disk IO; case counts and save counts are kept modest.
//!
//! Property implemented (design "Testing Strategy"):
//!   - Property 30: Rules version history cap and ordering
//!
//! **Validates: Requirements 14.4, 14.7, 14.9**

use std::cmp::min;
use std::path::PathBuf;

use proptest::prelude::*;
use proptest::sample::Index;
use tempfile::TempDir;

use prompthub_lib::services::rules::{self, AddProjectInput};
use prompthub_lib::storage::{create_memory_pool, init_schema, DbPool};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Builds an in-memory pool with the schema initialized.
fn schema_pool() -> DbPool {
    let pool = create_memory_pool().expect("memory pool");
    init_schema(&pool.get().expect("conn")).expect("schema");
    pool
}

/// Content seeded into a project rule when an initial snapshot is requested.
const INITIAL_CONTENT: &str = "seed-initial-content";

/// Unique, non-empty content for the `k`-th save (k starts at 1). Distinct from
/// [`INITIAL_CONTENT`] and from every other save, so presence/ordering checks
/// against content are unambiguous.
fn save_content(k: usize) -> String {
    format!("rule-save-content-#{k}")
}

/// Per-case directories for managed files, version snapshots, and the project
/// root (the `TempDir` is held to keep the directory alive for the case).
struct Dirs {
    _tmp: TempDir,
    managed: PathBuf,
    versions: PathBuf,
    project_root: PathBuf,
}

fn dirs() -> Dirs {
    let tmp = tempfile::tempdir().unwrap();
    let managed = tmp.path().join("managed");
    let versions = tmp.path().join("versions");
    let project_root = tmp.path().join("project");
    std::fs::create_dir_all(&project_root).unwrap();
    Dirs {
        _tmp: tmp,
        managed,
        versions,
        project_root,
    }
}

// ---------------------------------------------------------------------------
// Property 30: Rules version history cap and ordering
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    /// **Property 30: Rules version history cap and ordering.**
    ///
    /// Starting from a freshly created project rule (which may or may not carry
    /// an initial `create` snapshot), performing any number of `save` calls
    /// yields version snapshots that:
    ///   - number exactly `min(initial + saves, 20)` and never exceed 20 (14.9);
    ///   - are ordered most-recent first, the first element being the latest save
    ///     (14.7);
    ///   - retain the newest snapshots and discard the oldest once over the cap
    ///     (14.9);
    /// and deleting any retained version leaves the remaining snapshots equal to
    /// the prior set minus the deleted one, still most-recent first (14.7).
    ///
    /// **Validates: Requirements 14.4, 14.7, 14.9**
    #[test]
    fn version_history_cap_and_ordering(
        has_initial in any::<bool>(),
        n in 0usize..=24,
        del_sel in any::<Index>(),
    ) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let d = dirs();

        // Create the rule. An initial `create` snapshot is recorded iff the
        // seeded content is non-empty (14.5).
        let initial_content = if has_initial {
            Some(INITIAL_CONTENT.to_string())
        } else {
            Some(String::new())
        };
        let created = rules::add_project(
            &conn,
            AddProjectInput {
                name: "Proj".to_string(),
                project_root_path: d.project_root.to_string_lossy().to_string(),
                description: None,
                initial_content,
                id: None,
            },
            &d.managed,
            &d.versions,
        )
        .unwrap();
        let rule_id = created.id.clone();

        // Confirm the initial snapshot count by inspecting the rule right after
        // creation (the property accounts for this baseline).
        let initial_versions = created.versions.len();
        prop_assert_eq!(initial_versions, usize::from(has_initial));

        // Build the full sequence of (sequence-number, content) snapshots in
        // creation order. The initial `create` snapshot, when present, is the
        // oldest (sequence 0); the k-th save is sequence k.
        let mut all: Vec<(usize, String)> = Vec::new();
        if has_initial {
            all.push((0, INITIAL_CONTENT.to_string()));
        }
        for k in 1..=n {
            let content = save_content(k);
            rules::save(&conn, &rule_id, &content, None, &d.versions).unwrap();
            all.push((k, content));
        }

        let total = all.len();
        let expected_retained = min(total, 20);

        // Expected retained snapshots, most-recent first: the highest-sequence
        // entries (creation order reversed), capped at 20.
        let expected: Vec<String> = all
            .iter()
            .rev()
            .take(expected_retained)
            .map(|(_, c)| c.clone())
            .collect();

        let versions = rules::read(&conn, &rule_id).unwrap().versions;

        // Cap: count is exactly min(total, 20) and never exceeds 20 (14.9).
        prop_assert!(versions.len() <= 20);
        prop_assert_eq!(versions.len(), expected_retained);

        // Ordering + retention: the retained set is exactly the newest snapshots,
        // most-recent first. Matching content sequence proves the ordering is
        // monotonically non-increasing by recency, that the newest saves are all
        // present, and that the earliest were discarded once over the cap.
        let actual: Vec<String> = versions.iter().map(|v| v.content.clone()).collect();
        prop_assert_eq!(&actual, &expected);

        // The first element corresponds to the latest write.
        if n >= 1 {
            let latest = save_content(n);
            prop_assert_eq!(versions[0].content.as_str(), latest.as_str());
        } else if has_initial {
            prop_assert_eq!(versions[0].content.as_str(), INITIAL_CONTENT);
        }

        // Delete a random retained version (when any exist) and verify the
        // remaining list is the prior set minus that one, still most-recent first.
        if !versions.is_empty() {
            let del_idx = del_sel.index(versions.len());
            let deleted_id = versions[del_idx].id.clone();

            let remaining = rules::delete_version(&conn, &rule_id, &deleted_id).unwrap();

            // One fewer element, and the deleted id is gone.
            prop_assert_eq!(remaining.len(), versions.len() - 1);
            prop_assert!(remaining.iter().all(|v| v.id != deleted_id));

            // Still most-recent first: equals the prior content order with the
            // deleted index removed.
            let expected_after: Vec<String> = versions
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != del_idx)
                .map(|(_, v)| v.content.clone())
                .collect();
            let actual_after: Vec<String> =
                remaining.iter().map(|v| v.content.clone()).collect();
            prop_assert_eq!(actual_after, expected_after);
        }
    }
}
