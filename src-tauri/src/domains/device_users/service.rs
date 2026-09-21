use chrono::Utc;
use uuid::Uuid;

use crate::database::models::DeviceUserRecord;
use crate::database::repositories::{DeviceRepository, DeviceUserRepository, UserRepository};
use crate::database::DatabaseError;
use crate::domains::device_users::{DeviceUser, DeviceUserError};

/// Ensure a Matrix identity mapping exists for `(user, device)`.
///
/// Allocates `VS######` / ref-user-id when missing. Does **not** call Matrix
/// and does **not** set `provisioned_at`.
pub async fn ensure_mapping(
    device_users: &DeviceUserRepository,
    users: &UserRepository,
    devices: &DeviceRepository,
    user_id: Uuid,
    device_id: Uuid,
) -> Result<DeviceUser, DeviceUserError> {
    let _user = users
        .find_by_id(user_id)
        .await
        .map_err(map_db_error)?
        .ok_or(DeviceUserError::UserNotFound)?;
    let _device = devices
        .find_by_id(device_id)
        .await
        .map_err(map_db_error)?
        .ok_or(DeviceUserError::DeviceNotFound)?;

    let now = Utc::now();
    let record = device_users
        .ensure_mapping(user_id, device_id, now)
        .await
        .map_err(map_db_error)?;

    tracing::info!(
        command = "ensure_device_user_mapping",
        user_id = %user_id,
        device_id = %device_id,
        device_user_id = %record.id,
        matrix_user_id = %record.matrix_user_id,
        matrix_ref_user_id = record.matrix_ref_user_id,
        "ensured device user mapping"
    );

    Ok(from_record(record))
}

pub async fn get_device_user(
    device_users: &DeviceUserRepository,
    id: Uuid,
) -> Result<DeviceUser, DeviceUserError> {
    let record = device_users
        .find_by_id(id)
        .await
        .map_err(map_db_error)?
        .ok_or(DeviceUserError::NotFound)?;
    Ok(from_record(record))
}

pub async fn list_device_users_for_device(
    device_users: &DeviceUserRepository,
    device_id: Uuid,
) -> Result<Vec<DeviceUser>, DeviceUserError> {
    let rows = device_users
        .list_for_device(device_id)
        .await
        .map_err(map_db_error)?;
    Ok(rows.into_iter().map(from_record).collect())
}

/// Record that Sync successfully applied `set_user` for this mapping.
pub async fn mark_provisioned(
    device_users: &DeviceUserRepository,
    id: Uuid,
) -> Result<DeviceUser, DeviceUserError> {
    let now = Utc::now();
    let record = device_users
        .mark_provisioned(id, now)
        .await
        .map_err(map_db_error)?
        .ok_or(DeviceUserError::NotFound)?;

    tracing::info!(
        command = "mark_device_user_provisioned",
        device_user_id = %id,
        matrix_user_id = %record.matrix_user_id,
        "marked device user provisioned"
    );

    Ok(from_record(record))
}

fn from_record(record: DeviceUserRecord) -> DeviceUser {
    DeviceUser {
        id: record.id,
        user_id: record.user_id,
        device_id: record.device_id,
        matrix_user_id: record.matrix_user_id,
        matrix_ref_user_id: record.matrix_ref_user_id,
        provisioned_at: record.provisioned_at,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

fn map_db_error(error: DatabaseError) -> DeviceUserError {
    if error.is_unique_violation() {
        return DeviceUserError::Duplicate;
    }
    if error.is_foreign_key_violation() {
        return DeviceUserError::Unavailable;
    }
    let message = error.to_string();
    if message.contains("sequence exhausted") || message.contains("sequence overflow") {
        return DeviceUserError::SequenceExhausted;
    }
    tracing::error!(error = %error, "device_users persistence failed");
    DeviceUserError::Unavailable
}
