//! Sync_Service: remote backup/sync transports, local export, and upgrade
//! backups (Requirement 17).
//!
//! This module owns four responsibilities. Like the sibling services it is
//! written against *injected* dependencies (configuration values, explicit
//! directory `&Path`s, a borrowed cancellation flag) rather than reaching into
//! global [`crate::state::AppState`], so the rules are unit-testable without a
//! live window or a real remote server:
//!
//! 1. **WebDAV transport** ([`webdav_test`], [`webdav_upload`],
//!    [`webdav_download`], [`webdav_stat`], [`webdav_ensure_dir`]) — issued from
//!    Rust so browser cross-origin restrictions never apply (17.9). The
//!    connection test uses a 30-second timeout and returns an explicit pass/fail
//!    result (17.1).
//! 2. **S3 transport** ([`s3_test`], [`s3_upload`], [`s3_download`], [`s3_stat`])
//!    — built on the Sans-IO [`rusty_s3`] signer, with the signed request issued
//!    through `reqwest`. The connection test uses a 30-second timeout (17.3).
//! 3. **Local export** ([`export_zip`], [`selected_categories`]) — builds a ZIP
//!    of exactly the data categories named by the selected scope and returns the
//!    archive's absolute path (17.5); a cancellation produces no archive (17.11).
//! 4. **Upgrade backups** ([`backup_create`], [`backup_restore`],
//!    [`backup_list`], [`backup_delete`]) — snapshot/restore the data directory
//!    under the backup root. `create` returns `{ id, createdAt }` (17.6),
//!    `restore` reports restart-required (17.7), and an unknown backup id yields
//!    `NOT_FOUND` with the stored data/backups left unchanged (17.12).
//!
//! ## Configuration validation (security/robustness — 17.13)
//!
//! Every outbound entry point validates its configuration *first*, before any
//! client is built or any address is contacted. A malformed WebDAV or S3
//! configuration is rejected with `VALIDATION` and **no outbound request is
//! issued** (17.13, Property 37). [`validate_webdav_config`] and
//! [`validate_s3_config`] are pure (no I/O) and are the single gate the transport
//! functions call before doing anything else.
//!
//! ## Error handling (17.10)
//!
//! Transport and backup failures map to structured [`AppError`]s (`NETWORK`,
//! `TIMEOUT`, `NOT_FOUND`, `IO`, …). Connection *tests* never surface a transport
//! failure as an error: they return [`ConnectionTestResult`] with `success:
//! false` and a human-readable message so the Frontend can show an explicit
//! pass/fail (17.1, 17.3). Failures never delete or corrupt existing stored data
//! or backups (17.10, 17.12).
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use reqwest_dav::{Auth, ClientBuilder, DecodeError, Depth, Error as DavError};
use rusty_s3::{Bucket, Credentials, S3Action, UrlStyle};
use serde::{Deserialize, Serialize};
use url::Url;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::error::AppError;
use crate::state::RuntimePaths;
use crate::storage::time::{millis_to_iso8601, now_millis};

// ===========================================================================
// Timeouts and limits
// ===========================================================================

/// Connection-test request timeout for both WebDAV and S3 (17.1, 17.3).
const TEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Request timeout applied to data-transfer operations (upload/download/stat).
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(300);
/// Validity window of a presigned S3 URL. Independent of the request timeout; it
/// only needs to outlast the time between signing and issuing the request.
const PRESIGN_EXPIRY: Duration = Duration::from_secs(15 * 60);

// ===========================================================================
// Shared result DTOs
// ===========================================================================

/// Explicit pass/fail outcome of a connection test (17.1, 17.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    /// Whether the connection attempt succeeded.
    pub success: bool,
    /// Human-readable description of the outcome.
    pub message: String,
}

impl ConnectionTestResult {
    fn ok() -> Self {
        Self {
            success: true,
            message: "Connection successful".to_string(),
        }
    }

    fn fail(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
        }
    }
}

/// Result of a `stat` request: whether the remote object exists and, when known,
/// its last-modified timestamp as an ISO_8601 string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatResult {
    /// Whether the remote object exists.
    pub exists: bool,
    /// Last-modified timestamp (ISO_8601, UTC), when reported by the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
}

// ===========================================================================
// WebDAV configuration (17.13)
// ===========================================================================

/// WebDAV server configuration supplied by the Frontend.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavConfig {
    /// Base URL of the WebDAV collection (must be http/https).
    pub url: String,
    /// Username for HTTP Basic auth (empty for anonymous access).
    #[serde(default)]
    pub username: String,
    /// Password for HTTP Basic auth.
    #[serde(default)]
    pub password: String,
}

/// Validates a [`WebDavConfig`] and returns its parsed base URL, or `VALIDATION`
/// (17.13). Pure: performs no network I/O, so a rejected config issues no request.
pub fn validate_webdav_config(config: &WebDavConfig) -> Result<Url, AppError> {
    let trimmed = config.url.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("WebDAV URL must not be empty"));
    }
    let url = Url::parse(trimmed)
        .map_err(|e| AppError::validation(format!("invalid WebDAV URL `{trimmed}`: {e}")))?;
    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(AppError::validation(format!(
                "WebDAV URL scheme `{other}` is not supported (expected http or https)"
            )))
        }
    }
    if url.host_str().is_none() {
        return Err(AppError::validation("WebDAV URL has no host"));
    }
    Ok(url)
}

/// Builds a `reqwest_dav` client for `config` with the given request timeout.
///
/// The configuration must already have been validated by
/// [`validate_webdav_config`]; this only constructs the transport after the
/// shared SSRF pin.
async fn build_dav_client(
    config: &WebDavConfig,
    timeout: Duration,
    allow_private_network: bool,
) -> Result<reqwest_dav::Client, AppError> {
    let (_url, agent) = crate::services::network_safety::prepare_public_url(
        config.url.trim(),
        timeout,
        allow_private_network,
    )
    .await?;

    let auth = if config.username.is_empty() && config.password.is_empty() {
        Auth::Anonymous
    } else {
        Auth::Basic(config.username.clone(), config.password.clone())
    };

    ClientBuilder::new()
        .set_agent(agent)
        .set_host(config.url.trim_end_matches('/').to_string())
        .set_auth(auth)
        .build()
        .map_err(|e| AppError::network(format!("failed to build WebDAV client: {e}")))
}

/// Returns the HTTP status code carried by a `reqwest_dav` server error, if any.
fn dav_status_code(error: &DavError) -> Option<u16> {
    match error {
        DavError::Decode(DecodeError::Server(server)) => Some(server.response_code),
        _ => None,
    }
}

/// Maps a `reqwest_dav` error to a structured [`AppError`] (17.10).
fn map_dav_err(context: &str, error: DavError) -> AppError {
    if let DavError::Reqwest(ref e) = error {
        if e.is_timeout() {
            return AppError::timeout(format!("{context}: request timed out"));
        }
    }
    if let Some(code) = dav_status_code(&error) {
        if code == 404 {
            return AppError::not_found(format!("{context}: remote path not found"));
        }
        return AppError::network(format!("{context}: server returned HTTP {code}"));
    }
    AppError::network(format!("{context}: {error}"))
}

/// Tests a WebDAV connection with a 30-second timeout, returning an explicit
/// pass/fail result (17.1).
///
/// A malformed configuration is rejected with `VALIDATION` and issues no request
/// (17.13). Network, auth, and server failures are reported as `success: false`
/// rather than as an error, so the Frontend always receives a definite outcome.
pub async fn webdav_test(
    config: &WebDavConfig,
    allow_private_network: bool,
) -> Result<ConnectionTestResult, AppError> {
    validate_webdav_config(config)?;
    let client = build_dav_client(config, TEST_TIMEOUT, allow_private_network).await?;

    match client.list("", Depth::Number(0)).await {
        Ok(_) => Ok(ConnectionTestResult::ok()),
        Err(e) => match dav_status_code(&e) {
            Some(401) | Some(403) => Ok(ConnectionTestResult::fail(
                "Authentication failed, please check username and password",
            )),
            Some(code) => Ok(ConnectionTestResult::fail(format!(
                "Connection failed: server returned HTTP {code}"
            ))),
            None => Ok(ConnectionTestResult::fail(format!(
                "Connection failed: {e}"
            ))),
        },
    }
}

/// Uploads `data` to the WebDAV server at `remote_path` (17.2).
///
/// Rejects malformed config with `VALIDATION` before any request (17.13).
pub async fn webdav_upload(
    config: &WebDavConfig,
    remote_path: &str,
    data: Vec<u8>,
    allow_private_network: bool,
) -> Result<(), AppError> {
    validate_webdav_config(config)?;
    let client = build_dav_client(config, TRANSFER_TIMEOUT, allow_private_network).await?;
    client
        .put(remote_path, data)
        .await
        .map_err(|e| map_dav_err("WebDAV upload failed", e))
}

/// Downloads the WebDAV object at `remote_path`, returning its bytes (17.2).
///
/// Returns `NOT_FOUND` when the remote object does not exist, and rejects
/// malformed config with `VALIDATION` before any request (17.13).
pub async fn webdav_download(
    config: &WebDavConfig,
    remote_path: &str,
    allow_private_network: bool,
) -> Result<Vec<u8>, AppError> {
    validate_webdav_config(config)?;
    let client = build_dav_client(config, TRANSFER_TIMEOUT, allow_private_network).await?;
    let response = client
        .get(remote_path)
        .await
        .map_err(|e| map_dav_err("WebDAV download failed", e))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|e| AppError::network(format!("WebDAV download failed: {e}")))?;
    Ok(bytes.to_vec())
}

/// Retrieves metadata for the WebDAV object at `remote_path` (17.2).
///
/// Returns `{ exists: false }` when the object is absent, and rejects malformed
/// config with `VALIDATION` before any request (17.13).
pub async fn webdav_stat(
    config: &WebDavConfig,
    remote_path: &str,
    allow_private_network: bool,
) -> Result<StatResult, AppError> {
    validate_webdav_config(config)?;
    let client = build_dav_client(config, TRANSFER_TIMEOUT, allow_private_network).await?;
    match client.list(remote_path, Depth::Number(0)).await {
        Ok(entities) => {
            let last_modified = entities.first().map(|entity| match entity {
                reqwest_dav::list_cmd::ListEntity::File(file) => {
                    millis_to_iso8601(file.last_modified.timestamp_millis())
                }
                reqwest_dav::list_cmd::ListEntity::Folder(folder) => {
                    millis_to_iso8601(folder.last_modified.timestamp_millis())
                }
            });
            Ok(StatResult {
                exists: true,
                last_modified,
            })
        }
        Err(e) => match dav_status_code(&e) {
            Some(404) => Ok(StatResult {
                exists: false,
                last_modified: None,
            }),
            _ => Err(map_dav_err("WebDAV stat failed", e)),
        },
    }
}

/// Ensures the WebDAV collection at `remote_path` exists, creating it when absent
/// (17.2).
///
/// Rejects malformed config with `VALIDATION` before any request (17.13).
pub async fn webdav_ensure_dir(
    config: &WebDavConfig,
    remote_path: &str,
    allow_private_network: bool,
) -> Result<(), AppError> {
    validate_webdav_config(config)?;
    let client = build_dav_client(config, TRANSFER_TIMEOUT, allow_private_network).await?;

    match client.list(remote_path, Depth::Number(0)).await {
        Ok(_) => Ok(()), // Already exists.
        Err(e) => match dav_status_code(&e) {
            Some(404) => client
                .mkcol(remote_path)
                .await
                .map_err(|e| map_dav_err("WebDAV directory creation failed", e)),
            _ => Err(map_dav_err("WebDAV directory check failed", e)),
        },
    }
}

// ===========================================================================
// S3 configuration (17.13)
// ===========================================================================

/// S3 (or S3-compatible) bucket configuration supplied by the Frontend.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Config {
    /// Service endpoint URL (must be http/https), without the bucket name.
    pub endpoint: String,
    /// Region identifier used for request signing.
    pub region: String,
    /// Bucket name.
    pub bucket: String,
    /// Access key id.
    pub access_key_id: String,
    /// Secret access key.
    pub secret_access_key: String,
}

/// Validates an [`S3Config`] and returns the constructed [`Bucket`], or
/// `VALIDATION` (17.13). Pure: performs no network I/O, so a rejected config
/// issues no request. Path-style addressing is used to maximize compatibility
/// with S3-compatible servers (e.g. MinIO).
pub fn validate_s3_config(config: &S3Config) -> Result<Bucket, AppError> {
    let endpoint = config.endpoint.trim();
    if endpoint.is_empty() {
        return Err(AppError::validation("S3 endpoint must not be empty"));
    }
    if config.region.trim().is_empty() {
        return Err(AppError::validation("S3 region must not be empty"));
    }
    if config.bucket.trim().is_empty() {
        return Err(AppError::validation("S3 bucket must not be empty"));
    }
    if config.access_key_id.trim().is_empty() {
        return Err(AppError::validation("S3 access key id must not be empty"));
    }
    if config.secret_access_key.trim().is_empty() {
        return Err(AppError::validation(
            "S3 secret access key must not be empty",
        ));
    }

    let endpoint_url = Url::parse(endpoint)
        .map_err(|e| AppError::validation(format!("invalid S3 endpoint `{endpoint}`: {e}")))?;
    match endpoint_url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(AppError::validation(format!(
                "S3 endpoint scheme `{other}` is not supported (expected http or https)"
            )))
        }
    }

    Bucket::new(
        endpoint_url,
        UrlStyle::Path,
        config.bucket.trim().to_string(),
        config.region.trim().to_string(),
    )
    .map_err(|e| AppError::validation(format!("invalid S3 bucket configuration: {e}")))
}

/// Returns the [`Credentials`] for `config` (call only after validation).
fn s3_credentials(config: &S3Config) -> Credentials {
    Credentials::new(
        config.access_key_id.trim().to_string(),
        config.secret_access_key.trim().to_string(),
    )
}

/// Builds a DNS-pinned `reqwest` client for S3 requests with the given timeout.
async fn build_s3_agent(
    endpoint: &str,
    timeout: Duration,
    allow_private_network: bool,
) -> Result<reqwest::Client, AppError> {
    let (_url, client) = crate::services::network_safety::prepare_public_url(
        endpoint,
        timeout,
        allow_private_network,
    )
    .await?;
    Ok(client)
}

/// Normalizes an object key by stripping leading slashes (mirrors the reference).
fn normalize_key(key: &str) -> &str {
    key.trim_start_matches('/')
}

/// Maps a `reqwest` transport error to a structured [`AppError`] (17.10).
fn map_reqwest_err(context: &str, e: reqwest::Error) -> AppError {
    if e.is_timeout() {
        AppError::timeout(format!("{context}: request timed out"))
    } else {
        AppError::network(format!("{context}: {e}"))
    }
}

/// Tests an S3 connection with a 30-second timeout, returning an explicit
/// pass/fail result (17.3).
///
/// A malformed configuration is rejected with `VALIDATION` and issues no request
/// (17.13). Transport/auth failures are reported as `success: false`.
pub async fn s3_test(
    config: &S3Config,
    allow_private_network: bool,
) -> Result<ConnectionTestResult, AppError> {
    let bucket = validate_s3_config(config)?;
    let credentials = s3_credentials(config);
    let agent = build_s3_agent(&config.endpoint, TEST_TIMEOUT, allow_private_network).await?;

    let signed = bucket.head_bucket(Some(&credentials)).sign(PRESIGN_EXPIRY);

    match agent.head(signed).send().await {
        Ok(response) if response.status().is_success() => Ok(ConnectionTestResult::ok()),
        Ok(response) => {
            let code = response.status().as_u16();
            if code == 401 || code == 403 {
                Ok(ConnectionTestResult::fail(
                    "Authentication failed, please check the access key and secret",
                ))
            } else {
                Ok(ConnectionTestResult::fail(format!(
                    "Connection failed: server returned HTTP {code}"
                )))
            }
        }
        Err(e) if e.is_timeout() => Ok(ConnectionTestResult::fail(
            "Connection failed: request timed out",
        )),
        Err(e) => Ok(ConnectionTestResult::fail(format!(
            "Connection failed: {e}"
        ))),
    }
}

/// Uploads `data` to the S3 object `key` (17.4).
///
/// Rejects malformed config with `VALIDATION` before any request (17.13).
pub async fn s3_upload(
    config: &S3Config,
    key: &str,
    data: Vec<u8>,
    allow_private_network: bool,
) -> Result<(), AppError> {
    let bucket = validate_s3_config(config)?;
    let credentials = s3_credentials(config);
    let agent = build_s3_agent(&config.endpoint, TRANSFER_TIMEOUT, allow_private_network).await?;

    let signed = bucket
        .put_object(Some(&credentials), normalize_key(key))
        .sign(PRESIGN_EXPIRY);

    let response = agent
        .put(signed)
        .body(data)
        .send()
        .await
        .map_err(|e| map_reqwest_err("S3 upload failed", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(AppError::network(format!(
            "S3 upload failed: server returned HTTP {}",
            response.status().as_u16()
        )))
    }
}

/// Downloads the S3 object `key`, returning its bytes (17.4).
///
/// Returns `NOT_FOUND` when the object does not exist, and rejects malformed
/// config with `VALIDATION` before any request (17.13).
pub async fn s3_download(
    config: &S3Config,
    key: &str,
    allow_private_network: bool,
) -> Result<Vec<u8>, AppError> {
    let bucket = validate_s3_config(config)?;
    let credentials = s3_credentials(config);
    let agent = build_s3_agent(&config.endpoint, TRANSFER_TIMEOUT, allow_private_network).await?;

    let signed = bucket
        .get_object(Some(&credentials), normalize_key(key))
        .sign(PRESIGN_EXPIRY);

    let response = agent
        .get(signed)
        .send()
        .await
        .map_err(|e| map_reqwest_err("S3 download failed", e))?;

    let status = response.status();
    if status.as_u16() == 404 {
        return Err(AppError::not_found(format!("S3 object `{key}` not found")));
    }
    if !status.is_success() {
        return Err(AppError::network(format!(
            "S3 download failed: server returned HTTP {}",
            status.as_u16()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| map_reqwest_err("S3 download failed", e))?;
    Ok(bytes.to_vec())
}

/// Retrieves metadata for the S3 object `key` (17.4).
///
/// Returns `{ exists: false }` when the object is absent, and rejects malformed
/// config with `VALIDATION` before any request (17.13).
pub async fn s3_stat(
    config: &S3Config,
    key: &str,
    allow_private_network: bool,
) -> Result<StatResult, AppError> {
    let bucket = validate_s3_config(config)?;
    let credentials = s3_credentials(config);
    let agent = build_s3_agent(&config.endpoint, TRANSFER_TIMEOUT, allow_private_network).await?;

    let signed = bucket
        .head_object(Some(&credentials), normalize_key(key))
        .sign(PRESIGN_EXPIRY);

    let response = agent
        .head(signed)
        .send()
        .await
        .map_err(|e| map_reqwest_err("S3 stat failed", e))?;

    let status = response.status();
    if status.as_u16() == 404 {
        return Ok(StatResult {
            exists: false,
            last_modified: None,
        });
    }
    if !status.is_success() {
        return Err(AppError::network(format!(
            "S3 stat failed: server returned HTTP {}",
            status.as_u16()
        )));
    }

    let last_modified = response
        .headers()
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    Ok(StatResult {
        exists: true,
        last_modified,
    })
}

// ===========================================================================
// Local export (17.5, 17.11)
// ===========================================================================

/// The set of data categories an export may include (17.5).
///
/// Each selected flag maps to one runtime directory (see
/// [`selected_categories`]) whose contents are placed under a fixed prefix in
/// the produced archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportScope {
    /// Structured application data (the SQLite database directory).
    pub data: bool,
    /// Saved image/video media.
    pub media: bool,
    /// Platform rule files.
    pub rule: bool,
}

/// Outcome of an [`export_zip`] request (17.5, 17.11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    /// Whether the export was cancelled before an archive was produced.
    pub canceled: bool,
    /// Absolute path of the produced archive; `None` when cancelled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

/// Resolves an [`ExportScope`] to the ordered list of `(archive_prefix,
/// source_directory)` pairs it selects (17.5).
///
/// Pure and deterministic: the archive prefix for each category is fixed, so the
/// set of prefixes in a produced archive corresponds exactly to the selected
/// scope flags (Property 36).
pub fn selected_categories<'a>(
    scope: &ExportScope,
    paths: &'a RuntimePaths,
) -> Vec<(&'static str, &'a Path)> {
    let mut categories: Vec<(&'static str, &Path)> = Vec::new();
    if scope.data {
        categories.push(("data", paths.data.as_path()));
    }
    if scope.media {
        categories.push(("media", paths.media.as_path()));
    }
    if scope.rule {
        categories.push(("rule", paths.rule.as_path()));
    }
    categories
}

/// Collects every file under `base` into `out` as `(archive_name, source_path)`,
/// where `archive_name` is `{prefix}/{relative path}` using `/` separators.
fn collect_export_files(
    base: &Path,
    prefix: &str,
    dir: &Path,
    out: &mut Vec<(String, PathBuf)>,
) -> Result<(), AppError> {
    let entries = fs::read_dir(dir)
        .map_err(|e| AppError::io(format!("failed to read export directory: {e}")))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| AppError::io(format!("failed to read directory entry: {e}")))?;
        let file_type = entry
            .file_type()
            .map_err(|e| AppError::io(format!("failed to determine entry type: {e}")))?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_export_files(base, prefix, &path, out)?;
        } else if file_type.is_file() {
            let relative = path.strip_prefix(base).unwrap_or(&path);
            let rel_slash = relative
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect::<Vec<_>>()
                .join("/");
            out.push((format!("{prefix}/{rel_slash}"), path));
        }
    }
    Ok(())
}

/// Builds a ZIP archive of the selected export scope at `dest` and returns its
/// absolute path (17.5).
///
/// The archive contains exactly the categories named by `scope` (Property 36),
/// each under its fixed prefix. A category whose source directory does not exist
/// contributes no entries. If `cancel` is set before the archive file is created,
/// the export stops and **no archive is produced** (17.11): the function checks
/// the flag before any output file exists and again before each entry is written,
/// removing the in-progress temp file on cancellation.
pub fn export_zip(
    paths: &RuntimePaths,
    scope: &ExportScope,
    dest: &Path,
    cancel: &AtomicBool,
) -> Result<ExportResult, AppError> {
    // Gather the file list first (a cheap directory walk). No output file exists
    // yet, so an early cancellation cannot leave a partial archive (17.11).
    if cancel.load(Ordering::SeqCst) {
        return Ok(ExportResult {
            canceled: true,
            file_path: None,
        });
    }

    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    for (prefix, dir) in selected_categories(scope, paths) {
        if dir.is_dir() {
            collect_export_files(dir, prefix, dir, &mut entries)?;
        }
    }

    if cancel.load(Ordering::SeqCst) {
        return Ok(ExportResult {
            canceled: true,
            file_path: None,
        });
    }

    // Write to a temp file, then rename onto `dest` once complete, so a partial
    // archive is never observable at `dest`.
    let tmp = dest.with_extension("zip.partial");
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::io(format!("failed to create export directory: {e}")))?;
    }

    let file = fs::File::create(&tmp)
        .map_err(|e| AppError::io(format!("failed to create export archive: {e}")))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for (name, source) in &entries {
        if cancel.load(Ordering::SeqCst) {
            // Abandon the in-progress archive so no partial file remains (17.11).
            drop(zip);
            let _ = fs::remove_file(&tmp);
            return Ok(ExportResult {
                canceled: true,
                file_path: None,
            });
        }
        zip.start_file(name.clone(), options)
            .map_err(|e| AppError::io(format!("failed to add `{name}` to archive: {e}")))?;
        let mut input = fs::File::open(source)
            .map_err(|e| AppError::io(format!("failed to read `{name}` for export: {e}")))?;
        std::io::copy(&mut input, &mut zip)
            .map_err(|e| AppError::io(format!("failed to write `{name}` to archive: {e}")))?;
    }

    zip.finish()
        .map_err(|e| AppError::io(format!("failed to finalize export archive: {e}")))?;

    fs::rename(&tmp, dest)
        .map_err(|e| AppError::io(format!("failed to finalize export archive: {e}")))?;

    let absolute = fs::canonicalize(dest).unwrap_or_else(|_| dest.to_path_buf());
    Ok(ExportResult {
        canceled: false,
        file_path: Some(absolute.to_string_lossy().to_string()),
    })
}

// ===========================================================================
// Upgrade backups (17.6, 17.7, 17.8, 17.10, 17.12)
// ===========================================================================

/// File name of the per-backup manifest stored alongside the snapshot.
const BACKUP_MANIFEST_FILE: &str = "backup-manifest.json";

/// A backup entry: its stable identifier and ISO_8601 creation timestamp (17.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    /// Stable identifier (also the snapshot directory name).
    pub id: String,
    /// Creation timestamp as an ISO_8601 string (`YYYY-MM-DDTHH:mm:ss.sssZ`).
    pub created_at: String,
}

/// Result of a restore request: whether the app must restart (17.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    /// The restored backup's identifier.
    pub id: String,
    /// Whether a restart is required to complete the restore (always `true`).
    pub restart_required: bool,
}

/// Returns `true` when `id` is a safe backup identifier: non-empty and composed
/// only of ASCII alphanumerics, `-`, or `_`.
///
/// This is the single gate that prevents a malicious id from escaping the backup
/// root (no path separators, no `.`), so `restore`/`delete` of an unknown or
/// malformed id is rejected as `NOT_FOUND` without touching the filesystem
/// outside the backup root (17.12).
fn is_valid_backup_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Recursively copies the contents of `src` into `dest`, creating `dest` and any
/// missing parents. Symlinks are not followed.
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
        // Symlinks and other special entries are skipped.
    }
    Ok(())
}

/// Reads and parses the manifest for the backup at `backup_dir`.
fn read_backup_manifest(backup_dir: &Path) -> Option<BackupEntry> {
    let manifest_path = backup_dir.join(BACKUP_MANIFEST_FILE);
    let raw = fs::read_to_string(manifest_path).ok()?;
    serde_json::from_str::<BackupEntry>(&raw).ok()
}

/// Creates an upgrade backup: snapshots `data_dir` under `backup_root` and
/// returns the new entry's id and ISO_8601 creation timestamp (17.6).
///
/// The snapshot directory (`{backup_root}/{id}`) holds a copy of the data
/// directory's contents plus a manifest. Existing backups are left unchanged.
pub fn backup_create(data_dir: &Path, backup_root: &Path) -> Result<BackupEntry, AppError> {
    let now = now_millis();
    let created_at = millis_to_iso8601(now);
    // Timestamp-prefixed id sorts chronologically; the uuid suffix guarantees
    // uniqueness even for two backups created within the same millisecond.
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let id = format!("backup-{now}-{}", &suffix[..8]);

    let backup_dir = backup_root.join(&id);
    if backup_dir.exists() {
        return Err(AppError::conflict(format!("backup `{id}` already exists")));
    }

    let data_snapshot = backup_dir.join("data");
    if data_dir.is_dir() {
        copy_dir_recursive(data_dir, &data_snapshot)?;
    } else {
        // No data directory yet: still produce a well-formed (empty) snapshot.
        fs::create_dir_all(&data_snapshot)
            .map_err(|e| AppError::io(format!("failed to create backup directory: {e}")))?;
    }

    let entry = BackupEntry {
        id: id.clone(),
        created_at,
    };
    let manifest = serde_json::to_string_pretty(&entry)
        .map_err(|e| AppError::internal(format!("failed to encode backup manifest: {e}")))?;
    fs::write(backup_dir.join(BACKUP_MANIFEST_FILE), manifest)
        .map_err(|e| AppError::io(format!("failed to write backup manifest: {e}")))?;

    Ok(entry)
}

/// Lists upgrade backups under `backup_root`, most-recent first (17.8).
///
/// Directories without a readable manifest are ignored. Returns an empty list
/// (never an error) when the backup root does not exist.
pub fn backup_list(backup_root: &Path) -> Result<Vec<BackupEntry>, AppError> {
    if !backup_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let dir = fs::read_dir(backup_root)
        .map_err(|e| AppError::io(format!("failed to read backup directory: {e}")))?;
    for item in dir {
        let item = item.map_err(|e| AppError::io(format!("failed to read backup entry: {e}")))?;
        if item.path().is_dir() {
            if let Some(entry) = read_backup_manifest(&item.path()) {
                entries.push(entry);
            }
        }
    }

    // ISO_8601 strings sort chronologically; reverse for most-recent first, with
    // the id as a stable tiebreaker.
    entries.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(entries)
}

/// Deletes the upgrade backup `id` under `backup_root` (17.8).
///
/// Returns `NOT_FOUND` for a malformed or unknown id, leaving all stored data and
/// backups unchanged (17.12).
pub fn backup_delete(backup_root: &Path, id: &str) -> Result<(), AppError> {
    if !is_valid_backup_id(id) {
        return Err(AppError::not_found(format!("backup `{id}` not found")));
    }
    let backup_dir = backup_root.join(id);
    if !backup_dir.is_dir() {
        return Err(AppError::not_found(format!("backup `{id}` not found")));
    }
    fs::remove_dir_all(&backup_dir)
        .map_err(|e| AppError::io(format!("failed to delete backup `{id}`: {e}")))
}

/// Restores the upgrade backup `id` into `data_dir` and reports restart-required
/// (17.7).
///
/// Returns `NOT_FOUND` for a malformed or unknown id, leaving stored data and
/// backups unchanged (17.12). On success the data directory's contents are
/// replaced with the snapshot's and `restart_required` is `true`.
pub fn backup_restore(
    data_dir: &Path,
    backup_root: &Path,
    id: &str,
) -> Result<RestoreResult, AppError> {
    if !is_valid_backup_id(id) {
        return Err(AppError::not_found(format!("backup `{id}` not found")));
    }
    let backup_dir = backup_root.join(id);
    if !backup_dir.is_dir() {
        return Err(AppError::not_found(format!("backup `{id}` not found")));
    }
    let data_snapshot = backup_dir.join("data");
    if !data_snapshot.is_dir() {
        return Err(AppError::not_found(format!(
            "backup `{id}` has no data snapshot"
        )));
    }

    // Replace the data directory's contents with the snapshot.
    if data_dir.exists() {
        fs::remove_dir_all(data_dir)
            .map_err(|e| AppError::io(format!("failed to clear data directory: {e}")))?;
    }
    copy_dir_recursive(&data_snapshot, data_dir)?;

    Ok(RestoreResult {
        id: id.to_string(),
        restart_required: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use std::io::Read;
    use tempfile::TempDir;

    // --- WebDAV config validation (17.13) ---------------------------------

    #[test]
    fn webdav_config_accepts_valid_https_url() {
        let config = WebDavConfig {
            url: "https://dav.example.com/remote.php/dav/".into(),
            username: "u".into(),
            password: "p".into(),
        };
        let url = validate_webdav_config(&config).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("dav.example.com"));
    }

    #[test]
    fn webdav_config_rejects_empty_url() {
        let err = validate_webdav_config(&WebDavConfig::default()).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn webdav_config_rejects_non_http_scheme() {
        let config = WebDavConfig {
            url: "ftp://dav.example.com/".into(),
            ..Default::default()
        };
        let err = validate_webdav_config(&config).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn webdav_config_rejects_unparseable_url() {
        let config = WebDavConfig {
            url: "not a url".into(),
            ..Default::default()
        };
        let err = validate_webdav_config(&config).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    /// A malformed config must be rejected before any outbound request (17.13).
    /// `webdav_test` is async but the validation gate returns synchronously, so a
    /// rejected config never reaches the transport; we confirm the VALIDATION
    /// error without a server.
    #[test]
    fn webdav_test_rejects_malformed_config_without_request() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let err = rt
            .block_on(webdav_test(&WebDavConfig::default(), false))
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    // --- S3 config validation (17.13) -------------------------------------

    fn valid_s3_config() -> S3Config {
        S3Config {
            endpoint: "https://s3.example.com".into(),
            region: "us-east-1".into(),
            bucket: "my-bucket".into(),
            access_key_id: "AKIA".into(),
            secret_access_key: "secret".into(),
        }
    }

    #[test]
    fn s3_config_accepts_valid_config() {
        let bucket = validate_s3_config(&valid_s3_config()).unwrap();
        assert_eq!(bucket.name(), "my-bucket");
        assert_eq!(bucket.region(), "us-east-1");
    }

    #[test]
    fn s3_config_rejects_each_missing_field() {
        for mutate in [
            (|c: &mut S3Config| c.endpoint.clear()) as fn(&mut S3Config),
            |c: &mut S3Config| c.region.clear(),
            |c: &mut S3Config| c.bucket.clear(),
            |c: &mut S3Config| c.access_key_id.clear(),
            |c: &mut S3Config| c.secret_access_key.clear(),
        ] {
            let mut config = valid_s3_config();
            mutate(&mut config);
            let err = validate_s3_config(&config).unwrap_err();
            assert_eq!(err.code, ErrorCode::Validation);
        }
    }

    #[test]
    fn s3_config_rejects_non_http_endpoint() {
        let mut config = valid_s3_config();
        config.endpoint = "ftp://s3.example.com".into();
        let err = validate_s3_config(&config).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn s3_test_rejects_malformed_config_without_request() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let err = rt
            .block_on(s3_test(&S3Config::default(), false))
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[tokio::test]
    async fn webdav_test_blocks_localhost_when_private_network_disallowed() {
        let err = webdav_test(
            &WebDavConfig {
                url: "http://localhost/".into(),
                ..Default::default()
            },
            false,
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::SsrfBlocked);
    }

    #[tokio::test]
    async fn webdav_test_allows_localhost_attempt_when_private_network_enabled() {
        let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = closed.local_addr().unwrap().port();
        drop(closed);

        let result = webdav_test(
            &WebDavConfig {
                url: format!("http://localhost:{port}/"),
                ..Default::default()
            },
            true,
        )
        .await
        .unwrap();
        assert!(
            !result.success,
            "private URL must be attempted after policy: {result:?}"
        );
    }

    // --- Export scope mapping + archive (17.5, Property 36) ----------------

    /// Builds a `RuntimePaths` rooted under `base`, creating each directory with
    /// one marker file so the export has content to collect.
    fn populated_paths(base: &Path) -> RuntimePaths {
        let paths = RuntimePaths {
            data: base.join("data"),
            media: base.join("media"),
            rule: base.join("rule"),
            backup: base.join("backup"),
            log: base.join("log"),
        };
        for dir in [&paths.data, &paths.media, &paths.rule] {
            fs::create_dir_all(dir).unwrap();
            fs::write(dir.join("marker.txt"), "x").unwrap();
        }
        paths
    }

    #[test]
    fn selected_categories_maps_flags_to_prefixes() {
        let tmp = TempDir::new().unwrap();
        let paths = populated_paths(tmp.path());
        let scope = ExportScope {
            data: true,
            media: false,
            rule: false,
        };
        let categories = selected_categories(&scope, &paths);
        let prefixes: Vec<&str> = categories.iter().map(|(p, _)| *p).collect();
        assert_eq!(prefixes, vec!["data"]);
    }

    /// Reads the top-level category prefixes present in a produced archive.
    fn archive_prefixes(zip_path: &Path) -> Vec<String> {
        let file = fs::File::open(zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut prefixes = std::collections::BTreeSet::new();
        for i in 0..archive.len() {
            let entry = archive.by_index(i).unwrap();
            let name = entry.name().to_string();
            if let Some((prefix, _)) = name.split_once('/') {
                prefixes.insert(prefix.to_string());
            }
        }
        prefixes.into_iter().collect()
    }

    #[test]
    fn export_zip_contains_exactly_selected_scope() {
        let tmp = TempDir::new().unwrap();
        let paths = populated_paths(tmp.path());
        let dest = tmp.path().join("export.zip");
        let scope = ExportScope {
            data: true,
            media: true,
            rule: false,
        };
        let cancel = AtomicBool::new(false);

        let result = export_zip(&paths, &scope, &dest, &cancel).unwrap();
        assert!(!result.canceled);
        let file_path = result.file_path.expect("archive path");
        assert!(Path::new(&file_path).is_absolute());
        assert!(dest.is_file());

        // Exactly the selected categories appear, no more, no fewer (Property 36).
        assert_eq!(archive_prefixes(&dest), vec!["data", "media"]);
    }

    #[test]
    fn export_zip_empty_scope_produces_empty_archive() {
        let tmp = TempDir::new().unwrap();
        let paths = populated_paths(tmp.path());
        let dest = tmp.path().join("empty.zip");
        let cancel = AtomicBool::new(false);

        let result = export_zip(&paths, &ExportScope::default(), &dest, &cancel).unwrap();
        assert!(!result.canceled);
        assert!(result.file_path.is_some());
        assert!(archive_prefixes(&dest).is_empty());
    }

    #[test]
    fn export_zip_cancel_produces_no_archive() {
        let tmp = TempDir::new().unwrap();
        let paths = populated_paths(tmp.path());
        let dest = tmp.path().join("canceled.zip");
        let scope = ExportScope {
            data: true,
            media: true,
            rule: true,
        };
        // Pre-cancelled: the export must stop and produce nothing (17.11).
        let cancel = AtomicBool::new(true);

        let result = export_zip(&paths, &scope, &dest, &cancel).unwrap();
        assert!(result.canceled);
        assert!(result.file_path.is_none());
        assert!(!dest.exists(), "no archive must be produced on cancel");
        assert!(
            !dest.with_extension("zip.partial").exists(),
            "no partial archive must remain"
        );
    }

    // --- Upgrade backups (17.6, 17.7, 17.8, 17.12) ------------------------

    /// Creates a data directory with one file and returns `(data_dir,
    /// backup_root)` rooted under `base`.
    fn backup_dirs(base: &Path) -> (PathBuf, PathBuf) {
        let data = base.join("data");
        let backup = base.join("backups");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("prompthub.db"), b"DBDATA").unwrap();
        (data, backup)
    }

    #[test]
    fn backup_create_returns_id_and_iso8601_timestamp() {
        let tmp = TempDir::new().unwrap();
        let (data, backup_root) = backup_dirs(tmp.path());

        let entry = backup_create(&data, &backup_root).unwrap();
        assert!(!entry.id.is_empty());
        // ISO_8601 millisecond form (`...Z`, 24 chars) per Requirement 4.9.
        assert!(entry.created_at.ends_with('Z'));
        assert_eq!(entry.created_at.len(), 24);
        // The snapshot directory and a copy of the data exist.
        assert!(backup_root
            .join(&entry.id)
            .join("data")
            .join("prompthub.db")
            .is_file());
    }

    #[test]
    fn backup_list_returns_created_backups_most_recent_first() {
        let tmp = TempDir::new().unwrap();
        let (data, backup_root) = backup_dirs(tmp.path());

        let first = backup_create(&data, &backup_root).unwrap();
        // Ensure a strictly later id by spinning until the millisecond advances is
        // unnecessary: the uuid suffix + id tiebreaker make ordering deterministic
        // within the same millisecond, but we assert set membership for safety.
        let second = backup_create(&data, &backup_root).unwrap();

        let list = backup_list(&backup_root).unwrap();
        assert_eq!(list.len(), 2);
        let ids: Vec<&str> = list.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&first.id.as_str()));
        assert!(ids.contains(&second.id.as_str()));
    }

    #[test]
    fn backup_list_empty_when_root_missing() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("no-backups");
        assert!(backup_list(&missing).unwrap().is_empty());
    }

    #[test]
    fn backup_delete_removes_backup() {
        let tmp = TempDir::new().unwrap();
        let (data, backup_root) = backup_dirs(tmp.path());
        let entry = backup_create(&data, &backup_root).unwrap();

        backup_delete(&backup_root, &entry.id).unwrap();
        assert!(!backup_root.join(&entry.id).exists());
        assert!(backup_list(&backup_root).unwrap().is_empty());
    }

    #[test]
    fn backup_delete_unknown_returns_not_found() {
        let tmp = TempDir::new().unwrap();
        let (_data, backup_root) = backup_dirs(tmp.path());
        fs::create_dir_all(&backup_root).unwrap();
        let err = backup_delete(&backup_root, "backup-does-not-exist").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn backup_delete_malformed_id_returns_not_found_without_escape() {
        let tmp = TempDir::new().unwrap();
        let (_data, backup_root) = backup_dirs(tmp.path());
        // An outside file a traversal id would try to remove.
        let outside = tmp.path().join("outside.txt");
        fs::write(&outside, "KEEP").unwrap();

        let err = backup_delete(&backup_root, "../outside.txt").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
        // The traversal target is untouched.
        assert_eq!(fs::read_to_string(&outside).unwrap(), "KEEP");
    }

    #[test]
    fn backup_restore_reports_restart_required_and_restores_data() {
        let tmp = TempDir::new().unwrap();
        let (data, backup_root) = backup_dirs(tmp.path());
        let entry = backup_create(&data, &backup_root).unwrap();

        // Mutate the live data after the backup.
        fs::write(data.join("prompthub.db"), b"CHANGED").unwrap();
        fs::write(data.join("extra.txt"), b"new").unwrap();

        let result = backup_restore(&data, &backup_root, &entry.id).unwrap();
        assert!(result.restart_required);
        assert_eq!(result.id, entry.id);

        // The snapshot's contents are restored; the post-backup file is gone.
        let mut restored = String::new();
        fs::File::open(data.join("prompthub.db"))
            .unwrap()
            .read_to_string(&mut restored)
            .unwrap();
        assert_eq!(restored, "DBDATA");
        assert!(!data.join("extra.txt").exists());
    }

    #[test]
    fn backup_restore_unknown_returns_not_found_and_leaves_data() {
        let tmp = TempDir::new().unwrap();
        let (data, backup_root) = backup_dirs(tmp.path());
        fs::create_dir_all(&backup_root).unwrap();

        let err = backup_restore(&data, &backup_root, "nope").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
        // Existing data is unchanged (17.12).
        assert_eq!(
            fs::read_to_string(data.join("prompthub.db")).unwrap(),
            "DBDATA"
        );
    }

    #[test]
    fn valid_backup_id_rejects_path_characters() {
        assert!(is_valid_backup_id("backup-123_abc"));
        assert!(!is_valid_backup_id(""));
        assert!(!is_valid_backup_id("../escape"));
        assert!(!is_valid_backup_id("a/b"));
        assert!(!is_valid_backup_id("a.b"));
    }
}
