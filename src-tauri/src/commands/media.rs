use std::path::PathBuf;

use crate::error::CommandResult;
use crate::services::media as service;
use crate::state::AppState;
use tauri::Manager;

use super::{ensure_ready, into_command, CommandRuntimeState};

fn images(state: &AppState) -> PathBuf {
    service::images_dir(&state.paths.media)
}

fn videos(state: &AppState) -> PathBuf {
    service::videos_dir(&state.paths.media)
}

#[tauri::command(rename = "media.select")]
pub fn media_select_paths(
    paths: Vec<String>,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        let mut selected = runtime
            .selected_media_paths
            .lock()
            .map_err(|_| crate::error::AppError::internal("selected media lock is poisoned"))?;
        selected.clear();
        for path in paths {
            let path = PathBuf::from(path);
            selected.insert(path.canonicalize().unwrap_or(path));
        }
        Ok(())
    }))
}

#[tauri::command(rename = "image.save")]
pub fn media_image_save(
    paths: Vec<String>,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<Vec<String>> {
    into_command(ensure_ready(&state).and_then(|_| {
        let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        let selected = runtime
            .selected_media_paths
            .lock()
            .map_err(|_| crate::error::AppError::internal("selected media lock is poisoned"))?;
        service::save_images(&images(&state), &selected, &paths)
    }))
}

#[tauri::command(rename = "image.saveBuffer")]
pub fn media_image_save_buffer(
    bytes: Vec<u8>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<String> {
    into_command(
        ensure_ready(&state).and_then(|_| service::save_image_buffer(&images(&state), &bytes)),
    )
}

#[tauri::command(rename = "image.saveBase64")]
pub fn media_image_save_base64(
    data: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<String> {
    into_command(
        ensure_ready(&state).and_then(|_| service::save_image_base64(&images(&state), &data)),
    )
}

#[tauri::command(rename = "image.download")]
pub async fn media_image_download<R: tauri::Runtime>(
    url: String,
    app: tauri::AppHandle<R>,
) -> CommandResult<String> {
    let image_dir = {
        let state = app.state::<AppState>();
        if let Err(e) = ensure_ready(&state) {
            return CommandResult::Err(e);
        }
        images(&state)
    };

    match service::download_image(&image_dir, &url).await {
        Ok(path) => CommandResult::Ok(path),
        Err(e) => CommandResult::Err(e),
    }
}

#[tauri::command(rename = "image.list")]
pub fn media_image_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<String>> {
    into_command(
        ensure_ready(&state)
            .and_then(|_| service::list(&images(&state), service::IMAGE_EXTENSIONS)),
    )
}

#[tauri::command(rename = "image.read")]
pub fn media_image_read(name: String, state: tauri::State<'_, AppState>) -> CommandResult<Vec<u8>> {
    into_command(ensure_ready(&state).and_then(|_| service::read(&images(&state), &name)))
}

#[tauri::command(rename = "image.exists")]
pub fn media_image_exists(name: String, state: tauri::State<'_, AppState>) -> CommandResult<bool> {
    into_command(ensure_ready(&state).map(|_| service::exists(&images(&state), &name)))
}

#[tauri::command(rename = "image.getSize")]
pub fn media_image_get_size(name: String, state: tauri::State<'_, AppState>) -> CommandResult<u64> {
    into_command(ensure_ready(&state).and_then(|_| service::get_size(&images(&state), &name)))
}

#[tauri::command(rename = "image.delete")]
pub fn media_image_delete(name: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| service::delete(&images(&state), &name)))
}

#[tauri::command(rename = "image.clear")]
pub fn media_image_clear(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| service::clear(&images(&state))))
}

#[tauri::command(rename = "video.save")]
pub fn media_video_save(
    paths: Vec<String>,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<Vec<String>> {
    into_command(ensure_ready(&state).and_then(|_| {
        let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        let selected = runtime
            .selected_media_paths
            .lock()
            .map_err(|_| crate::error::AppError::internal("selected media lock is poisoned"))?;
        service::save_videos(&videos(&state), &selected, &paths)
    }))
}

#[tauri::command(rename = "video.saveBuffer")]
pub fn media_video_save_buffer(
    ext: String,
    bytes: Vec<u8>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<String> {
    into_command(
        ensure_ready(&state)
            .and_then(|_| service::save_video_buffer(&videos(&state), &ext, &bytes)),
    )
}

#[tauri::command(rename = "video.saveBase64")]
pub fn media_video_save_base64(
    ext: String,
    data: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<String> {
    into_command(
        ensure_ready(&state).and_then(|_| service::save_video_base64(&videos(&state), &ext, &data)),
    )
}

#[tauri::command(rename = "video.list")]
pub fn media_video_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<String>> {
    into_command(
        ensure_ready(&state)
            .and_then(|_| service::list(&videos(&state), service::VIDEO_EXTENSIONS)),
    )
}

#[tauri::command(rename = "video.read")]
pub fn media_video_read(name: String, state: tauri::State<'_, AppState>) -> CommandResult<Vec<u8>> {
    into_command(ensure_ready(&state).and_then(|_| service::read(&videos(&state), &name)))
}

#[tauri::command(rename = "video.exists")]
pub fn media_video_exists(name: String, state: tauri::State<'_, AppState>) -> CommandResult<bool> {
    into_command(ensure_ready(&state).map(|_| service::exists(&videos(&state), &name)))
}

#[tauri::command(rename = "video.getSize")]
pub fn media_video_get_size(name: String, state: tauri::State<'_, AppState>) -> CommandResult<u64> {
    into_command(ensure_ready(&state).and_then(|_| service::get_size(&videos(&state), &name)))
}

#[tauri::command(rename = "video.delete")]
pub fn media_video_delete(name: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| service::delete(&videos(&state), &name)))
}

#[tauri::command(rename = "video.clear")]
pub fn media_video_clear(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| service::clear(&videos(&state))))
}
