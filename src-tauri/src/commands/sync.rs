use std::sync::atomic::Ordering;

use crate::error::{AppError, CommandResult};
use crate::services::data_path;
use crate::services::sync::{
    self, BackupEntry, ConnectionTestResult, ExportResult, ExportScope, RestoreResult, S3Config,
    WebDavConfig,
};
use crate::state::AppState;
use crate::storage;

use super::{
    allow_private_network, ensure_app_ready, ensure_ready, into_command, CommandRuntimeState,
};

#[tauri::command(rename = "webdav.test")]
pub async fn sync_webdav_test<R: tauri::Runtime>(
    config: WebDavConfig,
    app: tauri::AppHandle<R>,
) -> CommandResult<ConnectionTestResult> {
    match ensure_app_ready(&app) {
        Ok(()) => {
            let allow = {
                use tauri::Manager as _;
                let state = app.state::<AppState>();
                allow_private_network(&state)
            };
            sync::webdav_test(&config, allow).await.into()
        }
        Err(e) => CommandResult::Err(e),
    }
}

#[tauri::command(rename = "s3.test")]
pub async fn sync_s3_test<R: tauri::Runtime>(
    config: S3Config,
    app: tauri::AppHandle<R>,
) -> CommandResult<ConnectionTestResult> {
    match ensure_app_ready(&app) {
        Ok(()) => {
            let allow = {
                use tauri::Manager as _;
                let state = app.state::<AppState>();
                allow_private_network(&state)
            };
            sync::s3_test(&config, allow).await.into()
        }
        Err(e) => CommandResult::Err(e),
    }
}

#[tauri::command(rename = "data.exportZip")]
pub fn sync_export_zip(
    scope: ExportScope,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<ExportResult> {
    into_command(ensure_ready(&state).and_then(|_| {
        runtime.export_cancel.store(false, Ordering::Release);
        let dest = state
            .paths
            .backup
            .join(format!("export-{}.zip", crate::storage::time::now_millis()));
        sync::export_zip(&state.paths, &scope, &dest, &runtime.export_cancel)
    }))
}

#[tauri::command(rename = "data.exportCancel")]
pub fn sync_export_cancel(
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).map(|_| {
        runtime.export_cancel.store(true, Ordering::Release);
    }))
}

#[tauri::command(rename = "backup.create")]
pub fn sync_backup_create(state: tauri::State<'_, AppState>) -> CommandResult<BackupEntry> {
    into_command(
        ensure_ready(&state)
            .and_then(|_| sync::backup_create(&state.paths.data, &state.paths.backup)),
    )
}

#[tauri::command(rename = "backup.list")]
pub fn sync_backup_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<BackupEntry>> {
    into_command(ensure_ready(&state).and_then(|_| sync::backup_list(&state.paths.backup)))
}

#[tauri::command(rename = "backup.restore")]
pub fn sync_backup_restore(
    id: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<RestoreResult> {
    into_command(restore_backup(&state, &id))
}

/// Unloads the live pool before replacing the data directory. An open SQLite
/// file can block the sidecar rename on Windows. On restore failure the original
/// directory is still in place, so the pool is reopened.
fn restore_backup(state: &AppState, id: &str) -> Result<RestoreResult, AppError> {
    ensure_ready(state)?;
    state.set_ready(false);
    drop(state.take_pool());
    match sync::backup_restore(&state.paths.data, &state.paths.backup, id) {
        Ok(result) => Ok(result),
        Err(error) => {
            if let Ok(pool) = storage::create_pool(data_path::database_path(&state.paths)) {
                state.set_pool(pool);
                state.set_ready(true);
            }
            Err(error)
        }
    }
}

#[tauri::command(rename = "backup.delete")]
pub fn sync_backup_delete(id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| sync::backup_delete(&state.paths.backup, &id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::startup::{ensure_directories, resolve_runtime_paths, run_startup};
    use crate::error::ErrorCode;
    use tempfile::TempDir;

    #[test]
    fn backup_restore_unloads_pool_and_restores() {
        let tmp = TempDir::new().unwrap();
        let paths = resolve_runtime_paths(tmp.path());
        ensure_directories(&paths).unwrap();
        let state = AppState::new(paths);
        run_startup(&state).unwrap();

        let entry = sync::backup_create(&state.paths.data, &state.paths.backup).unwrap();
        std::fs::write(state.paths.data.join("marker.txt"), b"LIVE").unwrap();

        let result = restore_backup(&state, &entry.id).unwrap();
        assert!(result.restart_required);
        assert!(!state.is_ready());
        assert!(state.pool.lock().unwrap().is_none());
        assert!(!state.paths.data.join("marker.txt").exists());
        assert!(data_path::database_path(&state.paths).is_file());
    }

    #[test]
    fn backup_restore_unknown_id_reopens_pool() {
        let tmp = TempDir::new().unwrap();
        let paths = resolve_runtime_paths(tmp.path());
        ensure_directories(&paths).unwrap();
        let state = AppState::new(paths);
        run_startup(&state).unwrap();

        let err = restore_backup(&state, "nope").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
        assert!(state.is_ready());
        assert!(state.pool.lock().unwrap().is_some());
        assert!(data_path::database_path(&state.paths).is_file());
    }
}
