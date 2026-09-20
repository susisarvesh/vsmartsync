//! Devices domain.
//!
//! Owns registered COSEC connection records and application reachability.
//! Does not own enrollment, sync, credentials, or Matrix CGI beyond a thin ping.

mod service;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub use service::{
    create_device, list_devices, set_device_password, test_device_connection, update_device,
};

pub const MAX_DEVICE_NAME_LENGTH: usize = 200;
pub const MAX_HOST_LENGTH: usize = 255;
pub const MAX_USERNAME_LENGTH: usize = 100;
pub const MAX_PASSWORD_LENGTH: usize = 200;
pub const DEFAULT_DEVICE_PORT: i32 = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    Unknown,
    Online,
    Offline,
}

impl ConnectionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Online => "online",
            Self::Offline => "offline",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DeviceError> {
        match value {
            "unknown" => Ok(Self::Unknown),
            "online" => Ok(Self::Online),
            "offline" => Ok(Self::Offline),
            _ => Err(DeviceError::Unavailable),
        }
    }
}

/// Public device view for the UI. Never includes password or ciphertext.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub connection_status: ConnectionStatus,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DeviceError {
    #[error("DEVICE_INVALID_NAME")]
    InvalidName,
    #[error("DEVICE_INVALID_HOST")]
    InvalidHost,
    #[error("DEVICE_INVALID_PORT")]
    InvalidPort,
    #[error("DEVICE_INVALID_USERNAME")]
    InvalidUsername,
    #[error("DEVICE_INVALID_PASSWORD")]
    InvalidPassword,
    #[error("DEVICE_DUPLICATE")]
    Duplicate,
    #[error("DEVICE_NOT_FOUND")]
    NotFound,
    #[error("DEVICE_OFFLINE")]
    Offline,
    #[error("DEVICE_AUTH_FAILED")]
    AuthFailed,
    #[error("DEVICE_BAD_RESPONSE")]
    BadResponse,
    #[error("DEVICE_SECRET_UNAVAILABLE")]
    SecretUnavailable,
    #[error("DATABASE_UNAVAILABLE")]
    Unavailable,
}

pub fn normalize_name(name: &str) -> Result<String, DeviceError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_DEVICE_NAME_LENGTH {
        return Err(DeviceError::InvalidName);
    }
    Ok(name.to_string())
}

pub fn normalize_host(host: &str) -> Result<String, DeviceError> {
    let host = host.trim();
    if host.is_empty() || host.chars().count() > MAX_HOST_LENGTH {
        return Err(DeviceError::InvalidHost);
    }
    if host.contains(['/', '?', '#', '@', ':', ' ', '\\']) {
        return Err(DeviceError::InvalidHost);
    }
    if !(is_ipv4(host) || is_hostname(host)) {
        return Err(DeviceError::InvalidHost);
    }
    Ok(host.to_string())
}

pub fn normalize_port(port: i32) -> Result<i32, DeviceError> {
    if !(1..=65535).contains(&port) {
        return Err(DeviceError::InvalidPort);
    }
    Ok(port)
}

pub fn normalize_username(username: &str) -> Result<String, DeviceError> {
    let username = username.trim();
    if username.is_empty() || username.chars().count() > MAX_USERNAME_LENGTH {
        return Err(DeviceError::InvalidUsername);
    }
    if username.contains(['/', '?', '#', '@', ':', ' ']) {
        return Err(DeviceError::InvalidUsername);
    }
    Ok(username.to_string())
}

pub fn normalize_password(password: &str) -> Result<String, DeviceError> {
    if password.is_empty() || password.chars().count() > MAX_PASSWORD_LENGTH {
        return Err(DeviceError::InvalidPassword);
    }
    Ok(password.to_string())
}

fn is_ipv4(value: &str) -> bool {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|part| {
        part.parse::<u8>().is_ok() && !(part.len() > 1 && part.starts_with('0'))
    })
}

fn is_hostname(value: &str) -> bool {
    let labels: Vec<_> = value.split('.').collect();
    if labels.is_empty() {
        return false;
    }
    labels.iter().all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    })
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_host, normalize_name, normalize_password, normalize_port, normalize_username,
        ConnectionStatus, DeviceError,
    };

    #[test]
    fn validates_connection_fields() {
        assert_eq!(normalize_name(" Door A ").unwrap(), "Door A");
        assert_eq!(normalize_host("192.168.1.10").unwrap(), "192.168.1.10");
        assert_eq!(normalize_host("door.local").unwrap(), "door.local");
        assert_eq!(normalize_port(80).unwrap(), 80);
        assert_eq!(normalize_username("admin").unwrap(), "admin");
        assert!(normalize_password("secret").is_ok());
    }

    #[test]
    fn rejects_unsafe_hosts_and_urls() {
        assert_eq!(
            normalize_host("http://192.168.1.10").unwrap_err(),
            DeviceError::InvalidHost
        );
        assert_eq!(
            normalize_host("192.168.1.10/path").unwrap_err(),
            DeviceError::InvalidHost
        );
        assert_eq!(
            normalize_host("user@host").unwrap_err(),
            DeviceError::InvalidHost
        );
        assert_eq!(normalize_port(0).unwrap_err(), DeviceError::InvalidPort);
        assert_eq!(
            normalize_password("").unwrap_err(),
            DeviceError::InvalidPassword
        );
    }

    #[test]
    fn connection_status_parse() {
        assert_eq!(
            ConnectionStatus::parse("online").unwrap(),
            ConnectionStatus::Online
        );
        assert!(ConnectionStatus::parse("syncing").is_err());
    }
}
