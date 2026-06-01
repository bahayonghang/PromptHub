use serde_json::Value;

use crate::error::CommandResult;
use crate::models::Settings;
use crate::services::settings;
use crate::state::AppState;

use super::{conn, into_command};

#[tauri::command(rename = "settings.get")]
pub fn settings_get(state: tauri::State<'_, AppState>) -> CommandResult<Settings> {
    into_command(conn(&state).and_then(|conn| settings::get(&conn)))
}

#[tauri::command(rename = "settings.update")]
pub fn settings_update(patch: Value, state: tauri::State<'_, AppState>) -> CommandResult<Settings> {
    into_command(conn(&state).and_then(|conn| settings::update(&conn, &patch)))
}

#[tauri::command(rename = "settings.list_system_fonts")]
pub fn settings_list_system_fonts() -> CommandResult<Vec<String>> {
    CommandResult::Ok(settings::list_system_fonts())
}
