use chrono::Utc;
use uuid::Uuid;

use crate::common::{SecretError, SecretVault};
use crate::database::models::{CredentialRecord, CredentialWriteRecord};
use crate::database::repositories::{CredentialListQuery, CredentialRepository, UserRepository};
use crate::database::DatabaseError;

use super::{
    display_hint, masked_value_from_hint, normalize_credential_value, value_digest, Credential,
    CredentialError, CredentialStatus, CredentialType,
};

#[derive(Debug, Clone, Default)]
pub struct CredentialListFilter {
    pub user_id: Option<Uuid>,
    pub credential_type: Option<CredentialType>,
    pub status: Option<CredentialStatus>,
}

pub async fn create_credential(
    credentials: &CredentialRepository,
    users: &UserRepository,
    vault: &SecretVault,
    user_id: Uuid,
    credential_type: &str,
    value: &str,
) -> Result<Credential, CredentialError> {
    ensure_user_exists(users, user_id).await?;
    let credential_type = CredentialType::parse(credential_type)?;
    let normalized = normalize_credential_value(credential_type, value)?;
    let ciphertext = vault.encrypt(&normalized).map_err(map_secret_error)?;
    let digest = value_digest(credential_type, &normalized);
    let hint = display_hint(credential_type, &normalized);
    let now = Utc::now();

    let record = CredentialWriteRecord {
        id: Uuid::new_v4(),
        user_id,
        credential_type: credential_type.as_str().to_string(),
        value_ciphertext: ciphertext,
        value_digest: digest,
        display_hint: hint,
        status: CredentialStatus::Active.as_str().to_string(),
        created_at: now,
        updated_at: now,
    };

    credentials
        .insert(&record)
        .await
        .map_err(map_db_error)?;

    tracing::info!(
        credential_id = %record.id,
        user_id = %user_id,
        credential_type = credential_type.as_str(),
        command = "create_credential",
        "created credential"
    );

    get_credential(credentials, record.id).await
}

pub async fn list_credentials(
    credentials: &CredentialRepository,
    filter: CredentialListFilter,
) -> Result<Vec<Credential>, CredentialError> {
    let query = CredentialListQuery {
        user_id: filter.user_id,
        credential_type: filter.credential_type.map(|value| value.as_str().to_string()),
        status: filter.status.map(|value| value.as_str().to_string()),
    };
    credentials
        .list(&query)
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(from_record)
        .collect()
}

pub async fn get_credential(
    credentials: &CredentialRepository,
    id: Uuid,
) -> Result<Credential, CredentialError> {
    let record = credentials
        .find_by_id(id)
        .await
        .map_err(map_db_error)?
        .ok_or(CredentialError::NotFound)?;
    from_record(record)
}

pub async fn update_credential_value(
    credentials: &CredentialRepository,
    vault: &SecretVault,
    id: Uuid,
    value: &str,
) -> Result<Credential, CredentialError> {
    let existing = credentials
        .find_by_id(id)
        .await
        .map_err(map_db_error)?
        .ok_or(CredentialError::NotFound)?;
    let credential_type = CredentialType::parse(&existing.credential_type)?;
    let normalized = normalize_credential_value(credential_type, value)?;
    let ciphertext = vault.encrypt(&normalized).map_err(map_secret_error)?;
    let digest = value_digest(credential_type, &normalized);
    let hint = display_hint(credential_type, &normalized);
    let updated_at = Utc::now();

    credentials
        .update_value(id, &ciphertext, &digest, hint.as_deref(), updated_at)
        .await
        .map_err(map_db_error)?
        .ok_or(CredentialError::NotFound)?;

    tracing::info!(
        credential_id = %id,
        credential_type = credential_type.as_str(),
        command = "update_credential",
        "updated credential value"
    );

    get_credential(credentials, id).await
}

pub async fn set_credential_status(
    credentials: &CredentialRepository,
    id: Uuid,
    status: &str,
) -> Result<Credential, CredentialError> {
    let status = CredentialStatus::parse(status)?;
    let _ = credentials
        .find_by_id(id)
        .await
        .map_err(map_db_error)?
        .ok_or(CredentialError::NotFound)?;
    let updated_at = Utc::now();
    credentials
        .update_status(id, status.as_str(), updated_at)
        .await
        .map_err(map_db_error)?
        .ok_or(CredentialError::NotFound)?;

    tracing::info!(
        credential_id = %id,
        status = status.as_str(),
        command = "set_credential_status",
        "updated credential status"
    );

    get_credential(credentials, id).await
}

pub async fn deactivate_credential(
    credentials: &CredentialRepository,
    id: Uuid,
) -> Result<Credential, CredentialError> {
    set_credential_status(credentials, id, CredentialStatus::Inactive.as_str()).await
}

async fn ensure_user_exists(users: &UserRepository, user_id: Uuid) -> Result<(), CredentialError> {
    users
        .find_by_id(user_id)
        .await
        .map_err(map_db_error)?
        .ok_or(CredentialError::UserNotFound)?;
    Ok(())
}

fn from_record(record: CredentialRecord) -> Result<Credential, CredentialError> {
    let credential_type = CredentialType::parse(&record.credential_type)?;
    Ok(Credential {
        id: record.id,
        user_id: record.user_id,
        user_name: record.user_name,
        credential_type,
        status: CredentialStatus::parse(&record.status)?,
        masked_value: masked_value_from_hint(credential_type, record.display_hint.as_deref()),
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn map_db_error(error: DatabaseError) -> CredentialError {
    if error.is_unique_violation() {
        return CredentialError::Duplicate;
    }
    if error.is_foreign_key_violation() {
        return CredentialError::UserNotFound;
    }
    tracing::error!(error = %error, "credentials persistence failed");
    CredentialError::Unavailable
}

fn map_secret_error(error: SecretError) -> CredentialError {
    tracing::error!(error = %error, "credential secret vault failed");
    CredentialError::SecretUnavailable
}

#[cfg(test)]
mod tests {
    use super::from_record;
    use crate::database::models::CredentialRecord;
    use crate::domains::credentials::{CredentialStatus, CredentialType};
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn public_view_omits_secrets() {
        let record = CredentialRecord {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            user_name: "Ada".into(),
            credential_type: "card".into(),
            display_hint: Some("••••1234".into()),
            status: "active".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let credential = from_record(record).unwrap();
        let json = serde_json::to_string(&credential).unwrap();
        assert!(!json.contains("ciphertext"));
        assert!(!json.contains("digest"));
        assert!(!json.contains("password"));
        assert_eq!(credential.credential_type, CredentialType::Card);
        assert_eq!(credential.status, CredentialStatus::Active);
        assert_eq!(credential.masked_value.as_deref(), Some("••••1234"));
    }

    #[test]
    fn pin_view_has_no_masked_value() {
        let record = CredentialRecord {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            user_name: "Ada".into(),
            credential_type: "pin".into(),
            display_hint: None,
            status: "active".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let credential = from_record(record).unwrap();
        let json = serde_json::to_string(&credential).unwrap();
        assert!(credential.masked_value.is_none());
        assert!(!json.contains("1234"));
    }
}
