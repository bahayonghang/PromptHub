//! Shared application state for the Tauri_Backend.
//!
//! [`AppState`] is managed via `tauri::State` and holds the process-wide resources
//! the Command_Layer and services depend on: the SQLite connection pool, the
//! resolved [`RuntimePaths`], the in-memory encryption state (behind a `Mutex`),
//! the in-flight AI/sync request registry, and a `ready` flag that gates command
//! execution until startup initialization completes (design: "Application State
//! and Startup Sequence"; Requirements 2.2, 4.6, 4.7).
//!
//! Several fields here are intentionally minimal placeholders. Later storage,
//! security, and AI tasks replace the pool, encryption, and registry payload
//! types with their concrete implementations.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tokio_util::sync::CancellationToken;

use crate::storage::DbPool;

/// Handle for an in-flight cancellable request (AI/sync).
///
/// A [`CancellationToken`] the AI_Client (task 11.1) registers when an outbound
/// request starts and cancels to abort that request and stop emitting further
/// events for it (Requirement 16.7).
pub type RequestHandle = CancellationToken;

/// The set of per-user runtime directories resolved at startup.
///
/// These mirror the six directories the Data_Path_Manager resolves under the
/// platform's per-user application-data root (data, media, skill, rule, backup,
/// log). Defined minimally here; task 23.1 wires up the real resolution and
/// writability checks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimePaths {
    /// Structured application data (SQLite database) directory.
    pub data: PathBuf,
    /// Saved image/video media directory.
    pub media: PathBuf,
    /// Skill local repositories directory.
    pub skill: PathBuf,
    /// Platform rule files directory.
    pub rule: PathBuf,
    /// Backup archives directory.
    pub backup: PathBuf,
    /// Application log directory.
    pub log: PathBuf,
}

/// In-memory encryption state, guarded by a `Mutex` within [`AppState`].
///
/// Holds the derived key (when unlocked) and a `locked` flag. The key material is
/// never serialized or exposed to the Frontend. Concrete key derivation and the
/// `ENC::` envelope land with the Security_Service (task 7.1).
#[derive(Debug, Default)]
pub struct EncryptionState {
    /// The derived encryption key, present only while the app is unlocked.
    pub derived_key: Option<Vec<u8>>,
    /// Whether encrypted data is currently locked from access.
    pub locked: bool,
}

impl EncryptionState {
    /// Returns `true` when a derived key is currently cached (unlocked).
    pub fn is_unlocked(&self) -> bool {
        self.derived_key.is_some() && !self.locked
    }
}

/// Process-wide application state shared across all commands.
pub struct AppState {
    /// SQLite connection pool. `None` until the Storage_Engine initializes it
    /// during startup. Guarded so it can be installed after the state is managed.
    pub pool: Mutex<Option<DbPool>>,
    /// Resolved per-user runtime directories.
    pub paths: RuntimePaths,
    /// In-memory encryption key + lock flag.
    pub encryption: Mutex<EncryptionState>,
    /// Registry of in-flight AI/sync requests keyed by request id.
    pub requests: Mutex<HashMap<String, RequestHandle>>,
    /// Gates command execution; `false` until startup initialization succeeds.
    ready: AtomicBool,
    /// Human-readable fatal initialization error, set when the startup sequence
    /// fails so the Frontend can surface it (Requirements 4.7, 23.3). `None` while
    /// startup is in progress or has succeeded.
    init_error: Mutex<Option<String>>,
}

impl AppState {
    /// Creates a new, not-yet-ready state with the given runtime paths.
    pub fn new(paths: RuntimePaths) -> Self {
        Self {
            pool: Mutex::new(None),
            paths,
            encryption: Mutex::new(EncryptionState::default()),
            requests: Mutex::new(HashMap::new()),
            ready: AtomicBool::new(false),
            init_error: Mutex::new(None),
        }
    }

    /// Returns whether the backend has completed initialization and may serve
    /// commands.
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    /// Sets the `ready` flag. Called once startup initialization completes (or
    /// fails) to enable or keep commands gated.
    pub fn set_ready(&self, ready: bool) {
        self.ready.store(ready, Ordering::Release);
    }

    /// Installs the SQLite connection pool produced by the Storage_Engine during
    /// startup. Replaces any previously installed pool.
    pub fn set_pool(&self, pool: DbPool) {
        *self.pool.lock().unwrap() = Some(pool);
    }

    /// Records a fatal initialization error and clears the `ready` flag so every
    /// command stays gated and the Frontend can surface the failure
    /// (Requirements 4.7, 23.3).
    pub fn set_init_error(&self, message: impl Into<String>) {
        *self.init_error.lock().unwrap() = Some(message.into());
        self.set_ready(false);
    }

    /// Returns the fatal initialization error recorded during startup, if any.
    pub fn init_error(&self) -> Option<String> {
        self.init_error.lock().unwrap().clone()
    }

    /// Registers a fresh [`CancellationToken`] for an in-flight request and
    /// returns it. Any token previously registered under `request_id` is
    /// cancelled and replaced so a reused id can never leave a stale request
    /// running (Requirement 16.7).
    pub fn register_request(&self, request_id: &str) -> RequestHandle {
        let token = CancellationToken::new();
        let mut requests = self.requests.lock().unwrap();
        if let Some(previous) = requests.insert(request_id.to_string(), token.clone()) {
            previous.cancel();
        }
        token
    }

    /// Cancels the in-flight request registered under `request_id`, if any, and
    /// removes it from the registry. Returns `true` when a request was found and
    /// cancelled (Requirement 16.7).
    pub fn cancel_request(&self, request_id: &str) -> bool {
        let token = self.requests.lock().unwrap().remove(request_id);
        match token {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    }

    /// Removes a request from the registry without cancelling it. Called when a
    /// request finishes normally so completed ids do not accumulate. A no-op when
    /// the id is absent (already cancelled/removed).
    pub fn finish_request(&self, request_id: &str) {
        self.requests.lock().unwrap().remove(request_id);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(RuntimePaths::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_not_ready() {
        let state = AppState::default();
        assert!(!state.is_ready());
    }

    #[test]
    fn set_ready_toggles_the_gate() {
        let state = AppState::default();
        state.set_ready(true);
        assert!(state.is_ready());
        state.set_ready(false);
        assert!(!state.is_ready());
    }

    #[test]
    fn pool_starts_empty() {
        let state = AppState::default();
        assert!(state.pool.lock().unwrap().is_none());
    }

    #[test]
    fn new_state_has_no_init_error() {
        let state = AppState::default();
        assert!(state.init_error().is_none());
    }

    #[test]
    fn set_init_error_records_message_and_clears_ready() {
        let state = AppState::default();
        state.set_ready(true);
        state.set_init_error("data directory not writable");
        assert_eq!(
            state.init_error().as_deref(),
            Some("data directory not writable")
        );
        assert!(!state.is_ready());
    }

    #[test]
    fn encryption_state_defaults_to_locked_without_key() {
        let state = AppState::default();
        let enc = state.encryption.lock().unwrap();
        assert!(enc.derived_key.is_none());
        assert!(!enc.is_unlocked());
    }

    #[test]
    fn request_registry_starts_empty() {
        let state = AppState::default();
        assert!(state.requests.lock().unwrap().is_empty());
    }

    #[test]
    fn register_request_inserts_a_live_token() {
        let state = AppState::default();
        let token = state.register_request("req-1");
        assert!(!token.is_cancelled());
        assert!(state.requests.lock().unwrap().contains_key("req-1"));
    }

    #[test]
    fn cancel_request_cancels_and_removes_the_token() {
        let state = AppState::default();
        let token = state.register_request("req-1");
        assert!(state.cancel_request("req-1"));
        assert!(token.is_cancelled());
        assert!(!state.requests.lock().unwrap().contains_key("req-1"));
    }

    #[test]
    fn cancel_request_returns_false_for_unknown_id() {
        let state = AppState::default();
        assert!(!state.cancel_request("missing"));
    }

    #[test]
    fn registering_a_reused_id_cancels_the_previous_token() {
        let state = AppState::default();
        let first = state.register_request("req-1");
        let second = state.register_request("req-1");
        assert!(first.is_cancelled());
        assert!(!second.is_cancelled());
    }

    #[test]
    fn finish_request_removes_without_cancelling() {
        let state = AppState::default();
        let token = state.register_request("req-1");
        state.finish_request("req-1");
        assert!(!token.is_cancelled());
        assert!(!state.requests.lock().unwrap().contains_key("req-1"));
    }

    #[test]
    fn runtime_paths_hold_six_distinct_fields() {
        let paths = RuntimePaths {
            data: PathBuf::from("/app/data"),
            media: PathBuf::from("/app/media"),
            skill: PathBuf::from("/app/skill"),
            rule: PathBuf::from("/app/rule"),
            backup: PathBuf::from("/app/backup"),
            log: PathBuf::from("/app/log"),
        };
        let state = AppState::new(paths.clone());
        assert_eq!(state.paths, paths);
    }
}
