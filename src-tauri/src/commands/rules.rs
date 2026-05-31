use crate::error::CommandResult;
use crate::models::{RuleFileContent, RuleVersionSnapshot};
use crate::services::rules::{self, AddProjectInput};
use crate::state::AppState;

use super::{conn, into_command};

fn managed_dir(state: &AppState) -> std::path::PathBuf {
    state.paths.rule.join("managed")
}

fn versions_dir(state: &AppState) -> std::path::PathBuf {
    state.paths.rule.join("versions")
}

#[tauri::command(rename = "rules.list")]
pub fn rules_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<RuleFileContent>> {
    into_command(conn(&state).and_then(|conn| rules::list(&conn)))
}

#[tauri::command(rename = "rules.scan")]
pub fn rules_scan(state: tauri::State<'_, AppState>) -> CommandResult<Vec<RuleFileContent>> {
    into_command(conn(&state).and_then(|conn| rules::scan(&conn)))
}

#[tauri::command(rename = "rules.read")]
pub fn rules_read(id: String, state: tauri::State<'_, AppState>) -> CommandResult<RuleFileContent> {
    into_command(conn(&state).and_then(|conn| rules::read(&conn, &id)))
}

#[tauri::command(rename = "rules.save")]
pub fn rules_save(
    id: String,
    content: String,
    source: Option<String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<RuleFileContent> {
    into_command(conn(&state).and_then(|conn| {
        rules::save(
            &conn,
            &id,
            &content,
            source.as_deref(),
            &versions_dir(&state),
        )
    }))
}

#[tauri::command(rename = "rules.addProject")]
pub fn rules_add_project(
    input: AddProjectInput,
    state: tauri::State<'_, AppState>,
) -> CommandResult<RuleFileContent> {
    into_command(conn(&state).and_then(|conn| {
        rules::add_project(&conn, input, &managed_dir(&state), &versions_dir(&state))
    }))
}

#[tauri::command(rename = "rules.removeProject")]
pub fn rules_remove_project(id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| rules::remove_project(&conn, &id)))
}

#[tauri::command(rename = "rules.deleteVersion")]
pub fn rules_delete_version(
    rule_id: String,
    version_id: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<RuleVersionSnapshot>> {
    into_command(conn(&state).and_then(|conn| rules::delete_version(&conn, &rule_id, &version_id)))
}
