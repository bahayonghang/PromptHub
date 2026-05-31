//! Property-based tests for the Security_Service (task 7.2).
//!
//! These run as an **integration test** against the public `prompthub_lib` API
//! (`services::security::*`, `state::EncryptionState`, `storage::*`,
//! `error::ErrorCode`), so they need no edits to any `mod.rs` — the same pattern
//! used by `tests/folder_properties.rs` (task 5.2). Each test builds a fresh
//! in-memory database ([`create_memory_pool`] + [`init_schema`]) and a local
//! [`EncryptionState`] behind a `Mutex`, then drives the service through its
//! public functions exactly as the Command_Layer will.
//!
//! Key derivation is scrypt (interactive cost, CPU-heavy), so the two properties
//! that call `set_master_password` / `change_master_password` / `unlock` keep
//! their case counts low; the pure-crypto round-trip and authentication
//! properties (no scrypt) run with a higher case count.
//!
//! Properties implemented (design "Testing Strategy"):
//!   - Property 31: Master password length validation
//!   - Property 32: Encryption round-trip
//!   - Property 33: Encryption authentication and wrong-password rejection
//!   - Property 34: Master password re-key preserves access
//!
//! **Validates: Requirements 15.3, 15.4, 15.9, 15.10, 15.11**

use std::sync::Mutex;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use proptest::prelude::*;
use proptest::sample::Index;
use rusqlite::params;

use prompthub_lib::error::ErrorCode;
use prompthub_lib::services::security::{
    change_master_password, decrypt, encrypt, lock, set_master_password, status, unlock,
};
use prompthub_lib::state::EncryptionState;
use prompthub_lib::storage::{create_memory_pool, init_schema, DbPool};

/// Inclusive master-password length bounds enforced by the service.
const MIN_LEN: usize = 8;
const MAX_LEN: usize = 128;
/// Envelope prefix produced by `encrypt`.
const ENC_PREFIX: &str = "ENC::";

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Builds an in-memory pool with the schema initialized.
fn schema_pool() -> DbPool {
    let pool = create_memory_pool().expect("memory pool");
    init_schema(&pool.get().expect("conn")).expect("schema");
    pool
}

/// A fresh, locked encryption state (no cached key).
fn fresh_enc() -> Mutex<EncryptionState> {
    Mutex::new(EncryptionState::default())
}

/// Reads the cached derived key (panics if locked).
fn cached_key(enc: &Mutex<EncryptionState>) -> Vec<u8> {
    enc.lock()
        .unwrap()
        .derived_key
        .clone()
        .expect("unlocked key")
}

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Master-password strategy biased toward the 8/128 length boundaries.
///
/// Each candidate repeats a single character `len` times; because validation
/// counts characters (not bytes), the unicode variant (`世`, 3 bytes/char)
/// confirms the rule is character-based. `len == 0` yields the empty string.
fn master_password() -> impl Strategy<Value = String> {
    let lens = prop_oneof![
        Just(0usize),
        Just(1usize),
        Just(7usize),
        Just(8usize),
        Just(9usize),
        Just(64usize),
        Just(127usize),
        Just(128usize),
        Just(129usize),
        Just(200usize),
        1usize..=140usize,
    ];
    (lens, any::<bool>()).prop_map(|(len, unicode)| {
        let ch = if unicode { '世' } else { 'a' };
        std::iter::repeat(ch).take(len).collect::<String>()
    })
}

/// Arbitrary plaintext: empty, arbitrary unicode, a unicode-heavy band, and long
/// strings — covering the inputs Property 32/34 must round-trip.
fn plaintext() -> impl Strategy<Value = String> {
    prop_oneof![
        1 => Just(String::new()),
        5 => any::<String>(),
        2 => proptest::string::string_regex("[\\x{4e00}-\\x{9fff}é \\n]{0,30}").unwrap(),
        2 => (0usize..=4000usize).prop_map(|k| "x".repeat(k)),
    ]
}

/// A valid (8–128 char) master password built from printable ASCII.
fn valid_password() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9!@#%]{8,40}").unwrap()
}

// ---------------------------------------------------------------------------
// Property 31: Master password length validation (scrypt — low case count)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    /// **Property 31: Master password length validation.**
    ///
    /// `set_master_password` succeeds iff the password's character length is in
    /// 8..=128. On acceptance the verifier is stored and the state is unlocked;
    /// on rejection it returns a `VALIDATION` error and leaves the master
    /// password and lock state unchanged (no verifier stored, still locked).
    ///
    /// **Validates: Requirements 15.3**
    #[test]
    fn master_password_length_validation(password in master_password()) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let enc = fresh_enc();

        let len = password.chars().count();
        let valid = (MIN_LEN..=MAX_LEN).contains(&len);

        match set_master_password(&conn, &enc, &password) {
            Ok(()) => {
                prop_assert!(valid, "accepted password of length {len}");
                let s = status(&conn, &enc).unwrap();
                prop_assert!(s.has_master_password, "verifier should be stored");
                prop_assert!(!s.is_locked, "state should be unlocked after set");
            }
            Err(err) => {
                prop_assert!(!valid, "rejected password of length {len}");
                prop_assert_eq!(err.code, ErrorCode::Validation);
                // No state change: nothing stored, still locked.
                let s = status(&conn, &enc).unwrap();
                prop_assert!(!s.has_master_password, "no verifier on rejection");
                prop_assert!(s.is_locked, "still locked on rejection");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Properties 32 & 33: pure crypto (no scrypt — higher case count)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// **Property 32: Encryption round-trip.**
    ///
    /// For any plaintext and any 32-byte key, `decrypt(encrypt(x, key), key)`
    /// returns `x` exactly, and the envelope carries the `ENC::` prefix.
    ///
    /// **Validates: Requirements 15.9, 15.11**
    #[test]
    fn encryption_round_trip(
        key in proptest::array::uniform32(any::<u8>()),
        plaintext in plaintext(),
    ) {
        let envelope = encrypt(&plaintext, &key).unwrap();
        prop_assert!(envelope.starts_with(ENC_PREFIX), "envelope must be ENC::-prefixed");

        let decrypted = decrypt(&envelope, &key).unwrap();
        prop_assert_eq!(decrypted, plaintext);
    }

    /// **Property 33: Encryption authentication and wrong-password rejection.**
    ///
    /// Encrypting under key A then decrypting under a distinct key B fails with a
    /// structured error and returns no plaintext; flipping any single bit of the
    /// payload (IV, tag, or ciphertext) likewise fails the authentication-tag
    /// check. Neither path panics or leaks plaintext.
    ///
    /// **Validates: Requirements 15.10**
    #[test]
    fn authentication_rejects_wrong_key_and_tampering(
        key_a in proptest::array::uniform32(any::<u8>()),
        key_b in proptest::array::uniform32(any::<u8>()),
        plaintext in plaintext(),
        tamper in any::<Index>(),
        bit in 0u8..8,
    ) {
        prop_assume!(key_a != key_b);

        let envelope = encrypt(&plaintext, &key_a).unwrap();

        // Wrong key: structured error, no decrypted content returned.
        match decrypt(&envelope, &key_b) {
            Ok(_) => prop_assert!(false, "wrong key must not decrypt"),
            Err(err) => {
                prop_assert_eq!(err.code, ErrorCode::Unauthorized);
                // The fixed error message must not echo the plaintext. (Skip the
                // check for very short plaintexts that could coincide with the
                // message's own vocabulary.)
                if plaintext.chars().count() >= 4 {
                    prop_assert!(
                        !err.message.contains(&plaintext),
                        "error message must not leak plaintext"
                    );
                }
            }
        }

        // Tamper: flip one bit of any payload byte -> authentication fails.
        let b64 = envelope.strip_prefix(ENC_PREFIX).unwrap();
        let mut buf = B64.decode(b64).unwrap();
        let idx = tamper.index(buf.len());
        buf[idx] ^= 1u8 << bit;
        let tampered = format!("{ENC_PREFIX}{}", B64.encode(&buf));
        prop_assert!(
            decrypt(&tampered, &key_a).is_err(),
            "tampered payload must fail authentication"
        );
    }
}

// ---------------------------------------------------------------------------
// Property 34: Master password re-key (scrypt-heavy — low case count)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]

    /// **Property 34: Master password re-key preserves access.**
    ///
    /// After setting a master password and encrypting a value (stored in
    /// `settings`) under the cached key, `change_master_password` re-keys the
    /// data: the value remains an `ENC::` envelope readable under the new cached
    /// key and equal to the original plaintext, while the old key can no longer
    /// decrypt it. The old password no longer unlocks; the new password does.
    ///
    /// **Validates: Requirements 15.4**
    #[test]
    fn rekey_preserves_access(
        old in valid_password(),
        new in valid_password(),
        plaintext in plaintext(),
    ) {
        prop_assume!(old != new);

        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let enc = fresh_enc();

        // Set the initial password and encrypt a value under the cached key.
        set_master_password(&conn, &enc, &old).unwrap();
        let old_key = cached_key(&enc);
        let envelope = encrypt(&plaintext, &old_key).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["secret_blob", envelope],
        )
        .unwrap();

        // Re-key to the new password.
        change_master_password(&conn, &enc, &old, &new).unwrap();
        let new_key = cached_key(&enc);
        prop_assert_ne!(&old_key, &new_key, "key must change after re-key");

        // The stored value was re-keyed: still ENC::, readable under the new key
        // (equal to the original plaintext), unreadable under the old key.
        let stored: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params!["secret_blob"],
                |row| row.get(0),
            )
            .unwrap();
        prop_assert!(stored.starts_with(ENC_PREFIX));
        let decrypted = decrypt(&stored, &new_key).unwrap();
        prop_assert_eq!(decrypted, plaintext);
        prop_assert!(decrypt(&stored, &old_key).is_err(), "old key must not decrypt re-keyed data");

        // Old password no longer unlocks; new password does.
        lock(&enc).unwrap();
        prop_assert!(unlock(&conn, &enc, &old).is_err(), "old password must not unlock");
        prop_assert!(status(&conn, &enc).unwrap().is_locked, "stays locked after wrong unlock");
        unlock(&conn, &enc, &new).unwrap();
        prop_assert!(!status(&conn, &enc).unwrap().is_locked, "new password unlocks");
    }
}
