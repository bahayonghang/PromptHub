use std::path::PathBuf;
use std::sync::MutexGuard;

use crate::error::{AppError, CommandResult};
use crate::services::data_path::{
    self, ApplyResult, DataPathStatus, PreviewResult, RecoveryPreview, RecoverySource,
};
use crate::state::AppState;

use super::{ensure_ready, into_command, CommandRuntimeState};

fn confirm_tokens(
    runtime: &CommandRuntimeState,
) -> Result<MutexGuard<'_, data_path::ConfirmTokenRegistry>, AppError> {
    runtime
        .confirm_tokens
        .lock()
        .map_err(|_| AppError::internal("confirm token lock is poisoned"))
}

fn config_path(state: &AppState) -> PathBuf {
    state.paths.data.join("data-path.json")
}

fn known_recovery_locations(state: &AppState) -> Vec<PathBuf> {
    let mut locations = vec![
        state.paths.data.clone(),
        state.paths.data.join("..").join("PromptHub"),
        state.paths.data.join("..").join("prompthub"),
    ];
    if let Some(parent) = state.paths.data.parent() {
        locations.push(parent.join("data"));
        locations.push(parent.join("PromptHub"));
    }
    locations
}

#[tauri::command(rename = "data.getPath")]
pub fn data_path_get_path(state: tauri::State<'_, AppState>) -> CommandResult<String> {
    into_command(ensure_ready(&state).and_then(|_| data_path::get_path(&state.paths.data)))
}

#[tauri::command(rename = "data.getStatus")]
pub fn data_path_get_status(state: tauri::State<'_, AppState>) -> CommandResult<DataPathStatus> {
    into_command(
        ensure_ready(&state)
            .and_then(|_| data_path::get_status(&state.paths.data, &config_path(&state))),
    )
}

#[tauri::command(rename = "data.previewChange")]
pub fn data_path_preview_change(
    target_path: String,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<PreviewResult> {
    into_command(ensure_ready(&state).and_then(|_| {
        let mut tokens = confirm_tokens(&runtime)?;
        data_path::preview_change(&state.paths.data, &PathBuf::from(target_path), &mut tokens)
    }))
}

#[tauri::command(rename = "data.applyChange")]
pub fn data_path_apply_change(
    target_path: String,
    action: String,
    confirm_token: String,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<ApplyResult> {
    into_command(ensure_ready(&state).and_then(|_| {
        let mut tokens = confirm_tokens(&runtime)?;
        data_path::apply_change(
            &state.paths.data,
            &config_path(&state),
            &PathBuf::from(target_path),
            &action,
            &confirm_token,
            &mut tokens,
        )
    }))
}

#[tauri::command(rename = "data.recoveryScan")]
pub fn data_path_recovery_scan(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<RecoverySource>> {
    into_command(ensure_ready(&state).and_then(|_| {
        data_path::recovery_scan(&state.paths.data, &known_recovery_locations(&state))
    }))
}

#[tauri::command(rename = "data.recoveryPreview")]
pub fn data_path_recovery_preview(
    source_path: String,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<RecoveryPreview> {
    into_command(ensure_ready(&state).and_then(|_| {
        let mut tokens = confirm_tokens(&runtime)?;
        data_path::recovery_preview(&PathBuf::from(source_path), &mut tokens)
    }))
}

#[tauri::command(rename = "data.recoveryApply")]
pub fn data_path_recovery_apply(
    source_path: String,
    confirm_token: String,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<ApplyResult> {
    into_command(ensure_ready(&state).and_then(|_| {
        let mut tokens = confirm_tokens(&runtime)?;
        data_path::recovery_apply(
            &state.paths.data,
            &PathBuf::from(source_path),
            &confirm_token,
            &mut tokens,
        )
    }))
}
