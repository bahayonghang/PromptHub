//! Property 3: Transaction atomicity (proptest).
//!
//! *For any* command that performs multiple related writes inside a single
//! SQLite transaction, if any write fails before the transaction commits, the
//! ROLLBACK SHALL leave the persistent store byte-for-snapshot identical to its
//! state immediately before the transaction began (full rollback, no partial
//! writes persist). Conversely, a successful COMMIT SHALL durably persist every
//! write in the transaction.
//!
//! The test seeds an in-memory database with a random set of valid rows
//! (folders + prompts), captures a deterministic content snapshot of every
//! table, then opens a transaction and performs several writes
//! (inserts/updates/deletes derived from random input). A failure is injected at
//! a random point before commit by executing a statement that violates a
//! constraint (foreign-key, duplicate primary key, or CHECK), and the
//! transaction is rolled back by dropping the rusqlite `Transaction` (whose
//! default drop behaviour is ROLLBACK). The post-rollback snapshot is asserted
//! equal to the pre-transaction snapshot. A separate positive case commits the
//! same shape of writes and asserts they are all present afterwards.
//!
//! Note: with SQLite's default `ON CONFLICT ABORT`, a constraint violation
//! aborts only the offending *statement*; the earlier successful writes remain
//! buffered in the open transaction. They are discarded only because the
//! transaction is rolled back — which is exactly the atomicity guarantee under
//! test (Requirement 4.8).
//!
//! **Validates: Requirements 2.6, 2.8, 4.8**

use std::collections::BTreeMap;

use proptest::prelude::*;
use rusqlite::types::Value;
use rusqlite::{params, Connection};

use crate::storage::{create_memory_pool, init_schema, DbPool};

// --------------------------------------------------------------------------
// Shared helpers
// --------------------------------------------------------------------------

/// Every base table in the schema, paired with the column its rows are ordered
/// by for a deterministic snapshot (`settings` is keyed by `key`, the rest by
/// `id`).
const SNAPSHOT_TABLES: &[(&str, &str)] = &[
    ("folders", "id"),
    ("prompts", "id"),
    ("prompt_versions", "id"),
    ("settings", "key"),
    ("skills", "id"),
    ("skill_versions", "id"),
    ("rules", "id"),
    ("rule_versions", "id"),
];

/// A deterministic, fully-generic snapshot of the database: for every table, all
/// rows ordered by a stable key, each row captured as its raw column [`Value`]s.
///
/// Comparing two of these captures both row counts and row content for every
/// table, so any partial write (an inserted, updated, or deleted row that
/// survived a rollback) makes the snapshots unequal.
type Snapshot = BTreeMap<String, Vec<Vec<Value>>>;

/// Builds an in-memory pool with the schema initialized.
fn schema_pool() -> DbPool {
    let pool = create_memory_pool().expect("memory pool");
    init_schema(&pool.get().expect("conn")).expect("schema");
    pool
}

/// Captures a [`Snapshot`] of the entire database on `conn`.
fn snapshot(conn: &Connection) -> Snapshot {
    let mut out: Snapshot = BTreeMap::new();
    for (table, order_by) in SNAPSHOT_TABLES {
        let sql = format!("SELECT * FROM {table} ORDER BY {order_by}");
        let mut stmt = conn.prepare(&sql).expect("prepare snapshot query");
        let ncols = stmt.column_count();
        let rows = stmt
            .query_map([], |row| {
                let mut values = Vec::with_capacity(ncols);
                for i in 0..ncols {
                    values.push(row.get::<_, Value>(i)?);
                }
                Ok(values)
            })
            .expect("run snapshot query");
        out.insert((*table).to_string(), rows.map(Result::unwrap).collect());
    }
    out
}

/// Inserts a minimal prompt row (autocommit / seed phase, or inside a tx).
fn insert_prompt(
    conn: &Connection,
    id: &str,
    title: &str,
    user_prompt: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO prompts (id,title,user_prompt,created_at,updated_at) VALUES (?1,?2,?3,0,0)",
        params![id, title, user_prompt],
    )
}

/// Inserts a minimal folder row.
fn insert_folder(conn: &Connection, id: &str, name: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO folders (id,name,created_at) VALUES (?1,?2,0)",
        params![id, name],
    )
}

// --------------------------------------------------------------------------
// Generators
// --------------------------------------------------------------------------

/// Arbitrary non-empty, control-character-free text suitable for SQLite TEXT.
fn text() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9 _你好.,!-]{1,20}").expect("valid regex")
}

/// A set of `prefix-{index}` rows of `(id, title, user_prompt)`. Using the index
/// for the id guarantees ids are unique within the prefix, and distinct prefixes
/// (`seed`, `new`) never collide with each other.
fn rows(
    prefix: &'static str,
    range: std::ops::Range<usize>,
) -> impl Strategy<Value = Vec<(String, String, String)>> {
    prop::collection::vec((text(), text()), range).prop_map(move |v| {
        v.into_iter()
            .enumerate()
            .map(|(i, (title, user))| (format!("{prefix}-{i}"), title, user))
            .collect()
    })
}

/// The kind of failure injected mid-transaction. Each is a single statement that
/// raises an immediate constraint error under the schema's PRAGMAs
/// (`foreign_keys = ON`), so it aborts the statement without committing anything.
#[derive(Debug, Clone, Copy)]
enum FailureKind {
    /// References a folder id that does not exist -> foreign-key violation.
    ForeignKey,
    /// Inserts the same primary key twice -> the second insert violates the PK.
    DuplicatePrimaryKey,
    /// Supplies a `prompt_type` outside the allowed set -> CHECK violation.
    CheckConstraint,
}

fn failure_kind() -> impl Strategy<Value = FailureKind> {
    prop_oneof![
        Just(FailureKind::ForeignKey),
        Just(FailureKind::DuplicatePrimaryKey),
        Just(FailureKind::CheckConstraint),
    ]
}

/// Executes the injected failure on the open transaction, returning the error
/// that the failing statement produced.
///
/// For [`FailureKind::DuplicatePrimaryKey`] the first insert is itself a
/// successful in-transaction write (which the rollback must also undo); the
/// second insert is the one that fails.
fn inject_failure(conn: &Connection, kind: FailureKind) -> rusqlite::Result<usize> {
    match kind {
        FailureKind::ForeignKey => conn.execute(
            "INSERT INTO prompts (id,title,user_prompt,folder_id,created_at,updated_at) \
             VALUES ('__fk_fail__','t','u','__missing_folder__',0,0)",
            [],
        ),
        FailureKind::DuplicatePrimaryKey => {
            insert_prompt(conn, "__dup__", "first", "u").expect("first dup insert succeeds");
            insert_prompt(conn, "__dup__", "second", "u")
        }
        FailureKind::CheckConstraint => conn.execute(
            "INSERT INTO prompts (id,title,prompt_type,user_prompt,created_at,updated_at) \
             VALUES ('__chk__','t','bogus','u',0,0)",
            [],
        ),
    }
}

/// Seeds the database (autocommit) with the supplied folders and prompts.
fn seed(
    conn: &Connection,
    folders: &[(String, String, String)],
    prompts: &[(String, String, String)],
) {
    for (id, name, _) in folders {
        insert_folder(conn, id, name).expect("seed folder");
    }
    for (id, title, user) in prompts {
        insert_prompt(conn, id, title, user).expect("seed prompt");
    }
}

/// Performs the pre-failure / pre-commit writes inside the transaction: insert
/// `n_insert` of the new prompts, update the first seed prompt's title, delete
/// the last seed prompt, and insert a new folder. These mirror the
/// "several related writes" a real mutating command performs.
fn apply_writes(
    conn: &Connection,
    new_prompts: &[(String, String, String)],
    seed_prompts: &[(String, String, String)],
    n_insert: usize,
) {
    for (id, title, user) in new_prompts.iter().take(n_insert) {
        insert_prompt(conn, id, title, user).expect("tx insert new prompt");
    }
    conn.execute(
        "UPDATE prompts SET title='__CHANGED__' WHERE id=?1",
        params![seed_prompts[0].0],
    )
    .expect("tx update seed prompt");
    let last_id = &seed_prompts[seed_prompts.len() - 1].0;
    conn.execute("DELETE FROM prompts WHERE id=?1", params![last_id])
        .expect("tx delete seed prompt");
    insert_folder(conn, "__new_folder__", "new").expect("tx insert new folder");
}

// --------------------------------------------------------------------------
// Property 3: transaction atomicity
// --------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Mid-transaction failure + rollback leaves the database byte-for-snapshot
    /// identical to its pre-transaction state (no partial writes persist).
    ///
    /// **Validates: Requirements 2.8, 4.8**
    #[test]
    fn rollback_restores_pre_transaction_snapshot(
        seed_folders in rows("folder", 0..4),
        seed_prompts in rows("seed", 2..6),
        new_prompts in rows("new", 1..5),
        fail_raw in 0usize..=12,
        kind in failure_kind(),
    ) {
        let pool = schema_pool();
        let mut conn = pool.get().unwrap();

        // Seed valid rows, then snapshot the pre-transaction state.
        seed(&conn, &seed_folders, &seed_prompts);
        let before = snapshot(&conn);

        // Vary the point of failure: how many of the new prompts are inserted
        // (successfully) before the rest of the writes and the injected failure.
        let n_insert = fail_raw.min(new_prompts.len());

        {
            let tx = conn.transaction().unwrap();
            apply_writes(&tx, &new_prompts, &seed_prompts, n_insert);

            // Inject the failure: the offending statement must error.
            let result = inject_failure(&tx, kind);
            prop_assert!(
                result.is_err(),
                "injected {:?} failure must raise an error",
                kind
            );

            // Dropping the Transaction without committing rolls back (rusqlite's
            // default DropBehavior::Rollback).
            drop(tx);
        }

        // After rollback, the database must be exactly as it was before the tx.
        let after = snapshot(&conn);
        prop_assert_eq!(after, before);
    }

    /// A transaction that commits successfully durably persists every write
    /// (validates that commit persists, Requirement 2.6).
    ///
    /// **Validates: Requirements 2.6**
    #[test]
    fn commit_persists_all_writes(
        seed_folders in rows("folder", 0..4),
        seed_prompts in rows("seed", 2..6),
        new_prompts in rows("new", 1..5),
    ) {
        let pool = schema_pool();
        let mut conn = pool.get().unwrap();

        seed(&conn, &seed_folders, &seed_prompts);

        let prompts_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompts", [], |r| r.get(0))
            .unwrap();

        {
            let tx = conn.transaction().unwrap();
            apply_writes(&tx, &new_prompts, &seed_prompts, new_prompts.len());
            tx.commit().expect("commit succeeds");
        }

        // The updated seed prompt carries its new title.
        let updated_title: String = conn
            .query_row(
                "SELECT title FROM prompts WHERE id=?1",
                params![seed_prompts[0].0],
                |r| r.get(0),
            )
            .unwrap();
        prop_assert_eq!(updated_title, "__CHANGED__");

        // The deleted seed prompt is gone.
        let last_id = &seed_prompts[seed_prompts.len() - 1].0;
        let deleted_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM prompts WHERE id=?1",
                params![last_id],
                |r| r.get(0),
            )
            .unwrap();
        prop_assert_eq!(deleted_count, 0);

        // Every inserted new prompt is present.
        for (id, _, _) in &new_prompts {
            let present: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM prompts WHERE id=?1",
                    params![id],
                    |r| r.get(0),
                )
                .unwrap();
            prop_assert_eq!(present, 1, "new prompt {} must be persisted", id);
        }

        // The new folder is present.
        let folder_present: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM folders WHERE id='__new_folder__'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        prop_assert_eq!(folder_present, 1);

        // Net prompt count: started with `prompts_before`, deleted 1 seed,
        // inserted all new prompts.
        let prompts_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompts", [], |r| r.get(0))
            .unwrap();
        prop_assert_eq!(prompts_after, prompts_before - 1 + new_prompts.len() as i64);
    }
}

// --------------------------------------------------------------------------
// Example-based unit tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn explicit_rollback_leaves_db_unchanged() {
        let pool = schema_pool();
        let mut conn = pool.get().unwrap();

        insert_prompt(&conn, "seed-0", "orig", "u").unwrap();
        let before = snapshot(&conn);

        {
            let tx = conn.transaction().unwrap();
            insert_prompt(&tx, "extra", "added", "u").unwrap();
            tx.execute("UPDATE prompts SET title='changed' WHERE id='seed-0'", [])
                .unwrap();
            // Foreign-key violation aborts the statement; we then roll back.
            let err = tx.execute(
                "INSERT INTO prompts (id,title,user_prompt,folder_id,created_at,updated_at) \
                 VALUES ('bad','t','u','nope',0,0)",
                [],
            );
            assert!(err.is_err(), "foreign-key violation must error");
            drop(tx);
        }

        let after = snapshot(&conn);
        assert_eq!(
            after, before,
            "rollback must restore the pre-transaction state"
        );
    }

    #[test]
    fn explicit_commit_persists_writes() {
        let pool = schema_pool();
        let mut conn = pool.get().unwrap();

        insert_prompt(&conn, "seed-0", "orig", "u").unwrap();

        {
            let tx = conn.transaction().unwrap();
            insert_prompt(&tx, "extra", "added", "u").unwrap();
            tx.execute("UPDATE prompts SET title='changed' WHERE id='seed-0'", [])
                .unwrap();
            tx.commit().unwrap();
        }

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
        let title: String = conn
            .query_row("SELECT title FROM prompts WHERE id='seed-0'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(title, "changed");
    }
}
