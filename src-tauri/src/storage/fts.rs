//! Search_Engine: FTS5 full-text index over `prompts`, kept in sync by triggers.
//!
//! This module owns the `prompts_fts` FTS5 virtual table and the three sync
//! triggers (`prompts_ai`, `prompts_ad`, `prompts_au`) that mirror every prompt
//! mutation into the index. It implements the Search_Engine schema described in
//! the design document ("Data Models / Storage_Engine schema") and satisfies
//! Requirements 5.1 (index title, description, system_prompt, user_prompt, tags)
//! and 5.2 (the index updates inside the same transaction as the prompt write,
//! so a search submitted after the write reflects the change).
//!
//! ## Why an external-content table
//!
//! `prompts_fts` is declared with `content='prompts', content_rowid='rowid'`, so
//! the FTS index does not duplicate the indexed text — it reads it back from the
//! base `prompts` table via the implicit integer `rowid`. Although
//! `prompts.id` is a TEXT primary key, every ordinary (non-`WITHOUT ROWID`) table
//! still has an implicit integer `rowid`; the external-content table keys off
//! that `rowid`, not the TEXT primary key, so the TEXT id is irrelevant here. This
//! mirrors the reference implementation (`@prompthub/db`) and the design.
//!
//! ## Transaction semantics (Requirement 5.2)
//!
//! SQLite always executes `AFTER INSERT/UPDATE/DELETE` triggers within the same
//! transaction as the statement that fired them. Because the FTS rows are written
//! by these triggers, the index is updated atomically with the prompt mutation —
//! no separate write and no separate transaction is needed. A `prompt.search`
//! issued after a committed prompt write therefore always sees the change.
//!
//! ## `tags` indexing
//!
//! `prompts.tags` is stored as a JSON array of strings (e.g. `["alpha","beta"]`).
//! The raw JSON text is indexed as-is; the default FTS5 (`unicode61`) tokenizer
//! splits on punctuation, so the bracket/quote/comma characters become token
//! separators and each tag value becomes its own searchable token. Keyword
//! membership search over tags therefore works without parsing the JSON here.
//! Query construction (escaping operators, combining filters) is a later task.
//!
//! ## Initialization contract
//!
//! [`init_fts`] is the standalone entry point and must be called once, on the
//! same connection, immediately after [`crate::storage::init_schema`] has created
//! the base `prompts` table (see `FTS_EXTENSION_POINT`). Table/trigger DDL uses
//! `IF NOT EXISTS`, so repeating it is safe. After the virtual table exists, if
//! `prompts` has rows and the FTS row count does not match the non-private
//! prompt count, non-private rows are inserted into the index. Private prompts
//! stay out of FTS. Triggers keep the index in sync for later writes.

use rusqlite::Connection;

use crate::error::AppError;

/// Creates the `prompts_fts` FTS5 virtual table and its sync triggers.
///
/// Executes the DDL in a single transaction so the table and all three triggers
/// are created atomically. Table/trigger statements are `IF NOT EXISTS`. When
/// `prompts` already has rows (schema upgrade of a non-empty library), a count
/// mismatch against the FTS index rebuilds from `is_private = 0` rows only.
///
/// Must be called on the same connection as (and immediately after)
/// [`crate::storage::init_schema`], so the `prompts` table the triggers reference
/// already exists.
pub fn init_fts(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(FTS_SQL)
        .map_err(|e| AppError::internal(format!("failed to initialize FTS index: {e}")))?;
    rebuild_fts_if_stale(conn)
}

/// Rebuilds `prompts_fts` from current non-private `prompts` when the index is
/// empty or its row count does not match. Uses an explicit `INSERT ... SELECT`
/// rather than FTS5 `rebuild`, which would index private bodies from the
/// external content table.
fn rebuild_fts_if_stale(conn: &Connection) -> Result<(), AppError> {
    let public_prompts: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM prompts WHERE is_private = 0",
            [],
            |row| row.get(0),
        )
        .map_err(|e| AppError::internal(format!("failed to count prompts for FTS rebuild: {e}")))?;
    // External-content FTS5: `COUNT(*) FROM prompts_fts` reads the content
    // table, including private rows and rows that were never indexed. The
    // docsize shadow table counts documents actually in the index.
    let indexed: i64 = conn
        .query_row("SELECT COUNT(*) FROM prompts_fts_docsize", [], |row| {
            row.get(0)
        })
        .map_err(|e| AppError::internal(format!("failed to count FTS rows: {e}")))?;
    if indexed == public_prompts {
        return Ok(());
    }

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| AppError::internal(format!("failed to start FTS rebuild: {e}")))?;
    tx.execute_batch(
        r#"
INSERT INTO prompts_fts(prompts_fts) VALUES('delete-all');
INSERT INTO prompts_fts(rowid, title, description, system_prompt, user_prompt, tags)
SELECT rowid, title, description, system_prompt, user_prompt, tags
FROM prompts
WHERE is_private = 0;
"#,
    )
    .map_err(|e| AppError::internal(format!("failed to rebuild FTS index: {e}")))?;
    tx.commit()
        .map_err(|e| AppError::internal(format!("failed to commit FTS rebuild: {e}")))?;
    Ok(())
}

/// The FTS5 virtual table plus the insert/delete/update sync triggers.
///
/// Transcribed from the design's Search_Engine schema and the reference
/// `@prompthub/db` triggers. The delete/update triggers use FTS5's special
/// `'delete'` command form required for external-content tables to remove the
/// previously indexed row before the new values are inserted.
const FTS_SQL: &str = r#"
BEGIN;

-- FTS5 mirror of prompts (external content), kept in sync by the triggers below.
CREATE VIRTUAL TABLE IF NOT EXISTS prompts_fts USING fts5(
  title, description, system_prompt, user_prompt, tags,
  content='prompts', content_rowid='rowid'
);

DROP TRIGGER IF EXISTS prompts_ai;
DROP TRIGGER IF EXISTS prompts_ad;
DROP TRIGGER IF EXISTS prompts_au;

-- AFTER INSERT: index the new prompt row.
CREATE TRIGGER prompts_ai AFTER INSERT ON prompts BEGIN
  INSERT INTO prompts_fts(rowid, title, description, system_prompt, user_prompt, tags)
  SELECT NEW.rowid, NEW.title, NEW.description, NEW.system_prompt, NEW.user_prompt, NEW.tags
  WHERE NEW.is_private = 0;
END;

-- AFTER DELETE: remove the prompt row from the index (external-content 'delete').
CREATE TRIGGER prompts_ad AFTER DELETE ON prompts BEGIN
  INSERT INTO prompts_fts(prompts_fts, rowid, title, description, system_prompt, user_prompt, tags)
  SELECT 'delete', OLD.rowid, OLD.title, OLD.description, OLD.system_prompt, OLD.user_prompt, OLD.tags
  WHERE OLD.is_private = 0;
END;

-- AFTER UPDATE: remove the old indexed values, then index the new ones.
CREATE TRIGGER prompts_au AFTER UPDATE ON prompts BEGIN
  INSERT INTO prompts_fts(prompts_fts, rowid, title, description, system_prompt, user_prompt, tags)
  SELECT 'delete', OLD.rowid, OLD.title, OLD.description, OLD.system_prompt, OLD.user_prompt, OLD.tags
  WHERE OLD.is_private = 0;
  INSERT INTO prompts_fts(rowid, title, description, system_prompt, user_prompt, tags)
  SELECT NEW.rowid, NEW.title, NEW.description, NEW.system_prompt, NEW.user_prompt, NEW.tags
  WHERE NEW.is_private = 0;
END;

COMMIT;
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{create_memory_pool, init_schema};

    /// Inserts a prompt with the given id/title/user_prompt/tags. Other text
    /// columns are left NULL/default. Returns nothing; panics on failure.
    fn insert_prompt(conn: &Connection, id: &str, title: &str, user_prompt: &str, tags: &str) {
        conn.execute(
            "INSERT INTO prompts (id, title, user_prompt, tags, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, 0, 0)",
            rusqlite::params![id, title, user_prompt, tags],
        )
        .unwrap();
    }

    /// Counts FTS rows matching the given FTS5 query expression.
    fn match_count(conn: &Connection, query: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM prompts_fts WHERE prompts_fts MATCH ?1",
            [query],
            |row| row.get(0),
        )
        .unwrap()
    }

    /// Builds an in-memory database with the base schema and the FTS index.
    fn setup() -> r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager> {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();
        init_fts(&conn).unwrap();
        conn
    }

    #[test]
    fn init_fts_creates_table_and_triggers() {
        let conn = setup();

        let table: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='prompts_fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table, 1, "prompts_fts virtual table should exist");

        for trigger in ["prompts_ai", "prompts_ad", "prompts_au"] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name=?1",
                    [trigger],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "trigger `{trigger}` should exist");
        }
    }

    #[test]
    fn init_fts_is_idempotent() {
        let conn = setup();
        // Running again must not error (table DDL is IF NOT EXISTS; triggers are replaced).
        init_fts(&conn).unwrap();
        init_fts(&conn).unwrap();

        let table: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='prompts_fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table, 1);
    }

    /// Builds an in-memory database with the base schema but without FTS, so
    /// rows can be inserted before [`init_fts`].
    fn setup_schema_only() -> r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager> {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn init_fts_indexes_existing_non_private_prompts() {
        let conn = setup_schema_only();
        for i in 0..3 {
            conn.execute(
                "INSERT INTO prompts (id, title, user_prompt, tags, created_at, updated_at, is_private) \
                 VALUES (?1, ?2, 'body', '[]', 0, 0, 0)",
                rusqlite::params![format!("p{i}"), format!("Keyword{i} UniqueTerm")],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO prompts (id, title, user_prompt, tags, created_at, updated_at, is_private) \
             VALUES ('priv', 'SecretTerm', 'private body', '[]', 0, 0, 1)",
            [],
        )
        .unwrap();

        init_fts(&conn).unwrap();

        assert_eq!(match_count(&conn, "UniqueTerm"), 3);
        assert_eq!(match_count(&conn, "Keyword0"), 1);
        assert_eq!(match_count(&conn, "SecretTerm"), 0);
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompts_fts_docsize", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(total, 3);
    }

    #[test]
    fn init_fts_rebuild_is_idempotent_with_existing_rows() {
        let conn = setup_schema_only();
        insert_prompt(&conn, "p1", "AlphaTerm", "body", "[]");
        init_fts(&conn).unwrap();
        init_fts(&conn).unwrap();
        assert_eq!(match_count(&conn, "AlphaTerm"), 1);
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompts_fts_docsize", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(total, 1);
    }

    #[test]
    fn private_prompt_inserted_after_init_is_not_indexed() {
        let conn = setup();
        conn.execute(
            "INSERT INTO prompts (id, title, user_prompt, tags, created_at, updated_at, is_private) \
             VALUES ('priv', 'SecretTerm', 'private body', '[]', 0, 0, 1)",
            [],
        )
        .unwrap();
        assert_eq!(match_count(&conn, "SecretTerm"), 0);
    }

    #[test]
    fn insert_trigger_indexes_prompt() {
        let conn = setup();
        insert_prompt(&conn, "p1", "Dragon Slayer", "A heroic quest", "[]");

        assert_eq!(match_count(&conn, "Dragon"), 1, "title term should match");
        assert_eq!(
            match_count(&conn, "heroic"),
            1,
            "user_prompt term should match"
        );
        assert_eq!(
            match_count(&conn, "wombat"),
            0,
            "absent term should not match"
        );
    }

    #[test]
    fn match_is_case_insensitive() {
        let conn = setup();
        insert_prompt(&conn, "p1", "Dragon Slayer", "A heroic quest", "[]");

        // FTS5 unicode61 tokenizer lowercases, so MATCH is case-insensitive.
        assert_eq!(match_count(&conn, "dragon"), 1);
        assert_eq!(match_count(&conn, "DRAGON"), 1);
    }

    #[test]
    fn tags_json_tokens_are_searchable() {
        let conn = setup();
        insert_prompt(&conn, "p1", "Untitled", "body", r#"["alpha","beta"]"#);

        // The JSON punctuation is treated as token separators, so each tag value
        // is independently searchable.
        assert_eq!(match_count(&conn, "alpha"), 1, "tag value should match");
        assert_eq!(match_count(&conn, "beta"), 1, "tag value should match");
    }

    #[test]
    fn update_trigger_reindexes_changed_terms() {
        let conn = setup();
        insert_prompt(&conn, "p1", "Dragon Slayer", "A heroic quest", "[]");
        assert_eq!(match_count(&conn, "Dragon"), 1);

        conn.execute(
            "UPDATE prompts SET title = 'Phoenix Rising' WHERE id = 'p1'",
            [],
        )
        .unwrap();

        assert_eq!(match_count(&conn, "Phoenix"), 1, "new term should match");
        assert_eq!(
            match_count(&conn, "Dragon"),
            0,
            "old term should no longer match"
        );
    }

    #[test]
    fn delete_trigger_removes_from_index() {
        let conn = setup();
        insert_prompt(&conn, "p1", "Dragon Slayer", "A heroic quest", "[]");
        assert_eq!(match_count(&conn, "Dragon"), 1);

        conn.execute("DELETE FROM prompts WHERE id = 'p1'", [])
            .unwrap();

        assert_eq!(
            match_count(&conn, "Dragon"),
            0,
            "deleted prompt should not match"
        );
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompts_fts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(total, 0, "index should be empty after delete");
    }

    #[test]
    fn search_reflects_write_within_same_connection() {
        // Read-after-write: a MATCH issued right after the committed insert sees
        // the new row, demonstrating the trigger-maintained sync (Req 5.2).
        let conn = setup();
        insert_prompt(&conn, "p1", "Searchable Term", "content", "[]");
        assert_eq!(match_count(&conn, "Searchable"), 1);
    }
}
