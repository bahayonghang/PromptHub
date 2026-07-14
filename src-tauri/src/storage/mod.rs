//! Storage_Engine: Rust-native SQLite persistence for the Tauri_Backend.
//!
//! This module owns the SQLite connection pool and the fresh, desktop-only schema
//! defined in the design document ("Data Models / Storage_Engine schema"). It
//! provides:
//!
//! - [`create_pool`] / [`create_memory_pool`]: build an r2d2 pool whose
//!   connections are configured with the required PRAGMAs (`journal_mode=WAL`,
//!   `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout`) via a connection
//!   customizer so every pooled connection is consistent (Requirements 4.1,
//!   4.10, 4.11).
//! - [`init_schema`]: execute every `CREATE TABLE IF NOT EXISTS` statement plus
//!   the supporting indexes, idempotently (Requirements 4.2, 4.6).
//!
//! The FTS5 virtual table and its sync triggers are implemented separately in
//! task 2.2; [`FTS_EXTENSION_POINT`] documents where that wiring attaches.

pub mod mapping;
pub mod time;

#[cfg(test)]
pub mod proptest_roundtrip;

#[cfg(test)]
pub mod proptest_atomicity;

use r2d2::{CustomizeConnection, Pool};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::error::AppError;

pub mod fts;

/// The connection pool type used throughout the Storage_Engine.
///
/// Replaces the former `DbPool = ()` placeholder in [`crate::state`].
pub type DbPool = Pool<SqliteConnectionManager>;

/// Busy-timeout applied to every pooled connection, in milliseconds.
///
/// Gives writers up to this long to acquire the database lock before returning
/// `SQLITE_BUSY`, which smooths over brief contention between pooled connections
/// under WAL.
const BUSY_TIMEOUT_MS: u64 = 5_000;

/// Marker note for the Search_Engine wiring (task 2.2).
///
/// The `prompts_fts` FTS5 virtual table and the `prompts_ai`/`prompts_ad`/
/// `prompts_au` sync triggers are created by [`crate::storage`]'s search module
/// once it exists. They attach immediately after [`init_schema`] runs, against
/// the same connection/transaction, so the index is created alongside the base
/// tables. This constant exists only to document that extension point.
pub const FTS_EXTENSION_POINT: &str = "prompts_fts + prompts_{ai,ad,au} triggers (task 2.2)";

/// r2d2 connection customizer that applies the Storage_Engine PRAGMAs to every
/// connection the pool hands out.
///
/// Running these on acquisition (rather than once at pool creation) guarantees
/// each physical connection in the pool shares the same durability and
/// concurrency configuration.
#[derive(Debug)]
struct PragmaCustomizer;

impl CustomizeConnection<Connection, rusqlite::Error> for PragmaCustomizer {
    fn on_acquire(&self, conn: &mut Connection) -> Result<(), rusqlite::Error> {
        apply_pragmas(conn)
    }
}

/// Applies the required PRAGMAs to a single connection.
///
/// - `journal_mode=WAL`: write-ahead logging for read/write concurrency.
/// - `synchronous=NORMAL`: durable under WAL while avoiding an fsync per commit.
/// - `foreign_keys=ON`: enforce the `ON DELETE CASCADE`/`SET NULL` relationships.
/// - `busy_timeout`: wait briefly for locks instead of failing immediately.
///
/// `execute_batch` is used because `PRAGMA journal_mode = WAL` returns the
/// resulting mode as a result row; `execute_batch` runs the statements and
/// discards any rows, which `execute`/`pragma_update` do not reliably do across
/// rusqlite versions. (For an in-memory database WAL is unsupported and is left
/// as the default journal mode, which is expected.)
fn apply_pragmas(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(&format!(
        "PRAGMA journal_mode = WAL;\n\
         PRAGMA synchronous = NORMAL;\n\
         PRAGMA foreign_keys = ON;\n\
         PRAGMA busy_timeout = {BUSY_TIMEOUT_MS};"
    ))
}

/// Creates a connection pool backed by the SQLite database file at `db_path`.
///
/// The parent directory is expected to exist (the Data_Path_Manager creates the
/// runtime directories at startup). Each connection is configured with the
/// Storage_Engine PRAGMAs via [`PragmaCustomizer`].
pub fn create_pool(db_path: impl AsRef<std::path::Path>) -> Result<DbPool, AppError> {
    let manager = SqliteConnectionManager::file(db_path.as_ref());
    build_pool(manager)
}

/// Creates an in-memory pool for tests and ephemeral use.
///
/// Uses a shared-cache in-memory database with a single connection so the schema
/// initialized on one checkout is visible to subsequent checkouts within the same
/// pool, which is convenient for unit tests.
pub fn create_memory_pool() -> Result<DbPool, AppError> {
    let manager = SqliteConnectionManager::memory();
    Pool::builder()
        .max_size(1)
        .connection_customizer(Box::new(PragmaCustomizer))
        .build(manager)
        .map_err(|e| AppError::io(format!("failed to build in-memory connection pool: {e}")))
}

/// Builds a pool from the given manager with the PRAGMA customizer attached.
fn build_pool(manager: SqliteConnectionManager) -> Result<DbPool, AppError> {
    Pool::builder()
        .connection_customizer(Box::new(PragmaCustomizer))
        .build(manager)
        .map_err(|e| AppError::io(format!("failed to build connection pool: {e}")))
}

/// Initializes the fresh, desktop-only schema on the given connection.
///
/// Executes every `CREATE TABLE IF NOT EXISTS` and `CREATE INDEX IF NOT EXISTS`
/// statement in a single batch, wrapped in a transaction so the schema is applied
/// atomically. Safe to call repeatedly: every statement is `IF NOT EXISTS`, so an
/// already-initialized database is left unchanged (Requirement 4.6).
///
/// The FTS5 virtual table and its sync triggers are intentionally *not* created
/// here; see [`FTS_EXTENSION_POINT`].
pub fn init_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(SCHEMA_SQL)
        .map_err(|e| AppError::internal(format!("failed to initialize schema: {e}")))
}

/// Acquires a connection from the pool and runs [`init_schema`] on it.
///
/// Convenience entry point for startup: build a pool, then call this once to make
/// sure the schema exists before any command runs.
pub fn init_schema_with_pool(pool: &DbPool) -> Result<(), AppError> {
    let conn = pool
        .get()
        .map_err(|e| AppError::io(format!("failed to acquire connection from pool: {e}")))?;
    init_schema(&conn)
}

/// The complete schema: every base table plus its supporting indexes.
///
/// Tables and column definitions are transcribed from the design document's
/// Storage_Engine schema section. The multi-user tables and
/// `owner_user_id`/`visibility` columns from the reference are intentionally
/// omitted (out of scope). `prompts.folder_id` uses `ON DELETE SET NULL` so
/// deleting a folder clears the association (Requirement 4.3); version tables use
/// `ON DELETE CASCADE` so deleting the parent removes history (Requirements 4.4,
/// 4.5). The FTS virtual table/triggers live in the Search_Engine module.
const SCHEMA_SQL: &str = r#"
BEGIN;

CREATE TABLE IF NOT EXISTS folders (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  icon TEXT,
  parent_id TEXT REFERENCES folders(id) ON DELETE CASCADE,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS prompts (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT,
  prompt_type TEXT NOT NULL DEFAULT 'text' CHECK(prompt_type IN ('text','image','video')),
  system_prompt TEXT,
  user_prompt TEXT NOT NULL,
  variables TEXT NOT NULL DEFAULT '[]',
  tags TEXT NOT NULL DEFAULT '[]',
  folder_id TEXT REFERENCES folders(id) ON DELETE SET NULL,
  images TEXT NOT NULL DEFAULT '[]',
  videos TEXT NOT NULL DEFAULT '[]',
  is_favorite INTEGER NOT NULL DEFAULT 0,
  is_pinned INTEGER NOT NULL DEFAULT 0,
  current_version INTEGER NOT NULL DEFAULT 0,
  usage_count INTEGER NOT NULL DEFAULT 0,
  source TEXT,
  notes TEXT,
  last_ai_response TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS prompt_versions (
  id TEXT PRIMARY KEY,
  prompt_id TEXT NOT NULL REFERENCES prompts(id) ON DELETE CASCADE,
  version INTEGER NOT NULL,
  system_prompt TEXT,
  user_prompt TEXT NOT NULL,
  variables TEXT NOT NULL DEFAULT '[]',
  note TEXT,
  ai_response TEXT,
  created_at INTEGER NOT NULL,
  UNIQUE(prompt_id, version)
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS rules (
  id TEXT PRIMARY KEY,
  scope TEXT NOT NULL CHECK(scope IN ('global','project')),
  platform_id TEXT NOT NULL,
  platform_name TEXT NOT NULL,
  platform_icon TEXT NOT NULL,
  platform_description TEXT NOT NULL,
  canonical_file_name TEXT NOT NULL,
  description TEXT NOT NULL,
  managed_path TEXT NOT NULL,
  target_path TEXT NOT NULL,
  project_root_path TEXT,
  sync_status TEXT NOT NULL CHECK(sync_status IN ('synced','target-missing','out-of-sync','sync-error')),
  current_version INTEGER NOT NULL DEFAULT 0,
  content_hash TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS rule_versions (
  id TEXT PRIMARY KEY,
  rule_id TEXT NOT NULL REFERENCES rules(id) ON DELETE CASCADE,
  version INTEGER NOT NULL,
  file_path TEXT NOT NULL,
  source TEXT NOT NULL CHECK(source IN ('manual-save','ai-rewrite','create')),
  created_at INTEGER NOT NULL,
  UNIQUE(rule_id, version)
);

CREATE INDEX IF NOT EXISTS idx_prompts_folder   ON prompts(folder_id);
CREATE INDEX IF NOT EXISTS idx_prompts_updated  ON prompts(updated_at);
CREATE INDEX IF NOT EXISTS idx_prompts_favorite ON prompts(is_favorite);
CREATE INDEX IF NOT EXISTS idx_prompts_pinned   ON prompts(is_pinned);
CREATE INDEX IF NOT EXISTS idx_prompts_created  ON prompts(created_at);
CREATE INDEX IF NOT EXISTS idx_prompts_usage    ON prompts(usage_count);
CREATE INDEX IF NOT EXISTS idx_versions_prompt  ON prompt_versions(prompt_id);
CREATE INDEX IF NOT EXISTS idx_folders_parent   ON folders(parent_id);
CREATE INDEX IF NOT EXISTS idx_folders_sort     ON folders(sort_order);
CREATE INDEX IF NOT EXISTS idx_rules_scope      ON rules(scope);
CREATE INDEX IF NOT EXISTS idx_rules_platform   ON rules(platform_id);
CREATE INDEX IF NOT EXISTS idx_rule_versions_rule ON rule_versions(rule_id);

COMMIT;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// Expected base tables created by [`init_schema`].
    const EXPECTED_TABLES: &[&str] = &[
        "folders",
        "prompts",
        "prompt_versions",
        "settings",
        "rules",
        "rule_versions",
    ];

    /// Expected indexes created by [`init_schema`].
    const EXPECTED_INDEXES: &[&str] = &[
        "idx_prompts_folder",
        "idx_prompts_updated",
        "idx_prompts_favorite",
        "idx_prompts_pinned",
        "idx_prompts_created",
        "idx_prompts_usage",
        "idx_versions_prompt",
        "idx_folders_parent",
        "idx_folders_sort",
        "idx_rules_scope",
        "idx_rules_platform",
        "idx_rule_versions_rule",
    ];

    /// Collects names from `sqlite_master` for the given object type.
    fn names_of_type(conn: &Connection, object_type: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = ?1")
            .unwrap();
        let rows = stmt
            .query_map([object_type], |row| row.get::<_, String>(0))
            .unwrap();
        rows.map(Result::unwrap).collect()
    }

    #[test]
    fn init_schema_creates_all_expected_tables() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();

        let tables = names_of_type(&conn, "table");
        for expected in EXPECTED_TABLES {
            assert!(
                tables.iter().any(|t| t == expected),
                "expected table `{expected}` to exist; found: {tables:?}"
            );
        }
    }

    #[test]
    fn init_schema_creates_all_expected_indexes() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();

        let indexes = names_of_type(&conn, "index");
        for expected in EXPECTED_INDEXES {
            assert!(
                indexes.iter().any(|i| i == expected),
                "expected index `{expected}` to exist; found: {indexes:?}"
            );
        }
    }

    #[test]
    fn fresh_schema_does_not_create_retired_skill_tables_or_indexes() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();

        let tables = names_of_type(&conn, "table");
        assert!(!tables.iter().any(|name| name == "skills"));
        assert!(!tables.iter().any(|name| name == "skill_versions"));

        let indexes = names_of_type(&conn, "index");
        assert!(!indexes.iter().any(|name| name == "idx_skills_updated"));
        assert!(!indexes
            .iter()
            .any(|name| name == "idx_skill_versions_skill"));
    }

    #[test]
    fn init_schema_preserves_legacy_skill_rows() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE skills (id TEXT PRIMARY KEY, name TEXT NOT NULL);
            CREATE TABLE skill_versions (
              id TEXT PRIMARY KEY,
              skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
              content TEXT
            );
            INSERT INTO skills (id, name) VALUES ('legacy-skill', 'Legacy');
            INSERT INTO skill_versions (id, skill_id, content)
              VALUES ('legacy-version', 'legacy-skill', '# Legacy');
            "#,
        )
        .unwrap();

        let skill_before: (String, String) = conn
            .query_row(
                "SELECT id, name FROM skills WHERE id = 'legacy-skill'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let version_before: (String, String, String) = conn
            .query_row(
                r#"
                SELECT id, skill_id, content
                FROM skill_versions
                WHERE id = 'legacy-version'
                "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        init_schema(&conn).unwrap();
        crate::services::prompt::create(
            &conn,
            crate::services::prompt::PromptCreate {
                title: "New prompt".into(),
                user_prompt: "Prompt content".into(),
                ..Default::default()
            },
        )
        .unwrap();

        let skill_after: (String, String) = conn
            .query_row(
                "SELECT id, name FROM skills WHERE id = 'legacy-skill'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        let version_after: (String, String, String) = conn
            .query_row(
                r#"
                SELECT id, skill_id, content
                FROM skill_versions
                WHERE id = 'legacy-version'
                "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(skill_after, skill_before);
        assert_eq!(version_after, version_before);
    }

    #[test]
    fn pragmas_enable_foreign_keys_wal_and_busy_timeout() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();

        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(foreign_keys, 1, "foreign_keys PRAGMA should be ON");

        let busy_timeout: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();
        assert_eq!(busy_timeout, BUSY_TIMEOUT_MS as i64);

        // synchronous=NORMAL maps to integer 1.
        let synchronous: i64 = conn
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        assert_eq!(synchronous, 1, "synchronous PRAGMA should be NORMAL (1)");
    }

    #[test]
    fn init_schema_is_idempotent() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();
        // Running again must not error (every statement is IF NOT EXISTS).
        init_schema(&conn).unwrap();

        let tables = names_of_type(&conn, "table");
        assert!(tables.iter().any(|t| t == "prompts"));
    }

    #[test]
    fn foreign_key_cascade_deletes_prompt_versions() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO prompts (id, title, user_prompt, created_at, updated_at) \
             VALUES ('p1', 'T', 'U', 0, 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO prompt_versions (id, prompt_id, version, user_prompt, created_at) \
             VALUES ('v1', 'p1', 1, 'U', 0)",
            [],
        )
        .unwrap();

        conn.execute("DELETE FROM prompts WHERE id = 'p1'", [])
            .unwrap();

        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompt_versions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(remaining, 0, "deleting a prompt should cascade to versions");
    }

    #[test]
    fn deleting_folder_sets_prompt_folder_id_null() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO folders (id, name, created_at) VALUES ('f1', 'F', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO prompts (id, title, user_prompt, folder_id, created_at, updated_at) \
             VALUES ('p1', 'T', 'U', 'f1', 0, 0)",
            [],
        )
        .unwrap();

        conn.execute("DELETE FROM folders WHERE id = 'f1'", [])
            .unwrap();

        let folder_id: Option<String> = conn
            .query_row("SELECT folder_id FROM prompts WHERE id = 'p1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(
            folder_id.is_none(),
            "deleting a folder should clear the prompt's folder association"
        );
    }

    #[test]
    fn create_pool_on_temp_file_initializes_schema() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("prompthub.db");
        let pool = create_pool(&db_path).unwrap();
        init_schema_with_pool(&pool).unwrap();

        let conn = pool.get().unwrap();
        let tables = names_of_type(&conn, "table");
        for expected in EXPECTED_TABLES {
            assert!(
                tables.iter().any(|t| t == expected),
                "expected table `{expected}` in file-backed db"
            );
        }
        assert!(db_path.exists(), "database file should have been created");
    }
}
