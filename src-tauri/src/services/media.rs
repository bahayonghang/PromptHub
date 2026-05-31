//! Media_Service — image and video file management (Requirement 18).
//!
//! This module stores user images/videos in a configured media directory, reads
//! and lists them, downloads images over HTTP(S) with SSRF protection, and
//! validates image format. Like the sibling services it is written against
//! *injected* dependencies — every operation takes the target directory (and, for
//! path-based saves, the set of session-selected source paths) as arguments
//! rather than reaching into global state — so the rules are unit-testable with a
//! [`tempfile`] tree and without a live window. The Command_Layer (task 17.1)
//! resolves the images/videos directories from [`crate::state::RuntimePaths`] and
//! owns the per-session selected-path set.
//!
//! ## Format validation (Req 18.8)
//!
//! Image saves are validated by **magic bytes** ([`detect_image_format`]) — the
//! pure, network-free core — accepting only JPEG, PNG, GIF, and WebP. Data that
//! is not one of those formats, a base64 payload that cannot be decoded, or a
//! source path that was not selected in the current session are each rejected
//! with `VALIDATION`, and no file is written. Video saves are validated by their
//! file extension against [`VIDEO_EXTENSIONS`].
//!
//! ## Download SSRF policy (Req 18.4, 18.5)
//!
//! [`download_image`] accepts HTTP and HTTPS URLs (unlike the HTTPS-only skill
//! fetch), follows at most [`MAX_REDIRECTS`] redirects within a
//! [`DOWNLOAD_TIMEOUT`], requires an image content-type, and caps the body at
//! [`MAX_DOWNLOAD_BYTES`]. The security-critical address classification reuses
//! [`crate::services::skill_safety::is_public_ip`] and
//! [`crate::services::skill_safety::is_blocked_hostname`] rather than duplicating
//! the SSRF ranges: a non-HTTP(S) scheme, a localhost/loopback name, or a host
//! that resolves to any non-public address is rejected with `SSRF_BLOCKED`
//! *before* any outbound request, and the policy is re-checked on every redirect
//! hop.
//!
//! ## File-name safety
//!
//! Read/size/delete operations resolve the supplied file name through
//! [`safe_join`], which rejects anything other than a bare file name (no path
//! separators, no `..`), so a caller cannot escape the media directory. A missing
//! file on read/size/delete yields `NOT_FOUND` (Req 18.7).
#![allow(dead_code)]

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use futures_util::StreamExt;
use uuid::Uuid;

use crate::error::AppError;
use crate::services::skill_safety::{is_blocked_hostname, is_public_ip};

// ===========================================================================
// Constants
// ===========================================================================

/// Recognized image file extensions (lowercase, without the dot) — used to
/// filter [`list`] results and to label downloaded/saved images.
pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp"];
/// Recognized video file extensions (lowercase, without the dot) — used to
/// validate video saves and filter [`list`] results.
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm", "mov", "avi", "mkv"];

/// Maximum image download size: 10 MB (Req 18.4).
const MAX_DOWNLOAD_BYTES: u64 = 10 * 1024 * 1024;
/// Maximum redirects followed by [`download_image`] (Req 18.4).
const MAX_REDIRECTS: usize = 5;
/// Per-request timeout for [`download_image`]: 30 seconds (Req 18.4).
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);

// ===========================================================================
// Directory resolution
// ===========================================================================

/// Returns the images subdirectory under a media root.
pub fn images_dir(media_root: &Path) -> PathBuf {
    media_root.join("images")
}

/// Returns the videos subdirectory under a media root.
pub fn videos_dir(media_root: &Path) -> PathBuf {
    media_root.join("videos")
}

// ===========================================================================
// Image format detection (Req 18.8) — pure, network-free core
// ===========================================================================

/// A supported image format, identified by its leading magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// JPEG (`FF D8 FF`).
    Jpeg,
    /// PNG (`89 50 4E 47 0D 0A 1A 0A`).
    Png,
    /// GIF (`GIF87a` / `GIF89a`).
    Gif,
    /// WebP (`RIFF`…`WEBP`).
    WebP,
}

impl ImageFormat {
    /// The canonical lowercase file extension (without the dot) for this format.
    pub fn extension(self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Gif => "gif",
            ImageFormat::WebP => "webp",
        }
    }
}

/// Detects the image format of `bytes` by inspecting its leading magic bytes,
/// returning `None` when it is not one of the four supported formats (Req 18.8).
///
/// Pure and network-free; this is the single authority for the image format gate
/// and is exhaustively unit-tested.
pub fn detect_image_format(bytes: &[u8]) -> Option<ImageFormat> {
    // JPEG — starts with FF D8 FF.
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some(ImageFormat::Jpeg);
    }
    // PNG — the 8-byte signature.
    const PNG: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() >= 8 && bytes[..8] == PNG {
        return Some(ImageFormat::Png);
    }
    // GIF — "GIF87a" or "GIF89a".
    if bytes.len() >= 6 && (&bytes[..6] == b"GIF87a" || &bytes[..6] == b"GIF89a") {
        return Some(ImageFormat::Gif);
    }
    // WebP — "RIFF" then a 4-byte length then "WEBP".
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(ImageFormat::WebP);
    }
    None
}

// ===========================================================================
// Shared filesystem helpers
// ===========================================================================

/// Maps a raw I/O error into an `IO` [`AppError`].
fn io_err(context: &str, e: std::io::Error) -> AppError {
    AppError::io(format!("{context}: {e}"))
}

/// Ensures `dir` exists, creating it (and parents) if necessary.
fn ensure_dir(dir: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dir).map_err(|e| io_err("failed to create media directory", e))
}

/// Resolves a bare file `name` to a path inside `dir`, rejecting anything that is
/// not a single path component (no separators, no `..`, no drive prefix) so a
/// caller cannot escape the media directory.
fn safe_join(dir: &Path, name: &str) -> Result<PathBuf, AppError> {
    if name.is_empty() {
        return Err(AppError::validation("file name must not be empty"));
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.contains('\0') {
        return Err(AppError::validation("invalid file name"));
    }
    // The name must be exactly its own final component (rejects `.`, `C:` etc.).
    if Path::new(name).file_name() != Some(OsStr::new(name)) {
        return Err(AppError::validation("invalid file name"));
    }
    Ok(dir.join(name))
}

/// Writes `bytes` to a freshly generated `<uuid>.<ext>` file in `dir`, creating
/// the directory if needed, and returns the generated file name.
fn write_new_file(dir: &Path, ext: &str, bytes: &[u8]) -> Result<String, AppError> {
    ensure_dir(dir)?;
    let name = format!("{}.{}", Uuid::new_v4(), ext);
    fs::write(dir.join(&name), bytes).map_err(|e| io_err("failed to write media file", e))?;
    Ok(name)
}

/// Returns the lowercase extension (without the dot) of a path, if any.
fn ext_of(path: &Path) -> Option<String> {
    path.extension()
        .and_then(OsStr::to_str)
        .map(|s| s.to_ascii_lowercase())
}

/// Returns `true` when `path` is one of the caller-supplied session-selected
/// source paths, comparing both the path as given and its canonicalized form so a
/// relative/absolute mismatch does not wrongly reject a legitimately selected
/// file (Req 18.1, 18.8).
fn is_session_path(allowed: &HashSet<PathBuf>, path: &Path) -> bool {
    if allowed.contains(path) {
        return true;
    }
    matches!(path.canonicalize(), Ok(canon) if allowed.contains(&canon))
}

// ===========================================================================
// Shared store operations (Req 18.3, 18.6, 18.7)
// ===========================================================================

/// Lists the stored file names in `dir` whose extension is one of `allowed_exts`
/// (Req 18.3), returning an empty list — never an error — when the directory does
/// not exist. Results are sorted for a deterministic listing.
pub fn list(dir: &Path, allowed_exts: &[&str]) -> Result<Vec<String>, AppError> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| io_err("failed to read media directory", e))? {
        let entry = entry.map_err(|e| io_err("failed to read directory entry", e))?;
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let matches = ext_of(&path).map_or(false, |e| allowed_exts.contains(&e.as_str()));
        if matches {
            if let Some(name) = path.file_name().and_then(OsStr::to_str) {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Reads the content of a stored media file (Req 18.3).
///
/// Returns `NOT_FOUND` when the named file does not exist (Req 18.7), and
/// `VALIDATION` for a file name that is not a bare component.
pub fn read(dir: &Path, name: &str) -> Result<Vec<u8>, AppError> {
    let path = safe_join(dir, name)?;
    if !path.is_file() {
        return Err(AppError::not_found(format!(
            "media file `{name}` not found"
        )));
    }
    fs::read(&path).map_err(|e| io_err("failed to read media file", e))
}

/// Returns whether a named media file exists in `dir` (Req 18.3).
///
/// An invalid (non-bare) file name simply reports `false` rather than erroring.
pub fn exists(dir: &Path, name: &str) -> bool {
    match safe_join(dir, name) {
        Ok(path) => path.is_file(),
        Err(_) => false,
    }
}

/// Returns the size in bytes of a stored media file (Req 18.3).
///
/// Returns `NOT_FOUND` when the named file does not exist (Req 18.7).
pub fn get_size(dir: &Path, name: &str) -> Result<u64, AppError> {
    let path = safe_join(dir, name)?;
    if !path.is_file() {
        return Err(AppError::not_found(format!(
            "media file `{name}` not found"
        )));
    }
    let meta = fs::metadata(&path).map_err(|e| io_err("failed to stat media file", e))?;
    Ok(meta.len())
}

/// Deletes a stored media file (Req 18.3).
///
/// Returns `NOT_FOUND` when the named file does not exist (Req 18.7), leaving the
/// media directory unchanged.
pub fn delete(dir: &Path, name: &str) -> Result<(), AppError> {
    let path = safe_join(dir, name)?;
    if !path.is_file() {
        return Err(AppError::not_found(format!(
            "media file `{name}` not found"
        )));
    }
    fs::remove_file(&path).map_err(|e| io_err("failed to delete media file", e))
}

/// Removes every file in the media directory (Req 18.6).
///
/// A non-existent directory is a no-op. Subdirectories are left untouched.
pub fn clear(dir: &Path) -> Result<(), AppError> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| io_err("failed to read media directory", e))? {
        let entry = entry.map_err(|e| io_err("failed to read directory entry", e))?;
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            fs::remove_file(entry.path()).map_err(|e| io_err("failed to delete media file", e))?;
        }
    }
    Ok(())
}

// ===========================================================================
// Image saves (Req 18.1, 18.2, 18.8)
// ===========================================================================

/// Saves one or more images from session-selected file paths (Req 18.1).
///
/// Validation is **atomic** (Req 18.8): every path is checked — it must be a
/// session-selected path and its content must be a supported image format —
/// before anything is written, so a single bad entry rejects the whole request
/// with `VALIDATION` and no file is stored. On success each image is copied into
/// `dir` under a generated `<uuid>.<ext>` name and the names are returned in input
/// order.
pub fn save_images(
    dir: &Path,
    allowed: &HashSet<PathBuf>,
    paths: &[PathBuf],
) -> Result<Vec<String>, AppError> {
    // Phase 1: validate everything, reading and format-checking each source.
    let mut prepared: Vec<(Vec<u8>, &'static str)> = Vec::with_capacity(paths.len());
    for path in paths {
        if !is_session_path(allowed, path) {
            return Err(AppError::validation(format!(
                "`{}` was not selected through the file picker in this session",
                path.display()
            )));
        }
        let bytes = fs::read(path).map_err(|e| io_err("failed to read source image", e))?;
        let format = detect_image_format(&bytes).ok_or_else(|| {
            AppError::validation(format!(
                "`{}` is not a supported image (JPEG, PNG, GIF, or WebP)",
                path.display()
            ))
        })?;
        prepared.push((bytes, format.extension()));
    }
    // Phase 2: persist (only reached when every entry validated).
    let mut names = Vec::with_capacity(prepared.len());
    for (bytes, ext) in prepared {
        names.push(write_new_file(dir, ext, &bytes)?);
    }
    Ok(names)
}

/// Saves an image from an in-memory buffer (Req 18.1).
///
/// The buffer's format is validated by magic bytes; data that is not a supported
/// image is rejected with `VALIDATION` and nothing is written (Req 18.8).
pub fn save_image_buffer(dir: &Path, bytes: &[u8]) -> Result<String, AppError> {
    let format = detect_image_format(bytes).ok_or_else(|| {
        AppError::validation("buffer is not a supported image (JPEG, PNG, GIF, or WebP)")
    })?;
    write_new_file(dir, format.extension(), bytes)
}

/// Saves a base64-encoded image, decoding it then storing it (Req 18.2).
///
/// Tolerates a `data:` URL prefix and embedded whitespace. Data that cannot be
/// decoded, or that decodes to something other than a supported image format, is
/// rejected with `VALIDATION` and nothing is written (Req 18.8).
pub fn save_image_base64(dir: &Path, data: &str) -> Result<String, AppError> {
    let bytes = decode_base64(data)?;
    let format = detect_image_format(&bytes).ok_or_else(|| {
        AppError::validation("decoded data is not a supported image (JPEG, PNG, GIF, or WebP)")
    })?;
    write_new_file(dir, format.extension(), &bytes)
}

/// Decodes a (possibly `data:`-prefixed, whitespace-laden) base64 string,
/// returning `VALIDATION` when it cannot be decoded (Req 18.8).
fn decode_base64(data: &str) -> Result<Vec<u8>, AppError> {
    // Drop an optional `data:[mediatype];base64,` prefix.
    let payload = match data.split_once("base64,") {
        Some((_, rest)) => rest,
        None => data,
    };
    // Strip all whitespace so wrapped/pretty-printed payloads still decode.
    let cleaned: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
    BASE64
        .decode(cleaned.as_bytes())
        .map_err(|e| AppError::validation(format!("image data could not be base64-decoded: {e}")))
}

// ===========================================================================
// Video saves (Req 18.1, 18.2)
// ===========================================================================

/// Returns the canonical lowercase extension for a session-selected video path,
/// or `VALIDATION` when the extension is not a supported video type.
fn video_ext(path: &Path) -> Result<String, AppError> {
    match ext_of(path) {
        Some(ext) if VIDEO_EXTENSIONS.contains(&ext.as_str()) => Ok(ext),
        _ => Err(AppError::validation(format!(
            "`{}` is not a supported video (MP4, WebM, MOV, AVI, or MKV)",
            path.display()
        ))),
    }
}

/// Saves one or more videos from session-selected file paths (Req 18.1).
///
/// Mirrors [`save_images`]: validation is atomic (every path must be
/// session-selected with a supported video extension) before anything is copied,
/// so a single bad entry rejects the whole request with `VALIDATION` and no file
/// is stored. Returns the generated file names in input order.
pub fn save_videos(
    dir: &Path,
    allowed: &HashSet<PathBuf>,
    paths: &[PathBuf],
) -> Result<Vec<String>, AppError> {
    let mut prepared: Vec<(Vec<u8>, String)> = Vec::with_capacity(paths.len());
    for path in paths {
        if !is_session_path(allowed, path) {
            return Err(AppError::validation(format!(
                "`{}` was not selected through the file picker in this session",
                path.display()
            )));
        }
        let ext = video_ext(path)?;
        let bytes = fs::read(path).map_err(|e| io_err("failed to read source video", e))?;
        prepared.push((bytes, ext));
    }
    let mut names = Vec::with_capacity(prepared.len());
    for (bytes, ext) in prepared {
        names.push(write_new_file(dir, &ext, &bytes)?);
    }
    Ok(names)
}

/// Saves a video from an in-memory buffer under the given extension (Req 18.1).
///
/// The extension must be a supported video type; an unsupported extension is
/// rejected with `VALIDATION` and nothing is written.
pub fn save_video_buffer(dir: &Path, ext: &str, bytes: &[u8]) -> Result<String, AppError> {
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    if !VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::validation(
            "unsupported video type (MP4, WebM, MOV, AVI, or MKV)",
        ));
    }
    write_new_file(dir, &ext, bytes)
}

/// Saves a base64-encoded video under the given extension (Req 18.2).
///
/// Data that cannot be decoded, or an unsupported extension, is rejected with
/// `VALIDATION` and nothing is written.
pub fn save_video_base64(dir: &Path, ext: &str, data: &str) -> Result<String, AppError> {
    let bytes = decode_base64(data)?;
    save_video_buffer(dir, ext, &bytes)
}

// ===========================================================================
// Image download with SSRF protection (Req 18.4, 18.5)
// ===========================================================================

/// Outcome of the synchronous, DNS-free URL precheck.
enum HostCheck {
    /// The host was an IP literal; the validated (public) address is carried.
    Literal(IpAddr),
    /// The host is a domain name that still requires DNS resolution.
    Domain(String),
}

/// Synchronously validates a URL's scheme and host against the SSRF policy
/// without performing DNS (Req 18.5).
///
/// Unlike the HTTPS-only skill fetch, image download permits **HTTP and HTTPS**
/// (Req 18.5). Rejects with `SSRF_BLOCKED` when the scheme is neither HTTP nor
/// HTTPS, when the host names the local machine, or when the host is an IP literal
/// in a non-public range — reusing [`is_public_ip`]/[`is_blocked_hostname`] so the
/// SSRF ranges are defined in exactly one place.
fn precheck_url(raw: &str) -> Result<(reqwest::Url, HostCheck), AppError> {
    let url = reqwest::Url::parse(raw)
        .map_err(|e| AppError::validation(format!("invalid URL `{raw}`: {e}")))?;

    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(AppError::ssrf_blocked(
            "only HTTP and HTTPS URLs are allowed for image download",
        ));
    }

    let host = url
        .host_str()
        .ok_or_else(|| AppError::validation("URL has no host"))?
        .to_string();

    // IPv6 literals appear bracketed in `host_str()` (e.g. `[::1]`).
    let ip_candidate = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = ip_candidate.parse::<IpAddr>() {
        if !is_public_ip(ip) {
            return Err(AppError::ssrf_blocked(format!(
                "host `{host}` resolves to a non-public address"
            )));
        }
        return Ok((url, HostCheck::Literal(ip)));
    }

    if is_blocked_hostname(&host) {
        return Err(AppError::ssrf_blocked(format!(
            "host `{host}` names the local machine"
        )));
    }

    Ok((url, HostCheck::Domain(host)))
}

/// Resolves a domain host and verifies every resolved address is public
/// (Req 18.5), returning the validated addresses.
async fn resolve_public_addrs(host: &str, port: u16) -> Result<Vec<IpAddr>, AppError> {
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| AppError::network(format!("failed to resolve host `{host}`: {e}")))?;

    let mut ips = Vec::new();
    for addr in addrs {
        let ip = addr.ip();
        if !is_public_ip(ip) {
            return Err(AppError::ssrf_blocked(format!(
                "host `{host}` resolves to a non-public address"
            )));
        }
        ips.push(ip);
    }
    if ips.is_empty() {
        return Err(AppError::network(format!("host `{host}` did not resolve")));
    }
    Ok(ips)
}

/// Maps a `reqwest` transport error into the appropriate [`AppError`].
fn map_reqwest_err(context: &str, e: reqwest::Error) -> AppError {
    if e.is_timeout() {
        AppError::timeout(format!("{context}: request timed out"))
    } else {
        AppError::network(format!("{context}: {e}"))
    }
}

/// Picks an image extension from a response content-type header, defaulting to
/// `png` for an image type that is not one of the recognized four.
fn ext_from_content_type(content_type: &str) -> &'static str {
    match content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "png",
    }
}

/// Downloads an image from an HTTP(S) URL and stores it, returning the generated
/// file name (Req 18.4, 18.5).
///
/// Enforces, in order: HTTP/HTTPS-only scheme; per-hop SSRF host classification
/// (every resolved address must be public) *before* connecting; at most
/// [`MAX_REDIRECTS`] redirects, re-checking the SSRF policy on each hop; a
/// [`DOWNLOAD_TIMEOUT`] per-request deadline; a successful status; an `image/*`
/// content-type; and a [`MAX_DOWNLOAD_BYTES`] body cap enforced both via the
/// declared `Content-Length` and while streaming. On any policy violation,
/// non-success status, non-image type, exceeded limit, or transport failure it
/// returns a structured error without storing partial content.
pub async fn download_image(dir: &Path, url: &str) -> Result<String, AppError> {
    let mut current = url.to_string();

    for _ in 0..=MAX_REDIRECTS {
        let (parsed, host_check) = precheck_url(&current)?;
        let host = parsed
            .host_str()
            .ok_or_else(|| AppError::validation("URL has no host"))?
            .to_string();
        let scheme_default = if parsed.scheme() == "https" { 443 } else { 80 };
        let port = parsed.port_or_known_default().unwrap_or(scheme_default);

        // Resolve + classify the addresses, then pin the client to exactly those
        // validated addresses so the connection cannot target a different host.
        let ips = match host_check {
            HostCheck::Literal(ip) => vec![ip],
            HostCheck::Domain(ref domain) => resolve_public_addrs(domain, port).await?,
        };
        let addrs: Vec<SocketAddr> = ips.iter().map(|ip| SocketAddr::new(*ip, port)).collect();

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(DOWNLOAD_TIMEOUT)
            .resolve_to_addrs(&host, &addrs)
            .build()
            .map_err(|e| map_reqwest_err("failed to build HTTP client", e))?;

        let response = client
            .get(parsed.clone())
            .header(reqwest::header::ACCEPT, "image/*")
            .send()
            .await
            .map_err(|e| map_reqwest_err("failed to download image", e))?;

        let status = response.status();

        // Manual redirect handling so each hop is re-checked against the policy.
        if status.is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| AppError::network("redirect response without a Location header"))?;
            let next = parsed
                .join(location)
                .map_err(|e| AppError::network(format!("invalid redirect target: {e}")))?;
            current = next.to_string();
            continue;
        }

        if !status.is_success() {
            return Err(AppError::network(format!(
                "image download returned HTTP {}",
                status.as_u16()
            )));
        }

        // Content-type must be an image (Req 18.4).
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        if !content_type
            .trim()
            .to_ascii_lowercase()
            .starts_with("image/")
        {
            return Err(AppError::validation("remote resource is not an image"));
        }

        // Reject oversize bodies up front when the server declares the length.
        if let Some(len) = response.content_length() {
            if len > MAX_DOWNLOAD_BYTES {
                return Err(AppError::network(
                    "remote image exceeds the 10 MB size limit",
                ));
            }
        }

        // Stream the body, enforcing the cap as bytes arrive (Req 18.4).
        let mut bytes: Vec<u8> = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| map_reqwest_err("failed to read image body", e))?;
            if bytes.len() as u64 + chunk.len() as u64 > MAX_DOWNLOAD_BYTES {
                return Err(AppError::network(
                    "remote image exceeds the 10 MB size limit",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }

        let ext = ext_from_content_type(&content_type);
        return write_new_file(dir, ext, &bytes);
    }

    Err(AppError::network(
        "too many redirects while downloading image",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use tempfile::TempDir;

    // --- detect_image_format (Req 18.8) ------------------------------------

    fn jpeg() -> Vec<u8> {
        vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]
    }
    fn png() -> Vec<u8> {
        vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x01]
    }
    fn gif() -> Vec<u8> {
        let mut v = b"GIF89a".to_vec();
        v.extend_from_slice(&[0x01, 0x02]);
        v
    }
    fn webp() -> Vec<u8> {
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&[0x24, 0x00, 0x00, 0x00]);
        v.extend_from_slice(b"WEBP");
        v
    }

    #[test]
    fn detects_each_supported_format() {
        assert_eq!(detect_image_format(&jpeg()), Some(ImageFormat::Jpeg));
        assert_eq!(detect_image_format(&png()), Some(ImageFormat::Png));
        assert_eq!(
            detect_image_format(b"GIF87a\x00\x00"),
            Some(ImageFormat::Gif)
        );
        assert_eq!(detect_image_format(&gif()), Some(ImageFormat::Gif));
        assert_eq!(detect_image_format(&webp()), Some(ImageFormat::WebP));
    }

    #[test]
    fn rejects_unknown_and_truncated_formats() {
        assert_eq!(detect_image_format(b"not an image"), None);
        assert_eq!(detect_image_format(&[]), None);
        // Truncated magic bytes must not match.
        assert_eq!(detect_image_format(&[0xFF, 0xD8]), None);
        assert_eq!(detect_image_format(b"GIF8"), None);
        // RIFF without WEBP (e.g. a WAV) is not an image.
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&[0x24, 0x00, 0x00, 0x00]);
        wav.extend_from_slice(b"WAVE");
        assert_eq!(detect_image_format(&wav), None);
    }

    #[test]
    fn format_extensions_are_canonical() {
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Gif.extension(), "gif");
        assert_eq!(ImageFormat::WebP.extension(), "webp");
    }

    // --- safe_join ----------------------------------------------------------

    #[test]
    fn safe_join_accepts_bare_name_rejects_traversal() {
        let base = TempDir::new().unwrap();
        assert!(safe_join(base.path(), "a.png").is_ok());
        for bad in ["", "../x.png", "a/b.png", "a\\b.png", "..", ".", "\0x"] {
            assert_eq!(
                safe_join(base.path(), bad).unwrap_err().code,
                ErrorCode::Validation,
                "expected `{bad}` to be rejected"
            );
        }
    }

    // --- save_image_base64 (Req 18.2, 18.8) --------------------------------

    #[test]
    fn save_base64_stores_decoded_image_and_returns_name() {
        let base = TempDir::new().unwrap();
        let encoded = BASE64.encode(png());
        let name = save_image_base64(base.path(), &encoded).unwrap();
        assert!(name.ends_with(".png"));
        assert_eq!(read(base.path(), &name).unwrap(), png());
    }

    #[test]
    fn save_base64_accepts_data_url_prefix_and_whitespace() {
        let base = TempDir::new().unwrap();
        let encoded = BASE64.encode(jpeg());
        let data = format!(
            "data:image/jpeg;base64,{}\n  {}",
            &encoded[..4],
            &encoded[4..]
        );
        let name = save_image_base64(base.path(), &data).unwrap();
        assert!(name.ends_with(".jpg"));
    }

    #[test]
    fn save_base64_rejects_undecodable_data_without_writing() {
        let base = TempDir::new().unwrap();
        let dir = images_dir(base.path());
        let err = save_image_base64(&dir, "%%%not base64%%%").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert!(!dir.exists() || list(&dir, IMAGE_EXTENSIONS).unwrap().is_empty());
    }

    #[test]
    fn save_base64_rejects_non_image_payload_without_writing() {
        let base = TempDir::new().unwrap();
        let dir = images_dir(base.path());
        let encoded = BASE64.encode(b"this is plain text, not an image");
        let err = save_image_base64(&dir, &encoded).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert!(!dir.exists() || list(&dir, IMAGE_EXTENSIONS).unwrap().is_empty());
    }

    // --- save_image_buffer (Req 18.1, 18.8) --------------------------------

    #[test]
    fn save_buffer_stores_supported_image() {
        let base = TempDir::new().unwrap();
        let name = save_image_buffer(base.path(), &webp()).unwrap();
        assert!(name.ends_with(".webp"));
        assert_eq!(read(base.path(), &name).unwrap(), webp());
    }

    #[test]
    fn save_buffer_rejects_unsupported_format() {
        let base = TempDir::new().unwrap();
        let err = save_image_buffer(base.path(), b"nope").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    // --- save_images from session paths (Req 18.1, 18.8) -------------------

    #[test]
    fn save_images_copies_session_selected_files() {
        let base = TempDir::new().unwrap();
        let src = base.path().join("picked.png");
        fs::write(&src, png()).unwrap();
        let allowed: HashSet<PathBuf> = [src.clone()].into_iter().collect();
        let dir = images_dir(base.path());

        let names = save_images(&dir, &allowed, &[src]).unwrap();
        assert_eq!(names.len(), 1);
        assert!(names[0].ends_with(".png"));
        assert_eq!(read(&dir, &names[0]).unwrap(), png());
    }

    #[test]
    fn save_images_rejects_non_session_path_without_writing() {
        let base = TempDir::new().unwrap();
        let src = base.path().join("sneaky.png");
        fs::write(&src, png()).unwrap();
        // `allowed` is empty: the path was never selected this session.
        let allowed: HashSet<PathBuf> = HashSet::new();
        let dir = images_dir(base.path());

        let err = save_images(&dir, &allowed, &[src]).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert!(!dir.exists() || list(&dir, IMAGE_EXTENSIONS).unwrap().is_empty());
    }

    #[test]
    fn save_images_is_atomic_one_bad_entry_writes_nothing() {
        let base = TempDir::new().unwrap();
        let good = base.path().join("good.png");
        let bad = base.path().join("bad.txt");
        fs::write(&good, png()).unwrap();
        fs::write(&bad, b"not an image").unwrap();
        let allowed: HashSet<PathBuf> = [good.clone(), bad.clone()].into_iter().collect();
        let dir = images_dir(base.path());

        let err = save_images(&dir, &allowed, &[good, bad]).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        // Nothing persisted, even though the first entry was valid.
        assert!(!dir.exists() || list(&dir, IMAGE_EXTENSIONS).unwrap().is_empty());
    }

    // --- videos (Req 18.1, 18.2) -------------------------------------------

    #[test]
    fn save_videos_copies_session_selected_files() {
        let base = TempDir::new().unwrap();
        let src = base.path().join("clip.mp4");
        fs::write(&src, b"\x00\x00\x00\x18ftypmp42").unwrap();
        let allowed: HashSet<PathBuf> = [src.clone()].into_iter().collect();
        let dir = videos_dir(base.path());

        let names = save_videos(&dir, &allowed, &[src]).unwrap();
        assert_eq!(names.len(), 1);
        assert!(names[0].ends_with(".mp4"));
    }

    #[test]
    fn save_videos_rejects_unsupported_extension() {
        let base = TempDir::new().unwrap();
        let src = base.path().join("clip.exe");
        fs::write(&src, b"MZ").unwrap();
        let allowed: HashSet<PathBuf> = [src.clone()].into_iter().collect();
        let dir = videos_dir(base.path());

        let err = save_videos(&dir, &allowed, &[src]).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert!(!dir.exists() || list(&dir, VIDEO_EXTENSIONS).unwrap().is_empty());
    }

    #[test]
    fn save_video_base64_round_trips() {
        let base = TempDir::new().unwrap();
        let payload = b"\x1aE\xdf\xa3 webm-ish bytes";
        let encoded = BASE64.encode(payload);
        let name = save_video_base64(base.path(), "webm", &encoded).unwrap();
        assert!(name.ends_with(".webm"));
        assert_eq!(read(base.path(), &name).unwrap(), payload);
    }

    #[test]
    fn save_video_base64_rejects_undecodable() {
        let base = TempDir::new().unwrap();
        let err = save_video_base64(base.path(), "mp4", "@@not base64@@").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    // --- list / read / exists / get_size / delete / clear (Req 18.3, 18.6, 18.7) ---

    #[test]
    fn list_returns_only_matching_extensions_sorted() {
        let base = TempDir::new().unwrap();
        let dir = images_dir(base.path());
        ensure_dir(&dir).unwrap();
        fs::write(dir.join("b.png"), png()).unwrap();
        fs::write(dir.join("a.jpg"), jpeg()).unwrap();
        fs::write(dir.join("notes.txt"), b"x").unwrap();

        let names = list(&dir, IMAGE_EXTENSIONS).unwrap();
        assert_eq!(names, vec!["a.jpg".to_string(), "b.png".to_string()]);
    }

    #[test]
    fn list_missing_dir_returns_empty() {
        let base = TempDir::new().unwrap();
        assert!(list(&images_dir(base.path()), IMAGE_EXTENSIONS)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn exists_reports_presence() {
        let base = TempDir::new().unwrap();
        let dir = images_dir(base.path());
        ensure_dir(&dir).unwrap();
        fs::write(dir.join("there.png"), png()).unwrap();
        assert!(exists(&dir, "there.png"));
        assert!(!exists(&dir, "missing.png"));
        // Traversal attempts simply report false.
        assert!(!exists(&dir, "../there.png"));
    }

    #[test]
    fn get_size_returns_byte_length() {
        let base = TempDir::new().unwrap();
        let dir = images_dir(base.path());
        ensure_dir(&dir).unwrap();
        fs::write(dir.join("img.png"), png()).unwrap();
        assert_eq!(get_size(&dir, "img.png").unwrap(), png().len() as u64);
    }

    #[test]
    fn read_get_size_delete_missing_return_not_found() {
        let base = TempDir::new().unwrap();
        let dir = images_dir(base.path());
        ensure_dir(&dir).unwrap();
        assert_eq!(read(&dir, "x.png").unwrap_err().code, ErrorCode::NotFound);
        assert_eq!(
            get_size(&dir, "x.png").unwrap_err().code,
            ErrorCode::NotFound
        );
        assert_eq!(delete(&dir, "x.png").unwrap_err().code, ErrorCode::NotFound);
    }

    #[test]
    fn delete_removes_file() {
        let base = TempDir::new().unwrap();
        let dir = images_dir(base.path());
        ensure_dir(&dir).unwrap();
        fs::write(dir.join("gone.png"), png()).unwrap();
        delete(&dir, "gone.png").unwrap();
        assert!(!dir.join("gone.png").exists());
    }

    #[test]
    fn clear_removes_all_files() {
        let base = TempDir::new().unwrap();
        let dir = videos_dir(base.path());
        ensure_dir(&dir).unwrap();
        fs::write(dir.join("a.mp4"), b"a").unwrap();
        fs::write(dir.join("b.webm"), b"b").unwrap();
        clear(&dir).unwrap();
        assert!(list(&dir, VIDEO_EXTENSIONS).unwrap().is_empty());
    }

    #[test]
    fn clear_missing_dir_is_noop() {
        let base = TempDir::new().unwrap();
        assert!(clear(&images_dir(base.path())).is_ok());
    }

    // --- download SSRF rejection (Req 18.5) --------------------------------
    // These reject in the synchronous precheck, so no DNS/network occurs.

    #[tokio::test]
    async fn download_rejects_non_http_scheme() {
        let base = TempDir::new().unwrap();
        let err = download_image(base.path(), "ftp://example.com/x.png")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::SsrfBlocked);
    }

    #[tokio::test]
    async fn download_allows_http_scheme_but_blocks_loopback_literal() {
        let base = TempDir::new().unwrap();
        // http:// is allowed (Req 18.5), but the loopback literal is blocked.
        let err = download_image(base.path(), "http://127.0.0.1/x.png")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::SsrfBlocked);
    }

    #[tokio::test]
    async fn download_rejects_ipv6_loopback_literal() {
        let base = TempDir::new().unwrap();
        let err = download_image(base.path(), "https://[::1]/x.png")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::SsrfBlocked);
    }

    #[tokio::test]
    async fn download_rejects_private_literal() {
        let base = TempDir::new().unwrap();
        let err = download_image(base.path(), "http://192.168.1.10/x.png")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::SsrfBlocked);
    }

    #[tokio::test]
    async fn download_rejects_localhost_name() {
        let base = TempDir::new().unwrap();
        let err = download_image(base.path(), "https://localhost/x.png")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::SsrfBlocked);
    }

    #[tokio::test]
    async fn download_rejects_link_local_literal() {
        let base = TempDir::new().unwrap();
        let err = download_image(base.path(), "http://169.254.169.254/latest/meta-data")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::SsrfBlocked);
    }

    #[test]
    fn ext_from_content_type_maps_known_and_defaults() {
        assert_eq!(ext_from_content_type("image/jpeg"), "jpg");
        assert_eq!(ext_from_content_type("image/png; charset=binary"), "png");
        assert_eq!(ext_from_content_type("image/gif"), "gif");
        assert_eq!(ext_from_content_type("image/webp"), "webp");
        assert_eq!(ext_from_content_type("image/svg+xml"), "png");
    }
}
