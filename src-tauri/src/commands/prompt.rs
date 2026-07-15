use std::collections::HashMap;

use crate::error::CommandResult;
use crate::models::{Prompt, PromptPage, SearchQuery};
use crate::services::prompt::{self, PromptCopy, PromptCreate, PromptUpdate};
use crate::state::AppState;

use super::{conn, into_command};

#[tauri::command(rename = "prompt.list")]
pub fn prompt_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<Prompt>> {
    into_command(conn(&state).and_then(|conn| prompt::list_secure(&conn, &state.encryption)))
}

#[tauri::command(rename = "prompt.get")]
pub fn prompt_get(id: String, state: tauri::State<'_, AppState>) -> CommandResult<Prompt> {
    into_command(conn(&state).and_then(|conn| prompt::get_secure(&conn, &state.encryption, &id)))
}

#[tauri::command(rename = "prompt.search")]
pub fn prompt_search(
    query: SearchQuery,
    state: tauri::State<'_, AppState>,
) -> CommandResult<PromptPage> {
    into_command(
        conn(&state).and_then(|conn| prompt::search_secure(&conn, &state.encryption, query)),
    )
}

#[tauri::command(rename = "prompt.create")]
pub fn prompt_create(
    input: PromptCreate,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Prompt> {
    into_command(
        conn(&state).and_then(|conn| prompt::create_secure(&conn, &state.encryption, input)),
    )
}

#[tauri::command(rename = "prompt.update")]
pub fn prompt_update(
    id: String,
    patch: PromptUpdate,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Prompt> {
    into_command(
        conn(&state).and_then(|conn| prompt::update_secure(&conn, &state.encryption, &id, patch)),
    )
}

#[tauri::command(rename = "prompt.delete")]
pub fn prompt_delete(id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| prompt::delete(&conn, &id)))
}

#[tauri::command(rename = "prompt.duplicate")]
pub fn prompt_duplicate(id: String, state: tauri::State<'_, AppState>) -> CommandResult<Prompt> {
    into_command(conn(&state).and_then(|conn| {
        let duplicated = prompt::duplicate(&conn, &id)?;
        let key = crate::services::security::unlocked_key(&state.encryption)?;
        prompt::present_prompt(duplicated, key.as_deref())
    }))
}

#[tauri::command(rename = "prompt.batchMove")]
pub fn prompt_batch_move(
    ids: Vec<String>,
    folder_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(
        conn(&state).and_then(|conn| prompt::batch_move(&conn, &ids, folder_id.as_deref())),
    )
}

#[tauri::command(rename = "prompt.batchTag")]
pub fn prompt_batch_tag(
    ids: Vec<String>,
    tags: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| prompt::batch_tag(&conn, &ids, &tags)))
}

#[tauri::command(rename = "prompt.batchDelete")]
pub fn prompt_batch_delete(
    ids: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| prompt::batch_delete(&conn, &ids)))
}

#[tauri::command(rename = "prompt.copy")]
pub fn prompt_copy(
    id: String,
    values: HashMap<String, String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<PromptCopy> {
    into_command(
        conn(&state).and_then(|conn| prompt::copy_secure(&conn, &state.encryption, &id, &values)),
    )
}

#[tauri::command(rename = "tag.list")]
pub fn prompt_tag_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<String>> {
    into_command(conn(&state).and_then(|conn| prompt::tag_list(&conn)))
}

#[tauri::command(rename = "tag.rename")]
pub fn prompt_tag_rename(
    old: String,
    new: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| prompt::tag_rename(&conn, &old, &new)))
}

#[tauri::command(rename = "tag.delete")]
pub fn prompt_tag_delete(tag: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| prompt::tag_delete(&conn, &tag)))
}
