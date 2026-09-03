use std::path::{Path, PathBuf};

use tauri::Manager;
use tauri_plugin_autostart::ManagerExt as AutoStartExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_notification::{NotificationExt, PermissionState};

use crate::error::{AppError, CommandResult};
use crate::services::window::{
    self, CloseDecision, PlatformFeature, RuntimePathsReport, Shortcut, ShortcutMode,
    ShortcutRegistry,
};
use crate::state::AppState;

use super::{ensure_ready, into_command, CommandRuntimeState};

fn main_window<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<tauri::WebviewWindow<R>, AppError> {
    app.get_webview_window("main")
        .ok_or_else(|| AppError::not_found("main window not found"))
}

fn map_tauri_error(context: &str, error: tauri::Error) -> AppError {
    AppError::internal(format!("{context}: {error}"))
}

fn cache_dir(state: &AppState) -> PathBuf {
    state.paths.data.join("cache")
}

#[tauri::command(rename = "window.minimize")]
pub fn window_minimize<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        main_window(&app)?
            .minimize()
            .map_err(|e| map_tauri_error("failed to minimize window", e))
    }))
}

#[tauri::command(rename = "window.maximize")]
pub fn window_maximize<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        main_window(&app)?
            .maximize()
            .map_err(|e| map_tauri_error("failed to maximize window", e))
    }))
}

#[tauri::command(rename = "window.restore")]
pub fn window_restore<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        main_window(&app)?
            .unmaximize()
            .map_err(|e| map_tauri_error("failed to restore window", e))
    }))
}

#[tauri::command(rename = "window.close")]
pub fn window_close<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        let action = *runtime
            .close_action
            .lock()
            .map_err(|_| AppError::internal("close action lock is poisoned"))?;
        let window = main_window(&app)?;
        match action.decision() {
            CloseDecision::EmitCloseRequested => {
                super::events::emit_close_requested(&app);
                Ok(())
            }
            CloseDecision::Hide => {
                window
                    .hide()
                    .map_err(|e| map_tauri_error("failed to hide window", e))?;
                super::events::emit_visibility_changed(&app, false);
                Ok(())
            }
            CloseDecision::Terminate => window
                .close()
                .map_err(|e| map_tauri_error("failed to close window", e)),
        }
    }))
}

#[tauri::command(rename = "window.toggleVisibility")]
pub fn window_toggle_visibility<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        let window = main_window(&app)?;
        let visible = window
            .is_visible()
            .map_err(|e| map_tauri_error("failed to read window visibility", e))?;
        let now_visible = !visible;
        if visible {
            window
                .hide()
                .map_err(|e| map_tauri_error("failed to hide window", e))?;
        } else {
            window
                .show()
                .map_err(|e| map_tauri_error("failed to show window", e))?;
        }
        super::events::emit_visibility_changed(&app, now_visible);
        Ok(())
    }))
}

#[tauri::command(rename = "window.enterFullscreen")]
pub fn window_enter_fullscreen<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    set_fullscreen(app, state, true)
}

#[tauri::command(rename = "window.exitFullscreen")]
pub fn window_exit_fullscreen<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    set_fullscreen(app, state, false)
}

fn set_fullscreen<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
    fullscreen: bool,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        main_window(&app)?
            .set_fullscreen(fullscreen)
            .map_err(|e| map_tauri_error("failed to set fullscreen", e))?;
        super::events::emit_fullscreen_changed(&app, fullscreen);
        Ok(())
    }))
}

#[tauri::command(rename = "window.toggleFullscreen")]
pub fn window_toggle_fullscreen<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        let window = main_window(&app)?;
        let fullscreen = window
            .is_fullscreen()
            .map_err(|e| map_tauri_error("failed to read fullscreen state", e))?;
        let next = !fullscreen;
        window
            .set_fullscreen(next)
            .map_err(|e| map_tauri_error("failed to toggle fullscreen", e))?;
        super::events::emit_fullscreen_changed(&app, next);
        Ok(())
    }))
}

#[tauri::command(rename = "window.setCloseAction")]
pub fn window_set_close_action(
    action: String,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        let action = window::parse_close_action(&action)?;
        *runtime
            .close_action
            .lock()
            .map_err(|_| AppError::internal("close action lock is poisoned"))? = action;
        Ok(())
    }))
}

#[tauri::command(rename = "app.setAutoLaunch")]
pub fn window_set_auto_launch<R: tauri::Runtime>(
    enabled: bool,
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        if !window::probe_capabilities().auto_launch {
            return Err(window::capability_unavailable_error(
                PlatformFeature::AutoLaunch,
            ));
        }
        let result = if enabled {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        };
        result.map_err(|e| AppError::internal(format!("failed to set auto launch: {e}")))
    }))
}

#[tauri::command(rename = "shortcut.register")]
pub fn window_shortcut_register<R: tauri::Runtime>(
    shortcuts: Vec<Shortcut>,
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        if !window::probe_capabilities().shortcuts {
            return Err(window::capability_unavailable_error(
                PlatformFeature::Shortcuts,
            ));
        }
        let registry = ShortcutRegistry::register_all(shortcuts)?;
        app.global_shortcut()
            .unregister_all()
            .map_err(|e| AppError::internal(format!("failed to unregister shortcuts: {e}")))?;
        for shortcut in registry.shortcuts() {
            if shortcut.mode == ShortcutMode::Global {
                let action = shortcut.action.clone();
                app.global_shortcut()
                    .on_shortcut(
                        shortcut.accelerator.as_str(),
                        move |app, _shortcut, event| {
                            if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed
                            {
                                super::events::emit_shortcut_triggered(app, action.clone());
                            }
                        },
                    )
                    .map_err(|e| {
                        AppError::conflict(format!(
                            "failed to register shortcut `{}`: {e}",
                            shortcut.accelerator
                        ))
                    })?;
            }
        }
        *runtime
            .shortcuts
            .lock()
            .map_err(|_| AppError::internal("shortcut registry lock is poisoned"))? = registry;
        Ok(())
    }))
}

#[tauri::command(rename = "app.showNotification")]
pub fn window_show_notification<R: tauri::Runtime>(
    title: String,
    body: String,
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        if !window::probe_capabilities().notifications {
            return Err(window::capability_unavailable_error(
                PlatformFeature::Notifications,
            ));
        }
        let payload = window::validate_notification(&title, &body)?;
        let permission = app.notification().request_permission().map_err(|e| {
            AppError::internal(format!("failed to request notification permission: {e}"))
        })?;
        if permission != PermissionState::Granted {
            return Err(window::notification_permission_denied());
        }
        app.notification()
            .builder()
            .title(payload.title)
            .body(payload.body)
            .show()
            .map_err(|e| AppError::internal(format!("failed to show notification: {e}")))
    }))
}

#[tauri::command(rename = "app.getCacheSize")]
pub fn window_get_cache_size(state: tauri::State<'_, AppState>) -> CommandResult<u64> {
    into_command(ensure_ready(&state).map(|_| window::get_cache_size(&cache_dir(&state))))
}

#[tauri::command(rename = "app.clearCache")]
pub fn window_clear_cache(state: tauri::State<'_, AppState>) -> CommandResult<u64> {
    into_command(ensure_ready(&state).and_then(|_| window::clear_cache(&cache_dir(&state))))
}

#[tauri::command(rename = "app.getRuntimePaths")]
pub fn window_get_runtime_paths(
    state: tauri::State<'_, AppState>,
) -> CommandResult<RuntimePathsReport> {
    into_command(ensure_ready(&state).map(|_| window::get_runtime_paths(&state.paths)))
}

#[tauri::command(rename = "app.openPath")]
pub fn window_open_path(path: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| {
        window::open_runtime_path(Path::new(&path), &state.paths, |allowed| {
            open::that(allowed).map_err(|e| AppError::io(format!("failed to open path: {e}")))
        })
    }))
}

#[tauri::command(rename = "app.revealPath")]
pub fn window_reveal_path(path: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    window_open_path(path, state)
}
