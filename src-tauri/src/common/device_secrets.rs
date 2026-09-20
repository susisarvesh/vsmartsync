//! Shared secret encryption (AES-256-GCM).
//!
//! One OS-keychain master key protects device passwords and user credential
//! secrets (card numbers, PINs). Ciphertext lives in PostgreSQL. Plaintext
//! never goes to React or logs.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use keyring::Entry;
use rand::RngCore;
use thiserror::Error;

const SERVICE: &str = "com.vsmart.sync";
/// Single workstation master key for all reversible app secrets at rest.
const ACCOUNT: &str = "device-password-master-key";
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecretError {
    #[error("SECRET_UNAVAILABLE")]
    Unavailable,
    #[error("SECRET_CORRUPT")]
    Corrupt,
}

/// AES-256-GCM vault backed by the OS credential store.
pub struct SecretVault {
    cipher: Aes256Gcm,
}

/// Backward-compatible name used by the Devices slice.
pub type DevicePasswordVault = SecretVault;

impl SecretVault {
    pub fn open() -> Result<Self, SecretError> {
        let key = load_or_create_key()?;
        Self::from_key(key)
    }

    pub fn from_key(key: [u8; KEY_LEN]) -> Result<Self, SecretError> {
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| SecretError::Unavailable)?;
        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>, SecretError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| SecretError::Unavailable)?;
        let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    pub fn decrypt(&self, blob: &[u8]) -> Result<String, SecretError> {
        if blob.len() <= NONCE_LEN {
            return Err(SecretError::Corrupt);
        }
        let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| SecretError::Corrupt)?;
        String::from_utf8(plaintext).map_err(|_| SecretError::Corrupt)
    }
}

fn load_or_create_key() -> Result<[u8; KEY_LEN], SecretError> {
    let entry = Entry::new(SERVICE, ACCOUNT).map_err(|_| SecretError::Unavailable)?;
    match entry.get_password() {
        Ok(encoded) => decode_key(&encoded),
        Err(keyring::Error::NoEntry) => {
            let mut key = [0u8; KEY_LEN];
            rand::thread_rng().fill_bytes(&mut key);
            let encoded = encode_key(&key);
            entry
                .set_password(&encoded)
                .map_err(|_| SecretError::Unavailable)?;
            Ok(key)
        }
        Err(_) => Err(SecretError::Unavailable),
    }
}

fn encode_key(key: &[u8; KEY_LEN]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_key(encoded: &str) -> Result<[u8; KEY_LEN], SecretError> {
    if encoded.len() != KEY_LEN * 2 {
        return Err(SecretError::Corrupt);
    }
    let mut key = [0u8; KEY_LEN];
    for (index, chunk) in encoded.as_bytes().chunks(2).enumerate() {
        let hex = std::str::from_utf8(chunk).map_err(|_| SecretError::Corrupt)?;
        key[index] = u8::from_str_radix(hex, 16).map_err(|_| SecretError::Corrupt)?;
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::{decode_key, encode_key, SecretVault, KEY_LEN};

    #[test]
    fn round_trips_password_without_logging_plaintext_in_blob() {
        let vault = SecretVault::from_key([7u8; KEY_LEN]).expect("vault");
        let blob = vault.encrypt("door-secret").expect("encrypt");
        assert!(!blob.is_empty());
        assert!(!String::from_utf8_lossy(&blob).contains("door-secret"));
        assert_eq!(vault.decrypt(&blob).expect("decrypt"), "door-secret");
    }

    #[test]
    fn rejects_corrupt_ciphertext() {
        let vault = SecretVault::from_key([3u8; KEY_LEN]).expect("vault");
        assert!(vault.decrypt(&[1, 2, 3]).is_err());
    }

    #[test]
    fn key_hex_round_trip() {
        let key = [9u8; KEY_LEN];
        assert_eq!(decode_key(&encode_key(&key)).unwrap(), key);
    }
}
