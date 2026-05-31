use crate::error::CommandResult;
use crate::services::security::{self, SecurityStatus};
use crate::state::AppState;

use super::{conn, into_command};

#[tauri::command(rename = "security.status")]
pub fn security_status(state: tauri::State<'_, AppState>) -> CommandResult<SecurityStatus> {
    into_command(conn(&state).and_then(|conn| security::status(&conn, &state.encryption)))
}

#[tauri::command(rename = "security.setMasterPassword")]
pub fn security_set_master_password(
    password: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(
        conn(&state)
            .and_then(|conn| security::set_master_password(&conn, &state.encryption, &password)),
    )
}

#[tauri::command(rename = "security.changeMasterPassword")]
pub fn security_change_master_password(
    current_password: String,
    new_password: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| {
        security::change_master_password(&conn, &state.encryption, &current_password, &new_password)
    }))
}

#[tauri::command(rename = "security.unlock")]
pub fn security_unlock(password: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(
        conn(&state).and_then(|conn| security::unlock(&conn, &state.encryption, &password)),
    )
}

#[tauri::command(rename = "security.lock")]
pub fn security_lock(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(super::ensure_ready(&state).and_then(|_| security::lock(&state.encryption)))
}
