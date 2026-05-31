use crate::error::CommandResult;
use crate::services::updater::{self, UpdateCheckResult};
use crate::state::AppState;
use tauri::Manager;

use super::{ensure_app_ready, ensure_ready, into_command, CommandRuntimeState};

#[tauri::command(rename = "app.getVersion")]
pub fn app_get_version(state: tauri::State<'_, AppState>) -> CommandResult<String> {
    into_command(ensure_ready(&state).map(|_| updater::get_version().to_string()))
}

#[tauri::command(rename = "app.getPlatform")]
pub fn app_get_platform(state: tauri::State<'_, AppState>) -> CommandResult<String> {
    into_command(ensure_ready(&state).map(|_| updater::get_platform().to_string()))
}

#[tauri::command(rename = "updater.check")]
pub async fn updater_check<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> CommandResult<UpdateCheckResult> {
    match ensure_app_ready(&app) {
        Ok(()) => match updater::check(&app).await {
            Ok((result, _)) => CommandResult::Ok(result),
            Err(e) => CommandResult::Err(e),
        },
        Err(e) => CommandResult::Err(e),
    }
}

#[tauri::command(rename = "updater.download")]
pub async fn updater_download<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> CommandResult<()> {
    if let Err(e) = ensure_app_ready(&app) {
        return CommandResult::Err(e);
    }
    match updater::check(&app).await {
        Ok((result, Some(update))) if result.available => {
            let sink = super::events::TauriUpdaterEventSink::new(app.clone());
            match updater::download(&update, &sink).await {
                Ok(bytes) => {
                    let result = {
                        let runtime = app.state::<CommandRuntimeState>();
                        let stored = match runtime.update_bytes.lock() {
                            Ok(mut slot) => {
                                *slot = Some(bytes);
                                Ok(())
                            }
                            Err(_) => Err(crate::error::AppError::internal(
                                "update bytes lock is poisoned",
                            )),
                        };
                        stored
                    };
                    match result {
                        Ok(()) => CommandResult::Ok(()),
                        Err(e) => CommandResult::Err(e),
                    }
                }
                Err(e) => CommandResult::Err(e),
            }
        }
        Ok((_result, None)) => {
            CommandResult::Err(crate::error::AppError::not_found("no update is available"))
        }
        Ok((_result, Some(_))) => {
            CommandResult::Err(crate::error::AppError::not_found("no update is available"))
        }
        Err(e) => CommandResult::Err(e),
    }
}

#[tauri::command(rename = "updater.install")]
pub async fn updater_install<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> CommandResult<()> {
    if let Err(e) = ensure_app_ready(&app) {
        return CommandResult::Err(e);
    }
    match updater::check(&app).await {
        Ok((_result, Some(update))) => {
            let runtime = app.state::<CommandRuntimeState>();
            let bytes = match runtime.update_bytes.lock() {
                Ok(mut slot) => slot.take(),
                Err(_) => {
                    return CommandResult::Err(crate::error::AppError::internal(
                        "update bytes lock is poisoned",
                    ))
                }
            };
            match bytes {
                Some(bytes) => updater::install(&update, &bytes).into(),
                None => CommandResult::Err(crate::error::AppError::not_found(
                    "no downloaded update is available",
                )),
            }
        }
        Ok((_result, None)) => {
            CommandResult::Err(crate::error::AppError::not_found("no update is available"))
        }
        Err(e) => CommandResult::Err(e),
    }
}
