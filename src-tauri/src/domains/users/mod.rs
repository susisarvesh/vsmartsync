//! Users domain.
//!
//! Owns application people records: create, list, update username, activate, deactivate,
//! and delete. Delete also removes the person from each Matrix device that has them.

mod removal;
mod service;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub use removal::delete_user;
pub use service::{activate_user, create_user, deactivate_user, list_users, update_user};

pub const MAX_USERNAME_LENGTH: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UserStatus {
    Active,
    Inactive,
}

impl UserStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }

    pub fn parse(value: &str) -> Result<Self, UserError> {
        match value {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            _ => Err(UserError::Unavailable),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UserError {
    #[error("USER_INVALID_USERNAME")]
    InvalidUsername,
    #[error("USER_NOT_FOUND")]
    NotFound,
    #[error("DATABASE_UNAVAILABLE")]
    Unavailable,
    #[error("DEVICE_SECRET_UNAVAILABLE")]
    SecretUnavailable,
    #[error("DEVICE_SECRET_CORRUPT")]
    SecretCorrupt,
    #[error("DEVICE_AUTH_FAILED")]
    AuthFailed,
    #[error("DEVICE_BAD_RESPONSE")]
    BadResponse,
    #[error("MATRIX_TIMEOUT")]
    Timeout,
    #[error("MATRIX_UNREACHABLE")]
    Unreachable,
    #[error("MATRIX_INVALID_ARGUMENT")]
    InvalidArgument,
    #[error("MATRIX_API_ERROR:{code}")]
    ApiError { code: i32 },
}

pub fn normalize_username(username: &str) -> Result<String, UserError> {
    let username = username.trim();
    if username.is_empty() || username.chars().count() > MAX_USERNAME_LENGTH {
        return Err(UserError::InvalidUsername);
    }
    Ok(username.to_string())
}

pub fn new_user(username: &str, id: Uuid, now: DateTime<Utc>) -> Result<User, UserError> {
    Ok(User {
        id,
        username: normalize_username(username)?,
        status: UserStatus::Active,
        created_at: now,
        updated_at: now,
    })
}

pub fn update_username(
    mut user: User,
    username: &str,
    now: DateTime<Utc>,
) -> Result<User, UserError> {
    user.username = normalize_username(username)?;
    user.updated_at = now;
    Ok(user)
}

pub fn activate(mut user: User, now: DateTime<Utc>) -> User {
    if user.status != UserStatus::Active {
        user.status = UserStatus::Active;
        user.updated_at = now;
    }
    user
}

pub fn deactivate(mut user: User, now: DateTime<Utc>) -> User {
    if user.status != UserStatus::Inactive {
        user.status = UserStatus::Inactive;
        user.updated_at = now;
    }
    user
}

#[cfg(test)]
mod tests {
    use super::{
        activate, deactivate, new_user, normalize_username, update_username, UserError, UserStatus,
    };
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn now() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 20, 12, 0, 0).unwrap()
    }

    #[test]
    fn normalize_username_trims_and_rejects_blank() {
        assert_eq!(normalize_username("  Ada  ").unwrap(), "Ada");
        assert_eq!(
            normalize_username("   ").unwrap_err(),
            UserError::InvalidUsername
        );
        assert_eq!(
            normalize_username("").unwrap_err(),
            UserError::InvalidUsername
        );
    }

    #[test]
    fn normalize_username_rejects_too_long() {
        let username = "a".repeat(201);
        assert_eq!(
            normalize_username(&username).unwrap_err(),
            UserError::InvalidUsername
        );
        assert!(normalize_username(&"a".repeat(200)).is_ok());
    }

    #[test]
    fn new_user_starts_active() {
        let user = new_user("Ada", Uuid::nil(), now()).expect("user");
        assert_eq!(user.status, UserStatus::Active);
        assert_eq!(user.username, "Ada");
    }

    #[test]
    fn update_username_does_not_change_status() {
        let user = new_user("Ada", Uuid::nil(), now()).expect("user");
        let renamed = update_username(user, "Ada Lovelace", now()).expect("rename");
        assert_eq!(renamed.username, "Ada Lovelace");
        assert_eq!(renamed.status, UserStatus::Active);
    }

    #[test]
    fn activate_and_deactivate_are_idempotent() {
        let user = new_user("Ada", Uuid::nil(), now()).expect("user");
        let still_active = activate(user.clone(), now());
        assert_eq!(still_active, user);

        let inactive = deactivate(user, now());
        assert_eq!(inactive.status, UserStatus::Inactive);
        let still_inactive = deactivate(inactive.clone(), now());
        assert_eq!(still_inactive, inactive);

        let active = activate(inactive, now());
        assert_eq!(active.status, UserStatus::Active);
        let still_active = activate(active.clone(), now());
        assert_eq!(still_active, active);
    }

    #[test]
    fn status_parse_only_allows_controlled_values() {
        assert_eq!(UserStatus::parse("active").unwrap(), UserStatus::Active);
        assert_eq!(UserStatus::parse("inactive").unwrap(), UserStatus::Inactive);
        assert_eq!(
            UserStatus::parse("deleted").unwrap_err(),
            UserError::Unavailable
        );
    }
}
