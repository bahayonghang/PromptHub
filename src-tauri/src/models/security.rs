//! Security-related DTOs (Requirement 15).

use serde::{Deserialize, Serialize};

/// Stored master-password verifier (v1).
///
/// `key = scrypt(password, salt, 32)`; the encryption envelope is
/// `"ENC::" + base64(iv12 || tag16 || ciphertext)`. Both fields hold
/// base64-encoded bytes (`salt`: 16 bytes, `hash`: 32 bytes).
///
/// v1 stores the AES data-encryption key in `hash`. Unlock migrates this
/// document to [`StoredMasterPasswordV2`] in one transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredMasterPassword {
    /// Base64-encoded 16-byte scrypt salt.
    pub salt: String,
    /// Base64-encoded 32-byte verifier hash (v1: this is the DEK).
    pub hash: String,
}

/// Stored master-password verifier (v2).
///
/// `KEK = scrypt(password, kdfSalt)`; a random DEK is wrapped with a key
/// derived from the KEK. `verifier` uses a different HMAC info string so it
/// is never equal to the DEK.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredMasterPasswordV2 {
    /// Format version. Always `2` for this document.
    pub v: u32,
    /// Base64-encoded 16-byte scrypt salt for the KEK.
    pub kdf_salt: String,
    /// Base64-encoded 32-byte password verifier (HMAC-SHA256, not the DEK).
    pub verifier: String,
    /// AES-256-GCM envelope of the DEK (`ENC::` layout).
    pub wrapped_dek: String,
}

/// Persisted master-password document. Detected by JSON shape (`v` vs `{salt,hash}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StoredVerifier {
    /// Random DEK wrapped by a password-derived KEK.
    V2(StoredMasterPasswordV2),
    /// Legacy document where `hash` is the DEK.
    V1(StoredMasterPassword),
}
