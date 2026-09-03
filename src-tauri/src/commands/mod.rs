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
        Err(AppError::internal(
            state
                .init_error()
                .unwrap_or_else(|| "backend is not ready".to_string()),
        ))
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
    let pool = state
        .pool
        .lock()
        .map_err(|_| AppError::internal("database pool lock is poisoned"))?
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
