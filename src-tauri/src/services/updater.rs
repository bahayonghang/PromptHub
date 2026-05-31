//! Updater: checks for, downloads, and installs signed application updates
//! (Requirement 24).
//!
//! The actual update transport — querying the configured endpoint, streaming the
//! package, verifying its minisign signature, and applying it on restart — is
//! owned by [`tauri_plugin_updater`]. This module is the thin layer around that
//! plugin that the Command_Layer (task 17.1) dispatches to. Following the pattern
//! of the sibling services ([`crate::services::ai`],
//! [`crate::services::sync`]), the **pure, decision-making logic is extracted and
//! unit-tested**, while the parts that genuinely require a live `AppHandle` + the
//! updater plugin are kept as small, well-marked glue:
//!
//! ## Pure, unit-tested logic
//!
//! - [`get_platform`] — returns exactly one of `Windows | macOS | Linux` from the
//!   compile-time target OS (24.6).
//! - [`get_version`] — the running application's version string (24.6).
//! - [`parse_version`] / [`is_newer`] — semver-based update-availability
//!   determination (24.2): a candidate is an update when it orders strictly above
//!   the current version.
//! - [`build_check_result`] — maps the plugin's "found / not found" check outcome
//!   to the [`UpdateCheckResult`] DTO returned to the Frontend (24.2).
//! - [`ProgressTracker`] + [`drive_progress`] — the download progress/event
//!   contract: a [`UpdaterStatus`] per chunk reporting `{ downloaded, total }`,
//!   followed by exactly one terminal completion event (24.3).
//! - [`UpdaterErrorKind`] — the failure → [`AppError`] code mapping, in
//!   particular signature failures → `SIGNATURE` (24.5). This is decoupled from
//!   the plugin's `#[non_exhaustive]` (and therefore non-constructable outside its
//!   crate) `Error` type precisely so the mapping table is unit-testable.
//!
//! ## Live runtime integration (thin glue; exercised end-to-end, not in unit tests)
//!
//! - [`check`] — builds the plugin updater with a 30-second timeout and queries
//!   the endpoint (24.2).
//! - [`download`] — streams the package through the plugin, emitting
//!   [`UpdaterStatus`] progress through an injected [`UpdaterEventSink`] (24.3).
//! - [`install`] — applies the verified package via the plugin (24.4).
//!
//! These three require the updater plugin to be registered on the Tauri builder
//! and an updater endpoint + public key configured in `tauri.conf.json` (tasks
//! 17.2 / 24.1); they are not reachable from a unit test without a live release
//! server. Every failure path maps to a structured [`AppError`] and leaves the
//! installed version unchanged (24.7).
#![allow(dead_code)]

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Timeout applied to the update *check* request (24.2). Set on the plugin
/// updater builder before [`tauri_plugin_updater::Updater::check`] is issued.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(30);

// ===========================================================================
// Platform + version (24.6)
// ===========================================================================

/// Returns the current platform identifier, exactly one of `Windows`, `macOS`,
/// or `Linux` (24.6).
///
/// Non-Windows, non-macOS targets (including Linux and the other unix variants
/// Tauri supports) report `Linux`, so the result is always one of the three
/// required values.
pub fn get_platform() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "Windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macOS"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        "Linux"
    }
}

/// Returns the running application's version (24.6).
///
/// Sourced from the compile-time package version, which is the same value the
/// Tauri bundler stamps into the application from `Cargo.toml`/`tauri.conf.json`.
pub fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ===========================================================================
// Version comparison / availability determination (24.2)
// ===========================================================================

/// Parses a version string into a [`semver::Version`], tolerating a leading
/// `v`/`V` prefix (e.g. `v1.2.3`), or returns `VALIDATION`.
///
/// Build metadata and pre-release identifiers are handled by `semver` per the
/// SemVer precedence rules, so `1.0.0-alpha` orders below `1.0.0`.
pub fn parse_version(raw: &str) -> Result<semver::Version, AppError> {
    let trimmed = raw.trim();
    let stripped = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    semver::Version::parse(stripped)
        .map_err(|e| AppError::validation(format!("invalid version `{raw}`: {e}")))
}

/// Returns `true` when `candidate` is a newer version than `current` (24.2).
///
/// Both arguments are parsed with [`parse_version`]; a malformed version yields
/// `VALIDATION`. An update is available iff the candidate orders strictly above
/// the current version under SemVer precedence.
pub fn is_newer(current: &str, candidate: &str) -> Result<bool, AppError> {
    let current = parse_version(current)?;
    let candidate = parse_version(candidate)?;
    Ok(candidate > current)
}

// ===========================================================================
// Check result DTO (24.2)
// ===========================================================================

/// The outcome of an update check returned to the Frontend (24.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    /// Whether an update is available.
    pub available: bool,
    /// The available version identifier, present only when `available` is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The currently running application version.
    pub current_version: String,
    /// Release notes for the available update, when the server provided them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// An update the plugin's check discovered: its announced version and optional
/// release notes. Decouples [`build_check_result`] from the plugin's `Update`
/// type so the mapping is unit-testable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredUpdate {
    /// The announced (available) version identifier.
    pub version: String,
    /// Release notes, when present.
    pub notes: Option<String>,
}

/// Builds the [`UpdateCheckResult`] from the plugin's check outcome (24.2).
///
/// The updater plugin's `check()` returns `Some(update)` only when its version
/// comparator considers the remote release newer than the running version, so a
/// present `found` means an update is available; `None` means the app is current.
pub fn build_check_result(
    current_version: &str,
    found: Option<DiscoveredUpdate>,
) -> UpdateCheckResult {
    match found {
        Some(update) => UpdateCheckResult {
            available: true,
            version: Some(update.version),
            current_version: current_version.to_string(),
            notes: update.notes,
        },
        None => UpdateCheckResult {
            available: false,
            version: None,
            current_version: current_version.to_string(),
            notes: None,
        },
    }
}

// ===========================================================================
// Download progress + events (24.3)
// ===========================================================================

/// Phase string for an in-progress download chunk event.
const PHASE_DOWNLOADING: &str = "downloading";
/// Phase string for the single terminal download-completion event.
const PHASE_DONE: &str = "done";

/// A single `updater:status` event payload (24.3).
///
/// Carries the cumulative `downloaded` byte count and, when the server reported a
/// `Content-Length`, the `total`. The terminal event uses phase [`PHASE_DONE`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterStatus {
    /// Lifecycle phase: `downloading` for progress, `done` for completion.
    pub phase: String,
    /// Bytes downloaded so far (cumulative).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloaded: Option<u64>,
    /// Total bytes to download, when the server reported a content length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

impl UpdaterStatus {
    fn downloading(downloaded: u64, total: Option<u64>) -> Self {
        Self {
            phase: PHASE_DOWNLOADING.to_string(),
            downloaded: Some(downloaded),
            total,
        }
    }

    fn done(downloaded: u64, total: Option<u64>) -> Self {
        Self {
            phase: PHASE_DONE.to_string(),
            downloaded: Some(downloaded),
            total,
        }
    }
}

/// Accumulates download progress across chunks (24.3).
///
/// `record` is called once per received chunk with the chunk length and the
/// server-reported content length (if any); it accumulates the running byte total
/// and produces the progress status to emit. `finish` produces the single
/// terminal completion status. The running total is monotonically non-decreasing.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProgressTracker {
    downloaded: u64,
    total: Option<u64>,
}

impl ProgressTracker {
    /// Creates a tracker with no bytes downloaded and an unknown total.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a received chunk and returns the progress status to emit.
    ///
    /// Accumulates `chunk_len` into the running total (saturating, so it never
    /// overflows) and latches the first reported `content_length` as the total.
    pub fn record(&mut self, chunk_len: usize, content_length: Option<u64>) -> UpdaterStatus {
        self.downloaded = self.downloaded.saturating_add(chunk_len as u64);
        if self.total.is_none() {
            self.total = content_length;
        }
        UpdaterStatus::downloading(self.downloaded, self.total)
    }

    /// Returns the single terminal completion status (24.3).
    pub fn finish(&self) -> UpdaterStatus {
        UpdaterStatus::done(self.downloaded, self.total)
    }
}

/// Receives the `updater:status` events the Updater would otherwise emit directly
/// on the Tauri event channel.
///
/// Implemented by the Command_Layer over a Tauri `AppHandle` (task 17.1) and by a
/// recording sink in tests, mirroring [`crate::services::ai::EventSink`]. Keeping
/// the Updater Tauri-free here makes the progress/event contract (24.3) testable
/// without a window or a live release server.
pub trait UpdaterEventSink: Send + Sync {
    /// Emits one `updater:status` event.
    fn emit_status(&self, status: &UpdaterStatus);
}

/// Drives a sequence of chunk `(len, content_length)` observations through a
/// fresh [`ProgressTracker`], emitting a progress status per chunk and a single
/// terminal completion through `sink` (24.3).
///
/// This is the exact event sequence [`download`] produces from the plugin's
/// download callbacks, extracted so the contract — ordered progress events with
/// `{ downloaded, total }` followed by exactly one terminal completion — is
/// unit-testable without the plugin.
pub fn drive_progress(chunks: &[(usize, Option<u64>)], sink: &dyn UpdaterEventSink) {
    let mut tracker = ProgressTracker::new();
    for (len, content_length) in chunks {
        let status = tracker.record(*len, *content_length);
        sink.emit_status(&status);
    }
    sink.emit_status(&tracker.finish());
}

// ===========================================================================
// Failure → AppError mapping (24.5, 24.7)
// ===========================================================================

/// The classes of failure the Updater maps to structured [`AppError`]s.
///
/// This intermediate enum decouples the failure → error-code policy from the
/// plugin's `#[non_exhaustive]` `Error` type (which cannot be constructed outside
/// its crate), so the mapping table — in particular that signature failures yield
/// `SIGNATURE` (24.5) — is unit-testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdaterErrorKind {
    /// Signature verification of the downloaded package failed (24.5).
    Signature,
    /// The check/download/install request exceeded its deadline.
    Timeout,
    /// The outbound request failed or the server returned an error.
    Network,
    /// The updater is misconfigured (no endpoints, insecure URL, bad URL).
    Validation,
    /// No release for the current target/platform was found.
    NotFound,
    /// A local file-system / extraction / install step failed.
    Io,
    /// An unexpected/unclassified failure.
    Internal,
}

impl UpdaterErrorKind {
    /// Builds the structured [`AppError`] for this failure class (24.7).
    pub fn to_app_error(self, message: impl Into<String>) -> AppError {
        let message = message.into();
        match self {
            UpdaterErrorKind::Signature => AppError::signature(message),
            UpdaterErrorKind::Timeout => AppError::timeout(message),
            UpdaterErrorKind::Network => AppError::network(message),
            UpdaterErrorKind::Validation => AppError::validation(message),
            UpdaterErrorKind::NotFound => AppError::not_found(message),
            UpdaterErrorKind::Io => AppError::io(message),
            UpdaterErrorKind::Internal => AppError::internal(message),
        }
    }
}

// ===========================================================================
// Live runtime integration (thin glue — requires the updater plugin)
// ===========================================================================

mod runtime {
    use super::*;
    use std::sync::Mutex;

    use tauri::AppHandle;
    use tauri_plugin_updater::{Error as PluginError, Update, UpdaterExt};

    /// Classifies a plugin [`PluginError`] into an [`UpdaterErrorKind`].
    ///
    /// Signature-related failures (minisign verification, signature base64/UTF-8
    /// decoding) classify as [`UpdaterErrorKind::Signature`] (24.5). The plugin's
    /// `Error` is `#[non_exhaustive]`, so a wildcard arm covers future variants
    /// and the Windows-only `Extract` variant.
    fn classify(error: &PluginError) -> UpdaterErrorKind {
        match error {
            PluginError::Minisign(_) | PluginError::Base64(_) | PluginError::SignatureUtf8(_) => {
                UpdaterErrorKind::Signature
            }
            PluginError::Reqwest(e) if e.is_timeout() => UpdaterErrorKind::Timeout,
            PluginError::Reqwest(_) | PluginError::Network(_) | PluginError::Http(_) => {
                UpdaterErrorKind::Network
            }
            PluginError::ReleaseNotFound => UpdaterErrorKind::Network,
            PluginError::TargetNotFound(_) | PluginError::TargetsNotFound(_) => {
                UpdaterErrorKind::NotFound
            }
            PluginError::EmptyEndpoints
            | PluginError::InsecureTransportProtocol
            | PluginError::UrlParse(_) => UpdaterErrorKind::Validation,
            PluginError::Io(_)
            | PluginError::FailedToDetermineExtractPath
            | PluginError::TempDirNotOnSameMountPoint
            | PluginError::TempDirNotFound
            | PluginError::BinaryNotFoundInArchive
            | PluginError::InvalidUpdaterFormat
            | PluginError::DebInstallFailed
            | PluginError::PackageInstallFailed
            | PluginError::AuthenticationFailed => UpdaterErrorKind::Io,
            _ => UpdaterErrorKind::Internal,
        }
    }

    /// Maps a plugin [`PluginError`] to a structured [`AppError`] (24.7).
    pub fn map_updater_err(context: &str, error: &PluginError) -> AppError {
        classify(error).to_app_error(format!("{context}: {error}"))
    }

    /// Queries the configured update endpoint with a 30-second timeout and reports
    /// availability + version (24.2).
    ///
    /// Returns the [`UpdateCheckResult`] for the Frontend together with the plugin
    /// [`Update`] handle (when available) so the Command_Layer can hand it to
    /// [`download`]/[`install`]. Any failure maps to a structured [`AppError`]
    /// and leaves the installed version unchanged (24.7).
    pub async fn check<R: tauri::Runtime>(
        app: &AppHandle<R>,
    ) -> Result<(UpdateCheckResult, Option<Update>), AppError> {
        let current_version = app.package_info().version.to_string();

        let updater = app
            .updater_builder()
            .timeout(CHECK_TIMEOUT)
            .build()
            .map_err(|e| map_updater_err("failed to build updater", &e))?;

        match updater.check().await {
            Ok(Some(update)) => {
                let result = build_check_result(
                    &current_version,
                    Some(DiscoveredUpdate {
                        version: update.version.clone(),
                        notes: update.body.clone(),
                    }),
                );
                Ok((result, Some(update)))
            }
            Ok(None) => Ok((build_check_result(&current_version, None), None)),
            Err(e) => Err(map_updater_err("update check failed", &e)),
        }
    }

    /// Streams the update package, emitting `updater:status` progress events with
    /// `{ downloaded, total }` followed by a terminal completion (24.3), and
    /// returns the verified package bytes.
    ///
    /// The plugin verifies the package's minisign signature at the end of the
    /// download; a verification failure surfaces as a `SIGNATURE` error (24.5).
    pub async fn download(
        update: &Update,
        sink: &dyn UpdaterEventSink,
    ) -> Result<Vec<u8>, AppError> {
        let tracker = Mutex::new(ProgressTracker::new());

        update
            .download(
                |chunk_len, content_length| {
                    let status = tracker.lock().unwrap().record(chunk_len, content_length);
                    sink.emit_status(&status);
                },
                || {
                    let status = tracker.lock().unwrap().finish();
                    sink.emit_status(&status);
                },
            )
            .await
            .map_err(|e| map_updater_err("update download failed", &e))
    }

    /// Installs the verified package; the plugin applies it on the next restart
    /// (24.4). A failure maps to a structured [`AppError`] and leaves the
    /// installed version unchanged (24.5, 24.7).
    pub fn install(update: &Update, bytes: &[u8]) -> Result<(), AppError> {
        update
            .install(bytes)
            .map_err(|e| map_updater_err("update install failed", &e))
    }
}

pub use runtime::{check, download, install};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // ---- platform + version (24.6) ----------------------------------------

    #[test]
    fn get_platform_returns_one_of_the_three_required_values() {
        let platform = get_platform();
        assert!(
            matches!(platform, "Windows" | "macOS" | "Linux"),
            "platform `{platform}` must be exactly one of Windows|macOS|Linux"
        );
    }

    #[test]
    fn get_platform_matches_the_compile_target() {
        let expected = if cfg!(target_os = "windows") {
            "Windows"
        } else if cfg!(target_os = "macos") {
            "macOS"
        } else {
            "Linux"
        };
        assert_eq!(get_platform(), expected);
    }

    #[test]
    fn get_version_is_the_package_version() {
        assert_eq!(get_version(), env!("CARGO_PKG_VERSION"));
        // Always a parseable semver so it round-trips through the comparator.
        assert!(parse_version(get_version()).is_ok());
    }

    // ---- version parsing + comparison (24.2) ------------------------------

    #[test]
    fn parse_version_strips_leading_v_prefix() {
        assert_eq!(
            parse_version("v1.2.3").unwrap(),
            parse_version("1.2.3").unwrap()
        );
        assert_eq!(
            parse_version("V1.2.3").unwrap(),
            parse_version("1.2.3").unwrap()
        );
    }

    #[test]
    fn parse_version_trims_whitespace() {
        assert_eq!(
            parse_version("  1.2.3  ").unwrap(),
            parse_version("1.2.3").unwrap()
        );
    }

    #[test]
    fn parse_version_rejects_malformed_input() {
        assert_eq!(
            parse_version("not-a-version").unwrap_err().code_str(),
            "VALIDATION"
        );
        assert_eq!(parse_version("1.2").unwrap_err().code_str(), "VALIDATION");
        assert_eq!(parse_version("").unwrap_err().code_str(), "VALIDATION");
    }

    #[test]
    fn is_newer_detects_a_higher_version() {
        assert!(is_newer("1.0.0", "1.0.1").unwrap());
        assert!(is_newer("1.0.0", "1.1.0").unwrap());
        assert!(is_newer("1.0.0", "2.0.0").unwrap());
        assert!(is_newer("v1.0.0", "v1.0.1").unwrap());
    }

    #[test]
    fn is_newer_is_false_for_same_or_older() {
        assert!(!is_newer("1.0.0", "1.0.0").unwrap());
        assert!(!is_newer("1.0.1", "1.0.0").unwrap());
        assert!(!is_newer("2.0.0", "1.9.9").unwrap());
    }

    #[test]
    fn is_newer_orders_prerelease_below_release() {
        // A pre-release is older than its release (SemVer precedence).
        assert!(is_newer("1.0.0-alpha", "1.0.0").unwrap());
        assert!(!is_newer("1.0.0", "1.0.0-alpha").unwrap());
        assert!(is_newer("1.0.0-alpha", "1.0.0-beta").unwrap());
    }

    #[test]
    fn is_newer_propagates_validation_for_bad_input() {
        assert_eq!(
            is_newer("oops", "1.0.0").unwrap_err().code_str(),
            "VALIDATION"
        );
        assert_eq!(
            is_newer("1.0.0", "oops").unwrap_err().code_str(),
            "VALIDATION"
        );
    }

    // ---- check result mapping (24.2) --------------------------------------

    #[test]
    fn build_check_result_reports_available_update() {
        let result = build_check_result(
            "1.0.0",
            Some(DiscoveredUpdate {
                version: "1.2.0".to_string(),
                notes: Some("bug fixes".to_string()),
            }),
        );
        assert_eq!(
            result,
            UpdateCheckResult {
                available: true,
                version: Some("1.2.0".to_string()),
                current_version: "1.0.0".to_string(),
                notes: Some("bug fixes".to_string()),
            }
        );
    }

    #[test]
    fn build_check_result_reports_no_update() {
        let result = build_check_result("1.0.0", None);
        assert_eq!(
            result,
            UpdateCheckResult {
                available: false,
                version: None,
                current_version: "1.0.0".to_string(),
                notes: None,
            }
        );
    }

    #[test]
    fn check_result_omits_absent_fields_on_the_wire() {
        let value = serde_json::to_value(build_check_result("1.0.0", None)).unwrap();
        assert_eq!(value.get("available").unwrap(), &serde_json::json!(false));
        assert_eq!(
            value.get("currentVersion").unwrap(),
            &serde_json::json!("1.0.0")
        );
        assert!(value.get("version").is_none());
        assert!(value.get("notes").is_none());
    }

    // ---- progress tracking + events (24.3) --------------------------------

    /// A recording [`UpdaterEventSink`] capturing the ordered emitted statuses.
    #[derive(Default)]
    struct RecordingSink {
        statuses: Mutex<Vec<UpdaterStatus>>,
    }

    impl UpdaterEventSink for RecordingSink {
        fn emit_status(&self, status: &UpdaterStatus) {
            self.statuses.lock().unwrap().push(status.clone());
        }
    }

    impl RecordingSink {
        fn statuses(&self) -> Vec<UpdaterStatus> {
            self.statuses.lock().unwrap().clone()
        }
    }

    #[test]
    fn progress_tracker_accumulates_downloaded_bytes() {
        let mut tracker = ProgressTracker::new();
        assert_eq!(tracker.record(100, Some(300)).downloaded, Some(100));
        assert_eq!(tracker.record(100, Some(300)).downloaded, Some(200));
        let last = tracker.record(100, Some(300));
        assert_eq!(last.downloaded, Some(300));
        assert_eq!(last.total, Some(300));
    }

    #[test]
    fn progress_tracker_latches_first_reported_total() {
        let mut tracker = ProgressTracker::new();
        assert_eq!(tracker.record(10, None).total, None);
        // A later content length is latched once known.
        assert_eq!(tracker.record(10, Some(50)).total, Some(50));
        // And is not overwritten by subsequent observations.
        assert_eq!(tracker.record(10, Some(999)).total, Some(50));
    }

    #[test]
    fn drive_progress_emits_ordered_chunks_then_one_completion() {
        let sink = RecordingSink::default();
        drive_progress(&[(50, Some(150)), (50, Some(150)), (50, Some(150))], &sink);

        let statuses = sink.statuses();
        assert_eq!(
            statuses,
            vec![
                UpdaterStatus {
                    phase: "downloading".into(),
                    downloaded: Some(50),
                    total: Some(150)
                },
                UpdaterStatus {
                    phase: "downloading".into(),
                    downloaded: Some(100),
                    total: Some(150)
                },
                UpdaterStatus {
                    phase: "downloading".into(),
                    downloaded: Some(150),
                    total: Some(150)
                },
                UpdaterStatus {
                    phase: "done".into(),
                    downloaded: Some(150),
                    total: Some(150)
                },
            ]
        );
    }

    #[test]
    fn drive_progress_terminates_with_exactly_one_completion() {
        let sink = RecordingSink::default();
        drive_progress(&[(10, None), (20, None)], &sink);

        let statuses = sink.statuses();
        let done = statuses.iter().filter(|s| s.phase == "done").count();
        assert_eq!(done, 1, "exactly one terminal completion event");
        assert_eq!(statuses.last().unwrap().phase, "done");
        // Cumulative bytes are reported even when the total is unknown.
        assert_eq!(statuses.last().unwrap().downloaded, Some(30));
        assert_eq!(statuses.last().unwrap().total, None);
    }

    #[test]
    fn empty_download_still_emits_a_single_completion() {
        let sink = RecordingSink::default();
        drive_progress(&[], &sink);

        let statuses = sink.statuses();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].phase, "done");
        assert_eq!(statuses[0].downloaded, Some(0));
    }

    #[test]
    fn updater_status_omits_unknown_total_on_the_wire() {
        let status = UpdaterStatus::downloading(42, None);
        let value = serde_json::to_value(&status).unwrap();
        assert_eq!(
            value.get("phase").unwrap(),
            &serde_json::json!("downloading")
        );
        assert_eq!(value.get("downloaded").unwrap(), &serde_json::json!(42));
        assert!(value.get("total").is_none());
    }

    // ---- failure → AppError mapping (24.5, 24.7) --------------------------

    #[test]
    fn signature_failure_maps_to_signature_code() {
        assert_eq!(
            UpdaterErrorKind::Signature
                .to_app_error("bad sig")
                .code_str(),
            "SIGNATURE"
        );
    }

    #[test]
    fn each_failure_kind_maps_to_its_stable_code() {
        let cases = [
            (UpdaterErrorKind::Signature, "SIGNATURE"),
            (UpdaterErrorKind::Timeout, "TIMEOUT"),
            (UpdaterErrorKind::Network, "NETWORK"),
            (UpdaterErrorKind::Validation, "VALIDATION"),
            (UpdaterErrorKind::NotFound, "NOT_FOUND"),
            (UpdaterErrorKind::Io, "IO"),
            (UpdaterErrorKind::Internal, "INTERNAL"),
        ];
        for (kind, expected) in cases {
            assert_eq!(kind.to_app_error("x").code_str(), expected);
        }
    }

    #[test]
    fn failure_mapping_preserves_the_message() {
        let err = UpdaterErrorKind::Network.to_app_error("update check failed: boom");
        assert_eq!(err.message, "update check failed: boom");
    }
}
