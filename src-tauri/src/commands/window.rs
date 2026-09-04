use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use serde_json::json;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt as AutoStartExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_notification::{NotificationExt, PermissionState};

use crate::error::{AppError, CommandResult};
use crate::logging::lock_mutex;
use crate::services::settings;
use crate::services::window::{
    self, CloseDecision, PlatformFeature, RuntimePathsReport, Shortcut, ShortcutMode,
    ShortcutRegistry,
};
use crate::state::AppState;

use super::{conn, ensure_ready, into_command, CommandRuntimeState};

fn main_window<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<tauri::WebviewWindow<R>, AppError> {
    app.get_webview_window("main")
        .ok_or_else(|| AppError::not_found("main window not found"))
}

fn map_tauri_error(context: &str, error: tauri::Error) -> AppError {
    AppError::internal(format!("{context}: {error}"))
}

/// Seeds the runtime close-action mutex from persisted Settings (None → Ask).
pub fn seed_close_action_from_settings(runtime: &CommandRuntimeState, stored: Option<&str>) {
    let action = stored
        .and_then(|raw| window::parse_close_action(raw).ok())
        .unwrap_or(window::CloseAction::Ask);
    if let Ok(mut guard) = runtime.close_action.lock() {
        *guard = action;
    }
}

/// Writes the parsed close action into the runtime mutex (Req 20.4).
pub fn apply_runtime_close_action(
    runtime: &CommandRuntimeState,
    stored: Option<&str>,
) -> Result<(), AppError> {
    let action = match stored {
        Some(raw) => window::parse_close_action(raw)?,
        None => window::CloseAction::Ask,
    };
    *lock_mutex(&runtime.close_action, "close action")? = action;
    Ok(())
}

/// Shared close-action side effects for `window.close` and native CloseRequested.
pub fn apply_close<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    runtime: &CommandRuntimeState,
) -> Result<CloseDecision, AppError> {
    let action = *lock_mutex(&runtime.close_action, "close action")?;
    let decision = action.decision();
    match decision {
        CloseDecision::EmitCloseRequested => {
            super::events::emit_close_requested(app);
        }
        CloseDecision::Hide => {
            hide_or_minimize(app, runtime)?;
        }
        CloseDecision::Terminate => {
            app.exit(0);
        }
    }
    Ok(decision)
}

fn hide_or_minimize<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    runtime: &CommandRuntimeState,
) -> Result<(), AppError> {
    let window = main_window(app)?;
    if runtime.tray_available.load(Ordering::Relaxed) {
        window
            .hide()
            .map_err(|e| map_tauri_error("failed to hide window", e))?;
    } else {
        window
            .minimize()
            .map_err(|e| map_tauri_error("failed to minimize window", e))?;
    }
    super::events::emit_visibility_changed(app, false);
    Ok(())
}

/// Shows, unminimizes, and focuses `main`, then emits visibility-changed(true).
pub fn show_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    super::events::emit_visibility_changed(app, true);
}

fn build_tray_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    language: &str,
) -> Result<Menu<R>, AppError> {
    let (show_label, exit_label) = window::tray_menu_labels(language);
    let show = MenuItem::with_id(app, "show", show_label, true, None::<&str>)
        .map_err(|e| map_tauri_error("failed to create tray Show item", e))?;
    let exit = MenuItem::with_id(app, "exit", exit_label, true, None::<&str>)
        .map_err(|e| map_tauri_error("failed to create tray Exit item", e))?;
    Menu::with_items(app, &[&show, &exit])
        .map_err(|e| map_tauri_error("failed to create tray menu", e))
}

/// Rebuilds the tray menu labels after `settings.language` changes.
pub fn rebuild_tray_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    language: &str,
) -> Result<(), AppError> {
    let Some(tray) = app.tray_by_id("main") else {
        return Ok(());
    };
    let menu = build_tray_menu(app, language)?;
    tray.set_menu(Some(menu))
        .map_err(|e| map_tauri_error("failed to update tray menu", e))
}

/// Creates the always-on tray icon. Never panics; callers degrade on `Err` (R9).
pub fn setup_tray<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    language: &str,
) -> Result<(), AppError> {
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| AppError::internal("app default window icon is missing"))?;
    let menu = build_tray_menu(app, language)?;
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .tooltip("PromptHub")
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "exit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| map_tauri_error("failed to create tray icon", e))?;
    app.state::<CommandRuntimeState>()
        .tray_available
        .store(true, Ordering::Relaxed);
    Ok(())
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
    into_command(ensure_ready(&state).and_then(|_| apply_close(&app, &runtime).map(|_| ())))
}

#[tauri::command(rename = "window.quit")]
pub fn window_quit<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> CommandResult<()> {
    // Quitting must work even when startup failed (Req 20.4 / R6).
    app.exit(0);
    CommandResult::Ok(())
}

/// Hides to tray when the icon exists, otherwise OS-minimizes (Req 20.4 / R7, R9).
#[tauri::command(rename = "window.hide")]
pub fn window_hide<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, CommandRuntimeState>,
) -> CommandResult<()> {
    into_command(ensure_ready(&state).and_then(|_| hide_or_minimize(&app, &runtime)))
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
        let parsed = window::parse_close_action(&action)?;
        let conn = conn(&state)?;
        settings::update(
            &conn,
            &state.encryption,
            &json!({ "closeAction": parsed.as_str() }),
        )?;
        *lock_mutex(&runtime.close_action, "close action")? = parsed;
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
