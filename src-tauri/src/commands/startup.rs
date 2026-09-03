//! Startup sequence for the Tauri_Backend (task 17.2, refined in task 23.1).
//!
//! Implements the "Application State and Startup Sequence" from the design:
//!
//! ```text
//! resolve data path -> ensure dirs -> open pool + schema -> set ready -> create window
//! ```
//!
//! The orchestration runs once, from the Tauri `setup` hook in
//! [`crate::run`], against the process-wide [`AppState`]. On success the SQLite
//! pool is installed and the `ready` gate is opened so commands may execute
//! (Requirements 4.6, 23.1). On any failure — a runtime directory that cannot be
//! created or is not writable (Requirement 23.3), or a database/schema
//! initialization failure (Requirement 4.7) — the gate stays closed, a fatal
//! init error is recorded on the state for the Frontend to surface, and existing
//! user data is left untouched (Requirements 4.7, 23.3).
//!
//! ## Ownership of directory resolution (task 23.1)
//!
//! The per-user directory resolution, writability probe, and database-path
//! convention are owned by the Data_Path_Manager
//! ([`crate::services::data_path`]) — the single source of truth shared with the
//! Window_Manager's `get_runtime_paths` report (Req 20.9, 23.2). This module
//! re-exports [`resolve_runtime_paths`], [`database_path`], and
//! [`ensure_directories`] so the startup sequence and [`crate::run`] keep their
//! existing call sites while the logic lives in one place.
//!
//! ## Testability
//!
//! The directory resolution, writability check, and storage initialization are
//! pure functions over plain paths (no `AppHandle`, no live window), so the unit
//! tests below drive the whole sequence with [`tempfile`] trees. The thin
//! `AppHandle`-bound glue — resolving the per-user application-data root via the
//! Tauri `path` API — lives in [`crate::run`].

use crate::error::AppError;
use crate::services::data_path;
use crate::services::sync;
use crate::state::AppState;
use crate::storage::{self, DbPool};

pub use crate::services::data_path::{database_path, ensure_directories, resolve_runtime_paths};

/// Opens the SQLite connection pool and initializes the schema and FTS index
/// (Requirements 4.6, 4.2, 5.1).
///
/// The database file is created at `<data>/prompthub.db` if absent, the base
/// schema is applied idempotently, and the FTS5 index + sync triggers are
/// installed on the same database. A failure returns a structured error; because
/// the schema statements are `CREATE ... IF NOT EXISTS`, any existing database is
/// left unchanged (Requirement 4.7).
pub fn initialize_storage(paths: &crate::state::RuntimePaths) -> Result<DbPool, AppError> {
    let db_path = data_path::database_path(paths);
    if db_path.is_file()
        && std::fs::metadata(&db_path)
            .map_err(|e| AppError::io(format!("failed to inspect database before migration: {e}")))?
            .len()
            > 0
    {
        let needs_migration = {
            let conn = rusqlite::Connection::open(&db_path)
                .map_err(|e| AppError::io(format!("failed to inspect database version: {e}")))?;
            storage::needs_migration(&conn)?
        };
        if needs_migration {
            sync::backup_create(&paths.data, &paths.backup).map_err(|error| {
                AppError::io(format!(
                    "schema migration requires a safety backup: {error}"
                ))
            })?;
        }
    }

    let pool = storage::create_pool(&db_path)?;
    {
        let conn = pool
            .get()
            .map_err(|e| AppError::io(format!("failed to acquire database connection: {e}")))?;
        storage::init_schema(&conn)?;
        storage::fts::init_fts(&conn)?;
    }
    Ok(pool)
}

/// Runs the full startup sequence against `state`: ensure runtime directories,
/// open the pool, initialize the schema + FTS, then open the `ready` gate
/// (design "Application State and Startup Sequence").
///
/// On success the pool is installed and `ready` is set so commands may execute
/// (Requirement 4.6). On failure the `ready` gate stays closed, the fatal error
/// is recorded on the state for the Frontend (Requirements 4.7, 23.3), and the
/// same error is returned to the caller for logging.
pub fn run_startup(state: &AppState) -> Result<(), AppError> {
    if let Err(error) = initialize(state) {
        crate::logging::event_to(
            &state.paths.log,
            crate::logging::Level::Error,
            "startup",
            format!("startup failed: {error}"),
        );
        state.set_init_error(error.clone());
        return Err(error);
    }
    state.set_ready(true);
    Ok(())
}

/// The fallible body of [`run_startup`]: ensure dirs, open pool + schema, install
/// the pool. Kept separate so the `ready`/`init_error` bookkeeping lives in one
/// place.
fn initialize(state: &AppState) -> Result<(), AppError> {
    data_path::ensure_directories(&state.paths)?;
    let pool = initialize_storage(&state.paths)?;
    state.set_pool(pool)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn initialize_storage_creates_database_and_schema() {
        let tmp = TempDir::new().unwrap();
        let paths = resolve_runtime_paths(tmp.path());
        ensure_directories(&paths).unwrap();

        let pool = initialize_storage(&paths).unwrap();
        assert!(database_path(&paths).exists());

        // Schema + FTS were applied: the base table and FTS table both exist.
        let conn = pool.get().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name IN ('prompts','prompts_fts')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
        assert_eq!(
            storage::schema_version(&conn).unwrap(),
            storage::CURRENT_SCHEMA_VERSION
        );
        assert!(sync::backup_list(&paths.backup).unwrap().is_empty());
    }

    #[test]
    fn initialize_storage_backs_up_existing_database_before_migration() {
        let tmp = TempDir::new().unwrap();
        let paths = resolve_runtime_paths(tmp.path());
        ensure_directories(&paths).unwrap();

        {
            let conn = rusqlite::Connection::open(database_path(&paths)).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE prompts (
                  id TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT,
                  prompt_type TEXT NOT NULL DEFAULT 'text', system_prompt TEXT,
                  user_prompt TEXT NOT NULL, variables TEXT NOT NULL DEFAULT '[]',
                  tags TEXT NOT NULL DEFAULT '[]', folder_id TEXT,
                  images TEXT NOT NULL DEFAULT '[]', videos TEXT NOT NULL DEFAULT '[]',
                  is_favorite INTEGER NOT NULL DEFAULT 0, is_pinned INTEGER NOT NULL DEFAULT 0,
                  current_version INTEGER NOT NULL DEFAULT 0, usage_count INTEGER NOT NULL DEFAULT 0,
                  source TEXT, notes TEXT, last_ai_response TEXT,
                  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
                );
                CREATE TABLE prompt_versions (
                  id TEXT PRIMARY KEY, prompt_id TEXT NOT NULL, version INTEGER NOT NULL,
                  system_prompt TEXT, user_prompt TEXT NOT NULL,
                  variables TEXT NOT NULL DEFAULT '[]', note TEXT, ai_response TEXT,
                  created_at INTEGER NOT NULL, UNIQUE(prompt_id, version)
                );
                INSERT INTO prompts (id,title,user_prompt,created_at,updated_at)
                  VALUES ('p1','T','U',0,0);
                INSERT INTO prompt_versions
                  (id,prompt_id,version,user_prompt,created_at)
                  VALUES ('v1','p1',1,'U',0);
                "#,
            )
            .unwrap();
        }

        let pool = initialize_storage(&paths).unwrap();
        let conn = pool.get().unwrap();
        assert_eq!(
            storage::schema_version(&conn).unwrap(),
            storage::CURRENT_SCHEMA_VERSION
        );
        assert_eq!(
            conn.query_row("SELECT title FROM prompts WHERE id = 'p1'", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap(),
            "T"
        );
        assert_eq!(sync::backup_list(&paths.backup).unwrap().len(), 1);
        let revision_title: String = conn
            .query_row(
                "SELECT title FROM prompt_versions WHERE id = 'v1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(revision_title, "T");
        let hits: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM prompts_fts WHERE prompts_fts MATCH 'T'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1);
    }

    #[test]
    fn initialize_storage_rebuilds_fts_for_existing_prompts() {
        let tmp = TempDir::new().unwrap();
        let paths = resolve_runtime_paths(tmp.path());
        ensure_directories(&paths).unwrap();

        {
            let conn = rusqlite::Connection::open(database_path(&paths)).unwrap();
            storage::init_schema(&conn).unwrap();
            conn.execute(
                "INSERT INTO prompts (id, title, user_prompt, created_at, updated_at, is_private) \
                 VALUES ('p1', 'UniqueKeyword', 'body', 0, 0, 0)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO prompts (id, title, user_prompt, created_at, updated_at, is_private) \
                 VALUES ('priv', 'SecretTerm', 'private body', 0, 0, 1)",
                [],
            )
            .unwrap();
        }

        let pool = initialize_storage(&paths).unwrap();
        let conn = pool.get().unwrap();
        let hits: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM prompts_fts WHERE prompts_fts MATCH 'UniqueKeyword'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1);
        let private_hits: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM prompts_fts WHERE prompts_fts MATCH 'SecretTerm'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(private_hits, 0);
        let indexed: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompts_fts_docsize", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(indexed, 1);
    }

    #[test]
    fn run_startup_opens_the_ready_gate_and_installs_the_pool() {
        let tmp = TempDir::new().unwrap();
        let paths = resolve_runtime_paths(tmp.path());
        let state = AppState::new(paths);

        run_startup(&state).unwrap();

        assert!(state.is_ready());
        assert!(state.init_error().is_none());
        assert!(state.pool.lock().unwrap().is_some());
    }

    #[test]
    fn run_startup_records_fatal_error_and_keeps_gate_closed_on_failure() {
        // Make the data directory uncreatable by placing a file where the
        // application-data root must be.
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("not-a-dir");
        std::fs::write(&file, b"x").unwrap();
        let paths = resolve_runtime_paths(&file);
        let state = AppState::new(paths);

        let err = run_startup(&state).unwrap_err();
        assert_eq!(err.code_str(), "IO");
        assert!(!state.is_ready());
        assert!(state.init_error().is_some());
        assert_eq!(
            state.init_failure().map(|error| error.code),
            Some(crate::error::ErrorCode::Io)
        );
        assert!(state.pool.lock().unwrap().is_none());
    }
}
