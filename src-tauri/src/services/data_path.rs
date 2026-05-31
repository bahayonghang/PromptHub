//! Data_Path_Manager: per-user directory resolution (Req 23.2) and the
//! configurable data directory (Requirement 19.3–19.10).
//!
//! The application stores all of its data under a single *data directory*. Which
//! directory is active is recorded in a small JSON *config file*
//! (`{ dataPath, updatedAt }`, mirroring the Reference_App's `data-path.json`).
//! The running process resolves the active directory once at startup, so a
//! configured change only takes effect after a restart — every mutating operation
//! here therefore reports `restartRequired` (19.3, 19.5, 19.8).
//!
//! Responsibilities:
//!
//! - [`resolve_runtime_paths`] / [`database_path`] / [`ensure_directories`]: the
//!   single source of truth for the six per-user runtime subdirectories (data,
//!   media, skill, rule, backup, log) under the platform's application-data root
//!   and the SQLite database path within the data directory, plus the
//!   create-and-verify-writable startup contract (23.2, 23.3, 20.9). The startup
//!   sequence and the Window_Manager's runtime-paths report reuse these so the
//!   resolved set is identical everywhere.
//! - [`get_path`] / [`get_status`]: report the active data directory and whether a
//!   configured change is pending a restart (19.3).
//! - [`preview_change`]: a **read-only** report for a target path — existence,
//!   whether it already holds PromptHub data, whether it is the active directory,
//!   and a recommended action of exactly `migrate` or `switch` (19.4, Property 39).
//! - [`apply_change`]: perform `migrate | switch | overwrite` and report
//!   restart-required (19.5). An unknown action is rejected with `VALIDATION` and
//!   the active directory is left unchanged (19.10).
//! - [`recovery_scan`] / [`recovery_preview`] / [`recovery_apply`]: discover
//!   recoverable PromptHub data in known locations, preview a candidate, and
//!   recover it (19.6–19.8).
//!
//! ## Failure atomicity (19.9)
//!
//! Operations that move data ([`apply_change`] for `migrate`/`overwrite`, and
//! [`recovery_apply`]) stage the copy in a sibling directory and promote it into
//! place with a rename only after the copy fully succeeds, restoring any
//! pre-existing destination on failure. The active (source) directory is only
//! ever read, never modified, and the config file is rewritten only after the
//! data movement succeeds. A failure therefore leaves the active directory
//! unchanged with no partially migrated or recovered state (19.9).
//!
//! ## Testability / dependency injection
//!
//! Every function takes its filesystem locations as arguments — the active data
//! directory, the config-file path, the set of known recovery locations — rather
//! than reaching into [`crate::state::AppState`] or a live Tauri window. The unit
//! tests below drive the module entirely with [`tempfile`] trees; the
//! Command_Layer (task 17.1) supplies the resolved runtime paths.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::RuntimePaths;
use crate::storage::time::{millis_to_iso8601, now_millis};

// ===========================================================================
// Runtime directory resolution (Req 23.2, 20.9)
// ===========================================================================
//
// The Data_Path_Manager is the single source of truth for *where* the
// application's per-user directories live: the six runtime subdirectories under
// the platform's application-data root, and the SQLite database within the data
// directory. The startup sequence ([`crate::commands::startup`]) re-exports these
// and the Window_Manager's `get_runtime_paths` report (Req 20.9) reuses
// [`database_path`], so the resolved set is identical everywhere (Req 23.2).

/// File name of the SQLite database within the data directory.
pub const DATABASE_FILE_NAME: &str = "prompthub.db";

/// Resolves the six per-user runtime directories beneath the application-data
/// root `base` (Req 23.2).
///
/// Each directory is a distinct child of `base`, mirroring the [`RuntimePaths`]
/// fields: `data`, `media`, `skill`, `rule`, `backup`, `log`. The SQLite database
/// lives at `<data>/prompthub.db` (see [`database_path`]).
pub fn resolve_runtime_paths(base: &Path) -> RuntimePaths {
    RuntimePaths {
        data: base.join("data"),
        media: base.join("media"),
        skill: base.join("skill"),
        rule: base.join("rule"),
        backup: base.join("backup"),
        log: base.join("log"),
    }
}

/// Returns the SQLite database file path within the resolved data directory.
pub fn database_path(paths: &RuntimePaths) -> PathBuf {
    paths.data.join(DATABASE_FILE_NAME)
}

/// The labelled runtime directories, in a stable order for deterministic error
/// reporting.
fn directory_entries(paths: &RuntimePaths) -> [(&'static str, &Path); 6] {
    [
        ("data", paths.data.as_path()),
        ("media", paths.media.as_path()),
        ("skill", paths.skill.as_path()),
        ("rule", paths.rule.as_path()),
        ("backup", paths.backup.as_path()),
        ("log", paths.log.as_path()),
    ]
}

/// Creates each runtime directory if absent and verifies it is writable
/// (Req 23.2, 23.3).
///
/// For every directory: it is created (with parents) if it does not exist, then a
/// probe file is written and removed to confirm write access. A failure to create
/// or write returns an `IO` error whose message identifies the affected directory
/// (Req 23.3); no existing data is modified.
pub fn ensure_directories(paths: &RuntimePaths) -> Result<(), AppError> {
    for (label, dir) in directory_entries(paths) {
        ensure_writable_dir(label, dir)?;
    }
    Ok(())
}

/// Creates `dir` if needed and confirms it is writable by writing then removing a
/// probe file. The error identifies the affected directory by `label` and path
/// (Req 23.3).
fn ensure_writable_dir(label: &str, dir: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dir).map_err(|e| {
        AppError::io(format!(
            "failed to create {label} directory `{}`: {e}",
            dir.display()
        ))
    })?;

    let probe = dir.join(".prompthub-write-test");
    fs::write(&probe, b"").map_err(|e| {
        AppError::io(format!(
            "{label} directory `{}` is not writable: {e}",
            dir.display()
        ))
    })?;
    // Best-effort cleanup; a leftover empty probe must never fail startup.
    let _ = fs::remove_file(&probe);
    Ok(())
}

/// Marker entries whose presence in a directory indicates it holds PromptHub
/// data. Ported from the Reference_App's `DATA_MARKERS`
/// (`apps/desktop/src/main/data-path.ts`) plus the unified database file.
const DATA_MARKERS: &[&str] = &[
    "data",
    "config",
    "backups",
    "logs",
    "images",
    "videos",
    "skills",
    "rules",
    "shortcuts.json",
    "shortcut-mode.json",
    "prompthub.db",
    "data/prompthub.db",
];

/// Recommended action when the target already holds PromptHub data: adopt it
/// without copying (19.4).
const ACTION_SWITCH: &str = "switch";
/// Recommended action when the target holds no PromptHub data: copy the current
/// data into it (19.4).
const ACTION_MIGRATE: &str = "migrate";
/// Apply action replacing the target's data with the current data (19.5).
const ACTION_OVERWRITE: &str = "overwrite";

// ===========================================================================
// Config file
// ===========================================================================

/// Persisted data-path configuration (`{ dataPath, updatedAt }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DataPathConfig {
    /// Absolute path of the configured data directory.
    data_path: String,
    /// ISO_8601 timestamp of when the configuration was last written.
    updated_at: String,
}

/// Reads the configured data path from `config_path`, or `None` when the file is
/// absent, unreadable, or carries an empty/invalid `dataPath`.
fn read_configured_path(config_path: &Path) -> Option<PathBuf> {
    let raw = fs::read_to_string(config_path).ok()?;
    let parsed: DataPathConfig = serde_json::from_str(&raw).ok()?;
    let trimmed = parsed.data_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

/// Writes `target` as the configured data path to `config_path`, creating parent
/// directories as needed.
fn write_configured_path(config_path: &Path, target: &Path) -> Result<(), AppError> {
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::io(format!("failed to create config directory: {e}")))?;
    }
    let config = DataPathConfig {
        data_path: target.to_string_lossy().into_owned(),
        updated_at: millis_to_iso8601(now_millis()),
    };
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| AppError::internal(format!("failed to encode data-path config: {e}")))?;
    fs::write(config_path, json)
        .map_err(|e| AppError::io(format!("failed to write data-path config: {e}")))
}

// ===========================================================================
// Path helpers
// ===========================================================================

/// Returns `true` when `a` and `b` refer to the same location. Canonicalizes when
/// both exist (resolving symlinks/`..`), otherwise compares the paths directly.
fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

/// Renders a path as an owned `String` for the wire.
fn path_string(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

// ===========================================================================
// Data inspection (read-only)
// ===========================================================================

/// Kind of a discovered marker entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkerKind {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// Anything else (symlink, device, …).
    Other,
}

/// A PromptHub data marker discovered inside an inspected directory (19.4, 19.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataMarker {
    /// The marker name (e.g. `data`, `prompthub.db`).
    pub name: String,
    /// Absolute path of the marker.
    pub path: String,
    /// Whether the marker is a file, directory, or other.
    pub kind: MarkerKind,
}

/// Discovers the PromptHub data markers present directly under `dir`.
///
/// Pure read-only inspection — performs no writes. Returns an empty vector when
/// `dir` does not exist or holds no markers.
fn discover_markers(dir: &Path) -> Vec<DataMarker> {
    let mut markers = Vec::new();
    if !dir.is_dir() {
        return markers;
    }
    for name in DATA_MARKERS {
        let marker_path = dir.join(name);
        let kind = match fs::symlink_metadata(&marker_path) {
            Ok(meta) if meta.is_dir() => MarkerKind::Directory,
            Ok(meta) if meta.is_file() => MarkerKind::File,
            Ok(_) => MarkerKind::Other,
            Err(_) => continue,
        };
        markers.push(DataMarker {
            name: (*name).to_string(),
            path: path_string(&marker_path),
            kind,
        });
    }
    markers
}

// ===========================================================================
// get_path / get_status (19.3)
// ===========================================================================

/// Active data-path status (19.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataPathStatus {
    /// The data directory the running process is currently using.
    pub active_path: String,
    /// The configured data directory, when one is recorded and differs from
    /// `active_path` (i.e. a change awaiting restart).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configured_path: Option<String>,
    /// Whether a restart is required to adopt a configured change.
    pub restart_required: bool,
}

/// Returns the active data directory (19.3).
pub fn get_path(active_data_dir: &Path) -> Result<String, AppError> {
    Ok(path_string(active_data_dir))
}

/// Returns the active data directory and whether a configured change is pending a
/// restart (19.3).
///
/// `restart_required` is `true` when the config file records a data path that
/// differs from the active directory — meaning the change will take effect on the
/// next launch.
pub fn get_status(active_data_dir: &Path, config_path: &Path) -> Result<DataPathStatus, AppError> {
    let configured = read_configured_path(config_path);
    let restart_required = configured
        .as_deref()
        .is_some_and(|configured| !same_path(configured, active_data_dir));
    let configured_path = if restart_required {
        configured.as_deref().map(path_string)
    } else {
        None
    };
    Ok(DataPathStatus {
        active_path: path_string(active_data_dir),
        configured_path,
        restart_required,
    })
}

// ===========================================================================
// preview_change (19.4)
// ===========================================================================

/// Read-only preview of a data-path change (19.4, Property 39).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    /// The target path that was previewed.
    pub target_path: String,
    /// Whether the target path exists.
    pub exists: bool,
    /// Whether the target already contains PromptHub data.
    pub has_prompt_hub_data: bool,
    /// Whether the target is the active data directory.
    pub is_current: bool,
    /// Recommended action: `migrate` (target empty) or `switch` (target has data).
    pub recommended_action: String,
    /// PromptHub data markers found at the target.
    pub markers: Vec<DataMarker>,
}

/// Reports the state of `target` for a prospective data-path change, **without
/// modifying** the target or the active data directory (19.4, Property 39).
///
/// The recommended action is `switch` when the target already holds PromptHub
/// data (adopt it as-is) and `migrate` otherwise (copy the current data into it).
pub fn preview_change(active_data_dir: &Path, target: &Path) -> Result<PreviewResult, AppError> {
    let exists = target.exists();
    let markers = discover_markers(target);
    let has_prompt_hub_data = !markers.is_empty();
    let is_current = same_path(target, active_data_dir);
    let recommended_action = if has_prompt_hub_data {
        ACTION_SWITCH
    } else {
        ACTION_MIGRATE
    };
    Ok(PreviewResult {
        target_path: path_string(target),
        exists,
        has_prompt_hub_data,
        is_current,
        recommended_action: recommended_action.to_string(),
        markers,
    })
}

// ===========================================================================
// apply_change (19.5, 19.9, 19.10)
// ===========================================================================

/// Result of an apply or recovery operation (19.5, 19.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    /// Whether a restart is required to complete the change (always `true`).
    pub restart_required: bool,
    /// The newly configured data directory.
    pub configured_path: String,
}

/// Applies a data-path change with `action` ∈ {`migrate`, `switch`, `overwrite`}
/// and reports restart-required (19.5).
///
/// - `migrate` / `overwrite`: copy the current data into `target`, then record
///   `target` as the configured data path. Both move the current data into the
///   target; `overwrite` is the user's explicit choice to replace data already
///   present there.
/// - `switch`: adopt `target` as the configured data path without copying.
///
/// An action outside that set is rejected with `VALIDATION`, leaving the active
/// directory unchanged (19.10). On any failure the active directory is left
/// unchanged with no partial state (19.9).
pub fn apply_change(
    active_data_dir: &Path,
    config_path: &Path,
    target: &Path,
    action: &str,
) -> Result<ApplyResult, AppError> {
    match action {
        ACTION_MIGRATE | ACTION_OVERWRITE => {
            promote_into(active_data_dir, target)?;
        }
        ACTION_SWITCH => {
            fs::create_dir_all(target)
                .map_err(|e| AppError::io(format!("failed to create target directory: {e}")))?;
        }
        _ => {
            return Err(AppError::validation(format!(
                "unknown data-path action `{action}`; expected migrate, switch, or overwrite"
            )));
        }
    }

    write_configured_path(config_path, target)?;
    Ok(ApplyResult {
        restart_required: true,
        configured_path: path_string(target),
    })
}

// ===========================================================================
// Recovery (19.6, 19.7, 19.8)
// ===========================================================================

/// A recoverable PromptHub data source discovered by [`recovery_scan`] (19.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverySource {
    /// Absolute path of the candidate source directory.
    pub path: String,
    /// PromptHub data markers found at the source.
    pub markers: Vec<DataMarker>,
}

/// Searches `known_locations` for recoverable PromptHub data, returning the
/// candidate sources without modifying anything (19.6).
///
/// A location qualifies when it holds PromptHub data and is not the active data
/// directory. Returns an empty list when nothing recoverable is found. Duplicate
/// locations (resolving to the same directory) are reported once.
pub fn recovery_scan(
    active_data_dir: &Path,
    known_locations: &[PathBuf],
) -> Result<Vec<RecoverySource>, AppError> {
    let mut sources: Vec<RecoverySource> = Vec::new();
    for location in known_locations {
        if same_path(location, active_data_dir) {
            continue;
        }
        if sources
            .iter()
            .any(|s| same_path(Path::new(&s.path), location))
        {
            continue;
        }
        let markers = discover_markers(location);
        if !markers.is_empty() {
            sources.push(RecoverySource {
                path: path_string(location),
                markers,
            });
        }
    }
    Ok(sources)
}

/// Read-only preview of a recovery candidate's recoverable contents (19.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPreview {
    /// Absolute path of the candidate source.
    pub source_path: String,
    /// Whether the source exists.
    pub exists: bool,
    /// Whether the source contains recoverable PromptHub data.
    pub has_prompt_hub_data: bool,
    /// PromptHub data markers found at the source.
    pub markers: Vec<DataMarker>,
}

/// Reports the recoverable contents of `source` without modifying current data
/// (19.7).
pub fn recovery_preview(source: &Path) -> Result<RecoveryPreview, AppError> {
    let markers = discover_markers(source);
    Ok(RecoveryPreview {
        source_path: path_string(source),
        exists: source.exists(),
        has_prompt_hub_data: !markers.is_empty(),
        markers,
    })
}

/// Recovers data from `source` into the active data directory and reports
/// restart-required (19.8).
///
/// `source` must contain recoverable PromptHub data, otherwise the request is
/// rejected with `VALIDATION` and nothing is changed. The recovery copies the
/// source into the active data directory using the same staged promotion as
/// [`apply_change`], so a failure leaves the active directory unchanged with no
/// partial state (19.9).
pub fn recovery_apply(active_data_dir: &Path, source: &Path) -> Result<ApplyResult, AppError> {
    if discover_markers(source).is_empty() {
        return Err(AppError::validation(
            "recovery source contains no recoverable PromptHub data",
        ));
    }
    promote_into(source, active_data_dir)?;
    Ok(ApplyResult {
        restart_required: true,
        configured_path: path_string(active_data_dir),
    })
}

// ===========================================================================
// Staged copy + promote (failure atomicity — 19.9)
// ===========================================================================

/// Copies `source`'s contents into `target`, replacing any existing target,
/// atomically with respect to failure: the copy is staged in a sibling directory
/// and only promoted into place once it fully succeeds; a pre-existing target is
/// restored if promotion fails (19.9).
///
/// `source` is only ever read, never modified.
fn promote_into(source: &Path, target: &Path) -> Result<(), AppError> {
    if !source.is_dir() {
        return Err(AppError::not_found(format!(
            "source data directory `{}` does not exist",
            source.display()
        )));
    }
    if same_path(source, target) {
        // Copying a directory onto itself is a no-op; avoid clobbering it.
        return Ok(());
    }

    let parent = target.parent().ok_or_else(|| {
        AppError::validation("target data directory must have a parent directory")
    })?;
    fs::create_dir_all(parent)
        .map_err(|e| AppError::io(format!("failed to create target parent directory: {e}")))?;

    let token = uuid::Uuid::new_v4().simple().to_string();
    let staging = parent.join(format!(".prompthub-staging-{token}"));

    // Stage the full copy first. Clean up on any failure so no partial state
    // is left behind (19.9).
    if let Err(e) = copy_dir_recursive(source, &staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }

    // Move any existing target aside so it can be restored if promotion fails.
    let backup = if target.exists() {
        let backup = parent.join(format!(".prompthub-backup-{token}"));
        if let Err(e) = fs::rename(target, &backup) {
            let _ = fs::remove_dir_all(&staging);
            return Err(AppError::io(format!(
                "failed to set aside existing target directory: {e}"
            )));
        }
        Some(backup)
    } else {
        None
    };

    // Promote the staged copy into place.
    match fs::rename(&staging, target) {
        Ok(()) => {
            if let Some(backup) = backup {
                let _ = fs::remove_dir_all(&backup);
            }
            Ok(())
        }
        Err(e) => {
            // Restore the original target, then clean up staging.
            if let Some(backup) = backup {
                let _ = fs::rename(&backup, target);
            }
            let _ = fs::remove_dir_all(&staging);
            Err(AppError::io(format!("failed to promote staged data: {e}")))
        }
    }
}

/// Recursively copies the contents of `src` into `dest`, creating `dest` and any
/// missing parents. Symlinks and other special entries are skipped.
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dest)
        .map_err(|e| AppError::io(format!("failed to create directory: {e}")))?;
    let entries =
        fs::read_dir(src).map_err(|e| AppError::io(format!("failed to read directory: {e}")))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| AppError::io(format!("failed to read directory entry: {e}")))?;
        let file_type = entry
            .file_type()
            .map_err(|e| AppError::io(format!("failed to determine entry type: {e}")))?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if file_type.is_file() {
            fs::copy(&from, &to).map_err(|e| AppError::io(format!("failed to copy file: {e}")))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Writes a PromptHub-data marker file into `dir` so it looks like a data dir.
    fn seed_data(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("prompthub.db"), b"DB").unwrap();
        fs::create_dir_all(dir.join("data")).unwrap();
        fs::write(dir.join("data").join("prompthub.db"), b"DB").unwrap();
    }

    // --- runtime directory resolution (23.2, 23.3, 20.9) ---

    #[test]
    fn resolve_runtime_paths_uses_distinct_children() {
        let base = Path::new("/app");
        let paths = resolve_runtime_paths(base);
        assert_eq!(paths.data, base.join("data"));
        assert_eq!(paths.media, base.join("media"));
        assert_eq!(paths.skill, base.join("skill"));
        assert_eq!(paths.rule, base.join("rule"));
        assert_eq!(paths.backup, base.join("backup"));
        assert_eq!(paths.log, base.join("log"));
        let all = [
            &paths.data,
            &paths.media,
            &paths.skill,
            &paths.rule,
            &paths.backup,
            &paths.log,
        ];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn database_path_is_under_data_dir() {
        let paths = resolve_runtime_paths(Path::new("/app"));
        assert_eq!(database_path(&paths), paths.data.join("prompthub.db"));
    }

    #[test]
    fn ensure_directories_creates_all_six() {
        let tmp = TempDir::new().unwrap();
        let paths = resolve_runtime_paths(tmp.path());
        ensure_directories(&paths).unwrap();
        for (_, dir) in directory_entries(&paths) {
            assert!(dir.is_dir(), "expected `{}` to be created", dir.display());
        }
    }

    #[test]
    fn ensure_directories_is_idempotent_and_leaves_no_probe_file() {
        let tmp = TempDir::new().unwrap();
        let paths = resolve_runtime_paths(tmp.path());
        ensure_directories(&paths).unwrap();
        // A second run over already-existing directories must succeed.
        ensure_directories(&paths).unwrap();
        for (_, dir) in directory_entries(&paths) {
            assert!(!dir.join(".prompthub-write-test").exists());
        }
    }

    #[test]
    fn ensure_writable_dir_error_identifies_the_directory() {
        // A path whose parent is a file cannot be created as a directory.
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("a-file");
        fs::write(&file, b"x").unwrap();
        let blocked = file.join("data");
        let err = ensure_writable_dir("data", &blocked).unwrap_err();
        assert_eq!(err.code_str(), "IO");
        assert!(err.message.contains("data"));
    }

    // --- get_path / get_status (19.3) ---

    #[test]
    fn get_path_returns_active_directory() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        assert_eq!(get_path(&active).unwrap(), path_string(&active));
    }

    #[test]
    fn get_status_no_config_means_no_restart() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        let config = tmp.path().join("data-path.json");
        let status = get_status(&active, &config).unwrap();
        assert!(!status.restart_required);
        assert!(status.configured_path.is_none());
    }

    #[test]
    fn get_status_matching_config_means_no_restart() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        fs::create_dir_all(&active).unwrap();
        let config = tmp.path().join("data-path.json");
        write_configured_path(&config, &active).unwrap();
        let status = get_status(&active, &config).unwrap();
        assert!(!status.restart_required);
    }

    #[test]
    fn get_status_differing_config_requires_restart() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        let configured = tmp.path().join("elsewhere");
        let config = tmp.path().join("data-path.json");
        write_configured_path(&config, &configured).unwrap();
        let status = get_status(&active, &config).unwrap();
        assert!(status.restart_required);
        assert_eq!(
            status.configured_path.as_deref(),
            Some(path_string(&configured).as_str())
        );
    }

    // --- preview_change (19.4) ---

    #[test]
    fn preview_empty_target_recommends_migrate() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        let target = tmp.path().join("empty");
        let preview = preview_change(&active, &target).unwrap();
        assert!(!preview.exists);
        assert!(!preview.has_prompt_hub_data);
        assert!(!preview.is_current);
        assert_eq!(preview.recommended_action, "migrate");
    }

    #[test]
    fn preview_target_with_data_recommends_switch() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        let target = tmp.path().join("with-data");
        seed_data(&target);
        let preview = preview_change(&active, &target).unwrap();
        assert!(preview.exists);
        assert!(preview.has_prompt_hub_data);
        assert_eq!(preview.recommended_action, "switch");
        assert!(!preview.markers.is_empty());
    }

    #[test]
    fn preview_active_directory_is_current() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        seed_data(&active);
        let preview = preview_change(&active, &active).unwrap();
        assert!(preview.is_current);
    }

    #[test]
    fn preview_does_not_modify_target_or_active() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        seed_data(&active);
        let target = tmp.path().join("missing");
        let before_active = fs::read_dir(&active).unwrap().count();
        preview_change(&active, &target).unwrap();
        // Target was not created.
        assert!(!target.exists());
        // Active is unchanged.
        assert_eq!(fs::read_dir(&active).unwrap().count(), before_active);
    }

    // --- apply_change (19.5, 19.10) ---

    #[test]
    fn apply_migrate_copies_data_and_writes_config() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        seed_data(&active);
        let target = tmp.path().join("target");
        let config = tmp.path().join("data-path.json");

        let result = apply_change(&active, &config, &target, "migrate").unwrap();
        assert!(result.restart_required);
        // Data was copied into the target.
        assert!(target.join("prompthub.db").is_file());
        // Active data is unchanged.
        assert!(active.join("prompthub.db").is_file());
        // Config now points at the target.
        assert_eq!(read_configured_path(&config).unwrap(), target);
    }

    #[test]
    fn apply_switch_does_not_copy_but_writes_config() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        seed_data(&active);
        let target = tmp.path().join("target");
        let config = tmp.path().join("data-path.json");

        let result = apply_change(&active, &config, &target, "switch").unwrap();
        assert!(result.restart_required);
        // Target dir created but no data copied.
        assert!(target.is_dir());
        assert!(!target.join("prompthub.db").exists());
        assert_eq!(read_configured_path(&config).unwrap(), target);
    }

    #[test]
    fn apply_overwrite_replaces_target_data() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        seed_data(&active);
        fs::write(active.join("marker-active.txt"), b"ACTIVE").unwrap();
        let target = tmp.path().join("target");
        seed_data(&target);
        fs::write(target.join("stale.txt"), b"OLD").unwrap();
        let config = tmp.path().join("data-path.json");

        apply_change(&active, &config, &target, "overwrite").unwrap();
        // Target now mirrors the active data; the stale file is gone.
        assert!(target.join("marker-active.txt").is_file());
        assert!(!target.join("stale.txt").exists());
    }

    #[test]
    fn apply_invalid_action_is_validation_and_changes_nothing() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        seed_data(&active);
        let target = tmp.path().join("target");
        let config = tmp.path().join("data-path.json");

        let err = apply_change(&active, &config, &target, "delete").unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
        // No target created, no config written, active untouched.
        assert!(!target.exists());
        assert!(!config.exists());
        assert!(active.join("prompthub.db").is_file());
    }

    // --- recovery (19.6, 19.7, 19.8) ---

    #[test]
    fn recovery_scan_finds_candidates_excluding_active() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        seed_data(&active);
        let candidate = tmp.path().join("old-location");
        seed_data(&candidate);
        let empty = tmp.path().join("empty");
        fs::create_dir_all(&empty).unwrap();

        let sources =
            recovery_scan(&active, &[active.clone(), candidate.clone(), empty.clone()]).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].path, path_string(&candidate));
    }

    #[test]
    fn recovery_scan_returns_empty_when_nothing_recoverable() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        let empty = tmp.path().join("empty");
        fs::create_dir_all(&empty).unwrap();
        let sources = recovery_scan(&active, &[empty]).unwrap();
        assert!(sources.is_empty());
    }

    #[test]
    fn recovery_preview_reports_contents_without_modifying() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source");
        seed_data(&source);
        let before = fs::read_dir(&source).unwrap().count();
        let preview = recovery_preview(&source).unwrap();
        assert!(preview.exists);
        assert!(preview.has_prompt_hub_data);
        assert!(!preview.markers.is_empty());
        assert_eq!(fs::read_dir(&source).unwrap().count(), before);
    }

    #[test]
    fn recovery_apply_recovers_into_active() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        fs::create_dir_all(&active).unwrap();
        let source = tmp.path().join("source");
        seed_data(&source);
        fs::write(source.join("recovered.txt"), b"R").unwrap();

        let result = recovery_apply(&active, &source).unwrap();
        assert!(result.restart_required);
        assert!(active.join("recovered.txt").is_file());
        assert!(active.join("prompthub.db").is_file());
        // Source is left intact.
        assert!(source.join("prompthub.db").is_file());
    }

    #[test]
    fn recovery_apply_rejects_empty_source() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        seed_data(&active);
        let source = tmp.path().join("empty");
        fs::create_dir_all(&source).unwrap();

        let err = recovery_apply(&active, &source).unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
        // Active data unchanged.
        assert!(active.join("prompthub.db").is_file());
    }
}
