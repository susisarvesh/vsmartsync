//! Live `enrolluser` session.
//!
//! Status lives here, not on the durable assignment row. Card numbers are
//! decrypted only when building the operator view and are never logged.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::{SecretError, SecretVault};
use crate::database::repositories::{CredentialRepository, EnrollmentSessionRepository};
use crate::database::DatabaseError;
use crate::matrix::{MatrixCardType, MatrixIdentifierType};

use super::EnrollmentError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentSessionStatus {
    Starting,
    WaitingForCard,
    Processing,
    Verifying,
    Saving,
    Success,
    Failed,
    Cancelled,
    Timeout,
    Busy,
    PersistenceFailed,
}

impl EnrollmentSessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::WaitingForCard => "waiting_for_card",
            Self::Processing => "processing",
            Self::Verifying => "verifying",
            Self::Saving => "saving",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Timeout => "timeout",
            Self::Busy => "busy",
            Self::PersistenceFailed => "persistence_failed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, EnrollmentError> {
        match value {
            "starting" => Ok(Self::Starting),
            "waiting_for_card" => Ok(Self::WaitingForCard),
            "processing" => Ok(Self::Processing),
            "verifying" => Ok(Self::Verifying),
            "saving" => Ok(Self::Saving),
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "timeout" => Ok(Self::Timeout),
            "busy" => Ok(Self::Busy),
            "persistence_failed" => Ok(Self::PersistenceFailed),
            _ => Err(EnrollmentError::InvalidTransition),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Success
                | Self::Failed
                | Self::Cancelled
                | Self::Timeout
                | Self::Busy
                | Self::PersistenceFailed
        )
    }
}

/// What the count check concluded before any database write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureVerdict {
    Increased,
    Unchanged,
    TimedOut,
    Failed,
}

/// What the post-capture database write concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistVerdict {
    Saved,
    Duplicate,
    Failed,
}

/// Cancel wins. A capture that increased counts is not success until the
/// database write succeeds. A database failure after that increase is
/// `persistence_failed`, not success.
pub fn terminal_after_capture(
    cancelled: bool,
    capture: CaptureVerdict,
    persist: Option<PersistVerdict>,
) -> EnrollmentSessionStatus {
    if cancelled {
        return EnrollmentSessionStatus::Cancelled;
    }
    match capture {
        CaptureVerdict::TimedOut => EnrollmentSessionStatus::Timeout,
        CaptureVerdict::Failed | CaptureVerdict::Unchanged => EnrollmentSessionStatus::Failed,
        CaptureVerdict::Increased => match persist {
            Some(PersistVerdict::Saved) => EnrollmentSessionStatus::Success,
            Some(PersistVerdict::Duplicate) => EnrollmentSessionStatus::Failed,
            Some(PersistVerdict::Failed) | None => EnrollmentSessionStatus::PersistenceFailed,
        },
    }
}

pub fn should_persist(cancelled: bool, increased: bool) -> bool {
    !cancelled && increased
}

/// Safe session view. `card_number` is the identifier just read, for the
/// operator confirming the tap. Lists keep the masked hint instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollmentSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub device_id: Uuid,
    pub device_name: String,
    pub enroll_type: String,
    pub status: EnrollmentSessionStatus,
    pub credential_id: Option<Uuid>,
    pub error_code: Option<String>,
    pub card_type: Option<String>,
    pub card_type_label: Option<String>,
    pub identifier_type: Option<String>,
    pub identifier_type_label: Option<String>,
    pub card_number: Option<String>,
    pub identifier_unavailable: bool,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Direct card read. Does not create an enrollment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardRead {
    pub device_id: Uuid,
    pub card_type: Option<String>,
    pub card_type_label: Option<String>,
    pub card_number: Option<String>,
}

/// One interface reported by `reader-config?action=get`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderSlotStatus {
    pub slot: String,
    pub code: i32,
    pub label: String,
    pub family: Option<String>,
}

/// Reader and smart-card configuration. No keys and no card number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardReaderStatus {
    pub device_id: Uuid,
    pub reader: Option<String>,
    pub reader_label: Option<String>,
    pub reader_code: Option<i32>,
    /// Every configured reader from `reader-config`, in slot order.
    pub readers: Vec<ReaderSlotStatus>,
    pub door_access_mode: Option<i32>,
    pub supported: bool,
    pub card_type: Option<String>,
    pub card_type_label: Option<String>,
    pub identifier: Option<String>,
    pub identifier_label: Option<String>,
    pub mifare_custom_key_enabled: bool,
    pub hid_iclass_custom_key_enabled: bool,
    pub card_custom_key_auto_update: bool,
    pub read_csn: Option<String>,
    pub max_card_bits: Option<u32>,
    pub message: String,
}

/// Result of `card-read-write?action=read`. This does not create a credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardTestResult {
    pub device_id: Uuid,
    pub outcome: String,
    pub message: String,
    pub compatible: bool,
    pub reader: Option<String>,
    pub reader_label: Option<String>,
    pub card_type: Option<String>,
    pub card_type_label: Option<String>,
    pub card_number: Option<String>,
    pub response_code: Option<i32>,
}

pub async fn get_enrollment_session(
    sessions: &EnrollmentSessionRepository,
    credentials: &CredentialRepository,
    vault: &SecretVault,
    id: Uuid,
) -> Result<EnrollmentSession, EnrollmentError> {
    let record = sessions
        .find_by_id(id)
        .await
        .map_err(map_db_error)?
        .ok_or(EnrollmentError::SessionNotFound)?;
    let mut view = to_view(&record);
    if let Some(credential_id) = record.credential_id {
        if let Some(number) = reveal_card_number(credentials, vault, credential_id).await? {
            view.card_number = Some(number);
        }
    }
    Ok(view)
}

pub async fn cancel_enrollment_session(
    sessions: &EnrollmentSessionRepository,
    credentials: &CredentialRepository,
    vault: &SecretVault,
    id: Uuid,
) -> Result<EnrollmentSession, EnrollmentError> {
    let now = Utc::now();
    let updated = sessions
        .update_if_open(
            id,
            EnrollmentSessionStatus::Cancelled.as_str(),
            None,
            None,
            None,
            false,
            None,
            Some(now),
            now,
        )
        .await
        .map_err(map_db_error)?;
    if updated {
        tracing::info!(
            enrollment_id = %id,
            result = "cancelled",
            "enrollment_cancelled"
        );
    }
    get_enrollment_session(sessions, credentials, vault, id).await
}

pub(crate) fn to_view(
    record: &crate::database::models::EnrollmentSessionRecord,
) -> EnrollmentSession {
    let card_type_label = record
        .card_type
        .as_deref()
        .and_then(MatrixCardType::parse)
        .map(|value| value.label().to_string());
    let identifier_type_label = record
        .identifier_type
        .as_deref()
        .and_then(MatrixIdentifierType::parse)
        .map(|value| value.label().to_string());
    EnrollmentSession {
        id: record.id,
        user_id: record.user_id,
        user_name: record.user_name.clone(),
        device_id: record.device_id,
        device_name: record.device_name.clone(),
        enroll_type: record.enroll_type.clone(),
        status: EnrollmentSessionStatus::parse(&record.status)
            .unwrap_or(EnrollmentSessionStatus::Failed),
        credential_id: record.credential_id,
        error_code: record.error_code.clone(),
        card_type: record.card_type.clone(),
        card_type_label,
        identifier_type: record.identifier_type.clone(),
        identifier_type_label,
        card_number: None,
        identifier_unavailable: record.identifier_unavailable,
        started_at: record.started_at,
        completed_at: record.completed_at,
    }
}

async fn reveal_card_number(
    credentials: &CredentialRepository,
    vault: &SecretVault,
    credential_id: Uuid,
) -> Result<Option<String>, EnrollmentError> {
    let Some(ciphertext) = credentials
        .find_value_ciphertext(credential_id)
        .await
        .map_err(map_db_error)?
    else {
        return Ok(None);
    };
    match vault.decrypt(&ciphertext) {
        Ok(plain) if !plain.starts_with("enrolled:") => Ok(Some(plain)),
        Ok(_) => Ok(None),
        Err(SecretError::Corrupt) => Ok(None),
        Err(SecretError::Unavailable) => Err(EnrollmentError::SecretUnavailable),
    }
}

fn map_db_error(error: DatabaseError) -> EnrollmentError {
    if error.is_unique_violation() {
        return EnrollmentError::Duplicate;
    }
    tracing::error!(error = %error, "enrollment session persistence failed");
    EnrollmentError::Unavailable
}

#[cfg(test)]
mod tests {
    use super::{
        should_persist, terminal_after_capture, CaptureVerdict, EnrollmentSessionStatus,
        PersistVerdict,
    };

    #[test]
    fn timeout_cancel_and_persistence_failure() {
        assert_eq!(
            terminal_after_capture(false, CaptureVerdict::TimedOut, None),
            EnrollmentSessionStatus::Timeout
        );
        assert_eq!(
            terminal_after_capture(true, CaptureVerdict::Increased, Some(PersistVerdict::Saved)),
            EnrollmentSessionStatus::Cancelled
        );
        assert!(!should_persist(true, true));
        assert!(should_persist(false, true));
        assert_eq!(
            terminal_after_capture(
                false,
                CaptureVerdict::Increased,
                Some(PersistVerdict::Failed)
            ),
            EnrollmentSessionStatus::PersistenceFailed
        );
        assert_eq!(
            terminal_after_capture(
                false,
                CaptureVerdict::Increased,
                Some(PersistVerdict::Duplicate)
            ),
            EnrollmentSessionStatus::Failed
        );
        assert_eq!(
            terminal_after_capture(false, CaptureVerdict::Unchanged, None),
            EnrollmentSessionStatus::Failed
        );
        assert_eq!(
            terminal_after_capture(
                false,
                CaptureVerdict::Increased,
                Some(PersistVerdict::Saved)
            ),
            EnrollmentSessionStatus::Success
        );
    }
}
