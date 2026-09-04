use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{AppError, CommandResult};
use crate::services::security::{self, SecurityStatus};
use crate::state::{AppState, EncryptionState};

use super::{conn, into_command};

/// Command_Layer gate for `security.setMasterPassword` (Req 15.2).
///
/// `set_master_password` is a primitive that replaces the verifier. The public
/// set-password command must refuse when a verifier already exists so existing
/// `ENC::` ciphertext is not orphaned. `change_master_password` re-keys first
/// and does not go through this gate.
fn reject_if_master_password_exists(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
) -> Result<(), AppError> {
    let status = security::status(conn, encryption)?;
    if status.has_master_password {
        return Err(AppError::conflict("master password already exists"));
    }
    Ok(())
}

#[tauri::command(rename = "security.status")]
pub fn security_status(state: tauri::State<'_, AppState>) -> CommandResult<SecurityStatus> {
    into_command(conn(&state).and_then(|conn| security::status(&conn, &state.encryption)))
}

#[tauri::command(rename = "security.setMasterPassword")]
pub fn security_set_master_password(
    password: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| {
        reject_if_master_password_exists(&conn, &state.encryption)
            .and_then(|_| security::set_master_password(&conn, &state.encryption, &password))
    }))
}

#[tauri::command(rename = "security.changeMasterPassword")]
pub fn security_change_master_password(
    current_password: String,
    new_password: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    into_command(conn(&state).and_then(|conn| {
        security::change_master_password(&conn, &state.encryption, &current_password, &new_password)
    }))
}

#[tauri::command(rename = "security.unlock")]
pub fn security_unlock(password: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(
        conn(&state).and_then(|conn| security::unlock(&conn, &state.encryption, &password)),
    )
}

#[tauri::command(rename = "security.lock")]
pub fn security_lock(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    into_command(super::ensure_ready(&state).and_then(|_| security::lock(&state.encryption)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{create_memory_pool, init_schema, DbPool};

    fn test_pool() -> DbPool {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();
        pool
    }

    #[test]
    fn set_master_password_gate_allows_first_set() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());
        reject_if_master_password_exists(&conn, &enc).unwrap();
        security::set_master_password(&conn, &enc, "password123").unwrap();
        assert!(security::status(&conn, &enc).unwrap().has_master_password);
    }

    #[test]
    fn set_master_password_gate_rejects_second_set_without_replacing_verifier() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());
        security::set_master_password(&conn, &enc, "password123").unwrap();
        let before = security::status(&conn, &enc).unwrap();
        let key = enc.lock().unwrap().derived_key.clone().unwrap();
        let envelope = security::encrypt("private body", &key).unwrap();

        let err = reject_if_master_password_exists(&conn, &enc).unwrap_err();
        assert_eq!(err.code_str(), "CONFLICT");

        let after = security::status(&conn, &enc).unwrap();
        assert_eq!(after.has_master_password, before.has_master_password);
        assert_eq!(after.is_locked, before.is_locked);
        assert_eq!(
            security::decrypt(&envelope, &key).unwrap(),
            "private body",
            "existing ENC:: ciphertext must stay readable under the original key"
        );
        security::lock(&enc).unwrap();
        security::unlock(&conn, &enc, "password123").unwrap();
    }
}
