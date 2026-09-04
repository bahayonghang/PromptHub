use serde_json::Value;

use crate::error::CommandResult;
use crate::models::Settings;
use crate::services::settings;
use crate::state::AppState;

use super::{conn, into_command, CommandRuntimeState};

#[tauri::command(rename = "settings.get")]
pub fn settings_get(state: tauri::State<'_, AppState>) -> CommandResult<Settings> {
    into_command(conn(&state).and_then(|conn| settings::get(&conn)))
}

#[tauri::command(rename = "settings.update")]
pub fn settings_update<R: tauri::Runtime>(
    patch: Value,
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<Settings> {
    into_command((|| {
        let language_changed = patch.get("language").is_some();
        let close_action_patched = patch.get("closeAction").is_some();
        let result =
            conn(&state).and_then(|conn| settings::update(&conn, &state.encryption, &patch))?;
        if close_action_patched {
            super::window::apply_runtime_close_action(&runtime, result.close_action.as_deref())?;
        }
        if language_changed {
            if let Err(error) = super::window::rebuild_tray_menu(&app, &result.language) {
                eprintln!("PromptHub tray menu rebuild failed: {error}");
            }
        }
        Ok(result)
    })())
}

#[tauri::command(rename = "settings.list_system_fonts")]
pub fn settings_list_system_fonts() -> CommandResult<Vec<String>> {
    CommandResult::Ok(settings::list_system_fonts())
}
