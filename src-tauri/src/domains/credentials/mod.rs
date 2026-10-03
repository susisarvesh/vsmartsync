//! Credentials domain.
//!
//! Application system of record for user credentials (card, PIN).
//! Does not own Matrix CGI, enrollment sessions, biometrics, or sync.

mod service;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

pub use service::{
    create_credential, deactivate_credential, get_credential, hardware_credential_write,
    list_credentials, record_hardware_credential, set_credential_status, update_credential_value,
    CredentialListFilter, HardwareCredentialDraft,
};

pub const MAX_CARD_LENGTH: usize = 64;
pub const MIN_PIN_LENGTH: usize = 4;
pub const MAX_PIN_LENGTH: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialType {
    #[serde(rename = "card")]
    Card,
    #[serde(rename = "pin")]
    Pin,
    #[serde(rename = "read_only_card")]
    ReadOnlyCard,
    #[serde(rename = "smart_card")]
    SmartCard,
    #[serde(rename = "finger")]
    Finger,
    #[serde(rename = "face")]
    Face,
    #[serde(rename = "duress_finger")]
    DuressFinger,
    #[serde(rename = "biometric_card")]
    BiometricCard,
}

impl CredentialType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Card => "card",
            Self::Pin => "pin",
            Self::ReadOnlyCard => "read_only_card",
            Self::SmartCard => "smart_card",
            Self::Finger => "finger",
            Self::Face => "face",
            Self::DuressFinger => "duress_finger",
            Self::BiometricCard => "biometric_card",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CredentialError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "card" => Ok(Self::Card),
            "pin" => Ok(Self::Pin),
            "read_only_card" => Ok(Self::ReadOnlyCard),
            "smart_card" => Ok(Self::SmartCard),
            "finger" => Ok(Self::Finger),
            "face" => Ok(Self::Face),
            "duress_finger" => Ok(Self::DuressFinger),
            "biometric_card" => Ok(Self::BiometricCard),
            _ => Err(CredentialError::InvalidType),
        }
    }

    /// Card and PIN are typed by an operator. Hardware enrollment types are not.
    pub fn is_operator_secret(self) -> bool {
        matches!(self, Self::Card | Self::Pin)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CredentialStatus {
    Active,
    Inactive,
}

impl CredentialStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CredentialError> {
        match value {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            _ => Err(CredentialError::InvalidStatus),
        }
    }
}

/// Safe credential view for React. Never includes plaintext or ciphertext.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credential {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub credential_type: CredentialType,
    pub status: CredentialStatus,
    /// Card: masked hint such as `••••6789`. PIN: always `None` (UI shows Configured).
    pub masked_value: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CredentialError {
    #[error("CREDENTIAL_INVALID_TYPE")]
    InvalidType,
    #[error("CREDENTIAL_INVALID_VALUE")]
    InvalidValue,
    #[error("CREDENTIAL_INVALID_STATUS")]
    InvalidStatus,
    #[error("CREDENTIAL_DUPLICATE")]
    Duplicate,
    #[error("CREDENTIAL_NOT_FOUND")]
    NotFound,
    #[error("USER_NOT_FOUND")]
    UserNotFound,
    #[error("CREDENTIAL_SECRET_UNAVAILABLE")]
    SecretUnavailable,
    #[error("DATABASE_UNAVAILABLE")]
    Unavailable,
}

pub fn normalize_card_identifier(raw: &str) -> Result<String, CredentialError> {
    normalize_card(raw)
}

pub fn normalize_credential_value(
    credential_type: CredentialType,
    raw: &str,
) -> Result<String, CredentialError> {
    if !credential_type.is_operator_secret() {
        return Err(CredentialError::InvalidValue);
    }
    match credential_type {
        CredentialType::Card => normalize_card(raw),
        CredentialType::Pin => normalize_pin(raw),
        CredentialType::ReadOnlyCard
        | CredentialType::SmartCard
        | CredentialType::Finger
        | CredentialType::Face
        | CredentialType::DuressFinger
        | CredentialType::BiometricCard => Err(CredentialError::InvalidValue),
    }
}

fn normalize_card(raw: &str) -> Result<String, CredentialError> {
    let compact: String = raw.chars().filter(|ch| !ch.is_whitespace()).collect();
    if compact.is_empty() || compact.chars().count() > MAX_CARD_LENGTH {
        return Err(CredentialError::InvalidValue);
    }
    if !compact
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(CredentialError::InvalidValue);
    }
    Ok(compact)
}

fn normalize_pin(raw: &str) -> Result<String, CredentialError> {
    let pin = raw.trim();
    let len = pin.chars().count();
    if !(MIN_PIN_LENGTH..=MAX_PIN_LENGTH).contains(&len) {
        return Err(CredentialError::InvalidValue);
    }
    if !pin.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(CredentialError::InvalidValue);
    }
    Ok(pin.to_string())
}

/// Digest used to reject the same card identity on one device.
///
/// Identifier type is part of the digest so a CSN and a UID with the same
/// digits stay distinct. The raw number is not stored in the enrollment row.
pub fn card_identifier_digest(identifier_type: Option<&str>, card_number: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(identifier_type.unwrap_or("unspecified").as_bytes());
    hasher.update(b":");
    hasher.update(card_number.as_bytes());
    hasher.finalize().to_vec()
}

pub fn value_digest(credential_type: CredentialType, normalized: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(credential_type.as_str().as_bytes());
    hasher.update(b":");
    hasher.update(normalized.as_bytes());
    hasher.finalize().to_vec()
}

pub fn display_hint(credential_type: CredentialType, normalized: &str) -> Option<String> {
    match credential_type {
        CredentialType::Card
        | CredentialType::ReadOnlyCard
        | CredentialType::SmartCard
        | CredentialType::BiometricCard => {
            let chars: Vec<char> = normalized.chars().collect();
            let hint = if chars.len() <= 4 {
                format!("••••{}", chars.iter().collect::<String>())
            } else {
                let tail: String = chars[chars.len() - 4..].iter().collect();
                format!("••••{tail}")
            };
            Some(hint)
        }
        CredentialType::Pin
        | CredentialType::Finger
        | CredentialType::Face
        | CredentialType::DuressFinger => None,
    }
}

pub fn masked_value_from_hint(
    credential_type: CredentialType,
    display_hint: Option<&str>,
) -> Option<String> {
    match credential_type {
        CredentialType::Card
        | CredentialType::ReadOnlyCard
        | CredentialType::SmartCard
        | CredentialType::BiometricCard => display_hint.map(str::to_string),
        CredentialType::Pin
        | CredentialType::Finger
        | CredentialType::Face
        | CredentialType::DuressFinger => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        display_hint, normalize_credential_value, value_digest, CredentialError, CredentialType,
    };

    #[test]
    fn normalizes_card_whitespace_consistently() {
        assert_eq!(
            normalize_credential_value(CredentialType::Card, "00 1234").unwrap(),
            "001234"
        );
        assert_eq!(
            normalize_credential_value(CredentialType::Card, "001234").unwrap(),
            "001234"
        );
        assert_eq!(
            value_digest(CredentialType::Card, "001234"),
            value_digest(
                CredentialType::Card,
                &normalize_credential_value(CredentialType::Card, "00 1234").unwrap()
            )
        );
    }

    #[test]
    fn rejects_invalid_card_and_pin() {
        assert_eq!(
            normalize_credential_value(CredentialType::Card, "   ").unwrap_err(),
            CredentialError::InvalidValue
        );
        assert_eq!(
            normalize_credential_value(CredentialType::Pin, "12").unwrap_err(),
            CredentialError::InvalidValue
        );
        assert_eq!(
            normalize_credential_value(CredentialType::Pin, "12ab").unwrap_err(),
            CredentialError::InvalidValue
        );
    }

    #[test]
    fn same_card_number_digests_match_across_calls() {
        let left = super::card_identifier_digest(Some("csn"), "12345678");
        let right = super::card_identifier_digest(Some("csn"), "12345678");
        let other_type = super::card_identifier_digest(Some("uid"), "12345678");
        assert_eq!(left, right);
        assert_ne!(left, other_type);
    }

    #[test]
    fn pin_has_no_display_hint() {
        assert!(display_hint(CredentialType::Pin, "1234").is_none());
        assert_eq!(
            display_hint(CredentialType::Card, "001234").as_deref(),
            Some("••••1234")
        );
    }

    #[test]
    fn parses_types() {
        assert_eq!(CredentialType::parse("CARD").unwrap(), CredentialType::Card);
        assert_eq!(
            CredentialType::parse("finger").unwrap(),
            CredentialType::Finger
        );
        assert_eq!(
            normalize_credential_value(CredentialType::Finger, "template").unwrap_err(),
            CredentialError::InvalidValue
        );
    }
}
