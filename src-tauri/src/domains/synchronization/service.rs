use chrono::Utc;
use uuid::Uuid;

use crate::common::{DevicePasswordVault, SecretError};
use crate::database::repositories::{
    DeviceRepository, DeviceUserRepository, UserDeviceRepository, UserRepository,
};
use crate::database::DatabaseError;
use crate::domains::device_users::{self, DeviceUserError};
use crate::domains::devices::DeviceStatus;
use crate::domains::users::UserStatus;
use crate::matrix::{MatrixAdapter, MatrixAdapterError, SetUserParams};

use super::{
    matrix_display_name, SyncError, SyncUserFailure, SyncUsersResult, SyncedDeviceUser,
    SYNC_USER_LIMIT,
};

/// Create or update each assigned user on the device via `/device.cgi/users?action=set`.
///
/// Local assignment must already exist. A device that rejects one user does not
/// stop the remaining users. Credentials are not sent.
pub async fn sync_assigned_users(
    users: &UserRepository,
    devices: &DeviceRepository,
    assignments: &UserDeviceRepository,
    device_users: &DeviceUserRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    device_id: Uuid,
    user_ids: &[Uuid],
) -> Result<SyncUsersResult, SyncError> {
    let user_ids = unique_ids(user_ids);
    if user_ids.is_empty() {
        return Err(SyncError::NoUsers);
    }
    if user_ids.len() > SYNC_USER_LIMIT {
        return Err(SyncError::TooMany);
    }

    let device = devices
        .find_by_id(device_id)
        .await
        .map_err(map_db_error)?
        .ok_or(SyncError::DeviceNotFound)?;
    if DeviceStatus::parse(&device.status).map_err(|_| SyncError::Unavailable)?
        != DeviceStatus::Active
    {
        return Err(SyncError::DeviceInactive);
    }
    let port = u16::try_from(device.port).map_err(|_| SyncError::InvalidPort)?;
    let password = vault
        .decrypt(&device.password_ciphertext)
        .map_err(map_secret_error)?;

    let mut synced = Vec::new();
    let mut failed = Vec::new();
    let mut saw_success = false;
    let mut saw_offline = false;

    for user_id in user_ids {
        match push_one(
            users,
            devices,
            assignments,
            device_users,
            matrix,
            &device.host,
            port,
            &device.username,
            &password,
            user_id,
            device_id,
        )
        .await
        {
            Ok(row) => {
                saw_success = true;
                synced.push(row);
            }
            Err(failure) => {
                if is_offline_code(&failure.code) {
                    saw_offline = true;
                }
                tracing::warn!(
                    device_id = %device_id,
                    user_id = %failure.user_id,
                    code = %failure.code,
                    "user was not added on the device"
                );
                failed.push(failure);
            }
        }
    }
    drop(password);

    if saw_success || saw_offline {
        let now = Utc::now();
        let status = if saw_success { "online" } else { "offline" };
        let last_seen = if saw_success {
            Some(now)
        } else {
            device.last_seen_at
        };
        if let Err(error) = devices
            .update_connection(device_id, status, last_seen, now)
            .await
        {
            tracing::error!(
                error = %error,
                device_id = %device_id,
                "could not record device reachability after user sync"
            );
        }
    }

    tracing::info!(
        device_id = %device_id,
        synced = synced.len(),
        failed = failed.len(),
        "finished user sync to device"
    );

    Ok(SyncUsersResult {
        device_id,
        synced,
        failed,
    })
}

async fn push_one(
    users: &UserRepository,
    devices: &DeviceRepository,
    assignments: &UserDeviceRepository,
    device_users_repo: &DeviceUserRepository,
    matrix: &MatrixAdapter,
    host: &str,
    port: u16,
    device_username: &str,
    password: &str,
    user_id: Uuid,
    device_id: Uuid,
) -> Result<SyncedDeviceUser, SyncUserFailure> {
    let assigned = assignments
        .contains_pair(user_id, device_id)
        .await
        .map_err(|error| fail(user_id, None, map_db_error(error).to_string()))?;
    if !assigned {
        return Err(fail(user_id, None, "USER_DEVICE_NOT_FOUND"));
    }

    let user = users
        .find_by_id(user_id)
        .await
        .map_err(|error| fail(user_id, None, map_db_error(error).to_string()))?
        .ok_or_else(|| fail(user_id, None, "USER_NOT_FOUND"))?;
    let username = user.username.clone();
    let active = match UserStatus::parse(&user.status) {
        Ok(UserStatus::Active) => true,
        Ok(UserStatus::Inactive) => false,
        Err(_) => return Err(fail(user_id, Some(username), "DATABASE_UNAVAILABLE")),
    };

    let mapping =
        device_users::ensure_mapping(device_users_repo, users, devices, user_id, device_id)
            .await
            .map_err(|error| {
                fail(
                    user_id,
                    Some(username.clone()),
                    map_device_user_error(error),
                )
            })?;
    let ref_user_id = u32::try_from(mapping.matrix_ref_user_id).unwrap_or(u32::MAX);
    if ref_user_id > 99_999_999 {
        return Err(fail(user_id, Some(username), "MATRIX_INVALID_ARGUMENT"));
    }

    let params = SetUserParams {
        user_id: mapping.matrix_user_id.clone(),
        ref_user_id,
        name: matrix_display_name(&username),
        user_active: Some(active),
        enable_fr: None,
    };
    matrix
        .set_user(host, port, device_username, password, &params)
        .await
        .map_err(|error| fail(user_id, Some(username.clone()), map_matrix_error(error)))?;

    let provisioned = device_users::mark_provisioned(device_users_repo, mapping.id)
        .await
        .map_err(|error| {
            fail(
                user_id,
                Some(username.clone()),
                map_device_user_error(error),
            )
        })?;

    Ok(SyncedDeviceUser {
        user_id,
        username,
        matrix_user_id: provisioned.matrix_user_id,
        matrix_ref_user_id: provisioned.matrix_ref_user_id,
        provisioned_at: provisioned
            .provisioned_at
            .ok_or_else(|| fail(user_id, None, "DATABASE_UNAVAILABLE"))?,
    })
}

fn fail(user_id: Uuid, username: Option<String>, code: impl Into<String>) -> SyncUserFailure {
    SyncUserFailure {
        user_id,
        username,
        code: code.into(),
    }
}

fn unique_ids(user_ids: &[Uuid]) -> Vec<Uuid> {
    let mut unique = Vec::new();
    for id in user_ids {
        if !unique.contains(id) {
            unique.push(*id);
        }
    }
    unique
}

fn is_offline_code(code: &str) -> bool {
    matches!(
        code,
        "MATRIX_TIMEOUT" | "MATRIX_UNREACHABLE" | "MATRIX_AUTH_FAILED"
    )
}

fn map_secret_error(error: SecretError) -> SyncError {
    tracing::error!(error = %error, "device secret vault failed");
    match error {
        SecretError::Unavailable => SyncError::SecretUnavailable,
        SecretError::Corrupt => SyncError::SecretCorrupt,
    }
}

fn map_db_error(error: DatabaseError) -> SyncError {
    tracing::error!(error = %error, "user sync persistence failed");
    SyncError::Unavailable
}

fn map_device_user_error(error: DeviceUserError) -> String {
    error.to_string()
}

fn map_matrix_error(error: MatrixAdapterError) -> String {
    error.to_string()
}
