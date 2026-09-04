use std::path::PathBuf;

use crate::error::CommandResult;
use crate::services::portable::{
    self, BundlePreview, ImportConflictPolicy, PortableExportResult, PortableImportResult,
};
use crate::state::AppState;

use super::{conn, into_command};

#[tauri::command(rename = "prompt.bundleExport")]
pub fn prompt_bundle_export(
    destination: Option<String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<PortableExportResult> {
    into_command(conn(&state).and_then(|conn| {
        let destination =
            portable::resolve_bundle_destination(destination.as_deref(), &state.paths)?;
        portable::export_bundle(&conn, &state.paths.media, &destination)
    }))
}

#[tauri::command(rename = "prompt.bundlePreview")]
pub fn prompt_bundle_preview(
    file_path: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<BundlePreview> {
    into_command(
        conn(&state)
            .and_then(|conn| portable::preview_bundle(&conn, PathBuf::from(file_path).as_path())),
    )
}

#[tauri::command(rename = "prompt.bundleImport")]
pub fn prompt_bundle_import(
    file_path: String,
    policy: ImportConflictPolicy,
    state: tauri::State<'_, AppState>,
) -> CommandResult<PortableImportResult> {
    into_command(conn(&state).and_then(|conn| {
        let key = crate::services::security::unlocked_key(&state.encryption)?;
        portable::import_bundle(
            &conn,
            PathBuf::from(file_path).as_path(),
            policy,
            &state.paths.data,
            &state.paths.backup,
            &state.paths.media,
            key.as_deref(),
        )
    }))
}
