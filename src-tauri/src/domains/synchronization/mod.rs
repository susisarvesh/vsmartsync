//! Push assigned application users onto a COSEC device.
//!
//! This slice calls `set_user` for people already stored in `user_devices`.
//! It does not sync credentials, enrollment, deletes, or a durable job queue.

mod service;

#[cfg(test)]
mod live;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub use service::sync_assigned_users;

pub const SYNC_USER_LIMIT: usize = 200;

/// Matrix `name` is optional and limited to 15 alphanumeric characters.
/// Usernames outside that limit are omitted; identity still uses `user-id`.
pub fn matrix_display_name(username: &str) -> Option<String> {
    let value = username.trim();
    if value.is_empty() || value.chars().count() > 15 {
        return None;
    }
    if !value.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(value.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncedDeviceUser {
    pub user_id: Uuid,
    pub username: String,
    pub matrix_user_id: String,
    pub matrix_ref_user_id: i64,
    pub provisioned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncUserFailure {
    pub user_id: Uuid,
    pub username: Option<String>,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncUsersResult {
    pub device_id: Uuid,
    pub synced: Vec<SyncedDeviceUser>,
    pub failed: Vec<SyncUserFailure>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SyncError {
    #[error("DEVICE_NOT_FOUND")]
    DeviceNotFound,
    #[error("DEVICE_INACTIVE")]
    DeviceInactive,
    #[error("DEVICE_INVALID_PORT")]
    InvalidPort,
    #[error("DEVICE_SECRET_UNAVAILABLE")]
    SecretUnavailable,
    #[error("DEVICE_SECRET_CORRUPT")]
    SecretCorrupt,
    #[error("SYNC_NO_USERS")]
    NoUsers,
    #[error("SYNC_TOO_MANY")]
    TooMany,
    #[error("DATABASE_UNAVAILABLE")]
    Unavailable,
}

#[cfg(test)]
mod tests {
    use super::matrix_display_name;

    #[test]
    fn display_name_keeps_short_alphanumeric_usernames() {
        assert_eq!(matrix_display_name(" DoorUser1 "), Some("DoorUser1".into()));
        assert_eq!(matrix_display_name("John Doe"), None);
        assert_eq!(matrix_display_name("ThisNameIsTooLong"), None);
        assert_eq!(matrix_display_name(""), None);
    }
}
