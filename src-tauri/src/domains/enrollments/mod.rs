//! Enrollments domain.
//!
//! Durable desired assignment of a user credential to a device.
//! Does not own Matrix CGI, sync jobs, physical enrolluser sessions, or secrets.

mod service;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub use service::{
    cancel_enrollment, create_enrollment, get_enrollment, list_enrollments, mark_enrollment_active,
    mark_enrollment_failed, retry_enrollment, revoke_enrollment, EnrollmentListFilter,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EnrollmentStatus {
    Pending,
    Active,
    Failed,
    Cancelled,
    Revoked,
}

impl EnrollmentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Revoked => "revoked",
        }
    }

    pub fn parse(value: &str) -> Result<Self, EnrollmentError> {
        match value {
            "pending" => Ok(Self::Pending),
            "active" => Ok(Self::Active),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "revoked" => Ok(Self::Revoked),
            _ => Err(EnrollmentError::InvalidTransition),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Cancelled | Self::Revoked)
    }
}

/// Safe enrollment view for React. Never includes secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Enrollment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub credential_id: Uuid,
    pub credential_type: String,
    pub masked_value: Option<String>,
    pub device_id: Uuid,
    pub device_name: String,
    pub status: EnrollmentStatus,
    pub activated_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EnrollmentError {
    #[error("ENROLLMENT_NOT_FOUND")]
    NotFound,
    #[error("ENROLLMENT_DUPLICATE")]
    Duplicate,
    #[error("ENROLLMENT_INVALID_TRANSITION")]
    InvalidTransition,
    #[error("ENROLLMENT_USER_INACTIVE")]
    UserInactive,
    #[error("ENROLLMENT_CREDENTIAL_INACTIVE")]
    CredentialInactive,
    #[error("ENROLLMENT_CREDENTIAL_OWNERSHIP")]
    CredentialOwnership,
    #[error("USER_NOT_FOUND")]
    UserNotFound,
    #[error("CREDENTIAL_NOT_FOUND")]
    CredentialNotFound,
    #[error("DEVICE_NOT_FOUND")]
    DeviceNotFound,
    #[error("DATABASE_UNAVAILABLE")]
    Unavailable,
}

/// Pure transition rules for the enrollment lifecycle.
pub fn can_transition(from: EnrollmentStatus, to: EnrollmentStatus) -> bool {
    matches!(
        (from, to),
        (EnrollmentStatus::Pending, EnrollmentStatus::Active)
            | (EnrollmentStatus::Pending, EnrollmentStatus::Failed)
            | (EnrollmentStatus::Pending, EnrollmentStatus::Cancelled)
            | (EnrollmentStatus::Failed, EnrollmentStatus::Pending)
            | (EnrollmentStatus::Failed, EnrollmentStatus::Cancelled)
            | (EnrollmentStatus::Active, EnrollmentStatus::Revoked)
    )
}

#[cfg(test)]
mod tests {
    use super::{can_transition, EnrollmentStatus};

    #[test]
    fn allows_approved_transitions() {
        assert!(can_transition(
            EnrollmentStatus::Pending,
            EnrollmentStatus::Active
        ));
        assert!(can_transition(
            EnrollmentStatus::Pending,
            EnrollmentStatus::Failed
        ));
        assert!(can_transition(
            EnrollmentStatus::Pending,
            EnrollmentStatus::Cancelled
        ));
        assert!(can_transition(
            EnrollmentStatus::Failed,
            EnrollmentStatus::Pending
        ));
        assert!(can_transition(
            EnrollmentStatus::Failed,
            EnrollmentStatus::Cancelled
        ));
        assert!(can_transition(
            EnrollmentStatus::Active,
            EnrollmentStatus::Revoked
        ));
    }

    #[test]
    fn rejects_invalid_and_terminal_transitions() {
        assert!(!can_transition(
            EnrollmentStatus::Active,
            EnrollmentStatus::Pending
        ));
        assert!(!can_transition(
            EnrollmentStatus::Cancelled,
            EnrollmentStatus::Pending
        ));
        assert!(!can_transition(
            EnrollmentStatus::Revoked,
            EnrollmentStatus::Active
        ));
        assert!(!can_transition(
            EnrollmentStatus::Failed,
            EnrollmentStatus::Active
        ));
        assert!(!can_transition(
            EnrollmentStatus::Pending,
            EnrollmentStatus::Revoked
        ));
    }

    #[test]
    fn terminal_states() {
        assert!(EnrollmentStatus::Cancelled.is_terminal());
        assert!(EnrollmentStatus::Revoked.is_terminal());
        assert!(!EnrollmentStatus::Pending.is_terminal());
        assert!(!EnrollmentStatus::Failed.is_terminal());
    }
}
