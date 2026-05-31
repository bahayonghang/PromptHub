//! Property-based tests for the Folder_Service (task 5.2).
//!
//! These run as an **integration test** against the public `prompthub_lib` API
//! (`services::folder::*`, `storage::*`, `models::*`, `error::*`), so they need
//! no edits to any `mod.rs`. Each test builds a fresh in-memory database
//! ([`create_memory_pool`] + [`init_schema`]) and drives the service through its
//! public functions, exactly as the Command_Layer (task 17.1) will.
//!
//! Properties implemented (design "Testing Strategy"):
//!   - Property 2:  Cascade delete integrity
//!   - Property 21: Folder name validation
//!   - Property 22: Folder sibling sort-order assignment
//!   - Property 23: Folder reorder assigns positional sort orders
//!   - Property 24: Folder hierarchy stays acyclic
//!
//! **Validates: Requirements 4.3, 8.1, 8.4, 8.5, 8.8, 8.9**

use std::collections::HashMap;

use proptest::prelude::*;
use proptest::sample::Index;
use rusqlite::{params, Connection};

use prompthub_lib::error::ErrorCode;
use prompthub_lib::services::folder::{self, CreateFolderInput, UpdateFolderInput};
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

/// Convenience constructor for a [`CreateFolderInput`] with no icon.
fn mk(name: &str, parent: Option<String>) -> CreateFolderInput {
    CreateFolderInput {
        name: name.to_string(),
        parent_id: parent,
        icon: None,
    }
}

/// `id -> parent_id` for every stored folder (via the public `list`).
fn parent_map(conn: &Connection) -> HashMap<String, Option<String>> {
    folder::list(conn)
        .unwrap()
        .into_iter()
        .map(|f| (f.id, f.parent_id))
        .collect()
}

/// `id -> sort_order` for every stored folder (via the public `list`).
fn sort_map(conn: &Connection) -> HashMap<String, i64> {
    folder::list(conn)
        .unwrap()
        .into_iter()
        .map(|f| (f.id, f.sort_order))
        .collect()
}

/// Inserts a prompt row directly (the Prompt_Service is out of scope here) so
/// the cascade test can attach prompts to arbitrary folders.
fn insert_prompt(conn: &Connection, id: &str, folder_id: Option<&str>) {
    conn.execute(
        "INSERT INTO prompts (id, title, user_prompt, folder_id, created_at, updated_at) \
         VALUES (?1, 'T', 'U', ?2, 0, 0)",
        params![id, folder_id],
    )
    .expect("insert prompt");
}

/// Reads a prompt's `folder_id` (errors if the prompt no longer exists).
fn prompt_folder_id(conn: &Connection, id: &str) -> Option<String> {
    conn.query_row("SELECT folder_id FROM prompts WHERE id = ?1", [id], |r| {
        r.get(0)
    })
    .expect("prompt should still exist")
}

/// Number of stored prompts.
fn prompt_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM prompts", [], |r| r.get(0))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Tree generation (shared by Property 24 and Property 2)
// ---------------------------------------------------------------------------

/// Derives an acyclic parent-index assignment from `raw`.
///
/// Node 0 is always a root. For node `i >= 1`, `m = raw[i] % (i + 1)` lands in
/// `0..=i`; `m == i` makes the node a root, otherwise its parent is the
/// earlier-created node `m`. Because every parent index is strictly less than
/// the child's, the resulting forest is acyclic by construction.
fn parent_indices(raw: &[usize]) -> Vec<Option<usize>> {
    let n = raw.len();
    let mut parents = vec![None; n];
    for i in 1..n {
        let m = raw[i] % (i + 1);
        parents[i] = if m == i { None } else { Some(m) };
    }
    parents
}

/// Creates the folders described by `parent_idx` in index order, returning their
/// generated ids (so `ids[i]` is the id of node `i`).
fn build_tree(conn: &Connection, parent_idx: &[Option<usize>]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::with_capacity(parent_idx.len());
    for (i, parent) in parent_idx.iter().enumerate() {
        let parent_id = parent.map(|p| ids[p].clone());
        let folder = folder::create(conn, mk(&format!("n{i}"), parent_id)).unwrap();
        ids.push(folder.id);
    }
    ids
}

/// Reports whether `node` lies in the subtree rooted at `root` (inclusive),
/// i.e. whether `root` appears on `node`'s ancestor chain.
fn in_subtree(parent_idx: &[Option<usize>], node: usize, root: usize) -> bool {
    let mut cur = Some(node);
    while let Some(c) = cur {
        if c == root {
            return true;
        }
        cur = parent_idx[c];
    }
    false
}

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Folder-name strategy biased toward the validation boundaries: empty,
/// whitespace-only, ordinary names, names with surrounding whitespace, the
/// 1/255/256-character length boundaries, and unicode (multi-byte) names.
fn folder_name() -> impl Strategy<Value = String> {
    prop_oneof![
        2 => Just(String::new()),
        2 => proptest::string::string_regex("[ \\t\\n\\r]{1,6}").unwrap(),
        6 => proptest::string::string_regex("[a-zA-Z0-9 _.,好世界-]{1,40}").unwrap(),
        4 => proptest::string::string_regex("[ \\t]{1,3}[a-zA-Z0-9好]{1,20}[ \\t]{1,3}").unwrap(),
        3 => (1usize..=260).prop_map(|k| "x".repeat(k)),
        2 => proptest::string::string_regex("[好世界🙂]{0,5}").unwrap(),
    ]
}

/// A non-empty `raw` vector (1..=10 nodes) used to derive an acyclic tree.
fn tree_raw() -> impl Strategy<Value = Vec<usize>> {
    prop::collection::vec(0usize..1000, 1..=10)
}

/// `(n, permutation_of_0..n)` for the reorder property.
fn sized_permutation() -> impl Strategy<Value = (usize, Vec<usize>)> {
    (1usize..=10).prop_flat_map(|n| (Just(n), Just((0..n).collect::<Vec<usize>>()).prop_shuffle()))
}

// ---------------------------------------------------------------------------
// Property 21: Folder name validation
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// `create` accepts a name iff its trimmed character length is in 1..=255,
    /// stores the trimmed form on success, and on rejection returns `VALIDATION`
    /// while creating no folder.
    ///
    /// **Validates: Requirements 8.1, 8.8**
    #[test]
    fn name_validation_on_create(name in folder_name()) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let trimmed = name.trim();
        let len = trimmed.chars().count();
        let valid = (1..=255).contains(&len);

        match folder::create(&conn, mk(&name, None)) {
            Ok(folder) => {
                prop_assert!(valid, "accepted name with trimmed length {len}");
                prop_assert_eq!(folder.name.as_str(), trimmed, "stored name must be trimmed");
                prop_assert_eq!(folder::list(&conn).unwrap().len(), 1);
            }
            Err(err) => {
                prop_assert!(!valid, "rejected name with trimmed length {len}");
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert!(folder::list(&conn).unwrap().is_empty(), "no folder on reject");
            }
        }
    }

    /// `update` accepts a new name under the same 1..=255 trimmed-length rule;
    /// a rejected update leaves the existing folder's name unchanged.
    ///
    /// **Validates: Requirements 8.1, 8.8**
    #[test]
    fn name_validation_on_update(name in folder_name()) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let seed = folder::create(&conn, mk("seed-keep", None)).unwrap();

        let trimmed = name.trim();
        let len = trimmed.chars().count();
        let valid = (1..=255).contains(&len);

        let result = folder::update(
            &conn,
            &seed.id,
            UpdateFolderInput { name: Some(name.clone()), parent_id: None },
        );

        match result {
            Ok(folder) => {
                prop_assert!(valid, "accepted update name with trimmed length {len}");
                prop_assert_eq!(folder.name.as_str(), trimmed);
            }
            Err(err) => {
                prop_assert!(!valid, "rejected update name with trimmed length {len}");
                prop_assert_eq!(err.code, ErrorCode::Validation);
                // Name unchanged on rejection.
                let stored = folder::list(&conn).unwrap();
                prop_assert_eq!(stored.len(), 1);
                prop_assert_eq!(stored[0].name.as_str(), "seed-keep");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Properties 22, 23, 24, 2 (structural)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// **Property 22: Folder sibling sort-order assignment.**
    ///
    /// Folders sharing a parent receive sequential sort orders 0..N-1 in
    /// creation order, and each parent's sequence is independent (starts at 0).
    /// Children of two distinct parents are created interleaved so a single
    /// global counter could not satisfy the assertion.
    ///
    /// **Validates: Requirements 8.1**
    #[test]
    fn sibling_sort_orders_are_sequential_and_independent(
        a in 0usize..=8,
        b in 0usize..=8,
        extra_roots in 0usize..=5,
    ) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        // Two distinct root parents take root sort orders 0 and 1.
        let p = folder::create(&conn, mk("P", None)).unwrap();
        prop_assert_eq!(p.sort_order, 0);
        let q = folder::create(&conn, mk("Q", None)).unwrap();
        prop_assert_eq!(q.sort_order, 1);

        // Interleave children of P and Q; each parent's sequence restarts at 0.
        let mut next_p = 0i64;
        let mut next_q = 0i64;
        for i in 0..a.max(b) {
            if i < a {
                let f = folder::create(&conn, mk("c", Some(p.id.clone()))).unwrap();
                prop_assert_eq!(f.sort_order, next_p);
                next_p += 1;
            }
            if i < b {
                let f = folder::create(&conn, mk("c", Some(q.id.clone()))).unwrap();
                prop_assert_eq!(f.sort_order, next_q);
                next_q += 1;
            }
        }

        // Further root folders continue the root sequence from 2.
        for i in 0..extra_roots {
            let f = folder::create(&conn, mk("r", None)).unwrap();
            prop_assert_eq!(f.sort_order, (2 + i) as i64);
        }
    }

    /// **Property 23: Folder reorder assigns positional sort orders.**
    ///
    /// Given existing folders and any permutation of their ids, `reorder` sets
    /// each folder's `sort_order` to its zero-based position in the supplied list.
    ///
    /// **Validates: Requirements 8.5**
    #[test]
    fn reorder_assigns_positional_sort_orders((n, perm) in sized_permutation()) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        // Create n folders at the root; ids[i] is the i-th created folder.
        let ids: Vec<String> = (0..n)
            .map(|i| folder::create(&conn, mk(&format!("f{i}"), None)).unwrap().id)
            .collect();

        // Supply the ids in the permuted order.
        let ordered: Vec<String> = perm.iter().map(|&i| ids[i].clone()).collect();
        folder::reorder(&conn, &ordered).unwrap();

        let sorts = sort_map(&conn);
        for (position, id) in ordered.iter().enumerate() {
            prop_assert_eq!(sorts[id], position as i64);
        }
    }

    /// **Property 24: Folder hierarchy stays acyclic.**
    ///
    /// Reparenting a node under itself or one of its descendants is rejected with
    /// `VALIDATION` and leaves the whole tree unchanged; reparenting under any
    /// non-descendant succeeds and changes only that node's parent.
    ///
    /// **Validates: Requirements 8.9**
    #[test]
    fn reparent_respects_acyclicity(
        raw in tree_raw(),
        x_sel in any::<Index>(),
        t_sel in any::<Index>(),
    ) {
        let n = raw.len();
        let parent_idx = parent_indices(&raw);

        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let ids = build_tree(&conn, &parent_idx);

        let x = x_sel.index(n); // node being reparented
        let t = t_sel.index(n); // proposed new parent

        // A cycle results exactly when the target is x itself or a descendant.
        let creates_cycle = in_subtree(&parent_idx, t, x);
        let before = parent_map(&conn);

        let result = folder::update(
            &conn,
            &ids[x],
            UpdateFolderInput { name: None, parent_id: Some(Some(ids[t].clone())) },
        );

        if creates_cycle {
            match result {
                Ok(_) => prop_assert!(false, "expected cycle rejection for x={x} t={t}"),
                Err(err) => prop_assert_eq!(err.code, ErrorCode::Validation),
            }
            // Tree unchanged.
            prop_assert_eq!(parent_map(&conn), before);
        } else {
            match result {
                Err(err) => prop_assert!(false, "expected ok, got {:?}", err.code),
                Ok(folder) => {
                    prop_assert_eq!(folder.parent_id.as_deref(), Some(ids[t].as_str()));
                }
            }
            // Only x's parent changed.
            let mut expected = before.clone();
            expected.insert(ids[x].clone(), Some(ids[t].clone()));
            prop_assert_eq!(parent_map(&conn), expected);
        }
    }

    /// **Property 2: Cascade delete integrity.**
    ///
    /// Deleting a folder removes it and all of its descendants, clears
    /// `folder_id` (to NULL) on every prompt that pointed into the deleted
    /// subtree while leaving those prompts persisted, and leaves all folders and
    /// prompts outside the subtree untouched.
    ///
    /// **Validates: Requirements 4.3, 8.4**
    #[test]
    fn delete_cascades_to_descendants_and_nulls_prompts(
        raw in tree_raw(),
        prompt_targets in prop::collection::vec(proptest::option::of(any::<Index>()), 0..=12),
        del_sel in any::<Index>(),
    ) {
        let n = raw.len();
        let parent_idx = parent_indices(&raw);

        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let ids = build_tree(&conn, &parent_idx);

        // Attach prompts: each prompt points at a chosen folder index, or none.
        // Record (prompt id, folder index) so expectations can be computed.
        let prompts: Vec<(String, Option<usize>)> = prompt_targets
            .iter()
            .enumerate()
            .map(|(k, sel)| {
                let folder_index = sel.map(|s| s.index(n));
                let pid = format!("p{k}");
                let folder_id = folder_index.map(|i| ids[i].as_str());
                insert_prompt(&conn, &pid, folder_id);
                (pid, folder_index)
            })
            .collect();

        let total_prompts = prompts.len() as i64;
        prop_assert_eq!(prompt_count(&conn), total_prompts);

        let deleted_root = del_sel.index(n);
        folder::delete(&conn, &ids[deleted_root]).unwrap();

        // Folders: subtree members gone, others survive with unchanged parents.
        let remaining = parent_map(&conn);
        for i in 0..n {
            let id = &ids[i];
            if in_subtree(&parent_idx, i, deleted_root) {
                prop_assert!(!remaining.contains_key(id), "folder {i} should be deleted");
            } else {
                prop_assert!(remaining.contains_key(id), "folder {i} should survive");
                let expected_parent = parent_idx[i].map(|p| ids[p].clone());
                prop_assert_eq!(remaining.get(id).cloned().flatten(), expected_parent);
            }
        }

        // Prompts: all still exist; those that pointed into the subtree are now
        // null, others keep their original folder association.
        prop_assert_eq!(prompt_count(&conn), total_prompts);
        for (pid, folder_index) in &prompts {
            let actual = prompt_folder_id(&conn, pid);
            let expected = match folder_index {
                Some(i) if in_subtree(&parent_idx, *i, deleted_root) => None,
                Some(i) => Some(ids[*i].clone()),
                None => None,
            };
            prop_assert_eq!(actual, expected);
        }
    }
}
