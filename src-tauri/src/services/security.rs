//! Security_Service: master password and AES-256-GCM encryption (Requirement 15).
//!
//! This service reproduces the construction used by the Reference_App
//! (`ref/PromptHub/apps/desktop/src/main/security.ts`) so the round-trip and
//! authentication-tag properties of Requirement 15 hold, then wraps a random
//! data-encryption key so a stolen database file does not contain the DEK:
//!
//! - **Key derivation.** `KEK = scrypt(password, kdf_salt16, params) -> 32 bytes`,
//!   with fixed interactive parameters `log_n = 15` (N = 32768), `r = 8`,
//!   `p = 1`. These parameters are held constant so previously stored verifiers
//!   stay valid.
//! - **DEK.** A random 32-byte data-encryption key is generated at
//!   `set_master_password` / re-key time and lives only in [`EncryptionState`]
//!   after unlock. `ENC::` rows are encrypted with the DEK, never with the KEK.
//! - **Verifier (v2).** `{ v: 2, kdfSalt, verifier, wrappedDek }` is persisted
//!   in the `settings` table under [`SETTINGS_KEY`]. `verifier` is
//!   HMAC-SHA256(KEK, "prompthub/v2/verifier"); the wrap key is
//!   HMAC-SHA256(KEK, "prompthub/v2/dek-wrap"). The stored verifier is not the
//!   DEK.
//! - **Verifier (v1).** Legacy `{ salt, hash }` where `hash` *is* the DEK.
//!   A successful unlock or password change migrates every owned `ENC::` value
//!   to a new DEK and writes v2 in the same transaction.
//! - **Envelope.** `encrypt` produces `"ENC::" + base64(iv12 || tag16 ||
//!   ciphertext)` using AES-256-GCM with a random 12-byte nonce and the 16-byte
//!   authentication tag. This byte layout (IV, then TAG, then CIPHERTEXT) matches
//!   the Reference_App. Note that the `aes-gcm` crate's `Aead::encrypt` returns
//!   `ciphertext || tag`, so `encrypt`/`decrypt` reorder the tag explicitly to
//!   keep the on-the-wire layout fixed.
//!
//! Security notes: the master-password verifier is compared in constant time
//! ([`ct_eq`]); secrets (passwords, derived keys, plaintext) are never logged or
//! echoed; salts, nonces, and the DEK come from the OS CSPRNG ([`OsRng`]);
//! cached DEK bytes are zeroized on `lock` and on Drop.
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
use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use scrypt::Params;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroize;

use crate::error::AppError;
use crate::models::{StoredMasterPassword, StoredMasterPasswordV2, StoredVerifier};
use crate::state::EncryptionState;

/// Settings-table key under which the master-password verifier JSON is stored.
const SETTINGS_KEY: &str = "master_password";
/// Settings-table key of the application settings JSON document.
const APP_SETTINGS_KEY: &str = "app";
/// HMAC info that produces the stored password verifier from the KEK.
const VERIFIER_INFO: &[u8] = b"prompthub/v2/verifier";
/// HMAC info that produces the DEK wrap key from the KEK.
const WRAP_INFO: &[u8] = b"prompthub/v2/dek-wrap";
/// Current persisted verifier format.
const VERIFIER_VERSION: u32 = 2;

type HmacSha256 = Hmac<Sha256>;

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

/// Returns whether a master-password verifier is persisted.
pub fn has_master_password(conn: &Connection) -> Result<bool, AppError> {
    Ok(get_stored(conn)?.is_some())
}

/// Establishes (or replaces) the master password and caches the DEK
/// (Requirements 15.2, 15.3).
///
/// Validates the password length is 8–128 inclusive, returning a `VALIDATION`
/// error and making **no change** otherwise. On success a random DEK is
/// generated, wrapped with a KEK derived from a fresh salt, a v2 verifier is
/// persisted, existing plaintext settings secrets are sealed, and the DEK is
/// cached (unlocked).
///
/// This is a low-level primitive used by the public "set master password"
/// flow (which the Command_Layer gates on "no master password exists" per 15.2).
/// Re-keying existing `ENC::` rows is owned by [`change_master_password`].
pub fn set_master_password(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    password: &str,
) -> Result<(), AppError> {
    validate_password_len(password)?;

    let dek = random_key()?;
    let stored = build_v2(password, &dek)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| AppError::internal(format!("failed to begin transaction: {e}")))?;
    seal_app_settings_secrets(&tx, &dek)?;
    save_stored(&tx, &StoredVerifier::V2(stored))?;
    tx.commit()
        .map_err(|e| AppError::internal(format!("failed to persist verifier: {e}")))?;
    cache_key(encryption, dek.to_vec())?;
    Ok(())
}

/// Re-keys encrypted data to a new master password (Requirements 15.4, 15.5).
///
/// Verifies `current_password` against the stored verifier in constant time;
/// on mismatch returns `UNAUTHORIZED` and leaves the master password and data
/// unchanged (15.5). Validates `new_password` length (15.3). On success it
/// generates a new DEK, re-encrypts every owned `ENC::` value under that DEK,
/// wraps the DEK with a new KEK, writes a v2 verifier, and caches the new DEK
/// — all within a single transaction so the verifier and the re-keyed data
/// update atomically (preserving access to previously encrypted data per 15.4).
///
/// Prompt and revision content fields, profile credentials, settings secrets,
/// and private evaluation-run payloads are re-keyed together so the verifier
/// and every protected value stay consistent.
pub fn change_master_password(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    current_password: &str,
    new_password: &str,
) -> Result<(), AppError> {
    let stored =
        get_stored(conn)?.ok_or_else(|| AppError::unauthorized("no master password is set"))?;
    let old_dek = recover_dek(&stored, current_password, "current password is incorrect")?;
    validate_password_len(new_password)?;
    persist_new_dek(conn, encryption, new_password, &old_dek)
}

/// Unlocks the application by verifying `password` against the stored verifier
/// and caching the DEK (Requirements 15.6, 15.7, 15.10).
///
/// On a correct password the DEK is cached (unlocked). A v1 verifier is migrated
/// to v2 in the same transaction as the re-key; failure leaves the v1 document
/// and `user_version` unchanged. On an incorrect password it returns
/// `UNAUTHORIZED` and leaves the lock state unchanged (the app stays locked);
/// no key is cached and no plaintext is exposed.
pub fn unlock(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    password: &str,
) -> Result<(), AppError> {
    let stored =
        get_stored(conn)?.ok_or_else(|| AppError::unauthorized("no master password is set"))?;
    let dek = recover_dek(&stored, password, "master password is incorrect")?;
    match stored {
        StoredVerifier::V2(_) => cache_key(encryption, dek.to_vec()),
        StoredVerifier::V1(_) => persist_new_dek(conn, encryption, password, &dek),
    }
}

/// Locks the application by clearing and zeroizing the cached DEK (Requirement 15.8).
pub fn lock(encryption: &Mutex<EncryptionState>) -> Result<(), AppError> {
    let mut enc = lock_state(encryption)?;
    if let Some(mut key) = enc.derived_key.take() {
        key.zeroize();
    }
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
fn get_stored(conn: &Connection) -> Result<Option<StoredVerifier>, AppError> {
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
            let parsed: StoredVerifier = serde_json::from_str(&raw).map_err(|e| {
                AppError::internal(format!("stored verifier is not valid JSON: {e}"))
            })?;
            Ok(Some(parsed))
        }
    }
}

/// Persists the verifier into the settings table (insert or replace).
fn save_stored(conn: &Connection, stored: &StoredVerifier) -> Result<(), AppError> {
    let json = serde_json::to_string(stored)
        .map_err(|e| AppError::internal(format!("failed to serialize verifier: {e}")))?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![SETTINGS_KEY, json],
    )
    .map_err(|e| AppError::internal(format!("failed to persist verifier: {e}")))?;
    Ok(())
}

/// Recovers the DEK from a stored verifier. Wrong passwords return `UNAUTHORIZED`.
fn recover_dek(
    stored: &StoredVerifier,
    password: &str,
    incorrect: &str,
) -> Result<[u8; KEY_LEN], AppError> {
    match stored {
        StoredVerifier::V1(v1) => recover_dek_v1(v1, password, incorrect),
        StoredVerifier::V2(v2) => recover_dek_v2(v2, password, incorrect),
    }
}

fn recover_dek_v1(
    stored: &StoredMasterPassword,
    password: &str,
    incorrect: &str,
) -> Result<[u8; KEY_LEN], AppError> {
    let (salt, stored_hash) = decode_v1(stored)?;
    let derived = derive_key(password.as_bytes(), &salt)?;
    if !ct_eq(&derived, &stored_hash) {
        return Err(AppError::unauthorized(incorrect));
    }
    Ok(derived)
}

fn recover_dek_v2(
    stored: &StoredMasterPasswordV2,
    password: &str,
    incorrect: &str,
) -> Result<[u8; KEY_LEN], AppError> {
    if stored.v != VERIFIER_VERSION {
        return Err(AppError::internal(
            "unsupported master password verifier version",
        ));
    }
    let kdf_salt = decode_fixed::<SALT_LEN>(&stored.kdf_salt)?;
    let stored_verifier = decode_fixed::<KEY_LEN>(&stored.verifier)?;
    let mut kek = derive_key(password.as_bytes(), &kdf_salt)?;
    let verifier = keyed_digest(&kek, VERIFIER_INFO)?;
    if !ct_eq(&verifier, &stored_verifier) {
        kek.zeroize();
        return Err(AppError::unauthorized(incorrect));
    }
    let mut wrap_key = keyed_digest(&kek, WRAP_INFO)?;
    kek.zeroize();
    let dek = unwrap_dek(&stored.wrapped_dek, &wrap_key);
    wrap_key.zeroize();
    dek
}

/// Generates a new DEK, re-keys owned ciphertext, writes v2, and caches the DEK.
fn persist_new_dek(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    password: &str,
    old_dek: &[u8],
) -> Result<(), AppError> {
    let new_dek = random_key()?;
    let stored = build_v2(password, &new_dek)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| AppError::internal(format!("failed to begin transaction: {e}")))?;
    rekey_all(&tx, old_dek, &new_dek)?;
    save_stored(&tx, &StoredVerifier::V2(stored))?;
    tx.commit()
        .map_err(|e| AppError::internal(format!("failed to commit re-key: {e}")))?;
    cache_key(encryption, new_dek.to_vec())
}

fn rekey_all(conn: &Connection, old_key: &[u8], new_key: &[u8]) -> Result<(), AppError> {
    rekey_settings(conn, old_key, new_key)?;
    rekey_app_settings_secrets(conn, old_key, new_key)?;
    rekey_prompt_fields(conn, old_key, new_key)?;
    rekey_prompt_messages(conn, old_key, new_key)?;
    rekey_profile_credentials(conn, old_key, new_key)?;
    rekey_prompt_runs(conn, old_key, new_key)?;
    Ok(())
}

fn build_v2(password: &str, dek: &[u8]) -> Result<StoredMasterPasswordV2, AppError> {
    let kdf_salt = random_salt()?;
    let mut kek = derive_key(password.as_bytes(), &kdf_salt)?;
    let verifier = keyed_digest(&kek, VERIFIER_INFO)?;
    let mut wrap_key = keyed_digest(&kek, WRAP_INFO)?;
    kek.zeroize();
    let wrapped_dek = wrap_dek(dek, &wrap_key)?;
    wrap_key.zeroize();
    Ok(StoredMasterPasswordV2 {
        v: VERIFIER_VERSION,
        kdf_salt: b64_encode(&kdf_salt),
        verifier: b64_encode(&verifier),
        wrapped_dek,
    })
}

fn wrap_dek(dek: &[u8], wrap_key: &[u8]) -> Result<String, AppError> {
    encrypt(&b64_encode(dek), wrap_key)
}

fn unwrap_dek(wrapped: &str, wrap_key: &[u8]) -> Result<[u8; KEY_LEN], AppError> {
    let encoded = decrypt(wrapped, wrap_key)?;
    decode_fixed::<KEY_LEN>(&encoded)
}

fn keyed_digest(key: &[u8], info: &[u8]) -> Result<[u8; KEY_LEN], AppError> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key)
        .map_err(|_| AppError::internal("invalid HMAC key"))?;
    mac.update(info);
    let result = mac.finalize().into_bytes();
    let mut out = [0u8; KEY_LEN];
    out.copy_from_slice(&result);
    Ok(out)
}

fn random_key() -> Result<[u8; KEY_LEN], AppError> {
    let mut key = [0u8; KEY_LEN];
    OsRng.fill_bytes(&mut key);
    Ok(key)
}

fn random_salt() -> Result<[u8; SALT_LEN], AppError> {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    Ok(salt)
}

fn decode_fixed<const N: usize>(value: &str) -> Result<[u8; N], AppError> {
    let bytes = b64_decode(value)?;
    if bytes.len() != N {
        return Err(AppError::internal(
            "stored master password verifier is malformed",
        ));
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn decode_v1(stored: &StoredMasterPassword) -> Result<([u8; SALT_LEN], [u8; KEY_LEN]), AppError> {
    Ok((decode_fixed(&stored.salt)?, decode_fixed(&stored.hash)?))
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

fn rekey_prompt_runs(conn: &Connection, old_key: &[u8], new_key: &[u8]) -> Result<(), AppError> {
    let rows: Vec<(String, String, String, Option<String>)> = conn
        .prepare("SELECT id, inputs, rendered_messages, output FROM prompt_runs")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect()
        })
        .map_err(|e| AppError::internal(format!("failed to read prompt runs: {e}")))?;
    for (id, inputs, rendered, output) in rows {
        let inputs = rekey_value(&inputs, old_key, new_key)?;
        let rendered = rekey_value(&rendered, old_key, new_key)?;
        let output = match output {
            Some(value) => Some(rekey_value(&value, old_key, new_key)?),
            None => None,
        };
        conn.execute(
            "UPDATE prompt_runs SET inputs=?1, rendered_messages=?2, output=?3 WHERE id=?4",
            params![inputs, rendered, output, id],
        )
        .map_err(|e| AppError::internal(format!("failed to re-key prompt run: {e}")))?;
    }
    Ok(())
}

fn load_app_settings_json(conn: &Connection) -> Result<Option<serde_json::Value>, AppError> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![APP_SETTINGS_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| AppError::internal(format!("failed to read app settings: {e}")))?;
    match raw {
        None => Ok(None),
        Some(json) => serde_json::from_str(&json)
            .map_err(|e| AppError::internal(format!("stored settings are corrupt: {e}")))
            .map(Some),
    }
}

fn save_app_settings_json(conn: &Connection, value: &serde_json::Value) -> Result<(), AppError> {
    let json = serde_json::to_string(value)
        .map_err(|e| AppError::internal(format!("failed to encode settings: {e}")))?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![APP_SETTINGS_KEY, json],
    )
    .map_err(|e| AppError::internal(format!("failed to persist settings: {e}")))?;
    Ok(())
}

fn seal_secret_string(value: &mut serde_json::Value, key: &[u8]) -> Result<bool, AppError> {
    let Some(plain) = value.as_str() else {
        return Ok(false);
    };
    if plain.is_empty() || is_encrypted_value(plain) {
        return Ok(false);
    }
    *value = serde_json::Value::String(encrypt(plain, key)?);
    Ok(true)
}

fn rekey_secret_string(
    value: &mut serde_json::Value,
    old_key: &[u8],
    new_key: &[u8],
) -> Result<bool, AppError> {
    let Some(current) = value.as_str() else {
        return Ok(false);
    };
    if current.is_empty() {
        return Ok(false);
    }
    if is_encrypted_value(current) {
        *value = serde_json::Value::String(rekey_value(current, old_key, new_key)?);
        return Ok(true);
    }
    *value = serde_json::Value::String(encrypt(current, new_key)?);
    Ok(true)
}

fn mutate_settings_secrets(
    doc: &mut serde_json::Value,
    mut mutate: impl FnMut(&mut serde_json::Value) -> Result<bool, AppError>,
) -> Result<bool, AppError> {
    let mut changed = false;
    if let Some(token) = doc.get_mut("githubToken") {
        changed |= mutate(token)?;
    }
    if let Some(password) = doc.pointer_mut("/sync/password") {
        changed |= mutate(password)?;
    }
    Ok(changed)
}

fn seal_app_settings_secrets(conn: &Connection, key: &[u8]) -> Result<(), AppError> {
    let Some(mut doc) = load_app_settings_json(conn)? else {
        return Ok(());
    };
    if mutate_settings_secrets(&mut doc, |value| seal_secret_string(value, key))? {
        save_app_settings_json(conn, &doc)?;
    }
    Ok(())
}

fn rekey_app_settings_secrets(
    conn: &Connection,
    old_key: &[u8],
    new_key: &[u8],
) -> Result<(), AppError> {
    let Some(mut doc) = load_app_settings_json(conn)? else {
        return Ok(());
    };
    if mutate_settings_secrets(&mut doc, |value| {
        rekey_secret_string(value, old_key, new_key)
    })? {
        save_app_settings_json(conn, &doc)?;
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

/// Caches the DEK in the encryption state (unlocked), zeroizing any previous key.
fn cache_key(encryption: &Mutex<EncryptionState>, key: Vec<u8>) -> Result<(), AppError> {
    let mut enc = lock_state(encryption)?;
    if let Some(mut previous) = enc.derived_key.take() {
        previous.zeroize();
    }
    enc.derived_key = Some(key);
    enc.locked = false;
    Ok(())
}

/// Locks the encryption-state mutex, mapping poisoning to an internal error.
fn lock_state(
    encryption: &Mutex<EncryptionState>,
) -> Result<std::sync::MutexGuard<'_, EncryptionState>, AppError> {
    crate::logging::lock_mutex(encryption, "encryption state")
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
    fn unlock_without_verifier_is_unauthorized_and_leaves_lock_unchanged() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());
        let before = status(&conn, &enc).unwrap();
        assert!(!before.has_master_password);
        assert!(before.is_locked);

        let err = unlock(&conn, &enc, "password123").unwrap_err();
        assert_eq!(err.code_str(), "UNAUTHORIZED");

        let after = status(&conn, &enc).unwrap();
        assert_eq!(after.has_master_password, before.has_master_password);
        assert_eq!(after.is_locked, before.is_locked);
        assert!(get_stored(&conn).unwrap().is_none());
    }

    #[test]
    fn change_master_password_without_verifier_is_unauthorized_and_leaves_lock_unchanged() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());
        let before = status(&conn, &enc).unwrap();
        assert!(!before.has_master_password);
        assert!(before.is_locked);

        let err = change_master_password(&conn, &enc, "oldpassword", "newpassword").unwrap_err();
        assert_eq!(err.code_str(), "UNAUTHORIZED");

        let after = status(&conn, &enc).unwrap();
        assert_eq!(after.has_master_password, before.has_master_password);
        assert_eq!(after.is_locked, before.is_locked);
        assert!(get_stored(&conn).unwrap().is_none());
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

    fn persist_v1(conn: &Connection, encryption: &Mutex<EncryptionState>, password: &str) {
        let salt = random_salt().unwrap();
        let dek = derive_key(password.as_bytes(), &salt).unwrap();
        save_stored(
            conn,
            &StoredVerifier::V1(StoredMasterPassword {
                salt: b64_encode(&salt),
                hash: b64_encode(&dek),
            }),
        )
        .unwrap();
        cache_key(encryption, dek.to_vec()).unwrap();
    }

    fn user_version(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn set_master_password_stores_v2_verifier_not_equal_to_dek() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());

        set_master_password(&conn, &enc, "password123").unwrap();
        let stored = get_stored(&conn).unwrap().unwrap();
        let StoredVerifier::V2(v2) = stored else {
            panic!("expected v2 verifier");
        };
        assert_eq!(v2.v, 2);
        let dek = enc.lock().unwrap().derived_key.clone().unwrap();
        let verifier = b64_decode(&v2.verifier).unwrap();
        assert_ne!(verifier, dek, "stored verifier must not equal the DEK");
        assert!(!v2.wrapped_dek.contains(&b64_encode(&dek)));
    }

    #[test]
    fn lock_clears_cached_dek() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());
        set_master_password(&conn, &enc, "password123").unwrap();
        assert!(enc.lock().unwrap().derived_key.is_some());
        lock(&enc).unwrap();
        assert!(enc.lock().unwrap().derived_key.is_none());
        assert!(enc.lock().unwrap().locked);
    }

    #[test]
    fn public_library_without_master_password_round_trips_plaintext() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());
        crate::services::settings::update(&conn, &enc, &serde_json::json!({ "theme": "light" }))
            .unwrap();
        let prompt = crate::services::prompt::create(
            &conn,
            crate::services::prompt::PromptCreate {
                title: "Public".into(),
                user_prompt: "hello public".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(prompt.user_prompt, "hello public");
        let stored: String = conn
            .query_row(
                "SELECT user_prompt FROM prompts WHERE id = ?1",
                params![prompt.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored, "hello public");
        assert!(!stored.starts_with(ENC_PREFIX));
        assert!(get_stored(&conn).unwrap().is_none());
    }

    #[test]
    fn v1_unlock_migrates_to_v2_and_preserves_ciphertext() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());
        persist_v1(&conn, &enc, "oldpassword");
        let old_dek = enc.lock().unwrap().derived_key.clone().unwrap();
        let envelope = encrypt("private prompt", &old_dek).unwrap();
        save_setting(&conn, "secret_blob", &envelope);

        lock(&enc).unwrap();
        unlock(&conn, &enc, "oldpassword").unwrap();

        let stored = get_stored(&conn).unwrap().unwrap();
        let StoredVerifier::V2(v2) = stored else {
            panic!("unlock must migrate v1 to v2");
        };
        assert_eq!(v2.v, 2);
        let new_dek = enc.lock().unwrap().derived_key.clone().unwrap();
        assert_ne!(new_dek, old_dek);
        let stored_blob = read_setting(&conn, "secret_blob");
        assert_eq!(decrypt(&stored_blob, &new_dek).unwrap(), "private prompt");
        assert!(decrypt(&stored_blob, &old_dek).is_err());
        let verifier = b64_decode(&v2.verifier).unwrap();
        assert_ne!(verifier, new_dek);
    }

    #[test]
    fn v1_migrate_failure_leaves_v1_and_user_version() {
        let pool = test_pool();
        let conn = pool.get().unwrap();
        let enc = Mutex::new(EncryptionState::default());
        persist_v1(&conn, &enc, "oldpassword");
        let old_dek = enc.lock().unwrap().derived_key.clone().unwrap();
        save_setting(&conn, "secret_blob", &encrypt("keep-me", &old_dek).unwrap());
        save_setting(&conn, "broken", "ENC::AAAA");
        let version_before = user_version(&conn);
        let before = get_stored(&conn).unwrap().unwrap();
        assert!(matches!(before, StoredVerifier::V1(_)));

        lock(&enc).unwrap();
        let err = unlock(&conn, &enc, "oldpassword").unwrap_err();
        assert_eq!(err.code_str(), "VALIDATION");
        assert!(status(&conn, &enc).unwrap().is_locked);
        let after = get_stored(&conn).unwrap().unwrap();
        assert!(matches!(after, StoredVerifier::V1(_)));
        assert_eq!(after, before);
        assert_eq!(user_version(&conn), version_before);
        assert_eq!(
            decrypt(&read_setting(&conn, "secret_blob"), &old_dek).unwrap(),
            "keep-me"
        );
    }

    #[tokio::test]
    async fn stolen_database_without_password_hides_owned_secrets() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("prompthub.db");
        let pool = crate::storage::create_pool(&database).unwrap();
        let enc = Mutex::new(EncryptionState::default());
        let conn = pool.get().unwrap();
        crate::storage::init_schema(&conn).unwrap();

        set_master_password(&conn, &enc, "password123").unwrap();
        let created = crate::services::prompt::create_secure(
            &conn,
            &enc,
            crate::services::prompt::PromptCreate {
                title: "Private metadata".into(),
                user_prompt: "classified body".into(),
                is_private: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        let credentialed = crate::services::evaluation::create_profile(
            &conn,
            &enc,
            crate::models::ExecutionProfileInput {
                profile_id: None,
                name: "Remote".into(),
                provider: "openai-compatible".into(),
                endpoint: Some("https://example.com/v1/chat/completions".into()),
                model: "model".into(),
                parameters: serde_json::json!({}),
                credential: Some("secret-token".into()),
            },
        )
        .unwrap();
        crate::services::settings::update(
            &conn,
            &enc,
            &serde_json::json!({
                "githubToken": "ghp_secret_token",
                "sync": { "enabled": true, "provider": "webdav", "password": "sync-secret" }
            }),
        )
        .unwrap();
        let profile = crate::services::evaluation::create_profile(
            &conn,
            &enc,
            crate::models::ExecutionProfileInput {
                profile_id: None,
                name: "Mock".into(),
                provider: "mock".into(),
                endpoint: None,
                model: "deterministic".into(),
                parameters: serde_json::json!({ "response": "classified-output" }),
                credential: None,
            },
        )
        .unwrap();
        drop(conn);

        crate::services::evaluation::execute_run(
            &pool,
            &enc,
            &crate::services::evaluation::DefaultProviderAdapter,
            crate::models::PromptRunInput {
                prompt_revision_id: crate::services::version::list(
                    &pool.get().unwrap(),
                    &created.id,
                )
                .unwrap()
                .pop()
                .unwrap()
                .id,
                profile_revision_id: profile.id,
                inputs: std::collections::BTreeMap::from([("name".into(), "Ada".into())]),
                test_case_id: None,
            },
            None,
            &tokio_util::sync::CancellationToken::new(),
            &NoopSink,
        )
        .await
        .unwrap();

        let dek = enc.lock().unwrap().derived_key.clone().unwrap();
        drop(pool);
        drop(enc);

        let stolen = crate::storage::create_pool(&database).unwrap();
        let conn = stolen.get().unwrap();
        let locked = Mutex::new(EncryptionState::default());
        let stored = get_stored(&conn).unwrap().unwrap();
        let StoredVerifier::V2(v2) = stored else {
            panic!("stolen db must persist v2");
        };
        assert_eq!(v2.v, 2);
        assert_ne!(b64_decode(&v2.verifier).unwrap(), dek);

        let haystack = dump_sensitive_text(&conn);
        for needle in [
            "classified body",
            "secret-token",
            "ghp_secret_token",
            "sync-secret",
            "classified-output",
        ] {
            assert!(
                !haystack.contains(needle),
                "stolen sqlite leaked `{needle}`"
            );
        }

        let dto = crate::services::settings::get(&conn).unwrap();
        assert_eq!(dto.has_github_token, Some(true));
        assert_eq!(dto.has_sync_password, Some(true));
        assert!(dto.github_token.is_none());
        assert!(dto
            .sync
            .as_ref()
            .and_then(|sync| sync.password.as_ref())
            .is_none());

        unlock(&conn, &locked, "password123").unwrap();
        let opened = crate::services::prompt::get_secure(&conn, &locked, &created.id).unwrap();
        assert_eq!(opened.user_prompt, "classified body");
        let key = unlocked_key(&locked).unwrap().unwrap();
        let stored_cred: String = conn
            .query_row(
                "SELECT credential FROM execution_profile_revisions WHERE id = ?1",
                params![credentialed.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(decrypt(&stored_cred, &key).unwrap(), "secret-token");
        let app: String = read_setting(&conn, APP_SETTINGS_KEY);
        let stored_settings: crate::models::Settings = serde_json::from_str(&app).unwrap();
        assert_eq!(
            decrypt(stored_settings.github_token.as_deref().unwrap(), &key).unwrap(),
            "ghp_secret_token"
        );
        assert_eq!(
            decrypt(
                stored_settings
                    .sync
                    .as_ref()
                    .and_then(|sync| sync.password.as_deref())
                    .unwrap(),
                &key
            )
            .unwrap(),
            "sync-secret"
        );
        let runs = crate::services::evaluation::list_runs(&conn, &locked).unwrap();
        assert_eq!(runs[0].output.as_deref(), Some("classified-output"));
    }

    struct NoopSink;
    impl crate::services::evaluation::EvaluationEventSink for NoopSink {
        fn emit_run_chunk(&self, _run_id: &str, _chunk: &str) {}
        fn emit_run_terminal(&self, _run_id: &str, _status: &str) {}
        fn emit_matrix_progress(&self, _id: &str, _completed: i64, _total: i64, _cell: &str) {}
    }

    fn dump_sensitive_text(conn: &Connection) -> String {
        let mut chunks = Vec::new();
        let mut stmt = conn.prepare("SELECT key, value FROM settings").unwrap();
        for row in stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
        {
            let (key, value) = row.unwrap();
            chunks.push(format!("{key}={value}"));
        }
        for sql in [
            "SELECT description, system_prompt, user_prompt, source, notes, last_ai_response, messages FROM prompts",
            "SELECT description, system_prompt, user_prompt, source, notes, ai_response, messages FROM prompt_versions",
            "SELECT credential FROM execution_profile_revisions",
            "SELECT inputs, rendered_messages, output FROM prompt_runs",
        ] {
            let mut stmt = conn.prepare(sql).unwrap();
            let width = stmt.column_count();
            let rows = stmt
                .query_map([], |row| {
                    let mut values = Vec::new();
                    for i in 0..width {
                        values.push(row.get::<_, Option<String>>(i)?.unwrap_or_default());
                    }
                    Ok(values.join("\n"))
                })
                .unwrap();
            for row in rows {
                chunks.push(row.unwrap());
            }
        }
        chunks.join("\n")
    }
}
