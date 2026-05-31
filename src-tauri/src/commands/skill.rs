use std::path::PathBuf;

use rusqlite::{params, OptionalExtension};

use crate::error::{AppError, CommandResult};
use crate::models::{ParsedSkillMd, Skill, SkillVersion};
use crate::services::skill::{self, SkillCreate, SkillUpdate};
use crate::services::skill_local;
use crate::services::skill_platform::{
    self, CustomPlatform, InstallResult, Platform, PlatformInstallStatus, SkillFile,
};
use crate::services::skill_safety::{self, DiscoveredSkill, SafetyReport, ScanAiConfig};
use crate::state::AppState;
use tauri::Manager;

use super::{conn, ensure_app_ready, into_command};

fn lookup_skill_version(conn: &rusqlite::Connection, id: &str) -> Result<(String, i64), AppError> {
    conn.query_row(
        "SELECT skill_id, version FROM skill_versions WHERE id = ?1",
        params![id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(|e| AppError::internal(format!("failed to look up skill version: {e}")))?
    .ok_or_else(|| AppError::not_found(format!("skill version `{id}` not found")))
}

fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn platforms() -> Vec<Platform> {
    skill_platform::list_platforms(&home_dir(), &[] as &[CustomPlatform])
}

fn safety_ai_config(conn: &rusqlite::Connection) -> Result<Option<ScanAiConfig>, AppError> {
    let settings = crate::services::settings::get(conn)?;
    let Some(sync) = settings.sync else {
        return Ok(None);
    };
    let Some(api_url) = sync.endpoint else {
        return Ok(None);
    };
    let Some(api_key) = sync.password else {
        return Ok(None);
    };
    Ok(Some(ScanAiConfig {
        api_url,
        api_key,
        model: sync.remote_path.unwrap_or_else(|| "default".to_string()),
    }))
}

#[tauri::command(rename = "skill.list")]
pub fn skill_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<Skill>> {
    into_command(conn(&state).and_then(|conn| skill::list(&conn)))
}

#[tauri::command(rename = "skill.get")]
pub fn skill_get(id: String, state: tauri::State<'_, AppState>) -> CommandResult<Skill> {
    into_command(conn(&state).and_then(|conn| skill::get(&conn, &id)))
}

#[tauri::command(rename = "skill.create")]
pub fn skill_create(input: SkillCreate, state: tauri::State<'_, AppState>) -> CommandResult<Skill> {
    into_command(conn(&state).and_then(|conn| skill::create(&conn, input)))
}

#[tauri::command(rename = "skill.update")]
pub fn skill_update(
    id: String,
    patch: SkillUpdate,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Skill> {
    into_command(conn(&state).and_then(|conn| skill::update(&conn, &id, patch)))
}

#[tauri::command(rename = "skill.delete")]
pub fn skill_delete(id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| skill::delete(&conn, &id)))
}

#[tauri::command(rename = "skill.version.list")]
pub fn skill_version_list(
    skill_id: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<SkillVersion>> {
    into_command(conn(&state).and_then(|conn| skill::version_list(&conn, &skill_id)))
}

#[tauri::command(rename = "skill.version.create")]
pub fn skill_version_create(
    skill_id: String,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<SkillVersion> {
    into_command(conn(&state).and_then(|conn| skill::version_create(&conn, &skill_id, note)))
}

#[tauri::command(rename = "skill.version.rollback")]
pub fn skill_version_rollback(
    skill_id: String,
    version: i64,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Skill> {
    into_command(conn(&state).and_then(|conn| skill::version_rollback(&conn, &skill_id, version)))
}

#[tauri::command(rename = "skill.version.delete")]
pub fn skill_version_delete(id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| {
        let (skill_id, version_no) = lookup_skill_version(&conn, &id)?;
        skill::version_delete(&conn, &skill_id, version_no)
    }))
}

#[tauri::command(rename = "skill.parseMd")]
pub fn skill_parse_md(
    content: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<ParsedSkillMd> {
    into_command(
        super::ensure_ready(&state).and_then(|_| crate::services::skill_md::parse_md(&content)),
    )
}

#[tauri::command(rename = "skill.serializeMd")]
pub fn skill_serialize_md(
    parsed: ParsedSkillMd,
    state: tauri::State<'_, AppState>,
) -> CommandResult<String> {
    into_command(
        super::ensure_ready(&state).and_then(|_| crate::services::skill_md::serialize_md(&parsed)),
    )
}

#[tauri::command(rename = "skill.import")]
pub fn skill_import_skill(json: String, state: tauri::State<'_, AppState>) -> CommandResult<Skill> {
    into_command(conn(&state).and_then(|conn| crate::services::skill_md::import(&conn, &json)))
}

#[tauri::command(rename = "skill.local.scan")]
pub fn skill_local_scan(
    locations: Option<Vec<String>>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<skill_local::ScanEntry>> {
    into_command(super::ensure_ready(&state).and_then(|_| {
        let roots: Vec<PathBuf> = locations
            .unwrap_or_else(|| {
                vec![home_dir()
                    .join(".codex")
                    .join("skills")
                    .to_string_lossy()
                    .to_string()]
            })
            .into_iter()
            .map(PathBuf::from)
            .collect();
        skill_local::scan(&roots)
    }))
}

#[tauri::command(rename = "skill.local.tree")]
pub fn skill_local_tree(
    repo_path: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<skill_local::TreeEntry>> {
    into_command(
        super::ensure_ready(&state).and_then(|_| skill_local::tree(&PathBuf::from(repo_path))),
    )
}

#[tauri::command(rename = "skill.local.read")]
pub fn skill_local_read(
    repo_path: String,
    relative_path: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<String> {
    into_command(
        super::ensure_ready(&state)
            .and_then(|_| skill_local::read(&PathBuf::from(repo_path), &relative_path)),
    )
}

#[tauri::command(rename = "skill.local.write")]
pub fn skill_local_write(
    repo_path: String,
    relative_path: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(
        super::ensure_ready(&state)
            .and_then(|_| skill_local::write(&PathBuf::from(repo_path), &relative_path, &content)),
    )
}

#[tauri::command(rename = "skill.local.mkdir")]
pub fn skill_local_mkdir(
    repo_path: String,
    relative_path: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(
        super::ensure_ready(&state)
            .and_then(|_| skill_local::mkdir(&PathBuf::from(repo_path), &relative_path)),
    )
}

#[tauri::command(rename = "skill.local.rename")]
pub fn skill_local_rename(
    repo_path: String,
    from_relative_path: String,
    to_relative_path: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(super::ensure_ready(&state).and_then(|_| {
        skill_local::rename(
            &PathBuf::from(repo_path),
            &from_relative_path,
            &to_relative_path,
        )
    }))
}

#[tauri::command(rename = "skill.local.delete")]
pub fn skill_local_delete(
    repo_path: String,
    relative_path: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(
        super::ensure_ready(&state)
            .and_then(|_| skill_local::delete(&PathBuf::from(repo_path), &relative_path)),
    )
}

#[tauri::command(rename = "skill.local.sync")]
pub fn skill_local_sync(
    skill_id: String,
    repo_path: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Skill> {
    into_command(
        conn(&state)
            .and_then(|conn| skill_local::sync(&conn, &skill_id, &PathBuf::from(repo_path))),
    )
}

#[tauri::command(rename = "skill.platform.list")]
pub fn skill_platform_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<Platform>> {
    into_command(super::ensure_ready(&state).map(|_| platforms()))
}

#[tauri::command(rename = "skill.platform.detect")]
pub fn skill_platform_detect(state: tauri::State<'_, AppState>) -> CommandResult<Vec<String>> {
    into_command(super::ensure_ready(&state).map(|_| skill_platform::detect(&platforms())))
}

#[tauri::command(rename = "skill.platform.install")]
pub fn skill_platform_install(
    platform_id: String,
    skill_name: String,
    files: Vec<SkillFile>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<InstallResult> {
    into_command(
        super::ensure_ready(&state)
            .and_then(|_| skill_platform::install(&platforms(), &platform_id, &skill_name, &files)),
    )
}

#[tauri::command(rename = "skill.platform.uninstall")]
pub fn skill_platform_uninstall(
    platform_id: String,
    skill_name: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(
        super::ensure_ready(&state)
            .and_then(|_| skill_platform::uninstall(&platforms(), &platform_id, &skill_name)),
    )
}

#[tauri::command(rename = "skill.platform.status")]
pub fn skill_platform_status(
    skill_name: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<PlatformInstallStatus>> {
    into_command(
        super::ensure_ready(&state).and_then(|_| skill_platform::status(&platforms(), &skill_name)),
    )
}

#[tauri::command(rename = "skill.safety.scan")]
pub async fn skill_safety_scan<R: tauri::Runtime>(
    content: String,
    app: tauri::AppHandle<R>,
) -> CommandResult<SafetyReport> {
    let config = {
        let state = app.state::<AppState>();
        match conn(&state).and_then(|conn| safety_ai_config(&conn)) {
            Ok(config) => config,
            Err(e) => return CommandResult::Err(e),
        }
    };
    skill_safety::scan(&content, config).await.into()
}

#[tauri::command(rename = "skill.safety.save")]
pub fn skill_safety_save(
    skill_id: String,
    report: SafetyReport,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Skill> {
    into_command(conn(&state).and_then(|conn| skill_safety::save_report(&conn, &skill_id, &report)))
}

#[tauri::command(rename = "skill.remote.fetchContent")]
pub async fn skill_remote_fetch_content<R: tauri::Runtime>(
    url: String,
    app: tauri::AppHandle<R>,
) -> CommandResult<String> {
    match ensure_app_ready(&app) {
        Ok(()) => skill_safety::fetch_content(&url).await.into(),
        Err(e) => CommandResult::Err(e),
    }
}

#[tauri::command(rename = "skill.remote.scanRepo")]
pub async fn skill_remote_scan_repo<R: tauri::Runtime>(
    listing_url: String,
    app: tauri::AppHandle<R>,
) -> CommandResult<Vec<DiscoveredSkill>> {
    match ensure_app_ready(&app) {
        Ok(()) => skill_safety::scan_repo(&listing_url).await.into(),
        Err(e) => CommandResult::Err(e),
    }
}
