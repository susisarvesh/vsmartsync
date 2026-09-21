//! Device-user Matrix identity mappings.
//!
//! Owns durable `(user, device) → Matrix user-id / ref-user-id` allocation.
//! Does not call Matrix HTTP, Sync, or expose a React UI in this slice.

mod service;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub use service::{
    ensure_mapping, get_device_user, list_device_users_for_device, mark_provisioned,
};

/// Application view of a device-scoped Matrix identity mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceUser {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub matrix_user_id: String,
    pub matrix_ref_user_id: i64,
    pub provisioned_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DeviceUserError {
    #[error("DEVICE_USER_NOT_FOUND")]
    NotFound,
    #[error("USER_NOT_FOUND")]
    UserNotFound,
    #[error("DEVICE_NOT_FOUND")]
    DeviceNotFound,
    #[error("DEVICE_USER_DUPLICATE")]
    Duplicate,
    #[error("DEVICE_USER_SEQUENCE_EXHAUSTED")]
    SequenceExhausted,
    #[error("DATABASE_UNAVAILABLE")]
    Unavailable,
}
