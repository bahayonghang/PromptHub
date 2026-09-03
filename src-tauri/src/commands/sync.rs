use std::sync::atomic::Ordering;

use crate::error::CommandResult;
use crate::services::sync::{
    self, BackupEntry, ConnectionTestResult, ExportResult, ExportScope, RestoreResult, S3Config,
    WebDavConfig,
};
use crate::state::AppState;

use super::{ensure_app_ready, ensure_ready, into_command, CommandRuntimeState};

#[tauri::command(rename = "webdav.test")]
pub async fn sync_webdav_test<R: tauri::Runtime>(
    config: WebDavConfig,
    app: tauri::AppHandle<R>,
) -> CommandResult<ConnectionTestResult> {
    match ensure_app_ready(&app) {
        Ok(()) => sync::webdav_test(&config).await.into(),
        Err(e) => CommandResult::Err(e),
    }
}

#[tauri::command(rename = "s3.test")]
pub async fn sync_s3_test<R: tauri::Runtime>(
    config: S3Config,
    app: tauri::AppHandle<R>,
) -> CommandResult<ConnectionTestResult> {
    match ensure_app_ready(&app) {
        Ok(()) => sync::s3_test(&config).await.into(),
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
    into_command(
        ensure_ready(&state)
            .and_then(|_| sync::backup_restore(&state.paths.data, &state.paths.backup, &id)),
    )
}

#[tauri::command(rename = "backup.delete")]
pub fn sync_backup_delete(id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| sync::backup_delete(&state.paths.backup, &id)))
}
