use crate::error::CommandResult;
use crate::models::PromptTypeDefinition;
use crate::services::prompt_type::{self, PromptTypeCreate};
use crate::state::AppState;

use super::{conn, into_command};

#[tauri::command(rename = "promptType.list")]
pub fn prompt_type_list(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<PromptTypeDefinition>> {
    into_command(conn(&state).and_then(|conn| prompt_type::list(&conn)))
}

#[tauri::command(rename = "promptType.create")]
pub fn prompt_type_create(
    input: PromptTypeCreate,
    state: tauri::State<'_, AppState>,
) -> CommandResult<PromptTypeDefinition> {
    into_command(conn(&state).and_then(|conn| prompt_type::create(&conn, input)))
}
