//! Delete a person locally and on every Matrix device that already has them.
//!
//! Hardware delete is `users?action=delete`. The CGI path stays in `matrix`.
//! Local rows are removed only after every device delete succeeds, so a
//! failed door does not leave the person only in the software.

use uuid::Uuid;

use crate::common::{DevicePasswordVault, SecretError};
use crate::database::repositories::{
    CredentialRepository, DeviceRepository, DeviceUserRepository, EnrollmentRepository,
    EnrollmentSessionRepository, UserDeviceRepository, UserRepository,
};
use crate::database::DatabaseError;
use crate::matrix::{MatrixAdapter, MatrixAdapterError};

use super::UserError;

#[allow(clippy::too_many_arguments)]
pub async fn delete_user(
    users: &UserRepository,
    devices: &DeviceRepository,
    device_users: &DeviceUserRepository,
    assignments: &UserDeviceRepository,
    credentials: &CredentialRepository,
    enrollments: &EnrollmentRepository,
    sessions: &EnrollmentSessionRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    user_id: Uuid,
) -> Result<(), UserError> {
    let user = users
        .find_by_id(user_id)
        .await
        .map_err(map_db_error)?
        .ok_or(UserError::NotFound)?;

    let mappings = device_users
        .list_for_user(user.id)
        .await
        .map_err(map_db_error)?;
    for mapping in mappings {
        let device = devices
            .find_by_id(mapping.device_id)
            .await
            .map_err(map_db_error)?
            .ok_or(UserError::Unavailable)?;
        let password = vault
            .decrypt(&device.password_ciphertext)
            .map_err(map_secret_error)?;
        let port = u16::try_from(device.port).map_err(|_| UserError::InvalidArgument)?;
        tracing::info!(
            user_id = %user.id,
            device_id = %device.id,
            "deleting user on device"
        );
        matrix
            .delete_user(
                &device.host,
                port,
                &device.username,
                &password,
                &mapping.matrix_user_id,
            )
            .await
            .map_err(map_matrix_error)?;
    }

    sessions
        .delete_for_user(user.id)
        .await
        .map_err(map_db_error)?;
    enrollments
        .delete_for_user(user.id)
        .await
        .map_err(map_db_error)?;
    credentials
        .delete_for_user(user.id)
        .await
        .map_err(map_db_error)?;
    device_users
        .delete_for_user(user.id)
        .await
        .map_err(map_db_error)?;
    assignments
        .delete_for_user(user.id)
        .await
        .map_err(map_db_error)?;
    let removed = users.delete(user.id).await.map_err(map_db_error)?;
    if !removed {
        return Err(UserError::NotFound);
    }
    tracing::info!(user_id = %user.id, "deleted user locally and on assigned devices");
    Ok(())
}

fn map_db_error(error: DatabaseError) -> UserError {
    tracing::error!(error = %error, "user delete persistence failed");
    UserError::Unavailable
}

fn map_secret_error(error: SecretError) -> UserError {
    tracing::error!(error = %error, "device secret vault failed");
    match error {
        SecretError::Unavailable => UserError::SecretUnavailable,
        SecretError::Corrupt => UserError::SecretCorrupt,
    }
}

fn map_matrix_error(error: MatrixAdapterError) -> UserError {
    match error {
        MatrixAdapterError::Timeout => UserError::Timeout,
        MatrixAdapterError::Unreachable => UserError::Unreachable,
        MatrixAdapterError::AuthFailed => UserError::AuthFailed,
        MatrixAdapterError::BadResponse => UserError::BadResponse,
        MatrixAdapterError::InvalidTarget | MatrixAdapterError::InvalidArgument => {
            UserError::InvalidArgument
        }
        MatrixAdapterError::ApiError { code } => UserError::ApiError { code },
    }
}
