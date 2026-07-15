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

/// Latest ordered schema migration understood by this binary.
pub const CURRENT_SCHEMA_VERSION: u32 = 5;

struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        // Version 1 adopts the pre-migration desktop schema as the baseline.
        sql: "",
    },
    Migration {
        version: 2,
        sql: r#"
ALTER TABLE prompt_versions ADD COLUMN title TEXT NOT NULL DEFAULT '';
ALTER TABLE prompt_versions ADD COLUMN description TEXT;
ALTER TABLE prompt_versions ADD COLUMN prompt_type TEXT NOT NULL DEFAULT 'text';
ALTER TABLE prompt_versions ADD COLUMN tags TEXT NOT NULL DEFAULT '[]';
ALTER TABLE prompt_versions ADD COLUMN folder_id TEXT;
ALTER TABLE prompt_versions ADD COLUMN images TEXT NOT NULL DEFAULT '[]';
ALTER TABLE prompt_versions ADD COLUMN videos TEXT NOT NULL DEFAULT '[]';
ALTER TABLE prompt_versions ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0;
ALTER TABLE prompt_versions ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE prompt_versions ADD COLUMN source TEXT;
ALTER TABLE prompt_versions ADD COLUMN notes TEXT;
ALTER TABLE prompt_versions ADD COLUMN source_action TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE prompt_versions ADD COLUMN parent_revision_id TEXT;

UPDATE prompt_versions
SET title = COALESCE((SELECT title FROM prompts WHERE prompts.id = prompt_versions.prompt_id), ''),
    description = (SELECT description FROM prompts WHERE prompts.id = prompt_versions.prompt_id),
    prompt_type = COALESCE((SELECT prompt_type FROM prompts WHERE prompts.id = prompt_versions.prompt_id), 'text'),
    tags = COALESCE((SELECT tags FROM prompts WHERE prompts.id = prompt_versions.prompt_id), '[]'),
    folder_id = (SELECT folder_id FROM prompts WHERE prompts.id = prompt_versions.prompt_id),
    images = COALESCE((SELECT images FROM prompts WHERE prompts.id = prompt_versions.prompt_id), '[]'),
    videos = COALESCE((SELECT videos FROM prompts WHERE prompts.id = prompt_versions.prompt_id), '[]'),
    is_favorite = COALESCE((SELECT is_favorite FROM prompts WHERE prompts.id = prompt_versions.prompt_id), 0),
    is_pinned = COALESCE((SELECT is_pinned FROM prompts WHERE prompts.id = prompt_versions.prompt_id), 0),
    source = (SELECT source FROM prompts WHERE prompts.id = prompt_versions.prompt_id),
    notes = (SELECT notes FROM prompts WHERE prompts.id = prompt_versions.prompt_id);

CREATE INDEX IF NOT EXISTS idx_versions_parent ON prompt_versions(parent_revision_id);
        "#,
    },
    Migration {
        version: 3,
        sql: r#"
ALTER TABLE prompts ADD COLUMN is_private INTEGER NOT NULL DEFAULT 0;
ALTER TABLE prompt_versions ADD COLUMN is_private INTEGER NOT NULL DEFAULT 0;
"#,
    },
    Migration {
        version: 4,
        sql: r#"
ALTER TABLE prompts ADD COLUMN messages TEXT NOT NULL DEFAULT '[]';
ALTER TABLE prompt_versions ADD COLUMN messages TEXT NOT NULL DEFAULT '[]';

CREATE TABLE execution_profile_revisions (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  name TEXT NOT NULL,
  provider TEXT NOT NULL CHECK(provider IN ('mock','openai-compatible')),
  endpoint TEXT,
  model TEXT NOT NULL,
  parameters TEXT NOT NULL DEFAULT '{}',
  credential TEXT,
  created_at INTEGER NOT NULL,
  UNIQUE(profile_id, revision)
);

CREATE TABLE prompt_runs (
  id TEXT PRIMARY KEY,
  prompt_revision_id TEXT NOT NULL,
  profile_revision_id TEXT NOT NULL,
  test_case_id TEXT,
  inputs TEXT NOT NULL DEFAULT '{}',
  rendered_messages TEXT NOT NULL DEFAULT '[]',
  output TEXT,
  status TEXT NOT NULL CHECK(status IN ('running','success','error','cancelled')),
  error TEXT,
  started_at INTEGER NOT NULL,
  completed_at INTEGER,
  duration_ms INTEGER,
  usage TEXT,
  cache_key TEXT
);

CREATE TABLE test_sets (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE test_cases (
  id TEXT PRIMARY KEY,
  test_set_id TEXT NOT NULL REFERENCES test_sets(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  inputs TEXT NOT NULL DEFAULT '{}',
  expected_output TEXT,
  annotations TEXT NOT NULL DEFAULT '{}',
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE evaluator_configs (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL CHECK(kind IN ('manual','exact','contains','regex','numeric')),
  config TEXT NOT NULL DEFAULT '{}',
  created_at INTEGER NOT NULL
);

CREATE TABLE evaluation_runs (
  id TEXT PRIMARY KEY,
  test_set_id TEXT NOT NULL,
  prompt_revision_ids TEXT NOT NULL,
  profile_revision_ids TEXT NOT NULL,
  evaluator_ids TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('running','success','error','cancelled')),
  total_cells INTEGER NOT NULL,
  completed_cells INTEGER NOT NULL DEFAULT 0,
  failed_cells INTEGER NOT NULL DEFAULT 0,
  started_at INTEGER NOT NULL,
  completed_at INTEGER,
  runtime_version TEXT NOT NULL
);

CREATE TABLE evaluation_cells (
  id TEXT PRIMARY KEY,
  evaluation_run_id TEXT NOT NULL REFERENCES evaluation_runs(id) ON DELETE CASCADE,
  prompt_revision_id TEXT NOT NULL,
  profile_revision_id TEXT NOT NULL,
  test_case_id TEXT NOT NULL,
  prompt_run_id TEXT,
  status TEXT NOT NULL CHECK(status IN ('pending','running','success','error','cancelled','skipped')),
  cache_hit INTEGER NOT NULL DEFAULT 0,
  results TEXT NOT NULL DEFAULT '[]',
  error TEXT,
  cache_key TEXT NOT NULL,
  sort_order INTEGER NOT NULL
);

CREATE TABLE prompt_labels (
  prompt_id TEXT NOT NULL,
  label TEXT NOT NULL,
  prompt_revision_id TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(prompt_id, label)
);

CREATE TABLE prompt_label_history (
  id TEXT PRIMARY KEY,
  prompt_id TEXT NOT NULL,
  label TEXT NOT NULL,
  from_revision_id TEXT,
  to_revision_id TEXT NOT NULL,
  action TEXT NOT NULL CHECK(action IN ('move','rollback')),
  created_at INTEGER NOT NULL
);

CREATE INDEX idx_profile_revisions_profile ON execution_profile_revisions(profile_id, revision);
CREATE INDEX idx_prompt_runs_started ON prompt_runs(started_at DESC);
CREATE INDEX idx_prompt_runs_revision ON prompt_runs(prompt_revision_id, profile_revision_id);
CREATE INDEX idx_test_cases_set ON test_cases(test_set_id, sort_order);
CREATE INDEX idx_evaluation_runs_started ON evaluation_runs(started_at DESC);
CREATE INDEX idx_evaluation_cells_run ON evaluation_cells(evaluation_run_id, sort_order);
CREATE INDEX idx_evaluation_cells_cache ON evaluation_cells(cache_key, status);
CREATE INDEX idx_label_history_prompt ON prompt_label_history(prompt_id, label, created_at DESC);
"#,
    },
    Migration {
        version: 5,
        sql: r#"
CREATE TABLE prompt_type_definitions (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL UNIQUE,
  base_kind TEXT NOT NULL CHECK(base_kind IN ('text','image','video')),
  created_at INTEGER NOT NULL
);

ALTER TABLE prompts ADD COLUMN type_definition_id TEXT REFERENCES prompt_type_definitions(id);
ALTER TABLE prompt_versions ADD COLUMN type_definition_id TEXT;
ALTER TABLE prompt_versions ADD COLUMN type_definition_name TEXT;
ALTER TABLE prompt_versions ADD COLUMN type_definition_base_kind TEXT CHECK(type_definition_base_kind IN ('text','image','video'));

CREATE INDEX idx_prompts_type_definition ON prompts(type_definition_id);
CREATE INDEX idx_versions_type_definition ON prompt_versions(type_definition_id);
"#,
    },
];

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
    if schema_version(conn)? == 0 && !table_exists(conn, "prompts")? {
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::internal(format!("failed to start schema creation: {e}")))?;
        tx.execute_batch(SCHEMA_SQL)
            .map_err(|e| AppError::internal(format!("failed to create schema: {e}")))?;
        tx.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)
            .map_err(|e| AppError::internal(format!("failed to record schema version: {e}")))?;
        tx.commit()
            .map_err(|e| AppError::internal(format!("failed to commit schema creation: {e}")))?;
        return Ok(());
    }
    run_migrations(conn, MIGRATIONS)
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool, AppError> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [name],
        |row| row.get(0),
    )
    .map_err(|e| AppError::internal(format!("failed to inspect schema tables: {e}")))
}

/// Returns the migration version recorded in SQLite's `user_version` pragma.
pub fn schema_version(conn: &Connection) -> Result<u32, AppError> {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| AppError::internal(format!("failed to read schema version: {e}")))
}

/// Reports whether an existing database must be upgraded before use.
pub fn needs_migration(conn: &Connection) -> Result<bool, AppError> {
    Ok(schema_version(conn)? < CURRENT_SCHEMA_VERSION)
}

fn run_migrations(conn: &Connection, migrations: &[Migration]) -> Result<(), AppError> {
    let mut current = schema_version(conn)?;
    if current > CURRENT_SCHEMA_VERSION {
        return Err(AppError::validation(format!(
            "database schema version {current} is newer than supported version {CURRENT_SCHEMA_VERSION}"
        )));
    }

    for migration in migrations {
        if migration.version <= current {
            continue;
        }
        if migration.version != current + 1 {
            return Err(AppError::internal(format!(
                "schema migration sequence jumps from {current} to {}",
                migration.version
            )));
        }

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::internal(format!("failed to start schema migration: {e}")))?;
        tx.execute_batch(migration.sql).map_err(|e| {
            AppError::internal(format!(
                "failed to apply schema migration {}: {e}",
                migration.version
            ))
        })?;
        tx.pragma_update(None, "user_version", migration.version)
            .map_err(|e| {
                AppError::internal(format!(
                    "failed to record schema migration {}: {e}",
                    migration.version
                ))
            })?;
        tx.commit().map_err(|e| {
            AppError::internal(format!(
                "failed to commit schema migration {}: {e}",
                migration.version
            ))
        })?;
        current = migration.version;
    }

    Ok(())
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
CREATE TABLE IF NOT EXISTS prompt_type_definitions (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL UNIQUE,
  base_kind TEXT NOT NULL CHECK(base_kind IN ('text','image','video')),
  created_at INTEGER NOT NULL
);

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
  type_definition_id TEXT REFERENCES prompt_type_definitions(id),
  system_prompt TEXT,
  user_prompt TEXT NOT NULL,
  messages TEXT NOT NULL DEFAULT '[]',
  variables TEXT NOT NULL DEFAULT '[]',
  tags TEXT NOT NULL DEFAULT '[]',
  folder_id TEXT REFERENCES folders(id) ON DELETE SET NULL,
  images TEXT NOT NULL DEFAULT '[]',
  videos TEXT NOT NULL DEFAULT '[]',
  is_favorite INTEGER NOT NULL DEFAULT 0,
  is_pinned INTEGER NOT NULL DEFAULT 0,
  is_private INTEGER NOT NULL DEFAULT 0,
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
  messages TEXT NOT NULL DEFAULT '[]',
  variables TEXT NOT NULL DEFAULT '[]',
  title TEXT NOT NULL DEFAULT '',
  description TEXT,
  prompt_type TEXT NOT NULL DEFAULT 'text' CHECK(prompt_type IN ('text','image','video')),
  type_definition_id TEXT,
  type_definition_name TEXT,
  type_definition_base_kind TEXT CHECK(type_definition_base_kind IN ('text','image','video')),
  tags TEXT NOT NULL DEFAULT '[]',
  folder_id TEXT,
  images TEXT NOT NULL DEFAULT '[]',
  videos TEXT NOT NULL DEFAULT '[]',
  is_favorite INTEGER NOT NULL DEFAULT 0,
  is_pinned INTEGER NOT NULL DEFAULT 0,
  is_private INTEGER NOT NULL DEFAULT 0,
  source TEXT,
  notes TEXT,
  note TEXT,
  ai_response TEXT,
  source_action TEXT NOT NULL DEFAULT 'manual',
  parent_revision_id TEXT,
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

CREATE TABLE IF NOT EXISTS execution_profile_revisions (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  name TEXT NOT NULL,
  provider TEXT NOT NULL CHECK(provider IN ('mock','openai-compatible')),
  endpoint TEXT,
  model TEXT NOT NULL,
  parameters TEXT NOT NULL DEFAULT '{}',
  credential TEXT,
  created_at INTEGER NOT NULL,
  UNIQUE(profile_id, revision)
);

CREATE TABLE IF NOT EXISTS prompt_runs (
  id TEXT PRIMARY KEY,
  prompt_revision_id TEXT NOT NULL,
  profile_revision_id TEXT NOT NULL,
  test_case_id TEXT,
  inputs TEXT NOT NULL DEFAULT '{}',
  rendered_messages TEXT NOT NULL DEFAULT '[]',
  output TEXT,
  status TEXT NOT NULL CHECK(status IN ('running','success','error','cancelled')),
  error TEXT,
  started_at INTEGER NOT NULL,
  completed_at INTEGER,
  duration_ms INTEGER,
  usage TEXT,
  cache_key TEXT
);

CREATE TABLE IF NOT EXISTS test_sets (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS test_cases (
  id TEXT PRIMARY KEY,
  test_set_id TEXT NOT NULL REFERENCES test_sets(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  inputs TEXT NOT NULL DEFAULT '{}',
  expected_output TEXT,
  annotations TEXT NOT NULL DEFAULT '{}',
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS evaluator_configs (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL CHECK(kind IN ('manual','exact','contains','regex','numeric')),
  config TEXT NOT NULL DEFAULT '{}',
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evaluation_runs (
  id TEXT PRIMARY KEY,
  test_set_id TEXT NOT NULL,
  prompt_revision_ids TEXT NOT NULL,
  profile_revision_ids TEXT NOT NULL,
  evaluator_ids TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('running','success','error','cancelled')),
  total_cells INTEGER NOT NULL,
  completed_cells INTEGER NOT NULL DEFAULT 0,
  failed_cells INTEGER NOT NULL DEFAULT 0,
  started_at INTEGER NOT NULL,
  completed_at INTEGER,
  runtime_version TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS evaluation_cells (
  id TEXT PRIMARY KEY,
  evaluation_run_id TEXT NOT NULL REFERENCES evaluation_runs(id) ON DELETE CASCADE,
  prompt_revision_id TEXT NOT NULL,
  profile_revision_id TEXT NOT NULL,
  test_case_id TEXT NOT NULL,
  prompt_run_id TEXT,
  status TEXT NOT NULL CHECK(status IN ('pending','running','success','error','cancelled','skipped')),
  cache_hit INTEGER NOT NULL DEFAULT 0,
  results TEXT NOT NULL DEFAULT '[]',
  error TEXT,
  cache_key TEXT NOT NULL,
  sort_order INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS prompt_labels (
  prompt_id TEXT NOT NULL,
  label TEXT NOT NULL,
  prompt_revision_id TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(prompt_id, label)
);

CREATE TABLE IF NOT EXISTS prompt_label_history (
  id TEXT PRIMARY KEY,
  prompt_id TEXT NOT NULL,
  label TEXT NOT NULL,
  from_revision_id TEXT,
  to_revision_id TEXT NOT NULL,
  action TEXT NOT NULL CHECK(action IN ('move','rollback')),
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_prompts_folder   ON prompts(folder_id);
CREATE INDEX IF NOT EXISTS idx_prompts_updated  ON prompts(updated_at);
CREATE INDEX IF NOT EXISTS idx_prompts_favorite ON prompts(is_favorite);
CREATE INDEX IF NOT EXISTS idx_prompts_pinned   ON prompts(is_pinned);
CREATE INDEX IF NOT EXISTS idx_prompts_created  ON prompts(created_at);
CREATE INDEX IF NOT EXISTS idx_prompts_usage    ON prompts(usage_count);
CREATE INDEX IF NOT EXISTS idx_versions_prompt  ON prompt_versions(prompt_id);
CREATE INDEX IF NOT EXISTS idx_versions_parent  ON prompt_versions(parent_revision_id);
CREATE INDEX IF NOT EXISTS idx_prompts_type_definition ON prompts(type_definition_id);
CREATE INDEX IF NOT EXISTS idx_versions_type_definition ON prompt_versions(type_definition_id);
CREATE INDEX IF NOT EXISTS idx_folders_parent   ON folders(parent_id);
CREATE INDEX IF NOT EXISTS idx_folders_sort     ON folders(sort_order);
CREATE INDEX IF NOT EXISTS idx_rules_scope      ON rules(scope);
CREATE INDEX IF NOT EXISTS idx_rules_platform   ON rules(platform_id);
CREATE INDEX IF NOT EXISTS idx_rule_versions_rule ON rule_versions(rule_id);
CREATE INDEX IF NOT EXISTS idx_profile_revisions_profile ON execution_profile_revisions(profile_id, revision);
CREATE INDEX IF NOT EXISTS idx_prompt_runs_started ON prompt_runs(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_prompt_runs_revision ON prompt_runs(prompt_revision_id, profile_revision_id);
CREATE INDEX IF NOT EXISTS idx_test_cases_set ON test_cases(test_set_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_evaluation_runs_started ON evaluation_runs(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_evaluation_cells_run ON evaluation_cells(evaluation_run_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_evaluation_cells_cache ON evaluation_cells(cache_key, status);
CREATE INDEX IF NOT EXISTS idx_label_history_prompt ON prompt_label_history(prompt_id, label, created_at DESC);

"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// Expected base tables created by [`init_schema`].
    const EXPECTED_TABLES: &[&str] = &[
        "prompt_type_definitions",
        "folders",
        "prompts",
        "prompt_versions",
        "settings",
        "rules",
        "rule_versions",
        "execution_profile_revisions",
        "prompt_runs",
        "test_sets",
        "test_cases",
        "evaluator_configs",
        "evaluation_runs",
        "evaluation_cells",
        "prompt_labels",
        "prompt_label_history",
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
        "idx_versions_parent",
        "idx_prompts_type_definition",
        "idx_versions_type_definition",
        "idx_folders_parent",
        "idx_folders_sort",
        "idx_rules_scope",
        "idx_rules_platform",
        "idx_rule_versions_rule",
        "idx_profile_revisions_profile",
        "idx_prompt_runs_started",
        "idx_prompt_runs_revision",
        "idx_test_cases_set",
        "idx_evaluation_runs_started",
        "idx_evaluation_cells_run",
        "idx_evaluation_cells_cache",
        "idx_label_history_prompt",
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
        assert_eq!(schema_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migration_failure_rolls_back_step_and_preserves_existing_data() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('kept', 'value')",
            [],
        )
        .unwrap();

        let failing = Migration {
            version: CURRENT_SCHEMA_VERSION + 1,
            sql: "CREATE TABLE should_roll_back (id TEXT); THIS IS INVALID SQL;",
        };
        let err = run_migrations(&conn, &[failing]).unwrap_err();
        assert!(err.message.contains("failed to apply schema migration"));
        assert_eq!(schema_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);
        assert_eq!(
            conn.query_row("SELECT value FROM settings WHERE key = 'kept'", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap(),
            "value"
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'should_roll_back'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
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

    #[test]
    fn schema_v5_upgrade_is_additive_and_idempotent() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE prompts (
              id TEXT PRIMARY KEY,
              title TEXT NOT NULL,
              prompt_type TEXT NOT NULL,
              user_prompt TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE prompt_versions (
              id TEXT PRIMARY KEY,
              prompt_id TEXT NOT NULL,
              version INTEGER NOT NULL,
              prompt_type TEXT NOT NULL,
              user_prompt TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            INSERT INTO prompts VALUES ('p1','Legacy','text','exact bytes',1,2);
            INSERT INTO prompt_versions VALUES ('v1','p1',1,'text','exact bytes',1);
            PRAGMA user_version = 4;
            "#,
        )
        .unwrap();

        run_migrations(&conn, MIGRATIONS).unwrap();
        run_migrations(&conn, MIGRATIONS).unwrap();

        assert_eq!(schema_version(&conn).unwrap(), 5);
        let prompt: (String, String, Option<String>) = conn
            .query_row(
                "SELECT title,user_prompt,type_definition_id FROM prompts WHERE id='p1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(prompt, ("Legacy".into(), "exact bytes".into(), None));
        let revision: (String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT user_prompt,type_definition_name,type_definition_base_kind FROM prompt_versions WHERE id='v1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(revision, ("exact bytes".into(), None, None));
        assert!(names_of_type(&conn, "table")
            .iter()
            .any(|name| name == "prompt_type_definitions"));
    }
}
