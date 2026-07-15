//! Security_Service: master password and AES-256-GCM encryption (Requirement 15).
//!
//! This service reproduces the construction used by the Reference_App
//! (`ref/PromptHub/apps/desktop/src/main/security.ts`) so the round-trip and
//! authentication-tag properties of Requirement 15 hold:
//!
//! - **Key derivation.** `key = scrypt(password, salt16, params) -> 32 bytes`,
//!   with fixed interactive parameters `log_n = 15` (N = 32768), `r = 8`,
//!   `p = 1`. These parameters are held constant so previously stored verifiers
//!   stay valid (changing them would invalidate every stored `{salt, hash}`).
//! - **Verifier.** A [`StoredMasterPassword`] `{ salt, hash }` is persisted in the
//!   `settings` table under the [`SETTINGS_KEY`] key as JSON. `salt` is 16 random
//!   bytes (base64); `hash` is the 32-byte derived key (base64). Because the
//!   stored `hash` *is* the derived encryption key, verifying a password also
//!   recovers the key for re-keying.
//! - **Envelope.** `encrypt` produces `"ENC::" + base64(iv12 || tag16 ||
//!   ciphertext)` using AES-256-GCM with a random 12-byte nonce and the 16-byte
//!   authentication tag. This byte layout (IV, then TAG, then CIPHERTEXT) matches
//!   the Reference_App. Note that the `aes-gcm` crate's `Aead::encrypt` returns
//!   `ciphertext || tag`, so `encrypt`/`decrypt` reorder the tag explicitly to
//!   keep the on-the-wire layout fixed.
//!
//! Security notes: the master-password verifier is compared in constant time
//! ([`ct_eq`]); secrets (passwords, derived keys, plaintext) are never logged or
//! echoed; salts and nonces come from the OS CSPRNG ([`OsRng`]).
//!
//! The service functions take a `&rusqlite::Connection` for verifier persistence
//! and a `&Mutex<EncryptionState>` for the cached in-memory key, so they are unit
//! testable with `storage::create_memory_pool` + `init_schema` and a local
//! [`EncryptionState`].
#![allow(dead_code)]

use std::sync::Mutex;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::Engine as _;
use rand::rngs::OsRng;
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use scrypt::Params;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::StoredMasterPassword;
use crate::state::EncryptionState;

/// Settings-table key under which the `{salt, hash}` verifier JSON is stored.
const SETTINGS_KEY: &str = "master_password";

/// scrypt cost parameter: log₂(N). `15` => N = 32768 (interactive cost).
const LOG_N: u8 = 15;
/// scrypt block-size parameter `r`.
const R: u32 = 8;
/// scrypt parallelism parameter `p`.
const P: u32 = 1;
/// Derived key / verifier-hash length in bytes (AES-256 key size).
const KEY_LEN: usize = 32;
/// Salt length in bytes.
const SALT_LEN: usize = 16;
/// AES-GCM nonce (IV) length in bytes.
const NONCE_LEN: usize = 12;
/// AES-GCM authentication tag length in bytes.
const TAG_LEN: usize = 16;
/// Envelope prefix marking a value as encrypted by this service.
const ENC_PREFIX: &str = "ENC::";
/// Inclusive lower bound for master-password length (characters).
const MIN_PASSWORD_LEN: usize = 8;
/// Inclusive upper bound for master-password length (characters).
const MAX_PASSWORD_LEN: usize = 128;

pub(crate) fn is_encrypted_value(value: &str) -> bool {
    value.starts_with(ENC_PREFIX)
}

/// Security status reported to the Frontend (Requirement 15.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityStatus {
    /// Whether a master-password verifier exists in settings.
    pub has_master_password: bool,
    /// Whether the application is currently locked (no cached key).
    pub is_locked: bool,
}

/// Returns `{ hasMasterPassword, isLocked }` (Requirement 15.1).
///
/// `hasMasterPassword` reflects whether a verifier is persisted; `isLocked`
/// reflects whether a derived key is currently cached (locked when none is).
pub fn status(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
) -> Result<SecurityStatus, AppError> {
    let has_master_password = get_stored(conn)?.is_some();
    let is_locked = {
        let enc = lock_state(encryption)?;
        !enc.is_unlocked()
    };
    Ok(SecurityStatus {
        has_master_password,
        is_locked,
    })
}

/// Establishes (or replaces) the master password and caches the derived key
/// (Requirements 15.2, 15.3).
///
/// Validates the password length is 8–128 inclusive, returning a `VALIDATION`
/// error and making **no change** otherwise. On success a fresh 16-byte salt is
/// generated, the key is derived, the `{salt, hash}` verifier is persisted, and
/// the derived key is cached (unlocked).
///
/// This is a low-level primitive used both by the public "set master password"
/// flow (which the Command_Layer gates on "no master password exists" per 15.2)
/// and by [`change_master_password`].
pub fn set_master_password(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    password: &str,
) -> Result<(), AppError> {
    validate_password_len(password)?;

    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let key = derive_key(password.as_bytes(), &salt)?;

    let stored = StoredMasterPassword {
        salt: b64_encode(&salt),
        hash: b64_encode(&key),
    };
    save_stored(conn, &stored)?;
    cache_key(encryption, key.to_vec())?;
    Ok(())
}

/// Re-keys encrypted data to a new master password (Requirements 15.4, 15.5).
///
/// Verifies `current_password` against the stored verifier in constant time;
/// on mismatch returns `UNAUTHORIZED` and leaves the master password and data
/// unchanged (15.5). Validates `new_password` length (15.3). On success it
/// derives a new key from a fresh salt, re-encrypts every `ENC::` value currently
/// stored in `settings` under the new key, replaces the stored verifier, and
/// caches the new key — all within a single transaction so the verifier and the
/// re-keyed data update atomically (preserving access to previously encrypted
/// data per 15.4).
///
/// Prompt and revision content fields are re-keyed together with encrypted
/// settings so the verifier and every protected value stay consistent.
pub fn change_master_password(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    current_password: &str,
    new_password: &str,
) -> Result<(), AppError> {
    let stored =
        get_stored(conn)?.ok_or_else(|| AppError::unauthorized("no master password is set"))?;
    let (salt, stored_hash) = decode_stored(&stored)?;

    // The stored hash IS the derived encryption key; deriving from the supplied
    // current password and comparing in constant time both authenticates the
    // password and recovers the old key for re-keying.
    let old_key = derive_key(current_password.as_bytes(), &salt)?;
    if !ct_eq(&old_key, &stored_hash) {
        return Err(AppError::unauthorized("current password is incorrect"));
    }

    validate_password_len(new_password)?;

    let mut new_salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut new_salt);
    let new_key = derive_key(new_password.as_bytes(), &new_salt)?;

    let new_stored = StoredMasterPassword {
        salt: b64_encode(&new_salt),
        hash: b64_encode(&new_key),
    };

    // Re-key settings and replace the verifier atomically.
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| AppError::internal(format!("failed to begin transaction: {e}")))?;
    rekey_settings(&tx, &old_key, &new_key)?;
    rekey_prompt_fields(&tx, &old_key, &new_key)?;
    rekey_prompt_messages(&tx, &old_key, &new_key)?;
    rekey_profile_credentials(&tx, &old_key, &new_key)?;
    save_stored(&tx, &new_stored)?;
    tx.commit()
        .map_err(|e| AppError::internal(format!("failed to commit re-key: {e}")))?;

    cache_key(encryption, new_key.to_vec())?;
    Ok(())
}

/// Unlocks the application by verifying `password` against the stored verifier
/// and caching the derived key (Requirements 15.6, 15.7, 15.10).
///
/// On a correct password the derived key is cached (unlocked). On an incorrect
/// password it returns `UNAUTHORIZED` and leaves the lock state unchanged (the
/// app stays locked); no key is cached and no plaintext is exposed.
pub fn unlock(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    password: &str,
) -> Result<(), AppError> {
    let stored =
        get_stored(conn)?.ok_or_else(|| AppError::unauthorized("no master password is set"))?;
    let (salt, stored_hash) = decode_stored(&stored)?;

    let derived = derive_key(password.as_bytes(), &salt)?;
    if ct_eq(&derived, &stored_hash) {
        cache_key(encryption, derived.to_vec())?;
        Ok(())
    } else {
        Err(AppError::unauthorized("master password is incorrect"))
    }
}

/// Locks the application by clearing the cached key (Requirement 15.8).
pub fn lock(encryption: &Mutex<EncryptionState>) -> Result<(), AppError> {
    let mut enc = lock_state(encryption)?;
    enc.derived_key = None;
    enc.locked = true;
    Ok(())
}

/// Returns a copy of the cached key, or `None` while locked.
pub(crate) fn unlocked_key(
    encryption: &Mutex<EncryptionState>,
) -> Result<Option<Vec<u8>>, AppError> {
    let enc = lock_state(encryption)?;
    if enc.is_unlocked() {
        Ok(enc.derived_key.clone())
    } else {
        Ok(None)
    }
}

/// Encrypts `plaintext` with `key` using AES-256-GCM, returning the
/// `"ENC::" + base64(iv12 || tag16 || ciphertext)` envelope (Requirement 15.9).
///
/// A fresh random 12-byte nonce is generated per call. `key` must be 32 bytes.
pub fn encrypt(plaintext: &str, key: &[u8]) -> Result<String, AppError> {
    let cipher = new_cipher(key)?;

    let mut iv = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut iv);

    // `Aead::encrypt` returns `ciphertext || tag`; reorder to `tag || ciphertext`
    // behind the IV to match the fixed envelope layout.
    let ct_and_tag = cipher
        .encrypt(Nonce::from_slice(&iv), plaintext.as_bytes())
        .map_err(|_| AppError::internal("encryption failed"))?;
    if ct_and_tag.len() < TAG_LEN {
        return Err(AppError::internal(
            "ciphertext shorter than authentication tag",
        ));
    }
    let ct_len = ct_and_tag.len() - TAG_LEN;
    let (ciphertext, tag) = ct_and_tag.split_at(ct_len);

    let mut payload = Vec::with_capacity(NONCE_LEN + TAG_LEN + ciphertext.len());
    payload.extend_from_slice(&iv);
    payload.extend_from_slice(tag);
    payload.extend_from_slice(ciphertext);

    Ok(format!("{ENC_PREFIX}{}", b64_encode(&payload)))
}

/// Decrypts an `"ENC::"`-prefixed envelope produced by [`encrypt`]
/// (Requirements 15.10, 15.11).
///
/// Values without the `ENC::` prefix are returned unchanged (Reference_App
/// parity: only encrypted values are transformed). A wrong key or tampered
/// data fails the GCM authentication-tag check and returns an error **without**
/// returning any decrypted content.
pub fn decrypt(data: &str, key: &[u8]) -> Result<String, AppError> {
    let Some(b64_payload) = data.strip_prefix(ENC_PREFIX) else {
        // Not an encrypted value; pass through unchanged.
        return Ok(data.to_string());
    };

    let cipher = new_cipher(key)?;
    let buf = b64_decode(b64_payload)?;
    if buf.len() < NONCE_LEN + TAG_LEN {
        return Err(AppError::validation("ciphertext is too short to decrypt"));
    }

    let iv = &buf[0..NONCE_LEN];
    let tag = &buf[NONCE_LEN..NONCE_LEN + TAG_LEN];
    let ciphertext = &buf[NONCE_LEN + TAG_LEN..];

    // Reassemble the `ciphertext || tag` layout the aes-gcm crate expects.
    let mut ct_and_tag = Vec::with_capacity(ciphertext.len() + TAG_LEN);
    ct_and_tag.extend_from_slice(ciphertext);
    ct_and_tag.extend_from_slice(tag);

    let plaintext = cipher
        .decrypt(Nonce::from_slice(iv), ct_and_tag.as_ref())
        .map_err(|_| AppError::unauthorized("decryption failed: wrong key or corrupted data"))?;

    String::from_utf8(plaintext)
        .map_err(|_| AppError::internal("decrypted data is not valid UTF-8"))
}

/// Re-keys a single value: an `ENC::` envelope is decrypted with `old_key` and
/// re-encrypted with `new_key`; any other value is returned unchanged.
pub fn rekey_value(value: &str, old_key: &[u8], new_key: &[u8]) -> Result<String, AppError> {
    if value.starts_with(ENC_PREFIX) {
        let plaintext = decrypt(value, old_key)?;
        encrypt(&plaintext, new_key)
    } else {
        Ok(value.to_string())
    }
}

// --- internal helpers -------------------------------------------------------

/// Builds an AES-256-GCM cipher, validating the key length.
fn new_cipher(key: &[u8]) -> Result<Aes256Gcm, AppError> {
    if key.len() != KEY_LEN {
        return Err(AppError::internal("encryption key must be 32 bytes"));
    }
    Ok(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key)))
}

/// Derives a 32-byte key via scrypt with the fixed interactive parameters.
fn derive_key(password: &[u8], salt: &[u8]) -> Result<[u8; KEY_LEN], AppError> {
    let params = Params::new(LOG_N, R, P, KEY_LEN)
        .map_err(|e| AppError::internal(format!("invalid scrypt params: {e}")))?;
    let mut out = [0u8; KEY_LEN];
    scrypt::scrypt(password, salt, &params, &mut out)
        .map_err(|e| AppError::internal(format!("key derivation failed: {e}")))?;
    Ok(out)
}

/// Validates the master-password character length is 8–128 inclusive.
fn validate_password_len(password: &str) -> Result<(), AppError> {
    let len = password.chars().count();
    if !(MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&len) {
        return Err(AppError::validation(format!(
            "master password must be {MIN_PASSWORD_LEN} to {MAX_PASSWORD_LEN} characters"
        )));
    }
    Ok(())
}

/// Reads and parses the stored verifier from the settings table.
fn get_stored(conn: &Connection) -> Result<Option<StoredMasterPassword>, AppError> {
    let json: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![SETTINGS_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| AppError::internal(format!("failed to read master password: {e}")))?;

    match json {
        None => Ok(None),
        Some(raw) => {
            let parsed: StoredMasterPassword = serde_json::from_str(&raw).map_err(|e| {
                AppError::internal(format!("stored verifier is not valid JSON: {e}"))
            })?;
            Ok(Some(parsed))
        }
    }
}

/// Persists the verifier into the settings table (insert or replace).
fn save_stored(conn: &Connection, stored: &StoredMasterPassword) -> Result<(), AppError> {
    let json = serde_json::to_string(stored)
        .map_err(|e| AppError::internal(format!("failed to serialize verifier: {e}")))?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![SETTINGS_KEY, json],
    )
    .map_err(|e| AppError::internal(format!("failed to persist verifier: {e}")))?;
    Ok(())
}

/// Decodes the base64 `{salt, hash}` verifier into fixed-size byte arrays,
/// validating their lengths.
fn decode_stored(
    stored: &StoredMasterPassword,
) -> Result<([u8; SALT_LEN], [u8; KEY_LEN]), AppError> {
    let salt_bytes = b64_decode(&stored.salt)?;
    let hash_bytes = b64_decode(&stored.hash)?;
    if salt_bytes.len() != SALT_LEN || hash_bytes.len() != KEY_LEN {
        return Err(AppError::internal(
            "stored master password verifier is malformed",
        ));
    }
    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&salt_bytes);
    let mut hash = [0u8; KEY_LEN];
    hash.copy_from_slice(&hash_bytes);
    Ok((salt, hash))
}

/// Re-keys every `ENC::` value in the settings table (excluding the verifier).
fn rekey_settings(conn: &Connection, old_key: &[u8], new_key: &[u8]) -> Result<(), AppError> {
    let rows: Vec<(String, String)> = {
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings WHERE key != ?1")
            .map_err(|e| AppError::internal(format!("failed to prepare re-key scan: {e}")))?;
        let mapped = stmt
            .query_map(params![SETTINGS_KEY], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| AppError::internal(format!("failed to scan settings: {e}")))?;
        mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::internal(format!("failed to read settings row: {e}")))?
    };

    for (key, value) in rows {
        if value.starts_with(ENC_PREFIX) {
            let reencrypted = rekey_value(&value, old_key, new_key)?;
            conn.execute(
                "UPDATE settings SET value = ?2 WHERE key = ?1",
                params![key, reencrypted],
            )
            .map_err(|e| AppError::internal(format!("failed to update re-keyed value: {e}")))?;
        }
    }
    Ok(())
}

fn rekey_prompt_fields(conn: &Connection, old_key: &[u8], new_key: &[u8]) -> Result<(), AppError> {
    for column in [
        "description",
        "system_prompt",
        "user_prompt",
        "source",
        "notes",
        "last_ai_response",
    ] {
        rekey_column(conn, "prompts", column, old_key, new_key)?;
    }
    for column in [
        "description",
        "system_prompt",
        "user_prompt",
        "source",
        "notes",
        "ai_response",
    ] {
        rekey_column(conn, "prompt_versions", column, old_key, new_key)?;
    }
    Ok(())
}

fn rekey_prompt_messages(
    conn: &Connection,
    old_key: &[u8],
    new_key: &[u8],
) -> Result<(), AppError> {
    for table in ["prompts", "prompt_versions"] {
        let select = format!("SELECT id, messages FROM {table} WHERE is_private = 1");
        let rows: Vec<(String, String)> = conn
            .prepare(&select)
            .and_then(|mut stmt| {
                stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect()
            })
            .map_err(|e| AppError::internal(format!("failed to read private messages: {e}")))?;
        let update = format!("UPDATE {table} SET messages = ?1 WHERE id = ?2");
        for (id, raw) in rows {
            let mut messages: Vec<crate::models::PromptMessage> = serde_json::from_str(&raw)
                .map_err(|e| {
                    AppError::internal(format!("failed to decode private messages: {e}"))
                })?;
            for message in &mut messages {
                if message.content.starts_with(ENC_PREFIX) {
                    message.content = rekey_value(&message.content, old_key, new_key)?;
                }
            }
            let encoded = serde_json::to_string(&messages).map_err(|e| {
                AppError::internal(format!("failed to encode private messages: {e}"))
            })?;
            conn.execute(&update, params![encoded, id]).map_err(|e| {
                AppError::internal(format!("failed to re-key private messages: {e}"))
            })?;
        }
    }
    Ok(())
}

fn rekey_profile_credentials(
    conn: &Connection,
    old_key: &[u8],
    new_key: &[u8],
) -> Result<(), AppError> {
    let rows: Vec<(String, String)> = conn
        .prepare(
            "SELECT id, credential FROM execution_profile_revisions WHERE credential IS NOT NULL",
        )
        .and_then(|mut stmt| {
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect()
        })
        .map_err(|e| AppError::internal(format!("failed to read profile credentials: {e}")))?;
    for (id, credential) in rows {
        if credential.starts_with(ENC_PREFIX) {
            let reencrypted = rekey_value(&credential, old_key, new_key)?;
            conn.execute(
                "UPDATE execution_profile_revisions SET credential = ?1 WHERE id = ?2",
                params![reencrypted, id],
            )
            .map_err(|e| AppError::internal(format!("failed to re-key profile credential: {e}")))?;
        }
    }
    Ok(())
}

fn rekey_column(
    conn: &Connection,
    table: &str,
    column: &str,
    old_key: &[u8],
    new_key: &[u8],
) -> Result<(), AppError> {
    let select = format!("SELECT id, {column} FROM {table} WHERE is_private = 1");
    let rows: Vec<(String, Option<String>)> = {
        let mut stmt = conn
            .prepare(&select)
            .map_err(|e| AppError::internal(format!("failed to prepare private re-key: {e}")))?;
        let mapped = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| AppError::internal(format!("failed to scan private values: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::internal(format!("failed to read private value: {e}")))?;
        mapped
    };
    let update = format!("UPDATE {table} SET {column} = ?1 WHERE id = ?2");
    for (id, value) in rows {
        if let Some(value) = value.filter(|value| value.starts_with(ENC_PREFIX)) {
            let reencrypted = rekey_value(&value, old_key, new_key)?;
            conn.execute(&update, params![reencrypted, id])
                .map_err(|e| {
                    AppError::internal(format!("failed to update private re-keyed value: {e}"))
                })?;
        }
    }
    Ok(())
}

/// Caches the derived key in the encryption state (unlocked).
fn cache_key(encryption: &Mutex<EncryptionState>, key: Vec<u8>) -> Result<(), AppError> {
    let mut enc = lock_state(encryption)?;
    enc.derived_key = Some(key);
    enc.locked = false;
    Ok(())
}

/// Locks the encryption-state mutex, mapping poisoning to an internal error.
fn lock_state(
    encryption: &Mutex<EncryptionState>,
) -> Result<std::sync::MutexGuard<'_, EncryptionState>, AppError> {
    encryption
        .lock()
        .map_err(|_| AppError::internal("encryption state lock poisoned"))
}

/// Constant-time byte-slice equality (no early return on first mismatch).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Base64-encodes bytes using the standard alphabet.
fn b64_encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Base64-decodes a string, mapping failures to a `VALIDATION` error.
fn b64_decode(s: &str) -> Result<Vec<u8>, AppError> {
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|_| AppError::validation("invalid base64 payload"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{create_memory_pool, init_schema, DbPool};

    /// Builds an in-memory pool with the schema initialized.
    fn test_pool() -> DbPool {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();
        pool
    }

    /// A 32-byte key derived directly (bypassing storage) for crypto-only tests.
    fn key_from(password: &str, salt: &[u8; SALT_LEN]) -> Vec<u8> {
        derive_key(password.as_bytes(), salt).unwrap().to_vec()
    }

    // --- password length validation (Requirement 15.3) ---------------------

    #[test]
    fn password_length_boundaries_7_8_128_129() {
        assert!(validate_password_len(&"a".repeat(7)).is_err(), "7 rejected");
        assert!(validate_password_len(&"a".repeat(8)).is_ok(), "8 accepted");
        assert!(
            validate_password_len(&"a".repeat(128)).is_ok(),
            "128 accepted"
        );
        assert!(
            validate_password_len(&"a".repeat(129)).is_err(),
            "129 rejected"
        );
    }

    #[test]
    fn set_master_password_rejects_short_password_without_changing_state() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());

        let err = set_master_password(&conn, &enc, "short").unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
        // No verifier was stored, state stays locked.
        assert!(get_stored(&conn).unwrap().is_none());
        assert!(!enc.lock().unwrap().is_unlocked());
    }

    #[test]
    fn set_master_password_rejects_too_long_password() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());

        let err = set_master_password(&conn, &enc, &"a".repeat(129)).unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
        assert!(get_stored(&conn).unwrap().is_none());
    }

    // --- encrypt/decrypt round-trip (Requirements 15.9, 15.11) -------------

    #[test]
    fn encrypt_decrypt_round_trip_returns_original() {
        let key = key_from("correct horse battery", &[7u8; SALT_LEN]);
        for plaintext in [
            "",
            "hello world",
            "unicode: 你好 🌍 émoji",
            &"x".repeat(5000),
        ] {
            let envelope = encrypt(plaintext, &key).unwrap();
            assert!(envelope.starts_with(ENC_PREFIX));
            assert!(!envelope.contains(plaintext) || plaintext.is_empty());
            let decrypted = decrypt(&envelope, &key).unwrap();
            assert_eq!(decrypted, plaintext);
        }
    }

    #[test]
    fn encrypt_uses_random_nonce_so_ciphertexts_differ() {
        let key = key_from("password123", &[1u8; SALT_LEN]);
        let a = encrypt("same plaintext", &key).unwrap();
        let b = encrypt("same plaintext", &key).unwrap();
        assert_ne!(a, b, "random nonce should make envelopes differ");
        assert_eq!(decrypt(&a, &key).unwrap(), "same plaintext");
        assert_eq!(decrypt(&b, &key).unwrap(), "same plaintext");
    }

    #[test]
    fn non_enc_prefixed_values_pass_through_unchanged() {
        let key = key_from("password123", &[2u8; SALT_LEN]);
        assert_eq!(decrypt("plain text", &key).unwrap(), "plain text");
        assert_eq!(rekey_value("plain text", &key, &key).unwrap(), "plain text");
    }

    // --- wrong key / tampered data (Requirement 15.10) ---------------------

    #[test]
    fn decrypt_with_wrong_key_fails_without_panicking() {
        let right = key_from("right password", &[3u8; SALT_LEN]);
        let wrong = key_from("wrong password", &[3u8; SALT_LEN]);
        let envelope = encrypt("secret data", &right).unwrap();

        let err = decrypt(&envelope, &wrong).unwrap_err();
        assert_eq!(err.code_str(), "UNAUTHORIZED");
        // Error message must not leak plaintext.
        assert!(!err.message.contains("secret data"));
    }

    #[test]
    fn decrypt_rejects_tampered_ciphertext() {
        let key = key_from("password123", &[4u8; SALT_LEN]);
        let envelope = encrypt("authentic", &key).unwrap();

        // Flip a bit in the payload (after the ENC:: prefix).
        let mut buf = b64_decode(envelope.strip_prefix(ENC_PREFIX).unwrap()).unwrap();
        let last = buf.len() - 1;
        buf[last] ^= 0x01;
        let tampered = format!("{ENC_PREFIX}{}", b64_encode(&buf));

        assert!(decrypt(&tampered, &key).is_err());
    }

    #[test]
    fn decrypt_rejects_truncated_and_invalid_payloads() {
        let key = key_from("password123", &[5u8; SALT_LEN]);
        // Empty payload after prefix.
        assert!(decrypt("ENC::", &key).is_err());
        // Invalid base64.
        assert!(decrypt("ENC::not valid base64!!!", &key).is_err());
        // Valid base64 but too short to hold IV + tag.
        let short = format!("{ENC_PREFIX}{}", b64_encode(&[0u8; 10]));
        assert!(decrypt(&short, &key).is_err());
    }

    // --- set + unlock + lock + status flow (Requirements 15.1, 15.6, 15.8) -

    #[test]
    fn set_unlock_lock_status_flow() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());

        // Initially: no master password, locked.
        let s0 = status(&conn, &enc).unwrap();
        assert!(!s0.has_master_password);
        assert!(s0.is_locked);

        // Set master password: now configured and unlocked.
        set_master_password(&conn, &enc, "password123").unwrap();
        let s1 = status(&conn, &enc).unwrap();
        assert!(s1.has_master_password);
        assert!(!s1.is_locked);

        // Lock: configured but locked.
        lock(&enc).unwrap();
        let s2 = status(&conn, &enc).unwrap();
        assert!(s2.has_master_password);
        assert!(s2.is_locked);

        // Unlock with wrong password: stays locked, error returned.
        let err = unlock(&conn, &enc, "wrong password").unwrap_err();
        assert_eq!(err.code_str(), "UNAUTHORIZED");
        assert!(status(&conn, &enc).unwrap().is_locked);

        // Unlock with correct password: unlocked again.
        unlock(&conn, &enc, "password123").unwrap();
        assert!(!status(&conn, &enc).unwrap().is_locked);
    }

    #[test]
    fn unlocked_key_can_decrypt_previously_encrypted_value() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());

        set_master_password(&conn, &enc, "password123").unwrap();
        let key = enc.lock().unwrap().derived_key.clone().unwrap();
        let envelope = encrypt("private prompt", &key).unwrap();

        lock(&enc).unwrap();
        unlock(&conn, &enc, "password123").unwrap();
        let key_after = enc.lock().unwrap().derived_key.clone().unwrap();
        assert_eq!(decrypt(&envelope, &key_after).unwrap(), "private prompt");
    }

    // --- change_master_password re-key (Requirements 15.3, 15.4, 15.5) -----

    #[test]
    fn change_master_password_rekeys_and_preserves_access() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());

        // Set an initial password and encrypt a value stored in settings.
        set_master_password(&conn, &enc, "oldpassword").unwrap();
        let old_key = enc.lock().unwrap().derived_key.clone().unwrap();
        let envelope = encrypt("sensitive value", &old_key).unwrap();
        save_setting(&conn, "secret_blob", &envelope);

        // Change the password.
        change_master_password(&conn, &enc, "oldpassword", "newpassword").unwrap();

        // The settings value was re-keyed: it is still ENC:: and readable under
        // the new key, but no longer under the old key.
        let new_key = enc.lock().unwrap().derived_key.clone().unwrap();
        assert_ne!(old_key, new_key, "key should change after re-key");
        let stored_blob = read_setting(&conn, "secret_blob");
        assert!(stored_blob.starts_with(ENC_PREFIX));
        assert_eq!(decrypt(&stored_blob, &new_key).unwrap(), "sensitive value");
        assert!(decrypt(&stored_blob, &old_key).is_err());

        // The old password no longer unlocks; the new password does.
        lock(&enc).unwrap();
        assert!(unlock(&conn, &enc, "oldpassword").is_err());
        assert!(status(&conn, &enc).unwrap().is_locked);
        unlock(&conn, &enc, "newpassword").unwrap();
        assert!(!status(&conn, &enc).unwrap().is_locked);
    }

    #[test]
    fn change_master_password_with_wrong_current_password_changes_nothing() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());

        set_master_password(&conn, &enc, "oldpassword").unwrap();
        let before = get_stored(&conn).unwrap().unwrap();

        let err = change_master_password(&conn, &enc, "incorrect", "newpassword").unwrap_err();
        assert_eq!(err.code_str(), "UNAUTHORIZED");

        // Verifier unchanged; old password still unlocks.
        let after = get_stored(&conn).unwrap().unwrap();
        assert_eq!(before, after);
        lock(&enc).unwrap();
        assert!(unlock(&conn, &enc, "oldpassword").is_ok());
    }

    #[test]
    fn change_master_password_rejects_invalid_new_password_length() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());

        set_master_password(&conn, &enc, "oldpassword").unwrap();
        let before = get_stored(&conn).unwrap().unwrap();

        let err = change_master_password(&conn, &enc, "oldpassword", "short").unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");

        // Verifier unchanged because validation fails before re-keying.
        assert_eq!(get_stored(&conn).unwrap().unwrap(), before);
    }

    // Small settings helpers for tests.
    fn save_setting(conn: &Connection, key: &str, value: &str) {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .unwrap();
    }

    fn read_setting(conn: &Connection, key: &str) -> String {
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .unwrap()
    }
}
