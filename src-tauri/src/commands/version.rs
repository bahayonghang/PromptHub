use crate::error::CommandResult;
use crate::models::{Prompt, PromptVersion};
use crate::services::version;
use crate::state::AppState;

use super::{conn, into_command};

#[tauri::command(rename = "version.list")]
pub fn prompt_version_list(
    prompt_id: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<PromptVersion>> {
    into_command(conn(&state).and_then(|conn| {
        let key = crate::services::security::unlocked_key(&state.encryption)?;
        version::list(&conn, &prompt_id)?
            .into_iter()
            .map(|revision| crate::services::prompt::present_version(revision, key.as_deref()))
            .collect()
    }))
}

#[tauri::command(rename = "version.create")]
pub fn prompt_version_create(
    prompt_id: String,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<PromptVersion> {
    into_command(conn(&state).and_then(|conn| {
        let key = crate::services::security::unlocked_key(&state.encryption)?;
        if crate::services::prompt::get(&conn, &prompt_id)?.is_private && key.is_none() {
            return Err(crate::error::AppError::unauthorized(
                "unlock the prompt library to version private content",
            ));
        }
        let revision = version::create(&conn, &prompt_id, note)?;
        crate::services::prompt::present_version(revision, key.as_deref())
    }))
}

#[tauri::command(rename = "version.rollback")]
pub fn prompt_version_rollback(
    prompt_id: String,
    version: i64,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Prompt> {
    into_command(conn(&state).and_then(|conn| {
        let key = crate::services::security::unlocked_key(&state.encryption)?;
        if crate::services::prompt::get(&conn, &prompt_id)?.is_private && key.is_none() {
            return Err(crate::error::AppError::unauthorized(
                "unlock the prompt library to restore private content",
            ));
        }
        let stored = version::rollback(&conn, &prompt_id, version)?;
        crate::services::prompt::present_prompt(stored, key.as_deref())
    }))
}
