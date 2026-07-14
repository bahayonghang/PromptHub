//! Window_Manager — window controls, tray, shortcuts, and notifications (Req 20).
//!
//! Like the sibling services, the Window_Manager keeps its **business rules pure**
//! and free of any live-window dependency so they are unit- and property-testable
//! without a running Tauri webview. Everything in this module is either a plain
//! data type (event payloads, the runtime-paths report) or a pure decision
//! function (shortcut conflict/cap rules, notification length validation, cache
//! size over a directory, path-existence checks). The Command_Layer (task 17.1)
//! is the thin adapter that owns the genuinely window-/OS-bound glue and calls
//! into these rules.
//!
//! ## What is pure here vs. what is runtime glue
//!
//! Pure (implemented and tested below):
//!
//! - **Shortcut registry** ([`ShortcutRegistry`]): registering up to
//!   [`MAX_SHORTCUTS`] (50) shortcuts in global/local mode, with conflict
//!   rejection that returns `CONFLICT` and **leaves the previously registered set
//!   unchanged** (Req 20.6, 20.11 — Property 40), plus accelerator normalization
//!   so equivalent key combinations collide deterministically.
//! - **Notification validation** ([`validate_notification`]): title ≤ 256 and
//!   body ≤ 1000 characters, else `VALIDATION` (Req 20.7).
//! - **Close-action decision** ([`CloseAction`] / [`CloseDecision`]): `ask` emits
//!   a close-requested event, `minimize` hides, `exit` terminates (Req 20.4).
//! - **Runtime-paths report** ([`get_runtime_paths`]): the resolved data, database,
//!   media, rule, backup, and log paths (Req 20.9).
//! - **Cache size / clear** ([`get_cache_size`], [`clear_cache`]) over a directory
//!   (Req 20.8).
//! - **Path-existence checks** ([`ensure_path_exists`]) for open/reveal, returning
//!   `NOT_FOUND` for a missing target (Req 20.10, 20.12).
//! - **Platform capability degradation** ([`PlatformCapabilities`],
//!   [`probe_capabilities`]): a per-feature availability probe for auto-launch,
//!   tray, shortcuts, and notifications; unsupported features are reported through
//!   the capability descriptor and the non-fatal [`CapabilityDegradation`] channel
//!   so the app keeps running (Req 23.4, 23.5).
//! - **Event payload shapes + names** for `fullscreen-changed`,
//!   `visibility-changed`, `close-requested`, and `shortcut:triggered`
//!   (Req 20.2, 20.3, 20.4, 20.6).
//!
//! Runtime glue (wired in the Command_Layer / startup, not here):
//!
//! - The actual window transitions (minimize/maximize/restore/close, enter/exit/
//!   toggle fullscreen, toggle visibility) act on the live `tauri::WebviewWindow`;
//!   this module supplies their event payloads and the close-action decision.
//! - `set_auto_launch` is a direct call into `tauri-plugin-autostart` whose only
//!   input is the requested enabled/disabled boolean — there is no additional pure
//!   decision to model (Req 20.5).
//! - Registering the accelerators with the OS and emitting `shortcut:triggered`
//!   when one fires is done through `tauri-plugin-global-shortcut`; the registry
//!   below is the source of truth for *which* shortcuts are accepted.
//! - Displaying the validated notification (and observing an OS permission denial,
//!   surfaced via [`notification_permission_denied`]) goes through
//!   `tauri-plugin-notification` (Req 20.13).
//! - Opening/revealing a path in the system shell happens after
//!   [`ensure_path_exists`] confirms the target is present.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::RuntimePaths;

// ===========================================================================
// Constants
// ===========================================================================

/// Maximum number of keyboard shortcuts that may be registered (Req 20.6).
pub const MAX_SHORTCUTS: usize = 50;
/// Maximum notification title length, in characters (Req 20.7).
pub const MAX_NOTIFICATION_TITLE: usize = 256;
/// Maximum notification body length, in characters (Req 20.7).
pub const MAX_NOTIFICATION_BODY: usize = 1000;

/// Tauri event emitted when the window enters/exits fullscreen (Req 20.2).
pub const EVENT_FULLSCREEN_CHANGED: &str = "window:fullscreen-changed";
/// Tauri event emitted when the window's visibility toggles (Req 20.3).
pub const EVENT_VISIBILITY_CHANGED: &str = "window:visibility-changed";
/// Tauri event emitted when a close is attempted under the `ask` action (Req 20.4).
pub const EVENT_CLOSE_REQUESTED: &str = "window:close-requested";
/// Tauri event emitted when a registered shortcut fires (Req 20.6).
pub const EVENT_SHORTCUT_TRIGGERED: &str = "shortcut:triggered";

// ===========================================================================
// Event payloads (Req 20.2, 20.3, 20.4, 20.6)
// ===========================================================================

/// Payload for [`EVENT_FULLSCREEN_CHANGED`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullscreenChanged {
    /// Whether the window is now fullscreen.
    pub fullscreen: bool,
}

/// Payload for [`EVENT_VISIBILITY_CHANGED`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibilityChanged {
    /// Whether the window is now visible.
    pub visible: bool,
}

/// Payload for [`EVENT_CLOSE_REQUESTED`] (carries no fields; serializes to `{}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseRequested {}

/// Payload for [`EVENT_SHORTCUT_TRIGGERED`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutTriggered {
    /// The action identifier of the shortcut that fired.
    pub action: String,
}

// ===========================================================================
// Close action (Req 20.4)
// ===========================================================================

/// The behavior to apply when a window-close is attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloseAction {
    /// Emit a close-requested event and keep the application running.
    Ask,
    /// Hide the window and keep the application running.
    Minimize,
    /// Terminate the application.
    Exit,
}

/// What the Command_Layer should do when a close is attempted, derived from the
/// configured [`CloseAction`] (Req 20.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseDecision {
    /// Emit [`EVENT_CLOSE_REQUESTED`]; the app keeps running.
    EmitCloseRequested,
    /// Hide the window; the app keeps running.
    Hide,
    /// Terminate the application.
    Terminate,
}

impl CloseAction {
    /// Returns the action to take when a close is attempted under this setting.
    pub fn decision(self) -> CloseDecision {
        match self {
            CloseAction::Ask => CloseDecision::EmitCloseRequested,
            CloseAction::Minimize => CloseDecision::Hide,
            CloseAction::Exit => CloseDecision::Terminate,
        }
    }
}

/// Parses a close-action string (`ask` | `minimize` | `exit`) into a
/// [`CloseAction`], rejecting anything else with `VALIDATION` (Req 20.4).
pub fn parse_close_action(raw: &str) -> Result<CloseAction, AppError> {
    match raw.trim().to_lowercase().as_str() {
        "ask" => Ok(CloseAction::Ask),
        "minimize" => Ok(CloseAction::Minimize),
        "exit" => Ok(CloseAction::Exit),
        other => Err(AppError::validation(format!(
            "unknown close action `{other}`; expected ask, minimize, or exit"
        ))),
    }
}

// ===========================================================================
// Keyboard shortcuts (Req 20.6, 20.11 — Property 40)
// ===========================================================================

/// Whether a shortcut fires globally or only while the window has focus (Req 20.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShortcutMode {
    /// Fires regardless of which application has focus.
    Global,
    /// Fires only while the application window has focus.
    Local,
}

/// A single keyboard shortcut: an action identifier, the accelerator (key
/// combination) it is bound to, and its firing mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    /// The action emitted (as [`ShortcutTriggered::action`]) when this fires.
    pub action: String,
    /// The key combination, e.g. `CmdOrCtrl+Shift+K`.
    pub accelerator: String,
    /// Whether this is a global or local shortcut.
    pub mode: ShortcutMode,
}

/// Normalizes an accelerator string so equivalent key combinations compare equal.
///
/// Lowercases each `+`-separated token, canonicalizes common modifier aliases
/// (`control`→`ctrl`, `command`→`cmd`, `option`→`alt`, `super`/`meta`/`win`/
/// `windows`→`super`), then sorts and de-duplicates the tokens. This makes
/// `Ctrl+Shift+A`, `shift+control+a`, and `A+Ctrl+Shift` all collide for conflict
/// detection. Returns `None` for an empty accelerator or one with an empty token
/// (e.g. a trailing `+`).
fn normalize_accelerator(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut tokens: Vec<String> = Vec::new();
    for part in trimmed.split('+') {
        let token = part.trim().to_lowercase();
        if token.is_empty() {
            return None;
        }
        let canonical = match token.as_str() {
            "control" | "ctrl" => "ctrl",
            "command" | "cmd" => "cmd",
            "option" | "alt" => "alt",
            "super" | "meta" | "win" | "windows" => "super",
            other => other,
        };
        tokens.push(canonical.to_string());
    }
    tokens.sort();
    tokens.dedup();
    Some(tokens.join("+"))
}

/// The set of keyboard shortcuts the application has accepted.
///
/// Conflict is defined on the **normalized accelerator**: two shortcuts conflict
/// when they bind the same key combination. The registry enforces the
/// [`MAX_SHORTCUTS`] cap and the conflict rule from Property 40.
#[derive(Debug, Clone, Default)]
pub struct ShortcutRegistry {
    /// Map from normalized accelerator to the registered shortcut.
    entries: BTreeMap<String, Shortcut>,
}

impl ShortcutRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Number of registered shortcuts.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry holds no shortcuts.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether `accelerator` (after normalization) is already registered.
    pub fn contains_accelerator(&self, accelerator: &str) -> bool {
        normalize_accelerator(accelerator).is_some_and(|norm| self.entries.contains_key(&norm))
    }

    /// Returns the registered shortcuts, ordered by normalized accelerator.
    pub fn shortcuts(&self) -> Vec<Shortcut> {
        self.entries.values().cloned().collect()
    }

    /// Registers a single shortcut.
    ///
    /// On success the shortcut is added and `Ok(())` is returned. On failure the
    /// registry is **left exactly as it was** (Property 40):
    ///
    /// - an empty/malformed accelerator → `VALIDATION`;
    /// - an accelerator that conflicts with an already-registered one →
    ///   `CONFLICT` (Req 20.11);
    /// - exceeding the [`MAX_SHORTCUTS`] cap with a new accelerator → `VALIDATION`.
    pub fn register(&mut self, shortcut: Shortcut) -> Result<(), AppError> {
        let normalized = normalize_accelerator(&shortcut.accelerator).ok_or_else(|| {
            AppError::validation(format!(
                "invalid shortcut accelerator `{}`",
                shortcut.accelerator
            ))
        })?;

        if self.entries.contains_key(&normalized) {
            return Err(AppError::conflict(format!(
                "shortcut accelerator `{}` conflicts with an already-registered shortcut",
                shortcut.accelerator
            )));
        }

        if self.entries.len() >= MAX_SHORTCUTS {
            return Err(AppError::validation(format!(
                "cannot register more than {MAX_SHORTCUTS} shortcuts"
            )));
        }

        self.entries.insert(normalized, shortcut);
        Ok(())
    }

    /// Builds a fresh registry from a full set of shortcuts (the "set up to 50
    /// shortcuts" batch operation, Req 20.6).
    ///
    /// Rejects the whole batch — returning no registry, so the caller keeps its
    /// prior set — when the batch exceeds [`MAX_SHORTCUTS`] (`VALIDATION`) or
    /// contains two shortcuts that conflict (`CONFLICT`).
    pub fn register_all(shortcuts: Vec<Shortcut>) -> Result<ShortcutRegistry, AppError> {
        if shortcuts.len() > MAX_SHORTCUTS {
            return Err(AppError::validation(format!(
                "cannot register more than {MAX_SHORTCUTS} shortcuts (got {})",
                shortcuts.len()
            )));
        }
        let mut registry = ShortcutRegistry::new();
        for shortcut in shortcuts {
            registry.register(shortcut)?;
        }
        Ok(registry)
    }
}

// ===========================================================================
// Notifications (Req 20.7, 20.13)
// ===========================================================================

/// A validated notification ready to be displayed by the Command_Layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPayload {
    /// Notification title (≤ [`MAX_NOTIFICATION_TITLE`] characters).
    pub title: String,
    /// Notification body (≤ [`MAX_NOTIFICATION_BODY`] characters).
    pub body: String,
}

/// Validates a notification's title and body lengths (Req 20.7).
///
/// Lengths are measured in Unicode scalar values (`char`s). A title longer than
/// [`MAX_NOTIFICATION_TITLE`] or a body longer than [`MAX_NOTIFICATION_BODY`] is
/// rejected with `VALIDATION`; the boundary lengths are accepted.
pub fn validate_notification(title: &str, body: &str) -> Result<NotificationPayload, AppError> {
    let title_len = title.chars().count();
    if title_len > MAX_NOTIFICATION_TITLE {
        return Err(AppError::validation(format!(
            "notification title exceeds {MAX_NOTIFICATION_TITLE} characters (was {title_len})"
        )));
    }
    let body_len = body.chars().count();
    if body_len > MAX_NOTIFICATION_BODY {
        return Err(AppError::validation(format!(
            "notification body exceeds {MAX_NOTIFICATION_BODY} characters (was {body_len})"
        )));
    }
    Ok(NotificationPayload {
        title: title.to_string(),
        body: body.to_string(),
    })
}

/// The error returned when the operating system denies notification permission
/// (Req 20.13). The Command_Layer returns this instead of displaying the
/// notification when the `tauri-plugin-notification` permission request fails.
pub fn notification_permission_denied() -> AppError {
    AppError::unauthorized("notifications are not permitted by the operating system")
}

// ===========================================================================
// Runtime paths (Req 20.9)
// ===========================================================================

/// The resolved absolute filesystem paths reported to the Frontend (Req 20.9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePathsReport {
    /// The data directory.
    pub data: String,
    /// The SQLite database file path.
    pub database: String,
    /// The media directory.
    pub media: String,
    /// The rule files directory.
    pub rule: String,
    /// The backup archives directory.
    pub backup: String,
    /// The log directory.
    pub log: String,
}

/// Returns the SQLite database file path within the data directory.
///
/// Reuses the Data_Path_Manager's convention so the runtime-paths report (Req
/// 20.9) and the startup sequence agree on the database location (Req 23.2).
pub fn database_path(paths: &RuntimePaths) -> PathBuf {
    crate::services::data_path::database_path(paths)
}

/// Builds the runtime-paths report from the resolved [`RuntimePaths`] (Req 20.9).
pub fn get_runtime_paths(paths: &RuntimePaths) -> RuntimePathsReport {
    RuntimePathsReport {
        data: path_string(&paths.data),
        database: path_string(&database_path(paths)),
        media: path_string(&paths.media),
        rule: path_string(&paths.rule),
        backup: path_string(&paths.backup),
        log: path_string(&paths.log),
    }
}

/// Renders a path as an owned `String` for the wire.
fn path_string(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

// ===========================================================================
// Cache size / clear (Req 20.8)
// ===========================================================================

/// Returns the total size in bytes of all files under `dir`, recursively.
///
/// Returns `0` when `dir` does not exist or is not a directory. Unreadable
/// entries are skipped rather than failing the whole computation.
pub fn get_cache_size(dir: &Path) -> u64 {
    if !dir.is_dir() {
        return 0;
    }
    let mut total = 0u64;
    let Ok(entries) = fs::read_dir(dir) else {
        return total;
    };
    for entry in entries.flatten() {
        match entry.file_type() {
            Ok(ft) if ft.is_dir() => total += get_cache_size(&entry.path()),
            Ok(ft) if ft.is_file() => {
                if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            }
            _ => {}
        }
    }
    total
}

/// Clears the cache by removing every entry under `dir`, leaving `dir` itself in
/// place, and reports the number of bytes freed (Req 20.8).
///
/// A non-existent `dir` is treated as an already-empty cache (`Ok(0)`).
pub fn clear_cache(dir: &Path) -> Result<u64, AppError> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let freed = get_cache_size(dir);
    for entry in fs::read_dir(dir)
        .map_err(|e| AppError::io(format!("failed to read cache directory: {e}")))?
    {
        let entry = entry.map_err(|e| AppError::io(format!("failed to read cache entry: {e}")))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| AppError::io(format!("failed to determine cache entry type: {e}")))?;
        if file_type.is_dir() {
            fs::remove_dir_all(&path)
                .map_err(|e| AppError::io(format!("failed to remove cache directory: {e}")))?;
        } else {
            fs::remove_file(&path)
                .map_err(|e| AppError::io(format!("failed to remove cache file: {e}")))?;
        }
    }
    Ok(freed)
}

// ===========================================================================
// Open / reveal path existence (Req 20.10, 20.12)
// ===========================================================================

/// Confirms that `path` exists before the Command_Layer opens or reveals it in
/// the system shell, returning `NOT_FOUND` for a missing target (Req 20.10,
/// 20.12).
pub fn ensure_path_exists(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        Ok(())
    } else {
        Err(AppError::not_found(format!(
            "path `{}` does not exist",
            path.display()
        )))
    }
}

// ===========================================================================
// Platform capability degradation (Req 23.4, 23.5)
// ===========================================================================

/// A platform-integration feature whose availability can vary across
/// Target_Platforms (Req 23.4, 23.5).
///
/// These are exactly the four features Requirement 23.4/23.5 enumerates:
/// auto-launch, tray, shortcuts, and notifications. Where the current platform
/// supports a feature the Window_Manager applies the Requirement 20 handling
/// (23.4); where it does not, the feature is skipped and reported through the
/// non-fatal degradation channel (23.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlatformFeature {
    /// Launch-on-login (autostart) integration.
    AutoLaunch,
    /// System tray / status-bar icon integration.
    Tray,
    /// Global/local keyboard shortcut registration.
    Shortcuts,
    /// System notification display.
    Notifications,
}

impl PlatformFeature {
    /// Every platform feature, in a stable order for deterministic reporting.
    pub const ALL: [PlatformFeature; 4] = [
        PlatformFeature::AutoLaunch,
        PlatformFeature::Tray,
        PlatformFeature::Shortcuts,
        PlatformFeature::Notifications,
    ];

    /// A stable, human-readable label identifying the feature (Req 23.5).
    pub fn label(self) -> &'static str {
        match self {
            PlatformFeature::AutoLaunch => "auto-launch",
            PlatformFeature::Tray => "tray",
            PlatformFeature::Shortcuts => "shortcuts",
            PlatformFeature::Notifications => "notifications",
        }
    }
}

/// The per-feature availability of platform integrations on the current
/// Target_Platform — the capability descriptor referenced by the design (Req
/// 23.4, 23.5).
///
/// Each field is `true` when the running platform supports the corresponding
/// feature. The descriptor is produced by [`probe_capabilities`] at startup and
/// drives both the features the Window_Manager attempts to apply and the
/// non-fatal degradation notices it surfaces for the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    /// Whether launch-on-login is available.
    pub auto_launch: bool,
    /// Whether system-tray integration is available.
    pub tray: bool,
    /// Whether keyboard-shortcut registration is available.
    pub shortcuts: bool,
    /// Whether system notifications are available.
    pub notifications: bool,
}

impl PlatformCapabilities {
    /// A descriptor with every feature available.
    pub const ALL_AVAILABLE: PlatformCapabilities = PlatformCapabilities {
        auto_launch: true,
        tray: true,
        shortcuts: true,
        notifications: true,
    };

    /// A descriptor with every feature unavailable.
    pub const NONE_AVAILABLE: PlatformCapabilities = PlatformCapabilities {
        auto_launch: false,
        tray: false,
        shortcuts: false,
        notifications: false,
    };

    /// Whether `feature` is available on this platform.
    pub fn is_available(self, feature: PlatformFeature) -> bool {
        match feature {
            PlatformFeature::AutoLaunch => self.auto_launch,
            PlatformFeature::Tray => self.tray,
            PlatformFeature::Shortcuts => self.shortcuts,
            PlatformFeature::Notifications => self.notifications,
        }
    }

    /// The non-fatal degradation notices for every unavailable feature, in the
    /// stable [`PlatformFeature::ALL`] order (Req 23.5).
    ///
    /// An empty result means the platform supports every feature; a non-empty
    /// result is the set of features the Window_Manager skips while keeping the
    /// application running.
    pub fn degradations(self) -> Vec<CapabilityDegradation> {
        PlatformFeature::ALL
            .into_iter()
            .filter(|&feature| !self.is_available(feature))
            .map(CapabilityDegradation::new)
            .collect()
    }
}

/// A non-fatal notice that a platform feature is unsupported on the current
/// Target_Platform and was skipped (Req 23.5).
///
/// This is the payload the Command_Layer forwards on the non-fatal capability
/// channel; it identifies the affected feature and carries a human-readable
/// message, and never aborts the application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityDegradation {
    /// The unsupported feature that was skipped.
    pub feature: PlatformFeature,
    /// Human-readable description identifying the unsupported feature (Req 23.5).
    pub message: String,
}

impl CapabilityDegradation {
    /// Builds the degradation notice for a skipped `feature`.
    pub fn new(feature: PlatformFeature) -> Self {
        Self {
            feature,
            message: format!(
                "{} is not supported on this platform; the feature was skipped",
                feature.label()
            ),
        }
    }
}

/// The structured error returned when the Command_Layer is asked to exercise a
/// feature that is unavailable on the current platform (Req 23.5, consistent with
/// 3.7).
///
/// Carries the stable `CAPABILITY_UNAVAILABLE` code and identifies the feature,
/// so a capability-gated call surfaces a structured error instead of attempting
/// an unsupported operation.
pub fn capability_unavailable_error(feature: PlatformFeature) -> AppError {
    AppError::capability_unavailable(format!(
        "{} is not supported on this platform",
        feature.label()
    ))
}

/// Probes the current Target_Platform for platform-integration feature
/// availability (Req 23.4, 23.5).
///
/// The `tauri-plugin-autostart`, `tauri-plugin-global-shortcut`, and
/// `tauri-plugin-notification` plugins, plus tray support, are available on all
/// three desktop Target_Platforms (Windows, macOS, Linux), so each is reported
/// available there. Any other (unrecognized) platform reports every feature
/// unavailable, which the caller surfaces as non-fatal degradation notices while
/// keeping the application running rather than aborting (Req 23.5).
pub fn probe_capabilities() -> PlatformCapabilities {
    probe_capabilities_for(std::env::consts::OS)
}

/// The pure core of [`probe_capabilities`]: maps an operating-system identifier
/// (as produced by [`std::env::consts::OS`]) to the platform capability
/// descriptor.
fn probe_capabilities_for(os: &str) -> PlatformCapabilities {
    match os {
        "windows" | "macos" | "linux" => PlatformCapabilities::ALL_AVAILABLE,
        _ => PlatformCapabilities::NONE_AVAILABLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    // --- event payloads (Req 20.2, 20.3, 20.4, 20.6) ---

    #[test]
    fn fullscreen_payload_serializes_camel_case() {
        let value = serde_json::to_value(FullscreenChanged { fullscreen: true }).unwrap();
        assert_eq!(value, json!({ "fullscreen": true }));
    }

    #[test]
    fn visibility_payload_serializes_camel_case() {
        let value = serde_json::to_value(VisibilityChanged { visible: false }).unwrap();
        assert_eq!(value, json!({ "visible": false }));
    }

    #[test]
    fn close_requested_payload_serializes_to_empty_object() {
        let value = serde_json::to_value(CloseRequested {}).unwrap();
        assert_eq!(value, json!({}));
    }

    #[test]
    fn shortcut_triggered_payload_serializes_camel_case() {
        let value = serde_json::to_value(ShortcutTriggered {
            action: "toggle-window".into(),
        })
        .unwrap();
        assert_eq!(value, json!({ "action": "toggle-window" }));
    }

    // --- close action (Req 20.4) ---

    #[test]
    fn parse_close_action_accepts_known_values() {
        assert_eq!(parse_close_action("ask").unwrap(), CloseAction::Ask);
        assert_eq!(
            parse_close_action("MINIMIZE").unwrap(),
            CloseAction::Minimize
        );
        assert_eq!(parse_close_action(" exit ").unwrap(), CloseAction::Exit);
    }

    #[test]
    fn parse_close_action_rejects_unknown_value() {
        let err = parse_close_action("quit").unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
    }

    #[test]
    fn close_action_decisions_match_requirement() {
        assert_eq!(
            CloseAction::Ask.decision(),
            CloseDecision::EmitCloseRequested
        );
        assert_eq!(CloseAction::Minimize.decision(), CloseDecision::Hide);
        assert_eq!(CloseAction::Exit.decision(), CloseDecision::Terminate);
    }

    // --- accelerator normalization ---

    #[test]
    fn normalize_treats_reordered_and_aliased_modifiers_as_equal() {
        let a = normalize_accelerator("Ctrl+Shift+A").unwrap();
        let b = normalize_accelerator("shift+control+a").unwrap();
        let c = normalize_accelerator("A+Ctrl+Shift").unwrap();
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn normalize_distinguishes_different_keys() {
        assert_ne!(
            normalize_accelerator("Ctrl+A"),
            normalize_accelerator("Ctrl+B")
        );
    }

    #[test]
    fn normalize_rejects_empty_and_trailing_plus() {
        assert!(normalize_accelerator("").is_none());
        assert!(normalize_accelerator("   ").is_none());
        assert!(normalize_accelerator("Ctrl+").is_none());
    }

    // --- shortcut registry (Req 20.6, 20.11 — Property 40) ---

    fn shortcut(action: &str, accelerator: &str, mode: ShortcutMode) -> Shortcut {
        Shortcut {
            action: action.into(),
            accelerator: accelerator.into(),
            mode,
        }
    }

    #[test]
    fn register_adds_a_shortcut() {
        let mut reg = ShortcutRegistry::new();
        reg.register(shortcut("a", "Ctrl+A", ShortcutMode::Global))
            .unwrap();
        assert_eq!(reg.len(), 1);
        assert!(reg.contains_accelerator("ctrl+a"));
    }

    #[test]
    fn register_conflict_returns_conflict_and_preserves_prior_set() {
        let mut reg = ShortcutRegistry::new();
        reg.register(shortcut("a", "Ctrl+A", ShortcutMode::Global))
            .unwrap();
        let before = reg.shortcuts();

        // A reordered/aliased equivalent of the same accelerator conflicts.
        let err = reg
            .register(shortcut("b", "A+Control", ShortcutMode::Local))
            .unwrap_err();
        assert_eq!(err.code_str(), "CONFLICT");
        // The previously registered set is unchanged (Property 40).
        assert_eq!(reg.shortcuts(), before);
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn register_rejects_invalid_accelerator() {
        let mut reg = ShortcutRegistry::new();
        let err = reg
            .register(shortcut("a", "Ctrl+", ShortcutMode::Global))
            .unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
        assert!(reg.is_empty());
    }

    #[test]
    fn register_enforces_fifty_shortcut_cap() {
        let mut reg = ShortcutRegistry::new();
        for i in 0..MAX_SHORTCUTS {
            reg.register(shortcut(
                &format!("a{i}"),
                &format!("Ctrl+F{i}"),
                ShortcutMode::Local,
            ))
            .unwrap();
        }
        assert_eq!(reg.len(), MAX_SHORTCUTS);
        // The 51st distinct shortcut is rejected and the set is unchanged.
        let err = reg
            .register(shortcut("overflow", "Ctrl+Alt+Z", ShortcutMode::Local))
            .unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
        assert_eq!(reg.len(), MAX_SHORTCUTS);
    }

    #[test]
    fn register_all_accepts_up_to_fifty() {
        let shortcuts: Vec<Shortcut> = (0..MAX_SHORTCUTS)
            .map(|i| {
                shortcut(
                    &format!("a{i}"),
                    &format!("Ctrl+F{i}"),
                    ShortcutMode::Global,
                )
            })
            .collect();
        let reg = ShortcutRegistry::register_all(shortcuts).unwrap();
        assert_eq!(reg.len(), MAX_SHORTCUTS);
    }

    #[test]
    fn register_all_rejects_more_than_fifty() {
        let shortcuts: Vec<Shortcut> = (0..=MAX_SHORTCUTS)
            .map(|i| {
                shortcut(
                    &format!("a{i}"),
                    &format!("Ctrl+F{i}"),
                    ShortcutMode::Global,
                )
            })
            .collect();
        let err = ShortcutRegistry::register_all(shortcuts).unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
    }

    #[test]
    fn register_all_rejects_internal_conflict() {
        let shortcuts = vec![
            shortcut("a", "Ctrl+A", ShortcutMode::Global),
            shortcut("b", "a+ctrl", ShortcutMode::Local),
        ];
        let err = ShortcutRegistry::register_all(shortcuts).unwrap_err();
        assert_eq!(err.code_str(), "CONFLICT");
    }

    // --- notifications (Req 20.7, 20.13) ---

    #[test]
    fn validate_notification_accepts_boundary_lengths() {
        let title = "t".repeat(MAX_NOTIFICATION_TITLE);
        let body = "b".repeat(MAX_NOTIFICATION_BODY);
        let payload = validate_notification(&title, &body).unwrap();
        assert_eq!(payload.title.chars().count(), MAX_NOTIFICATION_TITLE);
        assert_eq!(payload.body.chars().count(), MAX_NOTIFICATION_BODY);
    }

    #[test]
    fn validate_notification_rejects_long_title() {
        let title = "t".repeat(MAX_NOTIFICATION_TITLE + 1);
        let err = validate_notification(&title, "body").unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
    }

    #[test]
    fn validate_notification_rejects_long_body() {
        let body = "b".repeat(MAX_NOTIFICATION_BODY + 1);
        let err = validate_notification("title", &body).unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
    }

    #[test]
    fn notification_permission_denied_is_structured_error() {
        let err = notification_permission_denied();
        assert_eq!(err.code_str(), "UNAUTHORIZED");
    }

    // --- runtime paths (Req 20.9) ---

    fn sample_paths(base: &Path) -> RuntimePaths {
        RuntimePaths {
            data: base.join("data"),
            media: base.join("media"),
            rule: base.join("rule"),
            backup: base.join("backup"),
            log: base.join("log"),
        }
    }

    #[test]
    fn get_runtime_paths_reports_all_directories_and_db() {
        let base = Path::new("/app");
        let paths = sample_paths(base);
        let report = get_runtime_paths(&paths);
        assert_eq!(report.data, path_string(&paths.data));
        assert_eq!(report.media, path_string(&paths.media));
        assert_eq!(report.rule, path_string(&paths.rule));
        assert_eq!(report.backup, path_string(&paths.backup));
        assert_eq!(report.log, path_string(&paths.log));
        assert_eq!(
            report.database,
            path_string(&paths.data.join("prompthub.db"))
        );
    }

    #[test]
    fn runtime_paths_report_serializes_camel_case() {
        let report = get_runtime_paths(&sample_paths(Path::new("/app")));
        let value = serde_json::to_value(&report).unwrap();
        for key in ["data", "database", "media", "rule", "backup", "log"] {
            assert!(value.get(key).is_some(), "missing key {key}");
        }
    }

    // --- cache size / clear (Req 20.8) ---

    #[test]
    fn cache_size_is_zero_for_missing_directory() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(get_cache_size(&tmp.path().join("nope")), 0);
    }

    #[test]
    fn cache_size_sums_nested_files() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        fs::write(dir.join("a.bin"), [0u8; 10]).unwrap();
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("b.bin"), [0u8; 25]).unwrap();
        assert_eq!(get_cache_size(dir), 35);
    }

    #[test]
    fn clear_cache_removes_contents_and_reports_freed_bytes() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        fs::write(dir.join("a.bin"), [0u8; 10]).unwrap();
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("b.bin"), [0u8; 25]).unwrap();

        let freed = clear_cache(dir).unwrap();
        assert_eq!(freed, 35);
        // Directory itself remains but is now empty.
        assert!(dir.is_dir());
        assert_eq!(fs::read_dir(dir).unwrap().count(), 0);
        assert_eq!(get_cache_size(dir), 0);
    }

    #[test]
    fn clear_cache_on_missing_directory_is_noop() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(clear_cache(&tmp.path().join("nope")).unwrap(), 0);
    }

    // --- open / reveal existence (Req 20.10, 20.12) ---

    #[test]
    fn ensure_path_exists_ok_for_existing_target() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("f.txt");
        fs::write(&file, b"x").unwrap();
        assert!(ensure_path_exists(&file).is_ok());
        assert!(ensure_path_exists(tmp.path()).is_ok());
    }

    #[test]
    fn ensure_path_exists_not_found_for_missing_target() {
        let tmp = TempDir::new().unwrap();
        let err = ensure_path_exists(&tmp.path().join("missing")).unwrap_err();
        assert_eq!(err.code_str(), "NOT_FOUND");
    }

    // --- platform capability degradation (Req 23.4, 23.5) ---

    #[test]
    fn desktop_platforms_support_every_feature() {
        for os in ["windows", "macos", "linux"] {
            let caps = probe_capabilities_for(os);
            assert_eq!(caps, PlatformCapabilities::ALL_AVAILABLE, "for {os}");
            assert!(caps.degradations().is_empty(), "for {os}");
        }
    }

    #[test]
    fn unknown_platform_degrades_every_feature_without_aborting() {
        let caps = probe_capabilities_for("haiku");
        assert_eq!(caps, PlatformCapabilities::NONE_AVAILABLE);
        let degradations = caps.degradations();
        // Every feature is reported, in the stable order, each identifying itself.
        assert_eq!(degradations.len(), PlatformFeature::ALL.len());
        let features: Vec<PlatformFeature> = degradations.iter().map(|d| d.feature).collect();
        assert_eq!(features, PlatformFeature::ALL.to_vec());
        for degradation in &degradations {
            assert!(degradation.message.contains(degradation.feature.label()));
        }
    }

    #[test]
    fn probe_capabilities_matches_the_host_os() {
        assert_eq!(
            probe_capabilities(),
            probe_capabilities_for(std::env::consts::OS)
        );
    }

    #[test]
    fn degradations_list_only_unavailable_features() {
        let caps = PlatformCapabilities {
            auto_launch: true,
            tray: false,
            shortcuts: true,
            notifications: false,
        };
        let features: Vec<PlatformFeature> =
            caps.degradations().into_iter().map(|d| d.feature).collect();
        assert_eq!(
            features,
            vec![PlatformFeature::Tray, PlatformFeature::Notifications]
        );
        assert!(caps.is_available(PlatformFeature::AutoLaunch));
        assert!(!caps.is_available(PlatformFeature::Tray));
    }

    #[test]
    fn capability_unavailable_error_is_structured_and_identifies_feature() {
        let err = capability_unavailable_error(PlatformFeature::Tray);
        assert_eq!(err.code_str(), "CAPABILITY_UNAVAILABLE");
        assert!(err.message.contains("tray"));
    }

    #[test]
    fn platform_feature_serializes_kebab_case() {
        let value = serde_json::to_value(PlatformFeature::AutoLaunch).unwrap();
        assert_eq!(value, json!("auto-launch"));
    }

    #[test]
    fn capability_descriptor_serializes_camel_case() {
        let value = serde_json::to_value(PlatformCapabilities::ALL_AVAILABLE).unwrap();
        for key in ["autoLaunch", "tray", "shortcuts", "notifications"] {
            assert_eq!(value.get(key), Some(&json!(true)), "missing key {key}");
        }
    }

    #[test]
    fn degradation_payload_serializes_camel_case() {
        let value =
            serde_json::to_value(CapabilityDegradation::new(PlatformFeature::Shortcuts)).unwrap();
        assert_eq!(value.get("feature"), Some(&json!("shortcuts")));
        assert!(value.get("message").is_some());
    }
}
