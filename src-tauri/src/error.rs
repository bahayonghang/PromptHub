//! Error model for the Tauri_Backend.
//!
//! Every service returns `Result<T, AppError>`. The Command_Layer converts that
//! into a [`CommandResult`] envelope so the Frontend can branch on a discriminated
//! `{ ok: true, data } | { ok: false, error }` shape (Requirements 2.2, 2.3, 2.7, 2.8).
//!
//! An [`AppError`] carries a stable string `code` (driven by [`ErrorCode`]), a
//! human-readable `message`, and optional structured `details`. It serializes to
//! `{ code, message, details? }`, omitting `details` when absent.

use std::fmt;

use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::Value;

/// Stable, machine-readable error codes shared with the Frontend.
///
/// The string form (see [`ErrorCode::as_str`]) is the contract the Frontend
/// branches on and must remain stable. Codes mirror the error taxonomy in the
/// design document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// Input failed a precondition (validation error).
    Validation,
    /// Referenced entity does not exist.
    NotFound,
    /// Operation collides with existing state.
    Conflict,
    /// Wrong credential supplied.
    Unauthorized,
    /// App is locked; the operation needs an unlock first.
    ///
    /// Used when writing sealed settings secrets (`githubToken`, `sync.password`)
    /// while the library is locked. Keep this code; do not reuse it for
    /// evaluation cancellation.
    Locked,
    /// File-system failure.
    Io,
    /// Outbound request failed.
    Network,
    /// Host/scheme failed the SSRF policy.
    SsrfBlocked,
    /// Capability is not present in the current runtime.
    CapabilityUnavailable,
    /// Operation exceeded its deadline.
    Timeout,
    /// Update signature verification failed.
    Signature,
    /// Content could not be parsed.
    Parse,
    /// Unexpected/unclassified failure.
    Internal,
}

impl ErrorCode {
    /// Returns the stable string representation sent to the Frontend.
    pub const fn as_str(self) -> &'static str {
        match self {
            ErrorCode::Validation => "VALIDATION",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::Conflict => "CONFLICT",
            ErrorCode::Unauthorized => "UNAUTHORIZED",
            ErrorCode::Locked => "LOCKED",
            ErrorCode::Io => "IO",
            ErrorCode::Network => "NETWORK",
            ErrorCode::SsrfBlocked => "SSRF_BLOCKED",
            ErrorCode::CapabilityUnavailable => "CAPABILITY_UNAVAILABLE",
            ErrorCode::Timeout => "TIMEOUT",
            ErrorCode::Signature => "SIGNATURE",
            ErrorCode::Parse => "PARSE",
            ErrorCode::Internal => "INTERNAL",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A structured backend error carrying a stable [`ErrorCode`], a human-readable
/// message, and optional structured details.
#[derive(Debug, Clone)]
pub struct AppError {
    /// Stable machine-readable code.
    pub code: ErrorCode,
    /// Human-readable description of the failure.
    pub message: String,
    /// Optional structured context (omitted from the wire format when `None`).
    pub details: Option<Value>,
}

impl AppError {
    /// Creates an error with the given code and message and no details.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Attaches structured details, replacing any previously set details.
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    /// The stable string code for this error.
    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    /// `VALIDATION` — input failed a precondition.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Validation, message)
    }

    /// `NOT_FOUND` — referenced entity does not exist.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    /// `CONFLICT` — operation collides with existing state.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Conflict, message)
    }

    /// `UNAUTHORIZED` — wrong credential supplied.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, message)
    }

    /// `LOCKED` — app is locked; operation needs an unlock.
    pub fn locked(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Locked, message)
    }

    /// `IO` — file-system failure.
    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Io, message)
    }

    /// `NETWORK` — outbound request failed.
    pub fn network(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Network, message)
    }

    /// `SSRF_BLOCKED` — host/scheme failed the SSRF policy.
    pub fn ssrf_blocked(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::SsrfBlocked, message)
    }

    /// `CAPABILITY_UNAVAILABLE` — capability not present in the runtime.
    pub fn capability_unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::CapabilityUnavailable, message)
    }

    /// `TIMEOUT` — operation exceeded its deadline.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Timeout, message)
    }

    /// `SIGNATURE` — update signature verification failed.
    pub fn signature(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Signature, message)
    }

    /// `PARSE` — content could not be parsed.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Parse, message)
    }

    /// `INTERNAL` — unexpected/unclassified failure.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
    /// Serializes to `{ code, message, details? }`, omitting `details` when `None`.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let field_count = if self.details.is_some() { 3 } else { 2 };
        let mut state = serializer.serialize_struct("AppError", field_count)?;
        state.serialize_field("code", self.code.as_str())?;
        state.serialize_field("message", &self.message)?;
        if let Some(details) = &self.details {
            state.serialize_field("details", details)?;
        }
        state.end()
    }
}

/// Discriminated result envelope returned by every command in the Command_Layer.
///
/// Serializes to `{ ok: true, data }` on success or `{ ok: false, error }` on
/// failure, matching the Frontend's `CommandResult<T>` type.
#[derive(Debug, Clone)]
pub enum CommandResult<T> {
    /// Successful result carrying the operation output.
    Ok(T),
    /// Failure carrying a structured [`AppError`].
    Err(AppError),
}

impl<T> From<Result<T, AppError>> for CommandResult<T> {
    fn from(result: Result<T, AppError>) -> Self {
        match result {
            Ok(data) => CommandResult::Ok(data),
            Err(error) => CommandResult::Err(error),
        }
    }
}

impl<T> Serialize for CommandResult<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            CommandResult::Ok(data) => {
                let mut state = serializer.serialize_struct("CommandResult", 2)?;
                state.serialize_field("ok", &true)?;
                state.serialize_field("data", data)?;
                state.end()
            }
            CommandResult::Err(error) => {
                let mut state = serializer.serialize_struct("CommandResult", 2)?;
                state.serialize_field("ok", &false)?;
                state.serialize_field("error", error)?;
                state.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn app_error_serializes_without_details() {
        let err = AppError::not_found("prompt not found");
        let value = serde_json::to_value(&err).unwrap();
        assert_eq!(
            value,
            json!({ "code": "NOT_FOUND", "message": "prompt not found" })
        );
        assert!(value.get("details").is_none());
    }

    #[test]
    fn app_error_serializes_with_details() {
        let err =
            AppError::validation("title is required").with_details(json!({ "field": "title" }));
        let value = serde_json::to_value(&err).unwrap();
        assert_eq!(
            value,
            json!({
                "code": "VALIDATION",
                "message": "title is required",
                "details": { "field": "title" }
            })
        );
    }

    #[test]
    fn every_code_has_a_stable_string() {
        let codes = [
            (ErrorCode::Validation, "VALIDATION"),
            (ErrorCode::NotFound, "NOT_FOUND"),
            (ErrorCode::Conflict, "CONFLICT"),
            (ErrorCode::Unauthorized, "UNAUTHORIZED"),
            (ErrorCode::Locked, "LOCKED"),
            (ErrorCode::Io, "IO"),
            (ErrorCode::Network, "NETWORK"),
            (ErrorCode::SsrfBlocked, "SSRF_BLOCKED"),
            (ErrorCode::CapabilityUnavailable, "CAPABILITY_UNAVAILABLE"),
            (ErrorCode::Timeout, "TIMEOUT"),
            (ErrorCode::Signature, "SIGNATURE"),
            (ErrorCode::Parse, "PARSE"),
            (ErrorCode::Internal, "INTERNAL"),
        ];
        for (code, expected) in codes {
            assert_eq!(code.as_str(), expected);
        }
    }

    #[test]
    fn command_result_ok_serializes_to_ok_true_data() {
        let result: CommandResult<i32> = CommandResult::Ok(42);
        let value = serde_json::to_value(&result).unwrap();
        assert_eq!(value, json!({ "ok": true, "data": 42 }));
    }

    #[test]
    fn command_result_err_serializes_to_ok_false_error() {
        let result: CommandResult<i32> = CommandResult::Err(AppError::internal("boom"));
        let value = serde_json::to_value(&result).unwrap();
        assert_eq!(
            value,
            json!({ "ok": false, "error": { "code": "INTERNAL", "message": "boom" } })
        );
    }

    #[test]
    fn command_result_from_result_maps_both_arms() {
        let ok: CommandResult<&str> = Ok::<&str, AppError>("hi").into();
        assert!(matches!(ok, CommandResult::Ok("hi")));

        let err: CommandResult<&str> = Err::<&str, AppError>(AppError::conflict("dup")).into();
        assert!(matches!(err, CommandResult::Err(e) if e.code == ErrorCode::Conflict));
    }
}
