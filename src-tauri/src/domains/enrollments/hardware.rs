//! Start enrollment on a COSEC device after a documented capability check.
//!
//! Order: device configuration, create the user on the device, `enrolluser`,
//! then store the user–credential–device link. Template bytes are not downloaded.
//!
//! Guide v28 response code 16 means the reader is in another menu and will not
//! accept a command. While that screen is up, this module does not send another
//! request, so a card or face can be stored on the user.

use std::future::Future;

use chrono::Utc;
use uuid::Uuid;

/// Guide v28: enrollment was refused because the reader is in another menu.
const DEVICE_BUSY: i32 = 16;
/// Seconds with no device requests so the reader can accept a card or face.
const CAPTURE_QUIET_SECS: u64 = 12;

use crate::common::{DevicePasswordVault, SecretError};
use crate::database::models::EnrollmentWriteRecord;
use crate::database::repositories::{
    CredentialRepository, DeviceRepository, DeviceUserRepository, EnrollmentRepository,
    UserDeviceRepository, UserRepository,
};
use crate::database::DatabaseError;
use crate::domains::credentials::{self, CredentialType};
use crate::domains::device_users::{self, DeviceUserError};
use crate::domains::devices::DeviceStatus;
use crate::domains::synchronization::matrix_display_name;
use crate::domains::users::UserStatus;
use crate::matrix::{
    credential_present, HardwareEnrollType, MatrixAdapter, MatrixAdapterError, SetUserParams,
};

use super::{
    get_enrollment, DeviceEnrollmentOptions, Enrollment, EnrollmentError, EnrollmentOption,
    EnrollmentStatus,
};

pub async fn device_enrollment_options(
    devices: &DeviceRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    device_id: Uuid,
) -> Result<DeviceEnrollmentOptions, EnrollmentError> {
    let device = load_active_device(devices, device_id).await?;
    let password = decrypt_password(vault, &device.password_ciphertext)?;
    let port = device_port(device.port)?;
    let types = matrix
        .enrollment_capabilities(&device.host, port, &device.username, &password)
        .await
        .map_err(map_matrix_error)?;
    drop(password);
    record_online(devices, device_id).await;

    Ok(DeviceEnrollmentOptions {
        device_id,
        options: types
            .into_iter()
            .map(|enroll_type| EnrollmentOption {
                enroll_type: enroll_type.as_str().to_string(),
                label: enroll_type.label().to_string(),
            })
            .collect(),
    })
}

pub async fn enroll_on_device(
    enrollments: &EnrollmentRepository,
    users: &UserRepository,
    credentials: &CredentialRepository,
    devices: &DeviceRepository,
    assignments: &UserDeviceRepository,
    device_users_repo: &DeviceUserRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    device_id: Uuid,
    user_id: Uuid,
    enroll_type: &str,
) -> Result<Enrollment, EnrollmentError> {
    let enroll_type = HardwareEnrollType::parse(enroll_type).ok_or(EnrollmentError::InvalidType)?;
    let user = users
        .find_by_id(user_id)
        .await
        .map_err(map_db_error)?
        .ok_or(EnrollmentError::UserNotFound)?;
    if user.status != UserStatus::Active.as_str() {
        return Err(EnrollmentError::UserInactive);
    }
    let assigned = assignments
        .contains_pair(user_id, device_id)
        .await
        .map_err(map_db_error)?;
    if !assigned {
        return Err(EnrollmentError::UserNotAssigned);
    }

    let device = load_active_device(devices, device_id).await?;
    let password = decrypt_password(vault, &device.password_ciphertext)?;
    let port = device_port(device.port)?;

    let supported = retry_if_reader_busy(|| {
        matrix.enrollment_capabilities(&device.host, port, &device.username, &password)
    })
    .await?;
    if !supported.contains(&enroll_type) {
        record_online(devices, device_id).await;
        return Err(EnrollmentError::Unsupported);
    }

    let mapping =
        device_users::ensure_mapping(device_users_repo, users, devices, user_id, device_id)
            .await
            .map_err(map_device_user_error)?;
    let ref_user_id = u32::try_from(mapping.matrix_ref_user_id).unwrap_or(u32::MAX);
    if ref_user_id > 99_999_999 {
        return Err(EnrollmentError::InvalidArgument);
    }
    let params = SetUserParams {
        user_id: mapping.matrix_user_id.clone(),
        ref_user_id,
        name: matrix_display_name(&user.username),
        user_active: Some(true),
        enable_fr: match enroll_type {
            HardwareEnrollType::Face => Some(true),
            _ => None,
        },
    };
    match matrix
        .set_user(&device.host, port, &device.username, &password, &params)
        .await
    {
        Ok(()) => {}
        Err(error) if reader_is_busy(&error) => {
            quiet_for_reader();
            matrix
                .set_user(&device.host, port, &device.username, &password, &params)
                .await
                .map_err(map_matrix_error)?;
        }
        Err(error) => return Err(map_matrix_error(error)),
    }
    device_users::mark_provisioned(device_users_repo, mapping.id)
        .await
        .map_err(map_device_user_error)?;

    if matches!(
        enroll_type,
        HardwareEnrollType::ReadOnlyCard
            | HardwareEnrollType::SmartCard
            | HardwareEnrollType::BiometricThenCard
    ) {
        retry_if_reader_busy(|| {
            matrix.enable_card_and_face_access(&device.host, port, &device.username, &password)
        })
        .await?;
    }

    let captured = capture_on_reader(
        matrix,
        &device.host,
        port,
        &device.username,
        &password,
        &mapping.matrix_user_id,
        enroll_type,
        device_id,
        user_id,
    )
    .await?;
    if enroll_type == HardwareEnrollType::Face {
        if let Err(error) = matrix
            .set_face_recognition_enabled(
                &device.host,
                port,
                &device.username,
                &password,
                &mapping.matrix_user_id,
                true,
            )
            .await
        {
            if !matches!(error, MatrixAdapterError::BadResponse) {
                return Err(map_matrix_error(error));
            }
            tracing::warn!(
                device_id = %device_id,
                user_id = %user_id,
                "face recognition flag was not confirmed; enrollment is still stored for the user"
            );
        }
    }
    drop(password);
    if !captured {
        return Err(EnrollmentError::NotCaptured);
    }
    record_online(devices, device_id).await;

    let credential_type = CredentialType::parse(enroll_type.credential_type())
        .map_err(|_| EnrollmentError::InvalidType)?;
    let credential = credentials::record_hardware_credential(
        credentials,
        users,
        vault,
        user_id,
        credential_type,
    )
    .await
    .map_err(map_credential_error)?;

    let now = Utc::now();
    let record = EnrollmentWriteRecord {
        id: Uuid::new_v4(),
        user_id,
        credential_id: credential.id,
        device_id,
        status: EnrollmentStatus::Active.as_str().to_string(),
        cancelled_at: None,
        revoked_at: None,
        activated_at: Some(now),
        created_at: now,
        updated_at: now,
    };
    enrollments.insert(&record).await.map_err(map_db_error)?;
    tracing::info!(
        enrollment_id = %record.id,
        device_id = %device_id,
        user_id = %user_id,
        enroll_type = enroll_type.as_str(),
        "stored hardware enrollment"
    );
    get_enrollment(enrollments, record.id).await
}

async fn load_active_device(
    devices: &DeviceRepository,
    device_id: Uuid,
) -> Result<crate::database::models::DeviceRecord, EnrollmentError> {
    let device = devices
        .find_by_id(device_id)
        .await
        .map_err(map_db_error)?
        .ok_or(EnrollmentError::DeviceNotFound)?;
    if DeviceStatus::parse(&device.status).map_err(|_| EnrollmentError::Unavailable)?
        != DeviceStatus::Active
    {
        return Err(EnrollmentError::DeviceInactive);
    }
    Ok(device)
}

fn device_port(port: i32) -> Result<u16, EnrollmentError> {
    u16::try_from(port).map_err(|_| EnrollmentError::InvalidArgument)
}

fn reader_is_busy(error: &MatrixAdapterError) -> bool {
    matches!(error, MatrixAdapterError::ApiError { code: DEVICE_BUSY })
}

fn quiet_for_reader() {
    std::thread::sleep(std::time::Duration::from_secs(CAPTURE_QUIET_SECS));
}

async fn retry_if_reader_busy<T, F, Fut>(mut op: F) -> Result<T, EnrollmentError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, MatrixAdapterError>>,
{
    match op().await {
        Ok(value) => Ok(value),
        Err(error) if reader_is_busy(&error) => {
            quiet_for_reader();
            op().await.map_err(map_matrix_error)
        }
        Err(error) => Err(map_matrix_error(error)),
    }
}

async fn reader_in_menu(
    matrix: &MatrixAdapter,
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    matrix_user_id: &str,
) -> bool {
    matches!(
        matrix
            .credential_counts(host, port, username, password, matrix_user_id)
            .await,
        Err(error) if reader_is_busy(&error)
    )
}

async fn capture_on_reader(
    matrix: &MatrixAdapter,
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    matrix_user_id: &str,
    enroll_type: HardwareEnrollType,
    device_id: Uuid,
    user_id: Uuid,
) -> Result<bool, EnrollmentError> {
    if reader_in_menu(matrix, host, port, username, password, matrix_user_id).await {
        quiet_for_reader();
    }
    tracing::info!(
        device_id = %device_id,
        user_id = %user_id,
        enroll_type = enroll_type.as_str(),
        "starting hardware enrollment"
    );
    match matrix
        .enroll_user(host, port, username, password, matrix_user_id, enroll_type)
        .await
    {
        Ok(()) => {
            quiet_for_reader();
            confirm_capture(
                matrix,
                host,
                port,
                username,
                password,
                matrix_user_id,
                enroll_type,
            )
            .await
        }
        Err(error) if reader_is_busy(&error) => {
            // The reader is already prompting. Another command here is refused
            // and the card or face will not be stored.
            quiet_for_reader();
            if confirm_capture(
                matrix,
                host,
                port,
                username,
                password,
                matrix_user_id,
                enroll_type,
            )
            .await?
            {
                return Ok(true);
            }
            matrix
                .enroll_user(host, port, username, password, matrix_user_id, enroll_type)
                .await
                .map_err(map_matrix_error)?;
            quiet_for_reader();
            confirm_capture(
                matrix,
                host,
                port,
                username,
                password,
                matrix_user_id,
                enroll_type,
            )
            .await
        }
        Err(error) => Err(map_matrix_error(error)),
    }
}

async fn confirm_capture(
    matrix: &MatrixAdapter,
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    matrix_user_id: &str,
    enroll_type: HardwareEnrollType,
) -> Result<bool, EnrollmentError> {
    let mut saw_counts = false;
    let mut last_error = None;
    for attempt in 0..4 {
        if attempt > 0 {
            quiet_for_reader();
        }
        match matrix
            .credential_counts(host, port, username, password, matrix_user_id)
            .await
        {
            Ok(after) => {
                saw_counts = true;
                let linked = credential_present(enroll_type, after);
                if linked {
                    return Ok(true);
                }
            }
            Err(error) => last_error = Some(error),
        }
    }
    if saw_counts || last_error.as_ref().is_some_and(reader_is_busy) {
        return Ok(false);
    }
    // `enrolluser` already succeeded on the device. An unreadable count reply
    // must not drop the user link after the reader has finished enrollment.
    if last_error
        .as_ref()
        .is_some_and(|error| matches!(error, MatrixAdapterError::BadResponse))
    {
        return Ok(true);
    }
    Err(map_matrix_error(
        last_error.unwrap_or(MatrixAdapterError::BadResponse),
    ))
}

fn decrypt_password(
    vault: &DevicePasswordVault,
    ciphertext: &[u8],
) -> Result<String, EnrollmentError> {
    vault.decrypt(ciphertext).map_err(map_secret_error)
}

async fn record_online(devices: &DeviceRepository, device_id: Uuid) {
    let now = Utc::now();
    if let Err(error) = devices
        .update_connection(device_id, "online", Some(now), now)
        .await
    {
        tracing::error!(
            error = %error,
            device_id = %device_id,
            "could not record device reachability after enrollment"
        );
    }
}

fn map_secret_error(error: SecretError) -> EnrollmentError {
    tracing::error!(error = %error, "device secret vault failed");
    match error {
        SecretError::Unavailable => EnrollmentError::SecretUnavailable,
        SecretError::Corrupt => EnrollmentError::SecretCorrupt,
    }
}

fn map_db_error(error: DatabaseError) -> EnrollmentError {
    if error.is_unique_violation() {
        return EnrollmentError::Duplicate;
    }
    tracing::error!(error = %error, "enrollment persistence failed");
    EnrollmentError::Unavailable
}

fn map_device_user_error(error: DeviceUserError) -> EnrollmentError {
    match error {
        DeviceUserError::UserNotFound => EnrollmentError::UserNotFound,
        DeviceUserError::DeviceNotFound => EnrollmentError::DeviceNotFound,
        DeviceUserError::SequenceExhausted => EnrollmentError::InvalidArgument,
        DeviceUserError::NotFound | DeviceUserError::Duplicate | DeviceUserError::Unavailable => {
            EnrollmentError::Unavailable
        }
    }
}

fn map_credential_error(error: credentials::CredentialError) -> EnrollmentError {
    match error {
        credentials::CredentialError::UserNotFound => EnrollmentError::UserNotFound,
        credentials::CredentialError::SecretUnavailable => EnrollmentError::SecretUnavailable,
        credentials::CredentialError::InvalidType | credentials::CredentialError::InvalidValue => {
            EnrollmentError::InvalidType
        }
        _ => EnrollmentError::Unavailable,
    }
}

fn map_matrix_error(error: MatrixAdapterError) -> EnrollmentError {
    match error {
        MatrixAdapterError::Timeout => EnrollmentError::Timeout,
        MatrixAdapterError::Unreachable => EnrollmentError::Unreachable,
        MatrixAdapterError::AuthFailed => EnrollmentError::AuthFailed,
        MatrixAdapterError::BadResponse => EnrollmentError::BadResponse,
        MatrixAdapterError::InvalidTarget | MatrixAdapterError::InvalidArgument => {
            EnrollmentError::InvalidArgument
        }
        MatrixAdapterError::ApiError { code } => EnrollmentError::ApiError { code },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_code_16_means_the_reader_is_busy() {
        assert!(reader_is_busy(&MatrixAdapterError::ApiError {
            code: DEVICE_BUSY
        }));
        assert!(!reader_is_busy(&MatrixAdapterError::ApiError { code: 15 }));
        assert!(!reader_is_busy(&MatrixAdapterError::Timeout));
    }
}
