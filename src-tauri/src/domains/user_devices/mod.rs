//! Assignment of an application user to one or more devices.
//!
//! Owns the local `user_devices` relationship only.
//! Does not own Matrix identity (`device_users`), credentials, enrollment, or sync.

mod service;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub use service::{
    assign_user_device, list_user_devices, list_users_for_device, remove_user_device,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDeviceAssignment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub device_name: String,
    pub host: String,
    pub port: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOnDevice {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub username: String,
    pub status: crate::domains::users::UserStatus,
    pub created_at: DateTime<Utc>,
    pub provisioned_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UserDeviceError {
    #[error("USER_NOT_FOUND")]
    UserNotFound,
    #[error("DEVICE_NOT_FOUND")]
    DeviceNotFound,
    #[error("USER_DEVICE_DUPLICATE")]
    Duplicate,
    #[error("USER_DEVICE_NOT_FOUND")]
    NotFound,
    #[error("DATABASE_UNAVAILABLE")]
    Unavailable,
}
