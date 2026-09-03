//! Consistent SQLite file snapshots via the online Backup API.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::backup::Backup;
use rusqlite::Connection;

use crate::error::AppError;

fn sqlite_sidecar_path(dest: &Path, suffix: &str) -> PathBuf {
    let mut path = dest.as_os_str().to_os_string();
    path.push(suffix);
    PathBuf::from(path)
}

fn remove_snapshot_sidecars(dest: &Path) -> Result<(), AppError> {
    for suffix in ["-wal", "-shm", "-journal"] {
        let extra = sqlite_sidecar_path(dest, suffix);
        match fs::remove_file(&extra) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(AppError::io(format!(
                    "failed to remove snapshot sidecar `{}`: {e}",
                    extra.display()
                )));
            }
        }
    }
    Ok(())
}

/// Writes a consistent standalone copy of `conn`'s main database to `dest`.
///
/// Uses SQLite's online Backup API so a live WAL database can be snapshotted
/// without copying `-wal`/`-shm` as the authority. Concurrent writers may
/// proceed between backup steps. The destination is finalized as a
/// single-file (`journal_mode=DELETE`) database.
pub fn snapshot_database(conn: &Connection, dest: &Path) -> Result<(), AppError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::io(format!("failed to create snapshot directory: {e}")))?;
    }
    if dest.exists() {
        fs::remove_file(dest)
            .map_err(|e| AppError::io(format!("failed to replace existing snapshot: {e}")))?;
    }

    let mut dst = Connection::open(dest)
        .map_err(|e| AppError::io(format!("failed to open snapshot destination: {e}")))?;
    {
        let backup = Backup::new(conn, &mut dst)
            .map_err(|e| AppError::io(format!("failed to start database snapshot: {e}")))?;
        backup
            .run_to_completion(100, Duration::from_millis(25), None)
            .map_err(|e| AppError::io(format!("failed to snapshot database: {e}")))?;
    }
    dst.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode = DELETE;")
        .map_err(|e| AppError::io(format!("failed to finalize snapshot: {e}")))?;
    drop(dst);
    remove_snapshot_sidecars(dest)?;
    Ok(())
}
