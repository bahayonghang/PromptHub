//! Tauri event emission for the Command_Layer (task 17.2).
//!
//! The services keep their business rules Tauri-free and report asynchronous
//! progress through injected sinks ([`crate::services::ai::EventSink`],
//! [`crate::services::updater::UpdaterEventSink`]) or by returning typed payloads
//! (the Window_Manager's [`FullscreenChanged`], [`VisibilityChanged`],
//! [`CloseRequested`], [`ShortcutTriggered`]). This module is the thin adapter
//! that forwards each of those onto the concrete Tauri event channels the
//! Frontend's Runtime_Bridge subscribes to (Requirement 2.4; design "Event
//! channels" table):
//!
//! | Constant | Channel | Payload |
//! | --- | --- | --- |
//! | [`EVENT_UPDATER_STATUS`] | `updater:status` | `{ phase, downloaded?, total? }` |
//! | [`EVENT_SHORTCUT_TRIGGERED`] | `shortcut:triggered` | `{ action }` |
//! | [`EVENT_WINDOW_CLOSE_REQUESTED`] | `window:close-requested` | `{}` |
//! | [`EVENT_WINDOW_FULLSCREEN_CHANGED`] | `window:fullscreen-changed` | `{ fullscreen }` |
//! | [`EVENT_WINDOW_VISIBILITY_CHANGED`] | `window:visibility-changed` | `{ visible }` |
//! | [`EVENT_AI_STREAM_CHUNK`] | `ai:stream-chunk` | `{ requestId, chunk }` |
//! | [`EVENT_AI_STREAM_ERROR`] | `ai:stream-error` | `{ requestId, error }` |
//! | [`EVENT_AI_STREAM_COMPLETE`] | `ai:stream-complete` | `{ requestId, content }` |
//!
//! The window/shortcut channel names are re-exported from
//! [`crate::services::window`] (the single source of truth for those payload
//! shapes); the AI and updater channel names live here because their services
//! emit through a sink abstraction rather than naming a channel.
#![allow(dead_code)]

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

use crate::services::ai::EventSink;
use crate::services::evaluation::EvaluationEventSink;
use crate::services::updater::{UpdaterEventSink, UpdaterStatus};
use crate::services::window::{
    CloseRequested, FullscreenChanged, ShortcutTriggered, VisibilityChanged, EVENT_CLOSE_REQUESTED,
    EVENT_FULLSCREEN_CHANGED, EVENT_SHORTCUT_TRIGGERED, EVENT_VISIBILITY_CHANGED,
};

// ===========================================================================
// Event channel names (Requirement 2.4, design "Event channels")
// ===========================================================================

/// `updater:status` — download progress + terminal completion (24.3).
pub const EVENT_UPDATER_STATUS: &str = "updater:status";
/// `ai:stream-chunk` — one ordered piece of a streaming AI response (16.2).
pub const EVENT_AI_STREAM_CHUNK: &str = "ai:stream-chunk";
/// `ai:stream-error` — the single terminal error of a streaming AI response (16.4).
pub const EVENT_AI_STREAM_ERROR: &str = "ai:stream-error";
/// `ai:stream-complete` — the single terminal completion carrying the full
/// concatenated content of a streaming AI response (16.6).
pub const EVENT_AI_STREAM_COMPLETE: &str = "ai:stream-complete";
pub const EVENT_EVALUATION_RUN_CHUNK: &str = "evaluation:run-chunk";
pub const EVENT_EVALUATION_RUN_TERMINAL: &str = "evaluation:run-terminal";
pub const EVENT_EVALUATION_MATRIX_PROGRESS: &str = "evaluation:matrix-progress";

/// `shortcut:triggered` — a registered keyboard shortcut fired (20.6).
pub const EVENT_SHORTCUT: &str = EVENT_SHORTCUT_TRIGGERED;
/// `window:close-requested` — a close was attempted under the `ask` action (20.4).
pub const EVENT_WINDOW_CLOSE_REQUESTED: &str = EVENT_CLOSE_REQUESTED;
/// `window:fullscreen-changed` — the window entered/exited fullscreen (20.2).
pub const EVENT_WINDOW_FULLSCREEN_CHANGED: &str = EVENT_FULLSCREEN_CHANGED;
/// `window:visibility-changed` — the window's visibility toggled (20.3).
pub const EVENT_WINDOW_VISIBILITY_CHANGED: &str = EVENT_VISIBILITY_CHANGED;

// ===========================================================================
// AI streaming payloads (design "Event channels": ai:stream-* )
// ===========================================================================

/// Payload for [`EVENT_AI_STREAM_CHUNK`]: `{ requestId, chunk }` (16.2, 16.3).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AiStreamChunk<'a> {
    request_id: &'a str,
    chunk: &'a str,
}

/// Payload for [`EVENT_AI_STREAM_COMPLETE`]: `{ requestId, content }` (16.6).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AiStreamComplete<'a> {
    request_id: &'a str,
    content: &'a str,
}

/// Payload for [`EVENT_AI_STREAM_ERROR`]: `{ requestId, error }` (16.4).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AiStreamError<'a> {
    request_id: &'a str,
    error: &'a str,
}

// ===========================================================================
// AI_Client event sink (16.2, 16.3, 16.4, 16.6)
// ===========================================================================

/// A [`EventSink`] that forwards the AI_Client's streaming events onto the
/// `ai:stream-chunk` / `ai:stream-complete` / `ai:stream-error` Tauri channels.
///
/// The Command_Layer constructs one of these per `ai.stream` invocation (cloning
/// the `AppHandle`) and hands it to [`crate::services::ai::stream`]; the service
/// drives the ordered chunk/terminal-event contract through it without ever
/// depending on Tauri.
pub struct TauriEventSink<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriEventSink<R> {
    /// Creates a sink that emits on `app`'s event channels.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> EventSink for TauriEventSink<R> {
    fn emit_chunk(&self, request_id: &str, chunk: &str) {
        let _ = self
            .app
            .emit(EVENT_AI_STREAM_CHUNK, AiStreamChunk { request_id, chunk });
    }

    fn emit_complete(&self, request_id: &str, content: &str) {
        let _ = self.app.emit(
            EVENT_AI_STREAM_COMPLETE,
            AiStreamComplete {
                request_id,
                content,
            },
        );
    }

    fn emit_error(&self, request_id: &str, error: &str) {
        let _ = self
            .app
            .emit(EVENT_AI_STREAM_ERROR, AiStreamError { request_id, error });
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvaluationRunChunk<'a> {
    run_id: &'a str,
    chunk: &'a str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvaluationRunTerminal<'a> {
    run_id: &'a str,
    status: &'a str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvaluationMatrixProgress<'a> {
    evaluation_run_id: &'a str,
    completed: i64,
    total: i64,
    cell_id: &'a str,
}

pub struct TauriEvaluationEventSink<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriEvaluationEventSink<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> EvaluationEventSink for TauriEvaluationEventSink<R> {
    fn emit_run_chunk(&self, run_id: &str, chunk: &str) {
        let _ = self.app.emit(
            EVENT_EVALUATION_RUN_CHUNK,
            EvaluationRunChunk { run_id, chunk },
        );
    }

    fn emit_run_terminal(&self, run_id: &str, status: &str) {
        let _ = self.app.emit(
            EVENT_EVALUATION_RUN_TERMINAL,
            EvaluationRunTerminal { run_id, status },
        );
    }

    fn emit_matrix_progress(
        &self,
        evaluation_run_id: &str,
        completed: i64,
        total: i64,
        cell_id: &str,
    ) {
        let _ = self.app.emit(
            EVENT_EVALUATION_MATRIX_PROGRESS,
            EvaluationMatrixProgress {
                evaluation_run_id,
                completed,
                total,
                cell_id,
            },
        );
    }
}

// ===========================================================================
// Updater event sink (24.3)
// ===========================================================================

/// An [`UpdaterEventSink`] that forwards the Updater's download progress onto the
/// `updater:status` Tauri channel.
///
/// The Command_Layer constructs one per `updater.download` invocation and hands
/// it to [`crate::services::updater::download`], which emits a status per chunk
/// followed by a single terminal completion.
pub struct TauriUpdaterEventSink<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriUpdaterEventSink<R> {
    /// Creates a sink that emits on `app`'s `updater:status` channel.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> UpdaterEventSink for TauriUpdaterEventSink<R> {
    fn emit_status(&self, status: &UpdaterStatus) {
        let _ = self.app.emit(EVENT_UPDATER_STATUS, status);
    }
}

// ===========================================================================
// Window + shortcut emitters (20.2, 20.3, 20.4, 20.6)
// ===========================================================================

/// Emits [`EVENT_WINDOW_FULLSCREEN_CHANGED`] with `{ fullscreen }` (20.2).
pub fn emit_fullscreen_changed<R: Runtime>(app: &AppHandle<R>, fullscreen: bool) {
    let _ = app.emit(
        EVENT_WINDOW_FULLSCREEN_CHANGED,
        FullscreenChanged { fullscreen },
    );
}

/// Emits [`EVENT_WINDOW_VISIBILITY_CHANGED`] with `{ visible }` (20.3).
pub fn emit_visibility_changed<R: Runtime>(app: &AppHandle<R>, visible: bool) {
    let _ = app.emit(
        EVENT_WINDOW_VISIBILITY_CHANGED,
        VisibilityChanged { visible },
    );
}

/// Emits [`EVENT_WINDOW_CLOSE_REQUESTED`] with `{}` for the `ask` close action
/// (20.4).
pub fn emit_close_requested<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit(EVENT_WINDOW_CLOSE_REQUESTED, CloseRequested {});
}

/// Emits [`EVENT_SHORTCUT`] with `{ action }` when a registered shortcut fires
/// (20.6).
pub fn emit_shortcut_triggered<R: Runtime>(app: &AppHandle<R>, action: impl Into<String>) {
    let _ = app.emit(
        EVENT_SHORTCUT,
        ShortcutTriggered {
            action: action.into(),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // The event-channel names are the Frontend contract (Req 2.4): pin them so an
    // accidental rename is caught here rather than as a silent missed subscription.

    #[test]
    fn event_channel_names_match_the_design_table() {
        assert_eq!(EVENT_UPDATER_STATUS, "updater:status");
        assert_eq!(EVENT_AI_STREAM_CHUNK, "ai:stream-chunk");
        assert_eq!(EVENT_AI_STREAM_ERROR, "ai:stream-error");
        assert_eq!(EVENT_AI_STREAM_COMPLETE, "ai:stream-complete");
        assert_eq!(EVENT_EVALUATION_RUN_CHUNK, "evaluation:run-chunk");
        assert_eq!(EVENT_EVALUATION_RUN_TERMINAL, "evaluation:run-terminal");
        assert_eq!(
            EVENT_EVALUATION_MATRIX_PROGRESS,
            "evaluation:matrix-progress"
        );
        assert_eq!(EVENT_SHORTCUT, "shortcut:triggered");
        assert_eq!(EVENT_WINDOW_CLOSE_REQUESTED, "window:close-requested");
        assert_eq!(EVENT_WINDOW_FULLSCREEN_CHANGED, "window:fullscreen-changed");
        assert_eq!(EVENT_WINDOW_VISIBILITY_CHANGED, "window:visibility-changed");
    }

    #[test]
    fn ai_stream_payloads_serialize_camel_case() {
        assert_eq!(
            serde_json::to_value(AiStreamChunk {
                request_id: "r1",
                chunk: "hello"
            })
            .unwrap(),
            json!({ "requestId": "r1", "chunk": "hello" })
        );
        assert_eq!(
            serde_json::to_value(AiStreamComplete {
                request_id: "r1",
                content: "hello world"
            })
            .unwrap(),
            json!({ "requestId": "r1", "content": "hello world" })
        );
        assert_eq!(
            serde_json::to_value(AiStreamError {
                request_id: "r1",
                error: "boom"
            })
            .unwrap(),
            json!({ "requestId": "r1", "error": "boom" })
        );
    }
}
