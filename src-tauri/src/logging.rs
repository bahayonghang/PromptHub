//! Rolling file log under [`crate::state::RuntimePaths::log`].
//!
//! One append-only `prompthub.log` with a single size-based roll. This is not a
//! tracing platform. Lines are `time level module message`. Callers must not
//! pass a master password, DEK, `Authorization` header, or URL query token;
//! [`sanitize`] also redacts those labels if they appear.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use chrono::Utc;

use crate::error::AppError;

/// File name of the active log under the runtime log directory.
pub const LOG_FILE_NAME: &str = "prompthub.log";
const LOG_BACKUP_NAME: &str = "prompthub.log.1";
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

static LOG_DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static WRITE_LOCK: Mutex<()> = Mutex::new(());

/// Severity written as the second field of each log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Info,
}

impl Level {
    const fn as_str(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
        }
    }
}

fn dir_slot() -> &'static Mutex<Option<PathBuf>> {
    LOG_DIR.get_or_init(|| Mutex::new(None))
}

fn recover<T>(result: std::sync::LockResult<T>) -> T {
    result.unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Records the runtime log directory. Startup calls this after resolving
/// [`crate::state::RuntimePaths`]. Tests may call it with a temp directory.
pub fn init(log_dir: &Path) {
    let _ = fs::create_dir_all(log_dir);
    *recover(dir_slot().lock()) = Some(log_dir.to_path_buf());
}

fn current_dir() -> Option<PathBuf> {
    recover(dir_slot().lock()).clone()
}

/// Writes one line to the process log directory when [`init`] has run.
pub fn event(level: Level, module: &str, message: impl AsRef<str>) {
    if let Some(dir) = current_dir() {
        write_line(&dir, level, module, message.as_ref());
    }
}

/// Writes one line under `log_dir` even when [`init`] has not run.
pub fn event_to(log_dir: &Path, level: Level, module: &str, message: impl AsRef<str>) {
    write_line(log_dir, level, module, message.as_ref());
}

/// Maps a poisoned `Mutex` to `INTERNAL` and records the poison in the log.
pub fn lock_mutex<'a, T>(
    mutex: &'a Mutex<T>,
    what: &'static str,
) -> Result<MutexGuard<'a, T>, AppError> {
    mutex.lock().map_err(|_| {
        event(Level::Error, "mutex", format!("{what} lock is poisoned"));
        AppError::internal(format!("{what} lock is poisoned"))
    })
}

fn write_line(log_dir: &Path, level: Level, module: &str, message: &str) {
    let _guard = recover(WRITE_LOCK.lock());
    let _ = fs::create_dir_all(log_dir);
    let path = log_dir.join(LOG_FILE_NAME);
    roll_if_needed(&path, MAX_LOG_BYTES);
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    let line = format!(
        "{} {} {} {}\n",
        Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        level.as_str(),
        module,
        sanitize(message)
    );
    let _ = file.write_all(line.as_bytes());
}

fn roll_if_needed(path: &Path, max_bytes: u64) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.len() < max_bytes {
        return;
    }
    let backup = path.with_file_name(LOG_BACKUP_NAME);
    let _ = fs::remove_file(&backup);
    let _ = fs::rename(path, &backup);
}

/// Redacts password, DEK, Authorization, and query-token material so a log file
/// is shareable.
fn sanitize(message: &str) -> String {
    let mut output = redact_labeled(message, "authorization:");
    output = redact_labeled(&output, "authorization=");
    output = redact_labeled(&output, "bearer ");
    output = redact_labeled(&output, "password=");
    output = redact_labeled(&output, "password:");
    output = redact_labeled(&output, "dek=");
    output = redact_labeled(&output, "dek:");
    output = redact_labeled(&output, "token=");
    output
}

fn redact_labeled(input: &str, label: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let label_l = label.to_ascii_lowercase();
    let mut output = String::with_capacity(input.len());
    let mut offset = 0;
    while let Some(found) = lower[offset..].find(&label_l) {
        let start = offset + found;
        let label_end = start + label.len();
        output.push_str(&input[offset..label_end]);
        output.push_str("[REDACTED]");
        offset = skip_secret_token(input, label_end);
    }
    output.push_str(&input[offset..]);
    output
}

fn skip_secret_token(input: &str, mut index: usize) -> usize {
    let bytes = input.as_bytes();
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    let token_start = index;
    while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    if input[token_start..index].eq_ignore_ascii_case("bearer") {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
            index += 1;
        }
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    fn poison<T>(mutex: &Mutex<T>) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = mutex.lock().unwrap();
            panic!("test poison");
        }));
        assert!(mutex.lock().is_err());
    }

    #[test]
    fn event_to_writes_reason_without_secrets() {
        let tmp = TempDir::new().unwrap();
        event_to(
            tmp.path(),
            Level::Error,
            "startup",
            "startup failed: [IO] data directory not writable password=secret Authorization: Bearer super-secret",
        );
        let body = fs::read_to_string(tmp.path().join(LOG_FILE_NAME)).unwrap();
        assert!(body.contains("ERROR startup"));
        assert!(body.contains("data directory not writable"));
        assert!(body.contains("[REDACTED]"));
        assert!(!body.contains("super-secret"));
        assert!(!body.contains("password=secret"));
        assert!(!body.contains("Bearer super-secret"));
    }

    #[test]
    fn sanitize_redacts_query_tokens_and_dek() {
        let redacted = sanitize(
            "GET https://api.example.com/update?token=abc123&access_token=xyz DEK: deadbeef",
        );
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("abc123"));
        assert!(!redacted.contains("xyz"));
        assert!(!redacted.contains("deadbeef"));
        assert!(redacted.contains("token=[REDACTED]"));
        assert!(redacted.contains("DEK:[REDACTED]"));
    }

    #[test]
    fn roll_renames_oversized_log() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(LOG_FILE_NAME);
        fs::write(&path, vec![b'x'; 32]).unwrap();
        roll_if_needed(&path, 16);
        assert!(!path.exists());
        assert_eq!(
            fs::read(tmp.path().join(LOG_BACKUP_NAME)).unwrap().len(),
            32
        );
    }

    #[test]
    fn lock_mutex_returns_internal_on_poison() {
        let mutex = Mutex::new(1u8);
        poison(&mutex);
        let err = lock_mutex(&mutex, "requests").unwrap_err();
        assert_eq!(err.code_str(), "INTERNAL");
        assert_eq!(err.message, "requests lock is poisoned");
    }
}
