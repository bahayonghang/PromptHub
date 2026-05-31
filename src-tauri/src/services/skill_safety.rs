//! Skill_Service — safety scanning and remote fetch with SSRF protection
//! (Requirement 13).
//!
//! This module owns four responsibilities, each written against *injected*
//! dependencies (a borrowed [`rusqlite::Connection`], an injectable host
//! resolver, supplied AI configuration) rather than reaching into global state,
//! so the rules are unit-testable without real network I/O or a live window:
//!
//! 1. [`is_public_ip`] — the **pure, network-free** SSRF address classifier
//!    (Req 13.5). It is the security-critical core and is exhaustively tested.
//! 2. [`fetch_content`] — fetches remote skill content over **HTTPS only**,
//!    classifying every resolved address *before* connecting, re-checking on each
//!    redirect hop, and capping redirects/time/size (Req 13.3, 13.5).
//! 3. [`scan`] — sends skill content to the configured AI provider and returns a
//!    structured [`SafetyReport`]; rejects with `VALIDATION` when no AI
//!    configuration is available (Req 13.1, 13.6). Report parsing
//!    ([`parse_report`]) is isolated so it is testable without the HTTP call.
//! 4. [`save_report`] / [`scan_repo`] — persist a report to a skill record
//!    (Req 13.2, returning `NOT_FOUND` for a missing skill per Req 13.7) and
//!    discover `SKILL.md` entries from a fetched GitHub repository listing
//!    (Req 13.4). The listing parse ([`parse_repo_listing`]) is isolated so it is
//!    testable with a sample payload.
//!
//! ## SSRF policy (Req 13.5)
//!
//! A remote fetch is permitted only when the URL scheme is HTTPS *and* the host
//! resolves exclusively to genuinely public addresses. Non-HTTPS schemes,
//! localhost/loopback names, and hosts resolving to loopback, link-local,
//! private, unspecified, broadcast, documentation, or otherwise reserved
//! non-public IPv4/IPv6 addresses are rejected with `SSRF_BLOCKED` *before* any
//! outbound network request is performed.
//!
//! Because the address classifier is the part most prone to subtle off-by-one
//! errors, [`is_public_ip`] is a synchronous, dependency-free function with
//! explicit, individually documented range checks (the standard library's
//! `IpAddr::is_global` is still unstable, so the ranges are spelled out here).
#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::models::{SafetyLevel, Severity, Skill};
use crate::services::skill;
use crate::storage::time::now_millis;

// ===========================================================================
// SSRF address classification (Req 13.5) — pure, network-free, the critical core
// ===========================================================================

/// Returns `true` when `ip` is a genuinely public, routable address and `false`
/// when it falls in any loopback, link-local, private, unspecified, broadcast,
/// documentation, or otherwise reserved non-public range (Req 13.5).
///
/// This is a pure function with no I/O. It is the single authority for the SSRF
/// host check and is exhaustively unit-tested. Each rejected range is documented
/// inline because the standard library's `IpAddr::is_global` is unstable.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_ipv4(v4),
        IpAddr::V6(v6) => is_public_ipv6(v6),
    }
}

/// IPv4 classifier: `true` only for genuinely public addresses (Req 13.5).
fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _d] = ip.octets();

    // 0.0.0.0/8 — "this network" / unspecified host (incl. 0.0.0.0).
    if a == 0 {
        return false;
    }
    // 10.0.0.0/8 — private (RFC 1918).
    if a == 10 {
        return false;
    }
    // 127.0.0.0/8 — loopback.
    if a == 127 {
        return false;
    }
    // 100.64.0.0/10 — shared address space / Carrier-Grade NAT (RFC 6598).
    if a == 100 && (64..=127).contains(&b) {
        return false;
    }
    // 169.254.0.0/16 — link-local (RFC 3927).
    if a == 169 && b == 254 {
        return false;
    }
    // 172.16.0.0/12 — private (RFC 1918).
    if a == 172 && (16..=31).contains(&b) {
        return false;
    }
    // 192.0.0.0/24 — IETF protocol assignments (RFC 6890).
    if a == 192 && b == 0 && c == 0 {
        return false;
    }
    // 192.0.2.0/24 — documentation TEST-NET-1 (RFC 5737).
    if a == 192 && b == 0 && c == 2 {
        return false;
    }
    // 192.168.0.0/16 — private (RFC 1918).
    if a == 192 && b == 168 {
        return false;
    }
    // 198.18.0.0/15 — benchmarking (RFC 2544).
    if a == 198 && (b == 18 || b == 19) {
        return false;
    }
    // 198.51.100.0/24 — documentation TEST-NET-2 (RFC 5737).
    if a == 198 && b == 51 && c == 100 {
        return false;
    }
    // 203.0.113.0/24 — documentation TEST-NET-3 (RFC 5737).
    if a == 203 && b == 0 && c == 113 {
        return false;
    }
    // 224.0.0.0/4 — multicast (224–239) and 240.0.0.0/4 — reserved (240–255),
    // the latter including the 255.255.255.255 limited broadcast address.
    if a >= 224 {
        return false;
    }

    true
}

/// IPv6 classifier: `true` only for genuinely public addresses (Req 13.5).
fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    // :: — unspecified.
    if ip.is_unspecified() {
        return false;
    }
    // ::1 — loopback.
    if ip.is_loopback() {
        return false;
    }

    // ::ffff:a.b.c.d — IPv4-mapped: classify the embedded IPv4 address so a
    // mapped private/loopback address cannot slip through.
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_public_ipv4(v4);
    }

    let segments = ip.segments();

    // ::a.b.c.d — deprecated IPv4-compatible (first 96 bits zero): classify the
    // embedded IPv4 address. (:: and ::1 are already handled above.)
    if segments[..6].iter().all(|&s| s == 0) {
        let v4 = Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            (segments[6] & 0xff) as u8,
            (segments[7] >> 8) as u8,
            (segments[7] & 0xff) as u8,
        );
        return is_public_ipv4(v4);
    }

    let first = segments[0];
    // fe80::/10 — link-local unicast.
    if (first & 0xffc0) == 0xfe80 {
        return false;
    }
    // fc00::/7 — unique local addresses (ULA), including fd00::/8.
    if (first & 0xfe00) == 0xfc00 {
        return false;
    }
    // ff00::/8 — multicast.
    if (first & 0xff00) == 0xff00 {
        return false;
    }
    // 2001:db8::/32 — documentation (RFC 3849).
    if first == 0x2001 && segments[1] == 0x0db8 {
        return false;
    }
    // 100::/64 — discard-only address block (RFC 6666).
    if first == 0x0100 && segments[1] == 0 && segments[2] == 0 && segments[3] == 0 {
        return false;
    }

    true
}

/// Returns `true` for hostnames that name the local machine and so must never be
/// fetched, independent of DNS resolution (Req 13.5).
///
/// Exposed `pub(crate)` so the Media_Service (Req 18.5) reuses the exact same
/// local-name policy for its image-download SSRF check rather than duplicating it.
pub(crate) fn is_blocked_hostname(host: &str) -> bool {
    let normalized = host.trim().trim_end_matches('.').to_ascii_lowercase();
    normalized == "localhost"
        || normalized.ends_with(".localhost")
        || normalized == "localhost.localdomain"
        || normalized.ends_with(".localdomain")
}

// ===========================================================================
// Safety report types (Req 13.1)
// ===========================================================================

/// A structured safety report returned by [`scan`] (Req 13.1).
///
/// `level` is one of the [`SafetyLevel`] wire values (`safe`, `warn`,
/// `high-risk`, `blocked`); `findings` is a (possibly empty) list of individual
/// findings. `score` and `summary` are optional context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyReport {
    /// Overall safety classification.
    pub level: SafetyLevel,
    /// Individual findings; empty when nothing was flagged.
    pub findings: Vec<SafetyFinding>,
    /// Optional numeric safety score (0–100, higher is safer).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<i64>,
    /// Optional human-readable summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// A single safety finding (Req 13.1): a code, a severity, and a description.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyFinding {
    /// Machine-readable finding code (kebab-case).
    pub code: String,
    /// Severity: `info` | `warn` | `high`.
    pub severity: Severity,
    /// Short one-line title.
    pub title: String,
    /// Human-readable explanation of the risk.
    pub detail: String,
    /// Optional evidence snippet (truncated to 160 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// Outbound AI provider configuration for a safety scan (Req 13.1, 13.6).
///
/// All three fields must be present and non-empty for a scan to proceed; an
/// absent configuration (or any empty field) is treated as "no AI configuration
/// available" and rejected with `VALIDATION` (Req 13.6).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanAiConfig {
    /// Chat-completions endpoint URL.
    pub api_url: String,
    /// API key / bearer token.
    pub api_key: String,
    /// Model identifier.
    pub model: String,
}

impl ScanAiConfig {
    /// Returns `true` when every field is present and non-empty after trimming.
    fn is_usable(&self) -> bool {
        !self.api_url.trim().is_empty()
            && !self.api_key.trim().is_empty()
            && !self.model.trim().is_empty()
    }
}

/// A `SKILL.md` entry discovered in a repository listing (Req 13.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredSkill {
    /// Full repository-relative path to the `SKILL.md` file.
    pub path: String,
    /// Parent directory of the `SKILL.md` file (empty string at the repo root).
    pub directory: String,
}

// ===========================================================================
// Network limits (Req 13.3)
// ===========================================================================

/// Maximum number of redirects followed by [`fetch_content`] (Req 13.3).
const MAX_REDIRECTS: usize = 5;
/// Maximum response body size accepted by [`fetch_content`]: 5 MiB (Req 13.3).
const MAX_BODY_BYTES: u64 = 5 * 1024 * 1024;
/// Per-request timeout for [`fetch_content`]: 30 seconds (Req 13.3).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The system prompt instructing the AI provider to act as a skill security
/// auditor and emit a JSON report matching [`SafetyReport`]'s shape.
const AI_SAFETY_SYSTEM_PROMPT: &str = "You are a security auditor for AI skill files (SKILL.md). \
Analyze the provided skill content for security risks (shell injection, privilege escalation, \
data exfiltration, persistence, destructive commands, prompt injection, obfuscation). Respond \
with ONLY a JSON object of the form \
{\"level\":\"safe|warn|high-risk|blocked\",\"findings\":[{\"code\":\"kebab-case\",\
\"severity\":\"info|warn|high\",\"title\":\"...\",\"detail\":\"...\",\"evidence\":\"...\"}],\
\"summary\":\"...\"}. Output only the JSON object, with no markdown fences.";

// ===========================================================================
// Remote fetch with SSRF protection (Req 13.3, 13.5)
// ===========================================================================

/// Outcome of the synchronous, DNS-free URL precheck.
enum HostCheck {
    /// The host was an IP literal; the validated (public) address is carried.
    Literal(IpAddr),
    /// The host is a domain name that still requires DNS resolution.
    Domain(String),
}

/// Synchronously validates a URL's scheme and host against the SSRF policy
/// *without performing DNS* (Req 13.5).
///
/// Rejects with `SSRF_BLOCKED` when the scheme is not HTTPS, when the host names
/// the local machine, or when the host is an IP literal in a non-public range.
/// For IP-literal hosts the address is classified directly here (no DNS), which
/// is why `https://127.0.0.1/` and `https://[::1]/` are rejected without any
/// network access. For domain hosts the (validated-so-far) URL is returned with
/// [`HostCheck::Domain`] so the caller can resolve and re-check the addresses.
fn precheck_url(raw: &str) -> Result<(reqwest::Url, HostCheck), AppError> {
    let url = reqwest::Url::parse(raw)
        .map_err(|e| AppError::validation(format!("invalid URL `{raw}`: {e}")))?;

    if url.scheme() != "https" {
        return Err(AppError::ssrf_blocked(
            "only HTTPS URLs are allowed for remote skill fetch",
        ));
    }

    let host = url
        .host_str()
        .ok_or_else(|| AppError::validation("URL has no host"))?
        .to_string();

    // IPv6 literals appear bracketed in `host_str()` (e.g. `[::1]`); strip the
    // brackets before attempting to parse the host as an IP literal.
    let ip_candidate = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = ip_candidate.parse::<IpAddr>() {
        if !is_public_ip(ip) {
            return Err(AppError::ssrf_blocked(format!(
                "host `{host}` resolves to a non-public address"
            )));
        }
        return Ok((url, HostCheck::Literal(ip)));
    }

    // Named host: reject obvious local names before any DNS lookup.
    if is_blocked_hostname(&host) {
        return Err(AppError::ssrf_blocked(format!(
            "host `{host}` names the local machine"
        )));
    }

    Ok((url, HostCheck::Domain(host)))
}

/// Resolves a domain host to its addresses and verifies every one is public
/// (Req 13.5), returning the validated addresses.
///
/// Uses Tokio's async resolver so the runtime is never blocked. Any non-public
/// address rejects the whole host with `SSRF_BLOCKED`; a host that resolves to no
/// address is a `NETWORK` error.
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

/// Fetches remote skill content over HTTPS with full SSRF protection (Req 13.3,
/// 13.5).
///
/// Enforces, in order: HTTPS-only scheme; per-hop SSRF host classification
/// (every resolved address must be public) *before* connecting; at most
/// [`MAX_REDIRECTS`] redirects, re-checking the SSRF policy on each hop; a
/// [`REQUEST_TIMEOUT`] per-request deadline; and a [`MAX_BODY_BYTES`] response
/// body cap. On any policy violation, non-success status, exceeded limit, or
/// transport failure it returns a structured error without returning partial
/// content.
pub async fn fetch_content(url: &str) -> Result<String, AppError> {
    let mut current = url.to_string();

    // One initial request plus up to MAX_REDIRECTS redirect hops.
    for _ in 0..=MAX_REDIRECTS {
        // SSRF preflight for this hop (scheme + host), DNS-free for literals.
        let (parsed, host_check) = precheck_url(&current)?;
        let host = parsed
            .host_str()
            .ok_or_else(|| AppError::validation("URL has no host"))?
            .to_string();
        let port = parsed.port_or_known_default().unwrap_or(443);

        // Resolve + classify the addresses, then pin the client to exactly those
        // validated addresses so the connection cannot target a different host.
        let ips = match host_check {
            HostCheck::Literal(ip) => vec![ip],
            HostCheck::Domain(ref domain) => resolve_public_addrs(domain, port).await?,
        };
        let addrs: Vec<SocketAddr> = ips.iter().map(|ip| SocketAddr::new(*ip, port)).collect();

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .resolve_to_addrs(&host, &addrs)
            .build()
            .map_err(|e| map_reqwest_err("failed to build HTTP client", e))?;

        let response = client
            .get(parsed.clone())
            .send()
            .await
            .map_err(|e| map_reqwest_err("failed to fetch remote content", e))?;

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
                "remote fetch returned HTTP {}",
                status.as_u16()
            )));
        }

        // Reject oversize bodies up front when the server declares the length.
        if let Some(len) = response.content_length() {
            if len > MAX_BODY_BYTES {
                return Err(AppError::network(
                    "remote content exceeds the 5 MB size limit",
                ));
            }
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| map_reqwest_err("failed to read remote content", e))?;
        if bytes.len() as u64 > MAX_BODY_BYTES {
            return Err(AppError::network(
                "remote content exceeds the 5 MB size limit",
            ));
        }

        return String::from_utf8(bytes.to_vec())
            .map_err(|e| AppError::parse(format!("remote content is not valid UTF-8: {e}")));
    }

    Err(AppError::network(
        "too many redirects while fetching remote content",
    ))
}

// ===========================================================================
// AI safety scan (Req 13.1, 13.6)
// ===========================================================================

/// Scans skill `content` with the configured AI provider and returns a structured
/// [`SafetyReport`] (Req 13.1).
///
/// When no usable AI configuration is available (absent, or any field empty) the
/// request is rejected with `VALIDATION` and no report is produced (Req 13.6).
/// The HTTP call is isolated in [`chat_completion`] and the response parsing in
/// [`parse_report`], so both halves are independently testable.
pub async fn scan(
    content: &str,
    ai_config: Option<ScanAiConfig>,
) -> Result<SafetyReport, AppError> {
    let config = match ai_config {
        Some(c) if c.is_usable() => c,
        _ => {
            return Err(AppError::validation(
                "an AI provider configuration is required to scan skill safety",
            ))
        }
    };

    let raw = chat_completion(&config, content).await?;
    parse_report(&raw)
}

/// Issues the chat-completions request to the configured AI provider and returns
/// the assistant message content (the raw report text).
///
/// Isolated from [`scan`] so the report-parsing logic can be tested without any
/// network I/O. The configured endpoint is the user's own provider (mirroring the
/// AI_Client, Requirement 16) and is therefore not subject to the SSRF policy
/// that governs untrusted skill-content fetches.
async fn chat_completion(config: &ScanAiConfig, content: &str) -> Result<String, AppError> {
    let body = serde_json::json!({
        "model": config.model,
        "temperature": 0.2,
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": AI_SAFETY_SYSTEM_PROMPT },
            { "role": "user", "content": content },
        ],
    });

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| map_reqwest_err("failed to build AI client", e))?;

    let request_body = serde_json::to_vec(&body)
        .map_err(|e| AppError::internal(format!("failed to encode AI request: {e}")))?;

    let response = client
        .post(&config.api_url)
        .bearer_auth(&config.api_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(request_body)
        .send()
        .await
        .map_err(|e| map_reqwest_err("AI provider request failed", e))?;

    if !response.status().is_success() {
        return Err(AppError::network(format!(
            "AI provider returned HTTP {}",
            response.status().as_u16()
        )));
    }

    let raw = response
        .bytes()
        .await
        .map_err(|e| map_reqwest_err("failed to read AI provider response", e))?;
    let payload: Value = serde_json::from_slice(&raw)
        .map_err(|e| AppError::parse(format!("AI provider response is not valid JSON: {e}")))?;

    payload
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| AppError::parse("AI provider response had no message content"))
}

/// Parses the raw AI response text into a validated [`SafetyReport`] (Req 13.1).
///
/// Tolerates a leading/trailing markdown code fence, requires a valid `level`
/// (one of the [`SafetyLevel`] wire values), and keeps only findings that carry a
/// string `code`, a valid [`Severity`], and string `title`/`detail`. Evidence is
/// truncated to 160 characters. A response that is not JSON, or whose `level` is
/// missing/invalid, is a `PARSE` error. Pure and network-free for testability.
pub fn parse_report(raw: &str) -> Result<SafetyReport, AppError> {
    let cleaned = strip_code_fence(raw.trim());

    let value: Value = serde_json::from_str(cleaned)
        .map_err(|e| AppError::parse(format!("AI report is not valid JSON: {e}")))?;

    let level: SafetyLevel = value
        .get("level")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .ok_or_else(|| AppError::parse("AI report is missing a valid `level`"))?;

    let mut findings = Vec::new();
    if let Some(items) = value.get("findings").and_then(Value::as_array) {
        for item in items {
            if let Some(finding) = parse_finding(item) {
                findings.push(finding);
            }
        }
    }

    let summary = value
        .get("summary")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);

    let score = value.get("score").and_then(Value::as_i64);

    Ok(SafetyReport {
        level,
        findings,
        score,
        summary,
    })
}

/// Strips a single leading/trailing triple-backtick fence (optionally tagged
/// ```` ```json ````) from `text`, returning the inner content.
fn strip_code_fence(text: &str) -> &str {
    let Some(stripped) = text.strip_prefix("```") else {
        return text;
    };
    // Drop the optional language tag on the opening fence line.
    let after_tag = match stripped.find('\n') {
        Some(idx) => &stripped[idx + 1..],
        None => stripped,
    };
    after_tag
        .trim_end()
        .strip_suffix("```")
        .unwrap_or(after_tag)
        .trim()
}

/// Parses one raw finding object, returning `None` when required fields are
/// missing or malformed (such entries are skipped rather than failing the scan).
fn parse_finding(item: &Value) -> Option<SafetyFinding> {
    let code = item.get("code")?.as_str()?.to_owned();
    let severity: Severity = serde_json::from_value(item.get("severity")?.clone()).ok()?;
    let title = item.get("title")?.as_str()?.to_owned();
    let detail = item.get("detail")?.as_str()?.to_owned();
    let evidence = item
        .get("evidence")
        .and_then(Value::as_str)
        .map(|s| s.chars().take(160).collect::<String>());

    Some(SafetyFinding {
        code,
        severity,
        title,
        detail,
        evidence,
    })
}

// ===========================================================================
// Persisting a report (Req 13.2, 13.7)
// ===========================================================================

/// Persists a safety report onto a skill record and returns the updated skill
/// (Req 13.2).
///
/// Writes `safety_level`, `safety_score` (when present), the full `safety_report`
/// JSON, and `safety_scanned_at = now`. Returns `NOT_FOUND` when the skill does
/// not exist, persisting nothing (Req 13.7). A subsequent [`skill::get`] returns
/// the saved level, score, and report.
pub fn save_report(
    conn: &Connection,
    skill_id: &str,
    report: &SafetyReport,
) -> Result<Skill, AppError> {
    // NOT_FOUND (and no write) when the skill does not exist (Req 13.7).
    skill::get(conn, skill_id)?;

    let level = serde_json::to_value(report.level)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| AppError::internal("failed to encode safety level"))?;
    let report_json = serde_json::to_string(report)
        .map_err(|e| AppError::internal(format!("failed to encode safety report: {e}")))?;
    let now = now_millis();

    conn.execute(
        "UPDATE skills SET safety_level = ?1, safety_score = ?2, safety_report = ?3, \
         safety_scanned_at = ?4, updated_at = ?5 WHERE id = ?6",
        params![level, report.score, report_json, now, now, skill_id],
    )
    .map_err(|e| AppError::internal(format!("failed to save safety report: {e}")))?;

    skill::get(conn, skill_id)
}

// ===========================================================================
// Repository scanning (Req 13.4)
// ===========================================================================

/// Scans a remote GitHub repository listing for `SKILL.md` entries (Req 13.4).
///
/// Fetches `listing_url` (a GitHub tree/contents API URL) through
/// [`fetch_content`] — which enforces the HTTPS/SSRF policy — and parses the
/// payload with [`parse_repo_listing`]. Returns the discovered skills, or a
/// structured error when the listing cannot be retrieved or parsed.
pub async fn scan_repo(listing_url: &str) -> Result<Vec<DiscoveredSkill>, AppError> {
    let payload = fetch_content(listing_url).await?;
    parse_repo_listing(&payload)
}

/// Parses a GitHub repository listing payload into discovered `SKILL.md` entries
/// (Req 13.4). Pure and network-free for testability.
///
/// Accepts either the Git Trees API shape (`{ "tree": [ { "path", "type" } ] }`,
/// where files are `"blob"`) or the Contents API shape (a top-level array, where
/// files are `"file"`). An entry is a discovered skill when it is a file whose
/// path's final component is exactly `SKILL.md`.
pub fn parse_repo_listing(payload: &str) -> Result<Vec<DiscoveredSkill>, AppError> {
    let value: Value = serde_json::from_str(payload)
        .map_err(|e| AppError::parse(format!("repository listing is not valid JSON: {e}")))?;

    let entries = if let Some(tree) = value.get("tree").and_then(Value::as_array) {
        tree
    } else if let Some(array) = value.as_array() {
        array
    } else {
        return Err(AppError::parse(
            "repository listing did not contain a file tree",
        ));
    };

    let mut discovered = Vec::new();
    for entry in entries {
        let Some(path) = entry.get("path").and_then(Value::as_str) else {
            continue;
        };
        // Skip directory entries: trees use `tree`, contents use `dir`.
        if let Some(kind) = entry.get("type").and_then(Value::as_str) {
            if kind == "tree" || kind == "dir" {
                continue;
            }
        }
        if file_name(path) != "SKILL.md" {
            continue;
        }
        discovered.push(DiscoveredSkill {
            path: path.to_string(),
            directory: parent_dir(path).to_string(),
        });
    }

    Ok(discovered)
}

/// Returns the final `/`-separated component of `path`.
fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Returns the parent directory of a `/`-separated `path` (empty string at root).
fn parent_dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(idx) => &path[..idx],
        None => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::services::skill::{self, SkillCreate};
    use crate::storage::{create_memory_pool, init_schema, DbPool};
    use std::net::{Ipv4Addr, Ipv6Addr};

    // --- SSRF classifier: IPv4 (Req 13.5) ----------------------------------

    /// Parses a dotted-quad string into an `IpAddr` for concise assertions.
    fn v4(s: &str) -> IpAddr {
        IpAddr::V4(s.parse::<Ipv4Addr>().unwrap())
    }

    /// Parses an IPv6 string into an `IpAddr` for concise assertions.
    fn v6(s: &str) -> IpAddr {
        IpAddr::V6(s.parse::<Ipv6Addr>().unwrap())
    }

    #[test]
    fn classifier_rejects_ipv4_loopback() {
        assert!(!is_public_ip(v4("127.0.0.1")));
        assert!(!is_public_ip(v4("127.255.255.255")));
    }

    #[test]
    fn classifier_rejects_ipv4_private_ranges() {
        assert!(!is_public_ip(v4("10.0.0.1")));
        assert!(!is_public_ip(v4("10.255.255.255")));
        assert!(!is_public_ip(v4("172.16.0.1")));
        assert!(!is_public_ip(v4("172.31.255.255")));
        assert!(!is_public_ip(v4("192.168.0.1")));
        assert!(!is_public_ip(v4("192.168.255.255")));
    }

    #[test]
    fn classifier_rejects_ipv4_link_local_and_cgnat() {
        assert!(!is_public_ip(v4("169.254.0.1")));
        assert!(!is_public_ip(v4("169.254.255.255")));
        // Carrier-grade NAT 100.64.0.0/10.
        assert!(!is_public_ip(v4("100.64.0.1")));
        assert!(!is_public_ip(v4("100.127.255.255")));
    }

    #[test]
    fn classifier_rejects_ipv4_unspecified_and_broadcast() {
        assert!(!is_public_ip(v4("0.0.0.0")));
        assert!(!is_public_ip(v4("0.1.2.3")));
        assert!(!is_public_ip(v4("255.255.255.255")));
    }

    #[test]
    fn classifier_rejects_ipv4_documentation_and_reserved() {
        assert!(!is_public_ip(v4("192.0.2.1"))); // TEST-NET-1
        assert!(!is_public_ip(v4("198.51.100.1"))); // TEST-NET-2
        assert!(!is_public_ip(v4("203.0.113.1"))); // TEST-NET-3
        assert!(!is_public_ip(v4("198.18.0.1"))); // benchmarking
        assert!(!is_public_ip(v4("192.0.0.1"))); // IETF protocol assignments
        assert!(!is_public_ip(v4("224.0.0.1"))); // multicast
        assert!(!is_public_ip(v4("240.0.0.1"))); // reserved
    }

    #[test]
    fn classifier_accepts_public_ipv4() {
        assert!(is_public_ip(v4("8.8.8.8")));
        assert!(is_public_ip(v4("1.1.1.1")));
        assert!(is_public_ip(v4("140.82.121.3"))); // github.com
        assert!(is_public_ip(v4("172.15.255.255"))); // just below 172.16/12
        assert!(is_public_ip(v4("172.32.0.1"))); // just above 172.16/12
        assert!(is_public_ip(v4("100.63.255.255"))); // just below CGNAT
        assert!(is_public_ip(v4("100.128.0.0"))); // just above CGNAT
    }

    // --- SSRF classifier: IPv6 (Req 13.5) ----------------------------------

    #[test]
    fn classifier_rejects_ipv6_loopback_and_unspecified() {
        assert!(!is_public_ip(v6("::1"))); // loopback
        assert!(!is_public_ip(v6("::"))); // unspecified
    }

    #[test]
    fn classifier_rejects_ipv6_link_local_and_ula() {
        assert!(!is_public_ip(v6("fe80::1"))); // link-local
        assert!(!is_public_ip(v6("febf::1"))); // top of fe80::/10
        assert!(!is_public_ip(v6("fc00::1"))); // ULA
        assert!(!is_public_ip(v6("fd00::1"))); // ULA
        assert!(!is_public_ip(v6("ff02::1"))); // multicast
    }

    #[test]
    fn classifier_rejects_ipv6_documentation_and_discard() {
        assert!(!is_public_ip(v6("2001:db8::1"))); // documentation
        assert!(!is_public_ip(v6("100::1"))); // discard-only
    }

    #[test]
    fn classifier_rejects_ipv6_mapped_and_compatible_private_v4() {
        // IPv4-mapped loopback / private must be rejected via the embedded v4.
        assert!(!is_public_ip(v6("::ffff:127.0.0.1")));
        assert!(!is_public_ip(v6("::ffff:10.0.0.1")));
        // IPv4-compatible (deprecated) embedding a private address.
        assert!(!is_public_ip(v6("::192.168.1.1")));
    }

    #[test]
    fn classifier_accepts_public_ipv6() {
        assert!(is_public_ip(v6("2606:4700:4700::1111"))); // Cloudflare DNS
        assert!(is_public_ip(v6("2001:4860:4860::8888"))); // Google DNS
                                                           // IPv4-mapped public address is public.
        assert!(is_public_ip(v6("::ffff:8.8.8.8")));
    }

    #[test]
    fn blocked_hostname_matches_localhost_forms() {
        assert!(is_blocked_hostname("localhost"));
        assert!(is_blocked_hostname("LOCALHOST"));
        assert!(is_blocked_hostname("foo.localhost"));
        assert!(is_blocked_hostname("localhost.localdomain"));
        assert!(is_blocked_hostname("box.localdomain"));
        assert!(!is_blocked_hostname("example.com"));
        assert!(!is_blocked_hostname("github.com"));
    }

    // --- fetch_content rejection (Req 13.3, 13.5) --------------------------
    // These reject in the synchronous precheck, so no DNS/network occurs.

    #[tokio::test]
    async fn fetch_rejects_non_https_scheme() {
        let err = fetch_content("http://example.com/skill").await.unwrap_err();
        assert_eq!(err.code, ErrorCode::SsrfBlocked);
    }

    #[tokio::test]
    async fn fetch_rejects_non_http_scheme() {
        let err = fetch_content("ftp://example.com/skill").await.unwrap_err();
        assert_eq!(err.code, ErrorCode::SsrfBlocked);
    }

    #[tokio::test]
    async fn fetch_rejects_ipv4_loopback_literal_without_dns() {
        let err = fetch_content("https://127.0.0.1/").await.unwrap_err();
        assert_eq!(err.code, ErrorCode::SsrfBlocked);
    }

    #[tokio::test]
    async fn fetch_rejects_ipv6_loopback_literal_without_dns() {
        let err = fetch_content("https://[::1]/").await.unwrap_err();
        assert_eq!(err.code, ErrorCode::SsrfBlocked);
    }

    #[tokio::test]
    async fn fetch_rejects_private_literal_and_localhost_name() {
        assert_eq!(
            fetch_content("https://10.0.0.5/x").await.unwrap_err().code,
            ErrorCode::SsrfBlocked
        );
        assert_eq!(
            fetch_content("https://192.168.1.1/x")
                .await
                .unwrap_err()
                .code,
            ErrorCode::SsrfBlocked
        );
        assert_eq!(
            fetch_content("https://localhost/x").await.unwrap_err().code,
            ErrorCode::SsrfBlocked
        );
    }

    #[tokio::test]
    async fn fetch_rejects_malformed_url() {
        let err = fetch_content("not a url").await.unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    // --- scan requires AI config (Req 13.6) --------------------------------

    #[tokio::test]
    async fn scan_without_config_is_validation_error() {
        let err = scan("# Skill", None).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[tokio::test]
    async fn scan_with_empty_config_fields_is_validation_error() {
        let config = ScanAiConfig {
            api_url: "".into(),
            api_key: "key".into(),
            model: "gpt".into(),
        };
        let err = scan("# Skill", Some(config)).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    // --- report parsing (Req 13.1) -----------------------------------------

    #[test]
    fn parse_report_reads_level_findings_and_summary() {
        let raw = r#"{
            "level": "high-risk",
            "summary": "Found risky commands.",
            "score": 30,
            "findings": [
                { "code": "privilege-escalation", "severity": "high",
                  "title": "sudo use", "detail": "invokes sudo", "evidence": "sudo rm" }
            ]
        }"#;
        let report = parse_report(raw).unwrap();
        assert_eq!(report.level, SafetyLevel::HighRisk);
        assert_eq!(report.summary.as_deref(), Some("Found risky commands."));
        assert_eq!(report.score, Some(30));
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].code, "privilege-escalation");
        assert_eq!(report.findings[0].severity, Severity::High);
    }

    #[test]
    fn parse_report_strips_markdown_code_fence() {
        let raw = "```json\n{\"level\":\"safe\",\"findings\":[]}\n```";
        let report = parse_report(raw).unwrap();
        assert_eq!(report.level, SafetyLevel::Safe);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn parse_report_skips_malformed_findings() {
        let raw = r#"{
            "level": "warn",
            "findings": [
                { "code": "ok", "severity": "warn", "title": "t", "detail": "d" },
                { "code": "missing-severity", "title": "t", "detail": "d" },
                { "severity": "high", "title": "t", "detail": "d" }
            ]
        }"#;
        let report = parse_report(raw).unwrap();
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].code, "ok");
    }

    #[test]
    fn parse_report_rejects_invalid_level() {
        assert_eq!(
            parse_report(r#"{"level":"nope","findings":[]}"#)
                .unwrap_err()
                .code,
            ErrorCode::Parse
        );
    }

    #[test]
    fn parse_report_rejects_non_json() {
        assert_eq!(parse_report("not json").unwrap_err().code, ErrorCode::Parse);
    }

    // --- save_report persistence (Req 13.2, 13.7) --------------------------

    fn schema_pool() -> DbPool {
        let pool = create_memory_pool().expect("memory pool");
        init_schema(&pool.get().expect("conn")).expect("schema");
        pool
    }

    fn sample_report() -> SafetyReport {
        SafetyReport {
            level: SafetyLevel::Warn,
            findings: vec![SafetyFinding {
                code: "network-bootstrap".into(),
                severity: Severity::Warn,
                title: "downloads".into(),
                detail: "fetches remote resources".into(),
                evidence: Some("curl https://x".into()),
            }],
            score: Some(70),
            summary: Some("one warning".into()),
        }
    }

    #[test]
    fn save_report_persists_fields_and_round_trips() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let created = skill::create(
            &conn,
            SkillCreate {
                name: "Scanned".into(),
                ..Default::default()
            },
        )
        .unwrap();

        let report = sample_report();
        let updated = save_report(&conn, &created.id, &report).unwrap();

        assert_eq!(updated.safety_level, Some(SafetyLevel::Warn));
        assert_eq!(updated.safety_score, Some(70));
        assert!(updated.safety_scanned_at.is_some());

        // A subsequent get returns the saved level + report (Req 13.2).
        let fetched = skill::get(&conn, &created.id).unwrap();
        assert_eq!(fetched.safety_level, Some(SafetyLevel::Warn));
        let stored: SafetyReport =
            serde_json::from_value(fetched.safety_report.clone().unwrap()).unwrap();
        assert_eq!(stored, report);
    }

    #[test]
    fn save_report_missing_skill_returns_not_found_without_writing() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let err = save_report(&conn, "does-not-exist", &sample_report()).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);

        // Nothing was persisted.
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skills", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    // --- repository listing parse (Req 13.4) -------------------------------

    #[test]
    fn parse_repo_listing_reads_git_trees_shape() {
        // GitHub Git Trees API shape: files are `blob`, dirs are `tree`.
        let payload = r#"{
            "sha": "abc",
            "tree": [
                { "path": "SKILL.md", "type": "blob" },
                { "path": "skills", "type": "tree" },
                { "path": "skills/writer/SKILL.md", "type": "blob" },
                { "path": "skills/writer/README.md", "type": "blob" },
                { "path": "skills/coder", "type": "tree" },
                { "path": "skills/coder/SKILL.md", "type": "blob" }
            ]
        }"#;
        let discovered = parse_repo_listing(payload).unwrap();
        let paths: Vec<&str> = discovered.iter().map(|d| d.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "SKILL.md",
                "skills/writer/SKILL.md",
                "skills/coder/SKILL.md"
            ]
        );
        // Directory derivation.
        assert_eq!(discovered[0].directory, "");
        assert_eq!(discovered[1].directory, "skills/writer");
        assert_eq!(discovered[2].directory, "skills/coder");
    }

    #[test]
    fn parse_repo_listing_reads_contents_array_shape() {
        // GitHub Contents API shape: a top-level array; files are `file`.
        let payload = r#"[
            { "path": "SKILL.md", "type": "file" },
            { "path": "docs", "type": "dir" },
            { "path": "notes.txt", "type": "file" }
        ]"#;
        let discovered = parse_repo_listing(payload).unwrap();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].path, "SKILL.md");
        assert_eq!(discovered[0].directory, "");
    }

    #[test]
    fn parse_repo_listing_returns_empty_when_no_skill_md() {
        let payload = r#"{ "tree": [ { "path": "README.md", "type": "blob" } ] }"#;
        assert!(parse_repo_listing(payload).unwrap().is_empty());
    }

    #[test]
    fn parse_repo_listing_rejects_non_json() {
        assert_eq!(
            parse_repo_listing("not json").unwrap_err().code,
            ErrorCode::Parse
        );
    }

    #[test]
    fn parse_repo_listing_rejects_unexpected_shape() {
        assert_eq!(
            parse_repo_listing(r#"{ "unexpected": true }"#)
                .unwrap_err()
                .code,
            ErrorCode::Parse
        );
    }
}
