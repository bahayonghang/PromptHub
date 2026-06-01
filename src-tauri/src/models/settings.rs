//! Application settings DTO exchanged between the Command_Layer and the
//! Frontend (Requirement 19).
//!
//! Mirrors the existing TypeScript `Settings` type (Requirement 2.5). Optional
//! fields use `Option` and are skipped when absent so partial updates round-trip
//! cleanly. Unions that are not part of the required shared enums (theme,
//! language, update channel) are modeled as `String` to keep this task minimal.

use serde::{Deserialize, Serialize};

/// Persisted application settings.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Theme selection: `light` | `dark` | `system`.
    pub theme: String,
    /// UI language: `en` | `zh` | `zh-TW` | `ja` | `fr` | `de` | `es`.
    pub language: String,
    /// Whether auto-save is enabled.
    pub auto_save: bool,
    /// Theme flavor name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flavor: Option<String>,
    /// Named accent color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    /// Display font family.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_font: Option<String>,
    /// Body font family.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_font: Option<String>,
    /// Font scale preset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_scale: Option<String>,
    /// Density preset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<String>,
    /// Tag filter mode: `single` | `multi`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_filter_mode: Option<String>,
    /// Catalog of known prompt tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tag_catalog: Option<Vec<String>>,
    /// Default folder for new prompts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_folder_id: Option<String>,
    /// Background image file name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_image_file_name: Option<String>,
    /// Background image opacity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_image_opacity: Option<f64>,
    /// Background image blur radius.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_image_blur: Option<f64>,
    /// Time of the last manual backup as an ISO_8601 string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_manual_backup_at: Option<String>,
    /// App version at the last manual backup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_manual_backup_version: Option<String>,
    /// Backup/sync configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync: Option<SyncSettings>,
    /// Update channel: `stable` | `preview`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_channel: Option<String>,
    /// Whether the app launches at OS startup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch_at_startup: Option<bool>,
    /// Whether the app minimizes on launch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimize_on_launch: Option<bool>,
    /// Optional GitHub personal access token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_token: Option<String>,
    /// Security state summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<SecuritySettings>,
}

/// Backup/sync transport configuration.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSettings {
    /// Whether sync is enabled.
    pub enabled: bool,
    /// Provider kind: `manual` | `webdav` | `self-hosted` | `s3`.
    pub provider: String,
    /// Endpoint URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// Username/access key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// Password/secret key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Remote path/bucket prefix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_path: Option<String>,
    /// Whether automatic sync is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_sync: Option<bool>,
    /// Time of the last sync as an ISO_8601 string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<String>,
}

/// Master-password / lock state summary surfaced to the Frontend.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecuritySettings {
    /// Whether a master password has been configured.
    pub master_password_configured: bool,
    /// Whether the app is currently unlocked.
    pub unlocked: bool,
}
