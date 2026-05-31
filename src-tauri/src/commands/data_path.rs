use std::path::PathBuf;

use crate::error::CommandResult;
use crate::services::data_path::{
    self, ApplyResult, DataPathStatus, PreviewResult, RecoveryPreview, RecoverySource,
};
use crate::state::AppState;

use super::{ensure_ready, into_command};

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
) -> CommandResult<PreviewResult> {
    into_command(
        ensure_ready(&state).and_then(|_| {
            data_path::preview_change(&state.paths.data, &PathBuf::from(target_path))
        }),
    )
}

#[tauri::command(rename = "data.applyChange")]
pub fn data_path_apply_change(
    target_path: String,
    action: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<ApplyResult> {
    into_command(ensure_ready(&state).and_then(|_| {
        data_path::apply_change(
            &state.paths.data,
            &config_path(&state),
            &PathBuf::from(target_path),
            &action,
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
) -> CommandResult<RecoveryPreview> {
    into_command(
        ensure_ready(&state).and_then(|_| data_path::recovery_preview(&PathBuf::from(source_path))),
    )
}

#[tauri::command(rename = "data.recoveryApply")]
pub fn data_path_recovery_apply(
    source_path: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<ApplyResult> {
    into_command(
        ensure_ready(&state).and_then(|_| {
            data_path::recovery_apply(&state.paths.data, &PathBuf::from(source_path))
        }),
    )
}
