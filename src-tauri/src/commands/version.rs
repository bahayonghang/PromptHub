use rusqlite::{params, OptionalExtension};

use crate::error::{AppError, CommandResult};
use crate::models::{Prompt, PromptVersion};
use crate::services::version;
use crate::state::AppState;

use super::{conn, into_command};

fn lookup_prompt_version(conn: &rusqlite::Connection, id: &str) -> Result<(String, i64), AppError> {
    conn.query_row(
        "SELECT prompt_id, version FROM prompt_versions WHERE id = ?1",
        params![id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(|e| AppError::internal(format!("failed to look up prompt version: {e}")))?
    .ok_or_else(|| AppError::not_found(format!("prompt version `{id}` not found")))
}

#[tauri::command(rename = "version.list")]
pub fn prompt_version_list(
    prompt_id: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<PromptVersion>> {
    into_command(conn(&state).and_then(|conn| version::list(&conn, &prompt_id)))
}

#[tauri::command(rename = "version.create")]
pub fn prompt_version_create(
    prompt_id: String,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<PromptVersion> {
    into_command(conn(&state).and_then(|conn| version::create(&conn, &prompt_id, note)))
}

#[tauri::command(rename = "version.rollback")]
pub fn prompt_version_rollback(
    prompt_id: String,
    version: i64,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Prompt> {
    into_command(conn(&state).and_then(|conn| version::rollback(&conn, &prompt_id, version)))
}

#[tauri::command(rename = "version.delete")]
pub fn prompt_version_delete(id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| {
        let (prompt_id, version_no) = lookup_prompt_version(&conn, &id)?;
        version::delete(&conn, &prompt_id, version_no)
    }))
}
