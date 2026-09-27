//! Shared secret encryption (AES-256-GCM).
//!
//! One application master key protects device passwords and user credential
//! secrets (card numbers, PINs). Ciphertext lives in PostgreSQL. The key is a
//! file the app creates. Plaintext never goes to React or logs.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use thiserror::Error;

const KEY_DIR_NAME: &str = ".vsmart-sync";
const KEY_FILE_NAME: &str = "master.key";
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecretError {
    #[error("SECRET_UNAVAILABLE")]
    Unavailable,
    #[error("SECRET_CORRUPT")]
    Corrupt,
}

/// AES-256-GCM vault. The master key is not stored in an OS credential store.
pub struct SecretVault {
    cipher: Aes256Gcm,
}

/// Backward-compatible name used by the Devices slice.
pub type DevicePasswordVault = SecretVault;

impl SecretVault {
    pub fn open() -> Result<Self, SecretError> {
        Self::open_in(&default_key_dir()?)
    }

    pub fn open_in(dir: &Path) -> Result<Self, SecretError> {
        let key = load_or_create_key(dir)?;
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

fn default_key_dir() -> Result<PathBuf, SecretError> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or(SecretError::Unavailable)?;
    Ok(PathBuf::from(home).join(KEY_DIR_NAME))
}

fn load_or_create_key(dir: &Path) -> Result<[u8; KEY_LEN], SecretError> {
    fs::create_dir_all(dir).map_err(|_| SecretError::Unavailable)?;
    restrict_directory(dir)?;
    let path = dir.join(KEY_FILE_NAME);
    if path.exists() {
        return read_key(&path);
    }

    let mut key = [0u8; KEY_LEN];
    rand::thread_rng().fill_bytes(&mut key);
    match create_key_file(&path, &key) {
        Ok(()) => Ok(key),
        Err(CreateKeyError::AlreadyExists) => read_key(&path),
        Err(CreateKeyError::Unavailable) => Err(SecretError::Unavailable),
    }
}

fn read_key(path: &Path) -> Result<[u8; KEY_LEN], SecretError> {
    let bytes = fs::read(path).map_err(|_| SecretError::Unavailable)?;
    if bytes.len() != KEY_LEN {
        return Err(if bytes.is_empty() {
            SecretError::Unavailable
        } else {
            SecretError::Corrupt
        });
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&bytes);
    Ok(key)
}

enum CreateKeyError {
    AlreadyExists,
    Unavailable,
}

fn create_key_file(path: &Path, key: &[u8; KEY_LEN]) -> Result<(), CreateKeyError> {
    let mut file = open_new_key_file(path)?;
    if file.write_all(key).and_then(|_| file.sync_all()).is_err() {
        let _ = fs::remove_file(path);
        return Err(CreateKeyError::Unavailable);
    }
    Ok(())
}

fn open_new_key_file(path: &Path) -> Result<File, CreateKeyError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(file) => Ok(file),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(CreateKeyError::AlreadyExists)
        }
        Err(_) => Err(CreateKeyError::Unavailable),
    }
}

fn restrict_directory(dir: &Path) -> Result<(), SecretError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
            .map_err(|_| SecretError::Unavailable)?;
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{SecretError, SecretVault, KEY_FILE_NAME, KEY_LEN};
    use rand::RngCore;
    use std::fs;

    fn temp_dir() -> std::path::PathBuf {
        let mut nonce = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut nonce);
        let name = nonce
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        std::env::temp_dir().join(format!("vsmart-vault-{name}"))
    }

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
    fn same_directory_reuses_one_master_key() {
        let dir = temp_dir();
        let first = SecretVault::open_in(&dir).expect("create");
        let blob = first.encrypt("door-secret").expect("encrypt");
        let second = SecretVault::open_in(&dir).expect("reopen");
        assert_eq!(second.decrypt(&blob).expect("decrypt"), "door-secret");
        let stored = fs::read(dir.join(KEY_FILE_NAME)).expect("key file");
        assert_eq!(stored.len(), KEY_LEN);
        assert!(!String::from_utf8_lossy(&stored).contains("door-secret"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_key_file_is_not_replaced() {
        let dir = temp_dir();
        fs::create_dir_all(&dir).expect("dir");
        let path = dir.join(KEY_FILE_NAME);
        fs::write(&path, b"not-a-key").expect("write");
        match SecretVault::open_in(&dir) {
            Err(SecretError::Corrupt) => {}
            Err(error) => panic!("expected corrupt key, got {error}"),
            Ok(_) => panic!("expected corrupt key"),
        }
        assert_eq!(fs::read(&path).expect("still there"), b"not-a-key");
        let _ = fs::remove_dir_all(&dir);
    }
}
