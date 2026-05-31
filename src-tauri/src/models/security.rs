//! Security-related DTOs (Requirement 15).

use serde::{Deserialize, Serialize};

/// Stored master-password verifier.
///
/// `key = scrypt(password, salt, 32)`; the encryption envelope is
/// `"ENC::" + base64(iv12 || tag16 || ciphertext)`. Both fields hold
/// base64-encoded bytes (`salt`: 16 bytes, `hash`: 32 bytes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredMasterPassword {
    /// Base64-encoded 16-byte scrypt salt.
    pub salt: String,
    /// Base64-encoded 32-byte verifier hash.
    pub hash: String,
}
