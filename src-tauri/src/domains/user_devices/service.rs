use chrono::Utc;
use uuid::Uuid;

use crate::database::models::{UserDeviceRecord, UserOnDeviceRecord};
use crate::database::repositories::{DeviceRepository, UserDeviceRepository, UserRepository};
use crate::database::DatabaseError;
use crate::domains::users::UserStatus;

use super::{UserDeviceAssignment, UserDeviceError, UserOnDevice};

pub async fn list_user_devices(
    users: &UserRepository,
    assignments: &UserDeviceRepository,
    user_id: Uuid,
) -> Result<Vec<UserDeviceAssignment>, UserDeviceError> {
    ensure_user(users, user_id).await?;
    let records = assignments
        .list_for_user(user_id)
        .await
        .map_err(map_db_error)?;
    Ok(records.into_iter().map(from_record).collect())
}

pub async fn list_users_for_device(
    devices: &DeviceRepository,
    assignments: &UserDeviceRepository,
    device_id: Uuid,
) -> Result<Vec<UserOnDevice>, UserDeviceError> {
    ensure_device(devices, device_id).await?;
    let records = assignments
        .list_for_device(device_id)
        .await
        .map_err(map_db_error)?;
    records
        .into_iter()
        .map(from_user_on_device)
        .collect::<Result<Vec<_>, _>>()
}

pub async fn assign_user_device(
    users: &UserRepository,
    devices: &DeviceRepository,
    assignments: &UserDeviceRepository,
    user_id: Uuid,
    device_id: Uuid,
) -> Result<UserDeviceAssignment, UserDeviceError> {
    ensure_user(users, user_id).await?;
    ensure_device(devices, device_id).await?;
    let id = Uuid::new_v4();
    let created_at = Utc::now();
    assignments
        .insert(id, user_id, device_id, created_at)
        .await
        .map_err(map_db_error)?;
    tracing::info!(user_id = %user_id, device_id = %device_id, "assigned user to device");
    assignments
        .list_for_user(user_id)
        .await
        .map_err(map_db_error)?
        .into_iter()
        .find(|record| record.device_id == device_id)
        .map(from_record)
        .ok_or(UserDeviceError::Unavailable)
}

pub async fn remove_user_device(
    assignments: &UserDeviceRepository,
    user_id: Uuid,
    device_id: Uuid,
) -> Result<(), UserDeviceError> {
    let removed = assignments
        .delete_pair(user_id, device_id)
        .await
        .map_err(map_db_error)?;
    if !removed {
        return Err(UserDeviceError::NotFound);
    }
    tracing::info!(user_id = %user_id, device_id = %device_id, "removed user device assignment");
    Ok(())
}

async fn ensure_user(users: &UserRepository, user_id: Uuid) -> Result<(), UserDeviceError> {
    match users.find_by_id(user_id).await.map_err(map_db_error)? {
        Some(_) => Ok(()),
        None => Err(UserDeviceError::UserNotFound),
    }
}

async fn ensure_device(devices: &DeviceRepository, device_id: Uuid) -> Result<(), UserDeviceError> {
    match devices.find_by_id(device_id).await.map_err(map_db_error)? {
        Some(_) => Ok(()),
        None => Err(UserDeviceError::DeviceNotFound),
    }
}

fn from_record(record: UserDeviceRecord) -> UserDeviceAssignment {
    UserDeviceAssignment {
        id: record.id,
        user_id: record.user_id,
        device_id: record.device_id,
        device_name: record.device_name,
        host: record.host,
        port: record.port,
        created_at: record.created_at,
    }
}

fn from_user_on_device(record: UserOnDeviceRecord) -> Result<UserOnDevice, UserDeviceError> {
    Ok(UserOnDevice {
        id: record.id,
        user_id: record.user_id,
        device_id: record.device_id,
        username: record.username,
        status: UserStatus::parse(&record.status).map_err(|_| UserDeviceError::Unavailable)?,
        created_at: record.created_at,
        provisioned_at: record.provisioned_at,
    })
}

fn map_db_error(error: DatabaseError) -> UserDeviceError {
    if error.is_unique_violation() {
        return UserDeviceError::Duplicate;
    }
    tracing::error!(error = %error, "user device assignment persistence failed");
    UserDeviceError::Unavailable
}
