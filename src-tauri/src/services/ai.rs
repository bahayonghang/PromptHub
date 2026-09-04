//! AI_Client: outbound HTTP(S) requests to AI providers, including streaming
//! (Requirement 16).
//!
//! Like the sibling services this module is written against *injected*
//! dependencies rather than reaching into global [`crate::state::AppState`] or a
//! live Tauri window, so the rules are unit-testable without real network I/O:
//!
//! 1. [`request`] — issues a non-streaming request with a 120-second timeout and
//!    returns `{ requestId, content }` where `content` is the provider response
//!    body, or a structured error on timeout / network failure / provider error
//!    response (16.1, 16.4, 16.5).
//! 2. [`stream`] — issues a streaming request and reports progress through an
//!    injected [`EventSink`]: ordered `ai:stream-chunk` events, then exactly one
//!    `ai:stream-complete` carrying the in-order concatenation of all chunks, or a
//!    single `ai:stream-error`; every event carries the request id (16.2, 16.3,
//!    16.6). The stream-driving core ([`drive_stream`]) is generic over the byte
//!    source so it can be exercised with an in-memory stream and a recording sink.
//!
//! ## Cancellation (16.7)
//!
//! The Command_Layer registers a [`CancellationToken`] in
//! [`crate::state::AppState`]'s request registry (via `register_request`) and
//! hands it to [`stream`]; a later `ai.cancel` cancels that token (via
//! `cancel_request`). [`drive_stream`] selects on the token each iteration: once
//! cancelled it aborts the outbound request (the stream future is dropped) and
//! emits **no** further chunk, completion, or error events for that request.
//!
//! ## Events through an abstraction (testability)
//!
//! The service never depends on Tauri. It emits through the [`EventSink`] trait;
//! task 17.1 provides a Tauri-`AppHandle`-backed implementation that forwards to
//! the `ai:stream-chunk` / `ai:stream-complete` / `ai:stream-error` event
//! channels. Tests provide an in-memory recording sink so the ordered-chunk
//! concatenation property (Property 35, task 11.2) is checkable without a window.
//!
//! ## Outbound SSRF
//!
//! Every hop uses [`crate::services::network_safety::prepare_public_url`]. The
//! client disables automatic redirects and re-checks each Location. Loopback,
//! RFC1918, link-local, and metadata addresses return `SSRF_BLOCKED` with no
//! TCP connect unless `allow_private_network` is true. Cross-host redirects
//! drop `Authorization`.
#![allow(dead_code)]

use std::collections::HashMap;
use std::time::Duration;

use futures_util::{Stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::services::network_safety::{prepare_public_url, same_http_origin, MAX_REDIRECTS};

/// Per-request timeout for both non-streaming and streaming requests: 120
/// seconds (16.1, 16.4). For a streaming request this bounds the whole stream;
/// when it elapses `reqwest` yields a timeout error that surfaces as a single
/// `ai:stream-error`.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// User-Agent sent with every outbound AI request.
const USER_AGENT: &str = "PromptHub/1.0";

// ===========================================================================
// Request / response DTOs
// ===========================================================================

/// An outbound AI provider request (mirrors the Reference_App's transport shape).
///
/// `requestId` correlates every result and event back to the originating request
/// (16.3). `body` is the already-serialized provider payload (the AI_Client is a
/// transport: it does not construct provider-specific request bodies).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiRequest {
    /// Unique identifier the Frontend assigns to correlate result/events (16.3).
    pub request_id: String,
    /// HTTP method; defaults to `POST` when omitted. Only `GET`/`POST` are valid.
    #[serde(default = "default_method")]
    pub method: String,
    /// Absolute HTTP(S) endpoint URL of the configured provider.
    pub url: String,
    /// Request headers (e.g. `Authorization`, `Content-Type`).
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Serialized request body, when present.
    #[serde(default)]
    pub body: Option<String>,
}

fn default_method() -> String {
    "POST".to_string()
}

/// The result of a non-streaming request: the originating id and the provider
/// response body (16.1).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiResponse {
    /// The originating request identifier (16.1, 16.3).
    pub request_id: String,
    /// The provider response body content.
    pub content: String,
}

// ===========================================================================
// Event sink (Tauri-free abstraction over the ai:stream-* channels)
// ===========================================================================

/// Receives the streaming progress events the AI_Client would otherwise emit
/// directly on the `ai:stream-*` Tauri channels.
///
/// Implemented by the Command_Layer over a Tauri `AppHandle` (task 17.1) and by a
/// recording sink in tests. Every method carries the `request_id` so the Frontend
/// can correlate the event to its request (16.3).
pub trait EventSink: Send + Sync {
    /// Emits an `ai:stream-chunk` with a piece of response content, in order.
    fn emit_chunk(&self, request_id: &str, chunk: &str);
    /// Emits the single terminal `ai:stream-complete` with the full content.
    fn emit_complete(&self, request_id: &str, content: &str);
    /// Emits the single terminal `ai:stream-error` with a failure reason.
    fn emit_error(&self, request_id: &str, error: &str);
}

// ===========================================================================
// Validation
// ===========================================================================

/// Validates a request and returns the parsed `(method, url)`.
///
/// Rejects an empty `requestId`/`url`, an unsupported method, and any URL that is
/// not a well-formed HTTP(S) URL — all with `VALIDATION`, before any client is
/// built or any address is contacted.
fn validate(req: &AiRequest) -> Result<(reqwest::Method, reqwest::Url), AppError> {
    if req.request_id.trim().is_empty() {
        return Err(AppError::validation("requestId is required"));
    }
    if req.url.trim().is_empty() {
        return Err(AppError::validation("url is required"));
    }

    let method = match req.method.trim().to_ascii_uppercase().as_str() {
        "POST" => reqwest::Method::POST,
        "GET" => reqwest::Method::GET,
        other => {
            return Err(AppError::validation(format!(
                "unsupported HTTP method `{other}` (expected GET or POST)"
            )))
        }
    };

    let url = reqwest::Url::parse(req.url.trim())
        .map_err(|e| AppError::validation(format!("invalid URL `{}`: {e}", req.url)))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::validation(
            "url must use the http or https scheme",
        ));
    }

    Ok((method, url))
}

/// Maps a `reqwest` transport error to a structured [`AppError`] (16.4).
fn map_reqwest_err(context: &str, e: reqwest::Error) -> AppError {
    if e.is_timeout() {
        AppError::timeout(format!("{context}: request timed out"))
    } else {
        AppError::network(format!("{context}: {e}"))
    }
}

// ===========================================================================
// Client construction
// ===========================================================================

/// Applies method, URL, headers, and body to a fresh request builder.
fn build_request(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: reqwest::Url,
    req: &AiRequest,
    forward_authorization: bool,
) -> reqwest::RequestBuilder {
    let mut builder = client.request(method, url);
    builder = builder.header(reqwest::header::USER_AGENT, USER_AGENT);
    for (name, value) in &req.headers {
        if !forward_authorization && name.eq_ignore_ascii_case("authorization") {
            continue;
        }
        builder = builder.header(name.as_str(), value.as_str());
    }
    if let Some(body) = &req.body {
        builder = builder.body(body.clone());
    }
    builder
}

async fn send_following_redirects(
    req: &AiRequest,
    method: reqwest::Method,
    allow_private_network: bool,
) -> Result<reqwest::Response, AppError> {
    let mut current = req.url.trim().to_string();
    let mut allow = allow_private_network;
    let mut forward_authorization = true;

    for _ in 0..=MAX_REDIRECTS {
        let (url, client) = prepare_public_url(&current, REQUEST_TIMEOUT, allow).await?;
        let response = build_request(
            &client,
            method.clone(),
            url.clone(),
            req,
            forward_authorization,
        )
        .send()
        .await
        .map_err(|e| map_reqwest_err("AI provider request failed", e))?;

        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| AppError::network("AI provider redirect omitted Location"))?;
            let next = url
                .join(location)
                .map_err(|e| AppError::network(format!("invalid AI provider redirect: {e}")))?;
            forward_authorization = same_http_origin(&url, &next);
            allow = allow_private_network && forward_authorization;
            current = next.to_string();
            continue;
        }

        return Ok(response);
    }

    Err(AppError::network("too many AI provider redirects"))
}

// ===========================================================================
// Non-streaming request (16.1, 16.4, 16.5)
// ===========================================================================

/// Issues a non-streaming AI request and returns `{ requestId, content }` on a
/// successful provider response, or a structured error on timeout, network
/// failure, or a provider error response (16.1, 16.4, 16.5).
pub async fn request(req: &AiRequest, allow_private_network: bool) -> Result<AiResponse, AppError> {
    let (method, _) = validate(req)?;
    let response = send_following_redirects(req, method, allow_private_network).await?;

    let status = response.status();
    if !status.is_success() {
        return Err(AppError::network(format!(
            "AI provider returned HTTP {}",
            status.as_u16()
        )));
    }

    let content = response
        .text()
        .await
        .map_err(|e| map_reqwest_err("failed to read AI provider response", e))?;

    Ok(AiResponse {
        request_id: req.request_id.clone(),
        content,
    })
}

// ===========================================================================
// Streaming request (16.2, 16.3, 16.6, 16.7)
// ===========================================================================

/// Issues a streaming AI request and reports progress through `sink`.
///
/// On success: ordered [`EventSink::emit_chunk`] calls followed by exactly one
/// [`EventSink::emit_complete`] carrying the in-order concatenation of every
/// emitted chunk (16.2, 16.6). On a connection failure, a provider error
/// response, or a mid-stream read error: a single [`EventSink::emit_error`] and
/// no further events (16.4). If `cancel` is cancelled at any point the outbound
/// request is aborted and no further events are emitted (16.7).
pub async fn stream(
    req: &AiRequest,
    cancel: &CancellationToken,
    sink: &dyn EventSink,
    allow_private_network: bool,
) {
    let (method, _) = match validate(req) {
        Ok(parts) => parts,
        Err(e) => {
            if !cancel.is_cancelled() {
                sink.emit_error(&req.request_id, &e.message);
            }
            return;
        }
    };

    let send = send_following_redirects(req, method, allow_private_network);
    let response = tokio::select! {
        biased;
        _ = cancel.cancelled() => return,
        result = send => match result {
            Ok(response) => response,
            Err(e) => {
                if !cancel.is_cancelled() {
                    sink.emit_error(&req.request_id, &e.message);
                }
                return;
            }
        },
    };

    let status = response.status();
    if !status.is_success() {
        if !cancel.is_cancelled() {
            sink.emit_error(
                &req.request_id,
                &format!("AI provider returned HTTP {}", status.as_u16()),
            );
        }
        return;
    }

    let byte_stream = Box::pin(
        response
            .bytes_stream()
            .map_err(|e| map_reqwest_err("AI stream read failed", e)),
    );

    drive_stream(&req.request_id, byte_stream, cancel, sink).await;
}

/// Drives a byte stream to completion, emitting ordered chunks and a single
/// terminal event through `sink`.
///
/// Generic over the byte source so it can be exercised with an in-memory stream
/// and a recording sink (no network). Decodes bytes incrementally so a multi-byte
/// UTF-8 sequence split across network packets is never corrupted, accumulating
/// the decoded text so the completion content equals the in-order concatenation
/// of every emitted chunk (16.6). Selects on `cancel` each iteration: once
/// cancelled it returns immediately, emitting no further events (16.7).
async fn drive_stream<B, S>(
    request_id: &str,
    mut stream: S,
    cancel: &CancellationToken,
    sink: &dyn EventSink,
) where
    B: AsRef<[u8]>,
    S: Stream<Item = Result<B, AppError>> + Unpin,
{
    let mut decoder = Utf8StreamDecoder::new();
    let mut content = String::new();

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return,
            item = stream.next() => match item {
                Some(Ok(bytes)) => {
                    let text = decoder.push(bytes.as_ref());
                    if !text.is_empty() {
                        content.push_str(&text);
                        sink.emit_chunk(request_id, &text);
                    }
                }
                Some(Err(e)) => {
                    if !cancel.is_cancelled() {
                        sink.emit_error(request_id, &e.message);
                    }
                    return;
                }
                None => {
                    let tail = decoder.flush();
                    if !tail.is_empty() {
                        content.push_str(&tail);
                        sink.emit_chunk(request_id, &tail);
                    }
                    if !cancel.is_cancelled() {
                        sink.emit_complete(request_id, &content);
                    }
                    return;
                }
            },
        }
    }
}

// ===========================================================================
// Incremental UTF-8 decoder
// ===========================================================================

/// Decodes a byte stream to text incrementally, holding back a trailing
/// incomplete multi-byte UTF-8 sequence until the bytes that complete it arrive.
///
/// This keeps chunk content valid even when a provider's network packets split a
/// multi-byte character across reads. Any bytes still buffered when the stream
/// ends are flushed with lossy replacement.
struct Utf8StreamDecoder {
    buf: Vec<u8>,
}

impl Utf8StreamDecoder {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Appends `bytes` and returns the longest now-decodable valid-UTF-8 prefix,
    /// retaining any trailing incomplete sequence for the next call.
    fn push(&mut self, bytes: &[u8]) -> String {
        self.buf.extend_from_slice(bytes);
        match std::str::from_utf8(&self.buf) {
            Ok(s) => {
                let out = s.to_string();
                self.buf.clear();
                out
            }
            Err(e) => {
                let valid_up_to = e.valid_up_to();
                // Bytes in `[0, valid_up_to)` are valid UTF-8 by definition.
                let out =
                    String::from_utf8(self.buf[..valid_up_to].to_vec()).expect("valid prefix");
                self.buf.drain(..valid_up_to);
                out
            }
        }
    }

    /// Flushes any remaining buffered bytes, lossily decoding an incomplete or
    /// invalid trailing sequence.
    fn flush(self) -> String {
        if self.buf.is_empty() {
            String::new()
        } else {
            String::from_utf8_lossy(&self.buf).into_owned()
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// A recording [`EventSink`] capturing the ordered sequence of emitted events.
    #[derive(Debug, Default)]
    struct RecordingSink {
        events: Mutex<Vec<SinkEvent>>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum SinkEvent {
        Chunk { request_id: String, chunk: String },
        Complete { request_id: String, content: String },
        Error { request_id: String, error: String },
    }

    impl EventSink for RecordingSink {
        fn emit_chunk(&self, request_id: &str, chunk: &str) {
            self.events.lock().unwrap().push(SinkEvent::Chunk {
                request_id: request_id.to_string(),
                chunk: chunk.to_string(),
            });
        }
        fn emit_complete(&self, request_id: &str, content: &str) {
            self.events.lock().unwrap().push(SinkEvent::Complete {
                request_id: request_id.to_string(),
                content: content.to_string(),
            });
        }
        fn emit_error(&self, request_id: &str, error: &str) {
            self.events.lock().unwrap().push(SinkEvent::Error {
                request_id: request_id.to_string(),
                error: error.to_string(),
            });
        }
    }

    impl RecordingSink {
        fn events(&self) -> Vec<SinkEvent> {
            self.events.lock().unwrap().clone()
        }
        fn chunks(&self) -> Vec<String> {
            self.events()
                .into_iter()
                .filter_map(|e| match e {
                    SinkEvent::Chunk { chunk, .. } => Some(chunk),
                    _ => None,
                })
                .collect()
        }
    }

    /// Builds an in-memory byte stream from owned chunks.
    fn ok_stream(chunks: Vec<&[u8]>) -> impl Stream<Item = Result<Vec<u8>, AppError>> + Unpin {
        let items: Vec<Result<Vec<u8>, AppError>> =
            chunks.into_iter().map(|c| Ok(c.to_vec())).collect();
        futures_util::stream::iter(items)
    }

    // ---- validation -------------------------------------------------------

    fn sample_request() -> AiRequest {
        AiRequest {
            request_id: "req-1".to_string(),
            method: "POST".to_string(),
            url: "https://api.example.com/v1/chat".to_string(),
            headers: HashMap::new(),
            body: Some("{}".to_string()),
        }
    }

    #[test]
    fn validate_accepts_a_well_formed_request() {
        let (method, url) = validate(&sample_request()).unwrap();
        assert_eq!(method, reqwest::Method::POST);
        assert_eq!(url.scheme(), "https");
    }

    #[test]
    fn validate_defaults_and_accepts_get() {
        let mut req = sample_request();
        req.method = "get".to_string();
        let (method, _) = validate(&req).unwrap();
        assert_eq!(method, reqwest::Method::GET);
    }

    #[test]
    fn validate_rejects_empty_request_id() {
        let mut req = sample_request();
        req.request_id = "  ".to_string();
        assert_eq!(validate(&req).unwrap_err().code_str(), "VALIDATION");
    }

    #[test]
    fn validate_rejects_empty_url() {
        let mut req = sample_request();
        req.url = "".to_string();
        assert_eq!(validate(&req).unwrap_err().code_str(), "VALIDATION");
    }

    #[test]
    fn validate_rejects_unsupported_method() {
        let mut req = sample_request();
        req.method = "DELETE".to_string();
        assert_eq!(validate(&req).unwrap_err().code_str(), "VALIDATION");
    }

    #[test]
    fn validate_rejects_non_http_scheme() {
        let mut req = sample_request();
        req.url = "ftp://example.com/x".to_string();
        assert_eq!(validate(&req).unwrap_err().code_str(), "VALIDATION");
    }

    // ---- incremental UTF-8 decoder ---------------------------------------

    #[test]
    fn decoder_passes_through_ascii() {
        let mut d = Utf8StreamDecoder::new();
        assert_eq!(d.push(b"Hello"), "Hello");
        assert_eq!(d.push(b" world"), " world");
        assert_eq!(d.flush(), "");
    }

    #[test]
    fn decoder_holds_back_split_multibyte_sequence() {
        // "你" is E4 BD A0 in UTF-8; split it across two pushes.
        let bytes = "你好".as_bytes();
        let (first, second) = bytes.split_at(2); // breaks mid-character
        let mut d = Utf8StreamDecoder::new();
        let a = d.push(first);
        let b = d.push(second);
        assert_eq!(format!("{a}{b}"), "你好");
    }

    #[test]
    fn decoder_flush_is_lossy_for_incomplete_tail() {
        let mut d = Utf8StreamDecoder::new();
        // A lone lead byte with no continuation.
        assert_eq!(d.push(&[0xE4]), "");
        assert_eq!(d.flush(), "\u{FFFD}");
    }

    // ---- drive_stream: ordering, concatenation, completion ----------------

    #[tokio::test]
    async fn drive_stream_emits_ordered_chunks_then_one_complete() {
        let sink = RecordingSink::default();
        let cancel = CancellationToken::new();
        let stream = ok_stream(vec![b"Hello", b", ", b"world"]);

        drive_stream("req-1", stream, &cancel, &sink).await;

        let events = sink.events();
        assert_eq!(
            events,
            vec![
                SinkEvent::Chunk {
                    request_id: "req-1".into(),
                    chunk: "Hello".into()
                },
                SinkEvent::Chunk {
                    request_id: "req-1".into(),
                    chunk: ", ".into()
                },
                SinkEvent::Chunk {
                    request_id: "req-1".into(),
                    chunk: "world".into()
                },
                SinkEvent::Complete {
                    request_id: "req-1".into(),
                    content: "Hello, world".into()
                },
            ]
        );
    }

    #[tokio::test]
    async fn drive_stream_completion_equals_concatenation_of_chunks() {
        let sink = RecordingSink::default();
        let cancel = CancellationToken::new();
        let stream = ok_stream(vec![b"a", b"bc", b"", b"def"]);

        drive_stream("r", stream, &cancel, &sink).await;

        let concatenated: String = sink.chunks().concat();
        match sink.events().last().unwrap() {
            SinkEvent::Complete { content, .. } => assert_eq!(*content, concatenated),
            other => panic!("expected a single completion, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn drive_stream_empty_stream_completes_with_empty_content() {
        let sink = RecordingSink::default();
        let cancel = CancellationToken::new();
        let stream = ok_stream(vec![]);

        drive_stream("r", stream, &cancel, &sink).await;

        assert_eq!(
            sink.events(),
            vec![SinkEvent::Complete {
                request_id: "r".into(),
                content: String::new()
            }]
        );
    }

    // ---- drive_stream: errors are terminal --------------------------------

    #[tokio::test]
    async fn drive_stream_emits_single_error_and_stops() {
        let sink = RecordingSink::default();
        let cancel = CancellationToken::new();
        let items: Vec<Result<Vec<u8>, AppError>> = vec![
            Ok(b"partial".to_vec()),
            Err(AppError::network("boom")),
            Ok(b"never".to_vec()),
        ];
        let stream = futures_util::stream::iter(items);

        drive_stream("r", stream, &cancel, &sink).await;

        let events = sink.events();
        assert_eq!(
            events,
            vec![
                SinkEvent::Chunk {
                    request_id: "r".into(),
                    chunk: "partial".into()
                },
                SinkEvent::Error {
                    request_id: "r".into(),
                    error: "boom".into()
                },
            ]
        );
    }

    // ---- drive_stream: cancellation stops events (16.7) -------------------

    #[tokio::test]
    async fn drive_stream_precancelled_emits_nothing() {
        let sink = RecordingSink::default();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let stream = ok_stream(vec![b"a", b"b"]);

        drive_stream("r", stream, &cancel, &sink).await;

        assert!(sink.events().is_empty());
    }

    #[tokio::test]
    async fn drive_stream_cancel_before_completion_suppresses_complete() {
        // A stream that yields one chunk, then cancels, then would yield more.
        let sink = RecordingSink::default();
        let cancel = CancellationToken::new();

        // Cancel after the first poll by using a stream that cancels via a guard.
        let cancel_for_stream = cancel.clone();
        let items = futures_util::stream::unfold(0u8, move |state| {
            let cancel = cancel_for_stream.clone();
            async move {
                match state {
                    0 => Some((Ok::<Vec<u8>, AppError>(b"first".to_vec()), 1)),
                    1 => {
                        cancel.cancel();
                        Some((Ok::<Vec<u8>, AppError>(b"second".to_vec()), 2))
                    }
                    _ => None,
                }
            }
        });
        let items = Box::pin(items);

        drive_stream("r", items, &cancel, &sink).await;

        // The first chunk may be emitted; no completion or error must follow.
        let events = sink.events();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, SinkEvent::Complete { .. } | SinkEvent::Error { .. })),
            "no terminal complete/error after cancellation, got {events:?}"
        );
    }

    // ---- outbound SSRF ----------------------------------------------------

    fn ssrf_request(url: &str) -> AiRequest {
        AiRequest {
            request_id: "ssrf".to_string(),
            method: "POST".to_string(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: Some("{}".to_string()),
        }
    }

    async fn serve_http_once(response: &[u8]) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let body = response.to_vec();
        let handle = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = vec![0u8; 4096];
                let _ = tokio::io::AsyncReadExt::read(&mut socket, &mut buf).await;
                let _ = tokio::io::AsyncWriteExt::write_all(&mut socket, &body).await;
            }
        });
        (format!("http://127.0.0.1:{}", addr.port()), handle)
    }

    #[tokio::test]
    async fn request_blocks_loopback_and_link_local_without_connect() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let accepted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = accepted.clone();
        let server = tokio::spawn(async move {
            if listener.accept().await.is_ok() {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });

        let err = request(
            &ssrf_request(&format!("http://127.0.0.1:{}/", addr.port())),
            false,
        )
        .await
        .unwrap_err();
        assert_eq!(err.code_str(), "SSRF_BLOCKED");
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !accepted.load(std::sync::atomic::Ordering::SeqCst),
            "loopback must not be contacted"
        );
        server.abort();

        for blocked in [
            "http://169.254.169.254/",
            "http://10.0.0.1/",
            "http://192.168.0.1/",
        ] {
            let started = std::time::Instant::now();
            let err = request(&ssrf_request(blocked), false).await.unwrap_err();
            assert_eq!(err.code_str(), "SSRF_BLOCKED", "{blocked}");
            assert!(
                started.elapsed() < Duration::from_secs(2),
                "{blocked} must be rejected before connect"
            );
        }
    }

    #[tokio::test]
    async fn request_blocks_redirect_to_loopback() {
        let target = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target_addr = target.local_addr().unwrap();
        let accepted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = accepted.clone();
        let server = tokio::spawn(async move {
            if target.accept().await.is_ok() {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });

        // Different host than the origin literal so allow-private does not
        // follow. `localhost` is blocked when the hop is public-only.
        let location = format!("http://localhost:{}/", target_addr.port());
        let payload =
            format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\n\r\n");
        let (origin, origin_handle) = serve_http_once(payload.as_bytes()).await;

        let err = request(&ssrf_request(&origin), true).await.unwrap_err();
        assert_eq!(err.code_str(), "SSRF_BLOCKED");
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !accepted.load(std::sync::atomic::Ordering::SeqCst),
            "redirect target must not be contacted"
        );
        origin_handle.abort();
        server.abort();
    }

    // ---- Property 35: completion equals ordered chunk concatenation -------
    //
    // **Validates: Requirements 16.2, 16.3, 16.6**
    //
    // The example tests above pin down specific sequences; this property test
    // generalizes `drive_stream_completion_equals_concatenation_of_chunks`
    // across arbitrary chunk sequences, exercising the real (private) streaming
    // core without a window or network.

    use proptest::prelude::*;

    /// Runs [`drive_stream`] to completion on an in-memory byte stream built from
    /// `chunks`, returning the recording sink. A current-thread runtime drives the
    /// async core; the cancellation token is never cancelled, so the happy path
    /// (ordered chunks + one completion) is exercised.
    fn run_drive_stream(request_id: &str, chunks: Vec<Vec<u8>>) -> RecordingSink {
        let sink = RecordingSink::default();
        let cancel = CancellationToken::new();
        let items: Vec<Result<Vec<u8>, AppError>> = chunks.into_iter().map(Ok).collect();
        let stream = futures_util::stream::iter(items);
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("current-thread runtime");
        rt.block_on(drive_stream(request_id, stream, &cancel, &sink));
        sink
    }

    /// Text mixing ASCII, CJK, and accented characters, so partitioning its UTF-8
    /// bytes at arbitrary offsets routinely splits a multi-byte sequence.
    fn streamable_text() -> impl Strategy<Value = String> {
        proptest::string::string_regex(
            "[a-zA-Z0-9 \\n\\t\\x{4e00}-\\x{9fff}\u{e9}\u{e0}\u{fc}]{0,40}",
        )
        .unwrap()
    }

    /// A `(text, chunks)` pair where `chunks` is `text`'s UTF-8 bytes partitioned
    /// at arbitrary byte offsets: coincident offsets yield empty chunks, and an
    /// offset inside a character splits a multi-byte sequence across chunks.
    fn text_and_chunks() -> impl Strategy<Value = (String, Vec<Vec<u8>>)> {
        streamable_text().prop_flat_map(|text| {
            let len = text.len();
            proptest::collection::vec(0..=len, 0..=8).prop_map(move |mut cuts| {
                cuts.push(0);
                cuts.push(len);
                cuts.sort_unstable();
                let bytes = text.as_bytes();
                let chunks = cuts
                    .windows(2)
                    .map(|w| bytes[w[0]..w[1]].to_vec())
                    .collect::<Vec<_>>();
                (text.clone(), chunks)
            })
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        /// **Property 35: Stream completion equals ordered concatenation of chunks.**
        ///
        /// For any chunking of a valid-UTF-8 response, `drive_stream` emits zero or
        /// more ordered `ai:stream-chunk` events followed by exactly one
        /// `ai:stream-complete` and no `ai:stream-error` (16.2); every event carries
        /// the originating request id (16.3); and the completion content equals the
        /// in-order concatenation of every emitted chunk — which reconstructs the
        /// original response text (16.6).
        ///
        /// **Validates: Requirements 16.2, 16.3, 16.6**
        #[test]
        fn stream_complete_equals_ordered_chunk_concatenation(
            (text, chunks) in text_and_chunks(),
        ) {
            let request_id = "req-prop-35";
            let sink = run_drive_stream(request_id, chunks);
            let events = sink.events();

            // The stream always terminates with events (a completion at minimum).
            prop_assert!(!events.is_empty());

            // Exactly one terminal completion and no error (16.2).
            let completes = events
                .iter()
                .filter(|e| matches!(e, SinkEvent::Complete { .. }))
                .count();
            let errors = events
                .iter()
                .filter(|e| matches!(e, SinkEvent::Error { .. }))
                .count();
            prop_assert_eq!(completes, 1);
            prop_assert_eq!(errors, 0);

            // The completion is the final event; everything before it is a chunk.
            let (last, leading) = events.split_last().unwrap();
            for e in leading {
                prop_assert!(
                    matches!(e, SinkEvent::Chunk { .. }),
                    "expected only chunk events before completion, got {:?}",
                    e
                );
            }
            prop_assert!(
                matches!(last, SinkEvent::Complete { .. }),
                "expected a completion as the final event, got {:?}",
                last
            );

            // Every event carries the originating request id (16.3).
            for e in &events {
                let id = match e {
                    SinkEvent::Chunk { request_id, .. }
                    | SinkEvent::Complete { request_id, .. }
                    | SinkEvent::Error { request_id, .. } => request_id.as_str(),
                };
                prop_assert_eq!(id, request_id);
            }

            // The completion content equals the in-order concatenation of every
            // emitted chunk (Property 35 / 16.6), which reconstructs the original.
            let concatenation: String = sink.chunks().concat();
            if let SinkEvent::Complete { content, .. } = last {
                prop_assert_eq!(content, &concatenation);
                prop_assert_eq!(content, &text);
            }
        }
    }
}
