use std::collections::HashMap;

use crate::error::CommandResult;
use crate::models::{Prompt, SearchQuery};
use crate::services::prompt::{self, PromptCopy, PromptCreate, PromptUpdate};
use crate::state::AppState;

use super::{conn, into_command};

#[tauri::command(rename = "prompt.list")]
pub fn prompt_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<Prompt>> {
    into_command(conn(&state).and_then(|conn| prompt::list(&conn)))
}

#[tauri::command(rename = "prompt.get")]
pub fn prompt_get(id: String, state: tauri::State<'_, AppState>) -> CommandResult<Prompt> {
    into_command(conn(&state).and_then(|conn| prompt::get(&conn, &id)))
}

#[tauri::command(rename = "prompt.search")]
pub fn prompt_search(
    query: SearchQuery,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<Prompt>> {
    into_command(conn(&state).and_then(|conn| prompt::search(&conn, query)))
}

#[tauri::command(rename = "prompt.create")]
pub fn prompt_create(
    input: PromptCreate,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Prompt> {
    into_command(conn(&state).and_then(|conn| prompt::create(&conn, input)))
}

#[tauri::command(rename = "prompt.update")]
pub fn prompt_update(
    id: String,
    patch: PromptUpdate,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Prompt> {
    into_command(conn(&state).and_then(|conn| prompt::update(&conn, &id, patch)))
}

#[tauri::command(rename = "prompt.delete")]
pub fn prompt_delete(id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| prompt::delete(&conn, &id)))
}

#[tauri::command(rename = "prompt.copy")]
pub fn prompt_copy(
    id: String,
    values: HashMap<String, String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<PromptCopy> {
    into_command(conn(&state).and_then(|conn| prompt::copy(&conn, &id, &values)))
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
