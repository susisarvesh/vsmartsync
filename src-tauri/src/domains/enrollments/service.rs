use chrono::Utc;
use uuid::Uuid;

use crate::database::models::EnrollmentWriteRecord;
use crate::database::repositories::{
    CredentialRepository, DeviceRepository, EnrollmentListQuery, EnrollmentRepository,
    UserRepository,
};
use crate::database::DatabaseError;
use crate::domains::credentials::{masked_value_from_hint, CredentialStatus, CredentialType};
use crate::domains::users::UserStatus;

use super::{can_transition, Enrollment, EnrollmentError, EnrollmentStatus};

#[derive(Debug, Clone, Default)]
pub struct EnrollmentListFilter {
    pub user_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub credential_id: Option<Uuid>,
    pub status: Option<EnrollmentStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn create_enrollment(
    enrollments: &EnrollmentRepository,
    users: &UserRepository,
    credentials: &CredentialRepository,
    devices: &DeviceRepository,
    user_id: Uuid,
    credential_id: Uuid,
    device_id: Uuid,
) -> Result<Enrollment, EnrollmentError> {
    let user = users
        .find_by_id(user_id)
        .await
        .map_err(map_db_error)?
        .ok_or(EnrollmentError::UserNotFound)?;
    if user.status != UserStatus::Active.as_str() {
        return Err(EnrollmentError::UserInactive);
    }

    let credential = credentials
        .find_by_id(credential_id)
        .await
        .map_err(map_db_error)?
        .ok_or(EnrollmentError::CredentialNotFound)?;
    if credential.status != CredentialStatus::Active.as_str() {
        return Err(EnrollmentError::CredentialInactive);
    }
    if credential.user_id != user_id {
        return Err(EnrollmentError::CredentialOwnership);
    }

    devices
        .find_by_id(device_id)
        .await
        .map_err(map_db_error)?
        .ok_or(EnrollmentError::DeviceNotFound)?;

    let now = Utc::now();
    let record = EnrollmentWriteRecord {
        id: Uuid::new_v4(),
        user_id,
        credential_id,
        device_id,
        status: EnrollmentStatus::Pending.as_str().to_string(),
        cancelled_at: None,
        revoked_at: None,
        activated_at: None,
        created_at: now,
        updated_at: now,
    };

    enrollments.insert(&record).await.map_err(map_db_error)?;

    tracing::info!(
        enrollment_id = %record.id,
        user_id = %user_id,
        credential_id = %credential_id,
        device_id = %device_id,
        command = "create_enrollment",
        "created enrollment"
    );

    get_enrollment(enrollments, record.id).await
}

pub async fn list_enrollments(
    enrollments: &EnrollmentRepository,
    filter: EnrollmentListFilter,
) -> Result<Vec<Enrollment>, EnrollmentError> {
    let query = EnrollmentListQuery {
        user_id: filter.user_id,
        device_id: filter.device_id,
        credential_id: filter.credential_id,
        status: filter.status.map(|value| value.as_str().to_string()),
        limit: filter.limit,
        offset: filter.offset,
    };
    enrollments
        .list(&query)
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(from_record)
        .collect()
}

pub async fn get_enrollment(
    enrollments: &EnrollmentRepository,
    id: Uuid,
) -> Result<Enrollment, EnrollmentError> {
    let record = enrollments
        .find_by_id(id)
        .await
        .map_err(map_db_error)?
        .ok_or(EnrollmentError::NotFound)?;
    from_record(record)
}

pub async fn cancel_enrollment(
    enrollments: &EnrollmentRepository,
    id: Uuid,
) -> Result<Enrollment, EnrollmentError> {
    transition_enrollment(enrollments, id, EnrollmentStatus::Cancelled).await
}

pub async fn revoke_enrollment(
    enrollments: &EnrollmentRepository,
    id: Uuid,
) -> Result<Enrollment, EnrollmentError> {
    transition_enrollment(enrollments, id, EnrollmentStatus::Revoked).await
}

pub async fn retry_enrollment(
    enrollments: &EnrollmentRepository,
    id: Uuid,
) -> Result<Enrollment, EnrollmentError> {
    transition_enrollment(enrollments, id, EnrollmentStatus::Pending).await
}

/// Reserved for future Sync. No Matrix HTTP. Exposed for tests and Sync slice.
pub async fn mark_enrollment_active(
    enrollments: &EnrollmentRepository,
    id: Uuid,
) -> Result<Enrollment, EnrollmentError> {
    transition_enrollment(enrollments, id, EnrollmentStatus::Active).await
}

/// Reserved for future Sync. No Matrix HTTP. Exposed for tests and Sync slice.
pub async fn mark_enrollment_failed(
    enrollments: &EnrollmentRepository,
    id: Uuid,
) -> Result<Enrollment, EnrollmentError> {
    transition_enrollment(enrollments, id, EnrollmentStatus::Failed).await
}

async fn transition_enrollment(
    enrollments: &EnrollmentRepository,
    id: Uuid,
    next: EnrollmentStatus,
) -> Result<Enrollment, EnrollmentError> {
    let current = enrollments
        .find_by_id(id)
        .await
        .map_err(map_db_error)?
        .ok_or(EnrollmentError::NotFound)?;
    let from = EnrollmentStatus::parse(&current.status)?;
    if !can_transition(from, next) {
        return Err(EnrollmentError::InvalidTransition);
    }

    let now = Utc::now();
    let activated_at = match next {
        EnrollmentStatus::Active => Some(now),
        EnrollmentStatus::Pending => None,
        _ => current.activated_at,
    };
    let cancelled_at = match next {
        EnrollmentStatus::Cancelled => Some(now),
        _ => current.cancelled_at,
    };
    let revoked_at = match next {
        EnrollmentStatus::Revoked => Some(now),
        _ => current.revoked_at,
    };

    enrollments
        .update_status(
            id,
            next.as_str(),
            activated_at,
            cancelled_at,
            revoked_at,
            now,
        )
        .await
        .map_err(map_db_error)?
        .ok_or(EnrollmentError::NotFound)?;

    tracing::info!(
        enrollment_id = %id,
        from = from.as_str(),
        to = next.as_str(),
        "enrollment status updated"
    );

    get_enrollment(enrollments, id).await
}

fn from_record(
    record: crate::database::models::EnrollmentRecord,
) -> Result<Enrollment, EnrollmentError> {
    let credential_type =
        CredentialType::parse(&record.credential_type).map_err(|_| EnrollmentError::Unavailable)?;
    Ok(Enrollment {
        id: record.id,
        user_id: record.user_id,
        user_name: record.user_name,
        credential_id: record.credential_id,
        credential_type: credential_type.as_str().to_string(),
        masked_value: masked_value_from_hint(credential_type, record.display_hint.as_deref()),
        device_id: record.device_id,
        device_name: record.device_name,
        status: EnrollmentStatus::parse(&record.status)?,
        activated_at: record.activated_at,
        cancelled_at: record.cancelled_at,
        revoked_at: record.revoked_at,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn map_db_error(error: DatabaseError) -> EnrollmentError {
    if error.is_unique_violation() {
        return EnrollmentError::Duplicate;
    }
    if error.is_foreign_key_violation() {
        return EnrollmentError::Unavailable;
    }
    tracing::error!(error = %error, "enrollments persistence failed");
    EnrollmentError::Unavailable
}

#[cfg(test)]
mod tests {
    use super::from_record;
    use crate::database::models::EnrollmentRecord;
    use crate::domains::enrollments::EnrollmentStatus;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn public_view_omits_secrets() {
        let record = EnrollmentRecord {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            user_name: "Ada".into(),
            credential_id: Uuid::new_v4(),
            credential_type: "pin".into(),
            display_hint: None,
            device_id: Uuid::new_v4(),
            device_name: "Door".into(),
            status: "pending".into(),
            cancelled_at: None,
            revoked_at: None,
            activated_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let enrollment = from_record(record).unwrap();
        let json = serde_json::to_string(&enrollment).unwrap();
        assert!(!json.contains("ciphertext"));
        assert!(!json.contains("password"));
        assert!(!json.contains("digest"));
        assert_eq!(enrollment.status, EnrollmentStatus::Pending);
        assert!(enrollment.masked_value.is_none());
    }
}
