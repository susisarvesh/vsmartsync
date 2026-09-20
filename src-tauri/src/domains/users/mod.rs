//! Users domain.
//!
//! Owns application people records: create, list, update name, deactivate.
//! Does not own Matrix `user-id`, credentials, devices, enrollment, or sync.

mod service;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub use service::{create_user, deactivate_user, list_users, update_user_name};

pub const MAX_USER_NAME_LENGTH: usize = 200;

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
    pub name: String,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UserError {
    #[error("USER_INVALID_NAME")]
    InvalidName,
    #[error("USER_NOT_FOUND")]
    NotFound,
    #[error("DATABASE_UNAVAILABLE")]
    Unavailable,
}

pub fn normalize_name(name: &str) -> Result<String, UserError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_USER_NAME_LENGTH {
        return Err(UserError::InvalidName);
    }
    Ok(name.to_string())
}

pub fn new_user(name: &str, id: Uuid, now: DateTime<Utc>) -> Result<User, UserError> {
    Ok(User {
        id,
        name: normalize_name(name)?,
        status: UserStatus::Active,
        created_at: now,
        updated_at: now,
    })
}

pub fn rename_user(mut user: User, name: &str, now: DateTime<Utc>) -> Result<User, UserError> {
    user.name = normalize_name(name)?;
    user.updated_at = now;
    Ok(user)
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
    use super::{deactivate, new_user, normalize_name, rename_user, UserError, UserStatus};
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn now() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 20, 12, 0, 0).unwrap()
    }

    #[test]
    fn normalize_name_trims_and_rejects_blank() {
        assert_eq!(normalize_name("  Ada  ").unwrap(), "Ada");
        assert_eq!(normalize_name("   ").unwrap_err(), UserError::InvalidName);
        assert_eq!(normalize_name("").unwrap_err(), UserError::InvalidName);
    }

    #[test]
    fn normalize_name_rejects_too_long() {
        let name = "a".repeat(201);
        assert_eq!(normalize_name(&name).unwrap_err(), UserError::InvalidName);
        assert!(normalize_name(&"a".repeat(200)).is_ok());
    }

    #[test]
    fn new_user_starts_active() {
        let user = new_user("Ada", Uuid::nil(), now()).expect("user");
        assert_eq!(user.status, UserStatus::Active);
        assert_eq!(user.name, "Ada");
    }

    #[test]
    fn rename_user_does_not_change_status() {
        let user = new_user("Ada", Uuid::nil(), now()).expect("user");
        let renamed = rename_user(user, "Ada Lovelace", now()).expect("rename");
        assert_eq!(renamed.name, "Ada Lovelace");
        assert_eq!(renamed.status, UserStatus::Active);
    }

    #[test]
    fn deactivate_sets_inactive_and_is_idempotent() {
        let user = new_user("Ada", Uuid::nil(), now()).expect("user");
        let first = deactivate(user, now());
        assert_eq!(first.status, UserStatus::Inactive);
        let second = deactivate(first.clone(), now());
        assert_eq!(second, first);
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
