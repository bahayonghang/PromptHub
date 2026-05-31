pub mod error;
pub mod state;

pub mod commands;
pub mod models;
pub mod services;
pub mod storage;

use tauri::{Emitter, Manager};

use crate::services::window::{self, CapabilityDegradation};
use crate::state::{AppState, RuntimePaths};

/// Tauri event emitted to the Frontend when the startup sequence fails fatally,
/// carrying the human-readable reason so the Frontend can show a fatal error
/// surface (Requirements 4.7, 23.3).
const EVENT_INIT_FAILED: &str = "app:init-failed";

/// Tauri event emitted to the Frontend for each platform-integration feature that
/// is unsupported on the current Target_Platform and was skipped. This is the
/// **non-fatal** capability-degradation channel: the application keeps running and
/// the Frontend can surface an indication identifying the unsupported feature
/// (Requirement 23.5).
const EVENT_CAPABILITY_DEGRADED: &str = "app:capability-degraded";

/// Resolves the per-user application-data root, builds [`RuntimePaths`] beneath
/// it, and returns the not-yet-ready [`AppState`] to manage.
///
/// Uses the Tauri `path` API's `app_data_dir`, the platform's conventional
/// per-user application-data location (Requirement 23.2). The six runtime
/// subdirectories are resolved by the Data_Path_Manager so the resolved set
/// matches the Window_Manager's `get_runtime_paths` report (Req 20.9).
fn build_app_state<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppState {
    let base = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let paths: RuntimePaths = commands::startup::resolve_runtime_paths(&base);
    AppState::new(paths)
}

/// Probes the current Target_Platform for platform-integration feature
/// availability and emits a non-fatal [`EVENT_CAPABILITY_DEGRADED`] event for each
/// unsupported feature, so the Window_Manager skips it while the application keeps
/// running (Requirement 23.4, 23.5).
///
/// Supported features need no action here — the Command_Layer applies their
/// Requirement 20 handling on demand. Returns the resolved capability descriptor
/// for any caller that wants to record it.
fn announce_capability_degradations<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> window::PlatformCapabilities {
    let capabilities = window::probe_capabilities();
    for degradation in capabilities.degradations() {
        eprintln!(
            "PromptHub platform feature unavailable: {}",
            degradation.message
        );
        let _ = app.emit::<CapabilityDegradation>(EVENT_CAPABILITY_DEGRADED, degradation);
    }
    capabilities
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        // Updater (Requirement 24): register `tauri-plugin-updater` so the
        // Updater service's `app.updater_builder()` calls resolve, picking up the
        // `endpoints` + `pubkey` configured under `plugins.updater` in
        // `tauri.conf.json` (task 24.1). The plugin owns the signed
        // check/download/install transport (24.2–24.5).
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Startup sequence (task 17.2): resolve data path -> ensure dirs ->
            // open pool + schema -> set ready -> (window is created from the
            // tauri.conf.json config). On failure the `ready` gate stays closed,
            // the fatal error is recorded on the state, and an `app:init-failed`
            // event is emitted so the Frontend can show a fatal error surface
            // (Requirements 4.6, 4.7, 23.1, 23.3).
            let state = build_app_state(app.handle());
            app.manage(state);
            app.manage(commands::CommandRuntimeState::default());

            let state = app.state::<AppState>();
            if let Err(error) = commands::startup::run_startup(&state) {
                eprintln!("PromptHub startup failed: {error}");
                let _ = app.emit(EVENT_INIT_FAILED, error.to_string());
            }

            // Probe platform-integration features and surface a non-fatal
            // indication for any unsupported on this Target_Platform; the app
            // keeps running and the supported features are applied on demand by
            // the Command_Layer (Requirements 23.4, 23.5).
            announce_capability_degradations(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_status,
            commands::prompt::prompt_list,
            commands::prompt::prompt_get,
            commands::prompt::prompt_search,
            commands::prompt::prompt_create,
            commands::prompt::prompt_update,
            commands::prompt::prompt_delete,
            commands::prompt::prompt_copy,
            commands::prompt::prompt_tag_list,
            commands::prompt::prompt_tag_rename,
            commands::prompt::prompt_tag_delete,
            commands::folder::folder_list,
            commands::folder::folder_create,
            commands::folder::folder_update,
            commands::folder::folder_delete,
            commands::folder::folder_reorder,
            commands::version::prompt_version_list,
            commands::version::prompt_version_create,
            commands::version::prompt_version_rollback,
            commands::version::prompt_version_delete,
            commands::skill::skill_list,
            commands::skill::skill_get,
            commands::skill::skill_create,
            commands::skill::skill_update,
            commands::skill::skill_delete,
            commands::skill::skill_version_list,
            commands::skill::skill_version_create,
            commands::skill::skill_version_rollback,
            commands::skill::skill_version_delete,
            commands::skill::skill_parse_md,
            commands::skill::skill_serialize_md,
            commands::skill::skill_import_skill,
            commands::skill::skill_local_scan,
            commands::skill::skill_local_tree,
            commands::skill::skill_local_read,
            commands::skill::skill_local_write,
            commands::skill::skill_local_mkdir,
            commands::skill::skill_local_rename,
            commands::skill::skill_local_delete,
            commands::skill::skill_local_sync,
            commands::skill::skill_platform_list,
            commands::skill::skill_platform_detect,
            commands::skill::skill_platform_install,
            commands::skill::skill_platform_uninstall,
            commands::skill::skill_platform_status,
            commands::skill::skill_safety_scan,
            commands::skill::skill_safety_save,
            commands::skill::skill_remote_fetch_content,
            commands::skill::skill_remote_scan_repo,
            commands::settings::settings_get,
            commands::settings::settings_update,
            commands::security::security_status,
            commands::security::security_set_master_password,
            commands::security::security_change_master_password,
            commands::security::security_unlock,
            commands::security::security_lock,
            commands::data_path::data_path_get_path,
            commands::data_path::data_path_get_status,
            commands::data_path::data_path_preview_change,
            commands::data_path::data_path_apply_change,
            commands::data_path::data_path_recovery_scan,
            commands::data_path::data_path_recovery_preview,
            commands::data_path::data_path_recovery_apply,
            commands::sync::sync_webdav_test,
            commands::sync::sync_webdav_upload,
            commands::sync::sync_webdav_download,
            commands::sync::sync_webdav_stat,
            commands::sync::sync_webdav_ensure_dir,
            commands::sync::sync_s3_test,
            commands::sync::sync_s3_upload,
            commands::sync::sync_s3_download,
            commands::sync::sync_s3_stat,
            commands::sync::sync_export_zip,
            commands::sync::sync_export_cancel,
            commands::sync::sync_backup_create,
            commands::sync::sync_backup_list,
            commands::sync::sync_backup_restore,
            commands::sync::sync_backup_delete,
            commands::rules::rules_list,
            commands::rules::rules_scan,
            commands::rules::rules_read,
            commands::rules::rules_save,
            commands::rules::rules_add_project,
            commands::rules::rules_remove_project,
            commands::rules::rules_delete_version,
            commands::ai::ai_request,
            commands::ai::ai_stream,
            commands::ai::ai_cancel,
            commands::media::media_select_paths,
            commands::media::media_image_save,
            commands::media::media_image_save_buffer,
            commands::media::media_image_save_base64,
            commands::media::media_image_download,
            commands::media::media_image_list,
            commands::media::media_image_read,
            commands::media::media_image_exists,
            commands::media::media_image_get_size,
            commands::media::media_image_delete,
            commands::media::media_image_clear,
            commands::media::media_video_save,
            commands::media::media_video_save_buffer,
            commands::media::media_video_save_base64,
            commands::media::media_video_list,
            commands::media::media_video_read,
            commands::media::media_video_exists,
            commands::media::media_video_get_size,
            commands::media::media_video_delete,
            commands::media::media_video_clear,
            commands::window::window_minimize,
            commands::window::window_maximize,
            commands::window::window_restore,
            commands::window::window_close,
            commands::window::window_toggle_visibility,
            commands::window::window_enter_fullscreen,
            commands::window::window_exit_fullscreen,
            commands::window::window_toggle_fullscreen,
            commands::window::window_set_close_action,
            commands::window::window_set_auto_launch,
            commands::window::window_shortcut_register,
            commands::window::window_show_notification,
            commands::window::window_get_cache_size,
            commands::window::window_clear_cache,
            commands::window::window_get_runtime_paths,
            commands::window::window_open_path,
            commands::window::window_reveal_path,
            commands::updater::app_get_version,
            commands::updater::app_get_platform,
            commands::updater::updater_check,
            commands::updater::updater_download,
            commands::updater::updater_install
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
