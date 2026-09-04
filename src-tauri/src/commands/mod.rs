//! Command_Layer: the thin Tauri adapter over the backend services.

pub mod data_path;
pub mod evaluation;
pub mod events;
pub mod folder;
pub mod media;
pub mod portable;
pub mod prompt;
pub mod prompt_type;
pub mod rules;
pub mod security;
pub mod settings;
pub mod startup;
pub mod sync;
pub mod updater;
pub mod version;
pub mod window;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::error::{AppError, CommandResult};
use crate::services::data_path::ConfirmTokenRegistry;
use crate::services::window::{CloseAction, ShortcutRegistry};
use crate::state::AppState;

/// Runtime-only state owned by the command adapter.
pub struct CommandRuntimeState {
    pub export_cancel: AtomicBool,
    pub close_action: Mutex<CloseAction>,
    /// True once the system tray icon was created successfully (Req 20, 23.5).
    pub tray_available: AtomicBool,
    pub shortcuts: Mutex<ShortcutRegistry>,
    pub selected_media_paths: Mutex<HashSet<PathBuf>>,
    pub update_bytes: Mutex<Option<Vec<u8>>>,
    pub confirm_tokens: Mutex<ConfirmTokenRegistry>,
}

impl Default for CommandRuntimeState {
    fn default() -> Self {
        Self {
            export_cancel: AtomicBool::new(false),
            close_action: Mutex::new(CloseAction::Ask),
            tray_available: AtomicBool::new(false),
            shortcuts: Mutex::new(ShortcutRegistry::new()),
            selected_media_paths: Mutex::new(HashSet::new()),
            update_bytes: Mutex::new(None),
            confirm_tokens: Mutex::new(ConfirmTokenRegistry::default()),
        }
    }
}

pub(crate) fn into_command<T>(result: Result<T, AppError>) -> CommandResult<T> {
    result.into()
}

pub(crate) fn ensure_ready(state: &AppState) -> Result<(), AppError> {
    if state.is_ready() {
        Ok(())
    } else {
        Err(state
            .init_failure()
            .unwrap_or_else(|| AppError::internal("backend is not ready")))
    }
}

pub(crate) fn ensure_app_ready<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<(), AppError> {
    use tauri::Manager as _;

    let state = app.state::<AppState>();
    ensure_ready(&state)
}

pub(crate) fn allow_private_network(state: &AppState) -> bool {
    conn(state)
        .ok()
        .and_then(|conn| crate::services::settings::get(&conn).ok())
        .and_then(|settings| settings.allow_private_network)
        .unwrap_or(false)
}

pub(crate) fn conn(
    state: &AppState,
) -> Result<PooledConnection<SqliteConnectionManager>, AppError> {
    ensure_ready(state)?;
    let pool = crate::logging::lock_mutex(&state.pool, "database pool")?
        .clone()
        .ok_or_else(|| AppError::internal("database pool is not initialized"))?;
    pool.get()
        .map_err(|e| AppError::io(format!("failed to acquire database connection: {e}")))
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_error: Option<String>,
}

#[tauri::command]
pub fn app_status(state: tauri::State<'_, AppState>) -> CommandResult<AppStatus> {
    CommandResult::Ok(AppStatus {
        ready: state.is_ready(),
        init_error: state.init_error(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::startup::{
        database_path, ensure_directories, resolve_runtime_paths, run_startup,
    };
    use crate::error::ErrorCode;
    use tempfile::TempDir;

    #[test]
    fn ensure_ready_without_init_error_is_internal() {
        let state = AppState::default();
        let err = ensure_ready(&state).unwrap_err();
        assert_eq!(err.code, ErrorCode::Internal);
        assert_eq!(err.message, "backend is not ready");
    }

    #[test]
    fn ensure_ready_preserves_startup_io_code() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("not-a-dir");
        std::fs::write(&file, b"x").unwrap();
        let paths = resolve_runtime_paths(&file);
        let state = AppState::new(paths);
        let err = run_startup(&state).unwrap_err();
        assert_eq!(err.code, ErrorCode::Io);
        let ready_err = ensure_ready(&state).unwrap_err();
        assert_eq!(ready_err.code, ErrorCode::Io);
        assert_eq!(ready_err.code_str(), "IO");
        assert_eq!(ready_err.message, err.message);
    }

    #[test]
    fn startup_failure_writes_log_without_secrets() {
        let tmp = TempDir::new().unwrap();
        let paths = resolve_runtime_paths(tmp.path());
        ensure_directories(&paths).unwrap();
        std::fs::create_dir_all(database_path(&paths)).unwrap();
        let state = AppState::new(paths.clone());
        let err = run_startup(&state).unwrap_err();
        assert_eq!(err.code, ErrorCode::Io);
        let body = std::fs::read_to_string(paths.log.join(crate::logging::LOG_FILE_NAME)).unwrap();
        assert!(body.contains("ERROR startup"));
        assert!(body.contains("startup failed"));
        assert!(body.contains("[IO]"));
        assert!(body.contains(&err.message));
        assert!(!body.contains("password="));
        assert!(!body.contains("Authorization:"));
        assert!(!body.contains("DEK:"));
        assert!(!body.contains("token="));
        let ready_err = ensure_ready(&state).unwrap_err();
        assert_eq!(ready_err.code, ErrorCode::Io);
        assert_eq!(ready_err.message, err.message);
    }
}
