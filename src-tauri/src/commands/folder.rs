use crate::error::CommandResult;
use crate::models::Folder;
use crate::services::folder::{self, CreateFolderInput, UpdateFolderInput};
use crate::state::AppState;

use super::{conn, into_command};

#[tauri::command(rename = "folder.list")]
pub fn folder_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<Folder>> {
    into_command(conn(&state).and_then(|conn| folder::list(&conn)))
}

#[tauri::command(rename = "folder.create")]
pub fn folder_create(
    input: CreateFolderInput,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Folder> {
    into_command(conn(&state).and_then(|conn| folder::create(&conn, input)))
}

#[tauri::command(rename = "folder.update")]
pub fn folder_update(
    id: String,
    patch: UpdateFolderInput,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Folder> {
    into_command(conn(&state).and_then(|conn| folder::update(&conn, &id, patch)))
}

#[tauri::command(rename = "folder.delete")]
pub fn folder_delete(id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| folder::delete(&conn, &id)))
}

#[tauri::command(rename = "folder.reorder")]
pub fn folder_reorder(
    ordered_ids: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| folder::reorder(&conn, &ordered_ids)))
}
