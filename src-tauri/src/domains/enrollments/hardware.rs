#![allow(clippy::too_many_arguments)]
//! Start enrollment on a COSEC device after a documented capability check.
//!
//! Order: device configuration, create the user on the device, `enrolluser`,
//! confirm the count increased, then store the user–credential–device link.
//! Template bytes are not downloaded. Card numbers are encrypted at rest.
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
/// A card reader leaves enrollment if another request arrives before the tap.
/// This pause is the time the person has to hold the card on the reader.
const CARD_PRESENT_QUIET_SECS: u64 = 60;

use crate::common::{DevicePasswordVault, SecretError, SecretVault};
use crate::database::models::{EnrollmentSessionWriteRecord, EnrollmentWriteRecord};
use crate::database::repositories::{
    CredentialRepository, DeviceRepository, DeviceUserRepository, EnrollmentRepository,
    EnrollmentSessionRepository, FinishCapture, UserDeviceRepository, UserRepository,
};
use crate::database::{DatabaseError, DbPool};
use crate::domains::credentials::{
    self, card_identifier_digest, hardware_credential_write, normalize_card_identifier,
    CredentialType, HardwareCredentialDraft,
};
use crate::domains::device_users::{self, DeviceUserError};
use crate::domains::devices::DeviceStatus;
use crate::domains::synchronization::matrix_display_name;
use crate::domains::users::UserStatus;
use crate::matrix::{
    capture_increased, configured_reader, diagnose_card_read, parse_card_key_flags,
    parse_smart_card_format, CardReadAttempt, HardwareEnrollType, MatrixAdapter,
    MatrixAdapterError, ParsedCardCredential, SetUserParams,
};

use super::gate::DeviceEnrollmentGuard;
use super::session::{
    should_persist, terminal_after_capture, to_view, CaptureVerdict, CardRead, EnrollmentSession,
    EnrollmentSessionStatus, PersistVerdict,
};
use super::{
    get_enrollment, DeviceEnrollmentOptions, Enrollment, EnrollmentError, EnrollmentOption,
    EnrollmentStatus,
};

pub struct PreparedEnrollment {
    pub session: EnrollmentSession,
    pub guard: DeviceEnrollmentGuard,
}

pub async fn device_enrollment_options(
    devices: &DeviceRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    device_id: Uuid,
) -> Result<DeviceEnrollmentOptions, EnrollmentError> {
    let device = load_active_device(devices, device_id).await?;
    let password = decrypt_password(vault, &device.password_ciphertext)?;
    let port = device_port(device.port)?;
    let reported = matrix
        .enrollment_capabilities(&device.host, port, &device.username, &password)
        .await
        .map_err(map_matrix_error)?;
    drop(password);
    record_online(devices, device_id).await;

    let mut assignable_credential_types = Vec::new();
    for enroll_type in &reported.types {
        for credential_type in enroll_type.assignable_credential_types() {
            if !assignable_credential_types
                .iter()
                .any(|existing: &String| existing == credential_type)
            {
                assignable_credential_types.push((*credential_type).to_string());
            }
        }
    }
    Ok(DeviceEnrollmentOptions {
        device_id,
        reader_label: reported.reader_label,
        assignable_credential_types,
        options: reported
            .types
            .into_iter()
            .map(|enroll_type| EnrollmentOption {
                enroll_type: enroll_type.as_str().to_string(),
                label: enroll_type.label().to_string(),
            })
            .collect(),
    })
}

pub async fn card_reader_status(
    devices: &DeviceRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    device_id: Uuid,
) -> Result<super::CardReaderStatus, EnrollmentError> {
    let device = load_active_device(devices, device_id).await?;
    let password = decrypt_password(vault, &device.password_ciphertext)?;
    let port = device_port(device.port)?;
    let docs = matrix
        .card_config_documents(&device.host, port, &device.username, &password)
        .await
        .map_err(map_matrix_error)?;
    drop(password);
    record_online(devices, device_id).await;
    let reader = configured_reader(&docs.reader);
    let flags = parse_card_key_flags(&docs.basic);
    let format = docs
        .smart_card_format
        .as_deref()
        .map(parse_smart_card_format)
        .unwrap_or_default();
    if let Some(reader) = reader {
        tracing::info!(
            device_id = %device_id,
            reader = reader.label,
            reader_code = reader.code,
            "reader_config_loaded"
        );
    }
    if docs.smart_card_format.is_some() {
        tracing::info!(
            device_id = %device_id,
            card_type = format.card_type.map(|value| value.as_str()),
            identifier = format.identifier.map(|value| value.as_str()),
            "smart_card_config_loaded"
        );
    }
    tracing::info!(
        device_id = %device_id,
        mifare_custom_key_enabled = flags.mifare_custom_key_enabled,
        hid_iclass_custom_key_enabled = flags.hid_iclass_custom_key_enabled,
        card_custom_key_auto_update = flags.card_custom_key_auto_update,
        "card key flags read"
    );
    let family = reader.and_then(|value| value.family);
    let supported = family.is_some();
    let mut message = match reader {
        None => "No card reader is configured on this device.".to_string(),
        Some(value) if value.family.is_some_and(|family| family.is_proximity()) => format!(
            "This is a {}. It reads a proximity card when enrollment starts. Click Enroll on device, then hold the card flat on the reader on the door.",
            value.label
        ),
        Some(_) if supported => "Ready to test card.".to_string(),
        Some(_) => {
            "This reader is configured, but card compatibility is not documented for it."
                .to_string()
        }
    };
    if flags.mifare_custom_key_enabled || flags.hid_iclass_custom_key_enabled {
        message.push_str(" A custom card key is enabled. Reading may depend on that key.");
    }
    Ok(super::CardReaderStatus {
        device_id,
        reader: reader.and_then(|value| value.family.map(|family| family.as_str().to_string())),
        reader_label: reader.map(|value| value.label.to_string()),
        reader_code: reader.map(|value| value.code),
        supported,
        card_type: format.card_type.map(|value| value.as_str().to_string()),
        card_type_label: format.card_type.map(|value| value.label().to_string()),
        identifier: format.identifier.map(|value| value.as_str().to_string()),
        identifier_label: format.identifier.map(|value| value.label().to_string()),
        mifare_custom_key_enabled: flags.mifare_custom_key_enabled,
        hid_iclass_custom_key_enabled: flags.hid_iclass_custom_key_enabled,
        card_custom_key_auto_update: flags.card_custom_key_auto_update,
        message,
    })
}

pub async fn test_card(
    devices: &DeviceRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    device_id: Uuid,
) -> Result<super::CardTestResult, EnrollmentError> {
    let device = load_active_device(devices, device_id).await?;
    let password = decrypt_password(vault, &device.password_ciphertext)?;
    let port = device_port(device.port)?;
    let docs = matrix
        .card_config_documents(&device.host, port, &device.username, &password)
        .await
        .map_err(map_matrix_error)?;
    let reader = configured_reader(&docs.reader);
    tracing::info!(device_id = %device_id, "card_test_started");
    let family = reader.and_then(|value| value.family);
    let attempt = if family.is_none() {
        CardReadAttempt::NotAttempted
    } else if family.is_some_and(|value| value.is_proximity()) {
        tracing::info!(device_id = %device_id, "proximity reader skips smart-card read");
        CardReadAttempt::ProximityReader
    } else {
        match matrix
            .read_card(&device.host, port, &device.username, &password)
            .await
        {
            Ok(parsed) => CardReadAttempt::Read(parsed),
            Err(error) => attempt_from_adapter(error),
        }
    };
    drop(password);
    record_online(devices, device_id).await;
    let report = diagnose_card_read(&docs.reader, attempt);
    match report.outcome {
        "success" => tracing::info!(
            device_id = %device_id,
            card_type = report.card_type.as_deref(),
            card_number = report.card_number.as_deref().map(mask_card_number),
            "card_detected"
        ),
        "timeout" => tracing::info!(device_id = %device_id, "card_test_timeout"),
        "wrong_card_type" | "incompatible" => {
            tracing::info!(device_id = %device_id, outcome = report.outcome, "card_wrong_type")
        }
        "key_mismatch" => tracing::info!(device_id = %device_id, "card_key_mismatch"),
        "read_failed" | "parameters_not_applicable" => {
            tracing::info!(device_id = %device_id, outcome = report.outcome, "card_read_failed")
        }
        _ => tracing::info!(device_id = %device_id, outcome = report.outcome, "card_read_failed"),
    }
    Ok(super::CardTestResult {
        device_id,
        outcome: report.outcome.to_string(),
        message: report.message,
        compatible: report.compatible,
        reader: report.reader,
        reader_label: report.reader_label,
        card_type: report.card_type,
        card_type_label: report.card_type_label,
        card_number: report.card_number,
    })
}

fn attempt_from_adapter(error: MatrixAdapterError) -> CardReadAttempt {
    match error {
        MatrixAdapterError::Timeout => CardReadAttempt::TimedOut,
        MatrixAdapterError::Unreachable => CardReadAttempt::Unreachable,
        MatrixAdapterError::AuthFailed => CardReadAttempt::AuthFailed,
        MatrixAdapterError::ApiError { code } => CardReadAttempt::DeviceCode(code),
        _ => CardReadAttempt::BadResponse,
    }
}

fn mask_card_number(number: &str) -> String {
    let chars: Vec<char> = number.chars().collect();
    if chars.len() < 8 {
        return "****".to_string();
    }
    let head: String = chars.iter().take(4).collect();
    let tail: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{head}****{tail}")
}

pub async fn read_card(
    devices: &DeviceRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    device_id: Uuid,
) -> Result<CardRead, EnrollmentError> {
    let device = load_active_device(devices, device_id).await?;
    let password = decrypt_password(vault, &device.password_ciphertext)?;
    let port = device_port(device.port)?;
    let parsed = matrix
        .read_card(&device.host, port, &device.username, &password)
        .await
        .map_err(map_matrix_error)?;
    drop(password);
    record_online(devices, device_id).await;
    Ok(CardRead {
        device_id,
        card_type: parsed.card_type.map(|value| value.as_str().to_string()),
        card_type_label: parsed.card_type.map(|value| value.label().to_string()),
        card_number: parsed.card_number,
    })
}

/// Validate, lock the device, and insert a `starting` session.
///
/// The guard must be held until `execute_device_enrollment` finishes. There is
/// no documented Matrix cancel, so dropping the guard only releases our lock.
pub async fn prepare_device_enrollment(
    sessions: &EnrollmentSessionRepository,
    users: &UserRepository,
    devices: &DeviceRepository,
    assignments: &UserDeviceRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    gate: &super::DeviceEnrollmentGate,
    device_id: Uuid,
    user_id: Uuid,
    enroll_type: &str,
) -> Result<PreparedEnrollment, EnrollmentError> {
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
    if !supported.types.contains(&enroll_type) {
        record_online(devices, device_id).await;
        return Err(EnrollmentError::Unsupported);
    }
    drop(password);

    let session_id = Uuid::new_v4();
    let guard = DeviceEnrollmentGuard::acquire(gate, device_id, session_id)?;
    let now = Utc::now();
    let record = EnrollmentSessionWriteRecord {
        id: session_id,
        user_id,
        device_id,
        enroll_type: enroll_type.as_str().to_string(),
        status: EnrollmentSessionStatus::Starting.as_str().to_string(),
        started_at: now,
        created_at: now,
        updated_at: now,
    };
    if let Err(error) = sessions.insert(&record).await {
        return Err(map_db_error(error));
    }
    let stored = sessions
        .find_by_id(session_id)
        .await
        .map_err(map_db_error)?
        .ok_or(EnrollmentError::Unavailable)?;
    tracing::info!(
        enrollment_id = %session_id,
        user_id = %user_id,
        device_id = %device_id,
        credential_type = enroll_type.as_str(),
        "enrollment_started"
    );
    Ok(PreparedEnrollment {
        session: to_view(&stored),
        guard,
    })
}

pub async fn enroll_on_device(
    sessions: &EnrollmentSessionRepository,
    enrollments: &EnrollmentRepository,
    users: &UserRepository,
    _credentials: &CredentialRepository,
    devices: &DeviceRepository,
    assignments: &UserDeviceRepository,
    _device_users_repo: &DeviceUserRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    gate: &super::DeviceEnrollmentGate,
    pool: DbPool,
    device_id: Uuid,
    user_id: Uuid,
    enroll_type: &str,
) -> Result<Enrollment, EnrollmentError> {
    let prepared = prepare_device_enrollment(
        sessions,
        users,
        devices,
        assignments,
        vault,
        matrix,
        gate,
        device_id,
        user_id,
        enroll_type,
    )
    .await?;
    let enrollment_id =
        execute_device_enrollment(pool, prepared.guard, prepared.session.id).await?;
    get_enrollment(enrollments, enrollment_id).await
}

/// Run the physical enrollment. The guard releases the device lock on drop.
pub async fn execute_device_enrollment(
    pool: DbPool,
    guard: DeviceEnrollmentGuard,
    session_id: Uuid,
) -> Result<Uuid, EnrollmentError> {
    let _guard = guard;
    let sessions = EnrollmentSessionRepository::new(pool.clone());
    let users = UserRepository::new(pool.clone());
    let devices = DeviceRepository::new(pool.clone());
    let device_users_repo = DeviceUserRepository::new(pool.clone());

    let session = sessions
        .find_by_id(session_id)
        .await
        .map_err(map_db_error)?
        .ok_or(EnrollmentError::SessionNotFound)?;
    let enroll_type =
        HardwareEnrollType::parse(&session.enroll_type).ok_or(EnrollmentError::InvalidType)?;
    let user_id = session.user_id;
    let device_id = session.device_id;

    let vault = match SecretVault::open() {
        Ok(vault) => vault,
        Err(error) => {
            close_session(
                &sessions,
                session_id,
                EnrollmentSessionStatus::Failed,
                &map_secret_error(error).to_string(),
                user_id,
                device_id,
                enroll_type.as_str(),
            )
            .await;
            return Err(EnrollmentError::SecretUnavailable);
        }
    };
    let matrix = match MatrixAdapter::new() {
        Ok(matrix) => matrix,
        Err(_) => {
            close_session(
                &sessions,
                session_id,
                EnrollmentSessionStatus::Failed,
                "DEVICE_OFFLINE",
                user_id,
                device_id,
                enroll_type.as_str(),
            )
            .await;
            return Err(EnrollmentError::Offline);
        }
    };

    let outcome = run_capture(
        &sessions,
        &users,
        &devices,
        &device_users_repo,
        &vault,
        &matrix,
        session_id,
        user_id,
        device_id,
        enroll_type,
    )
    .await;

    match outcome {
        Ok(saved) => {
            persist_capture(
                &sessions,
                &vault,
                session_id,
                user_id,
                device_id,
                enroll_type,
                saved,
            )
            .await
        }
        Err(EnrollmentError::Cancelled) => Err(EnrollmentError::Cancelled),
        Err(error) => {
            let capture = match error {
                EnrollmentError::Timeout => CaptureVerdict::TimedOut,
                EnrollmentError::NotCaptured => CaptureVerdict::Unchanged,
                _ => CaptureVerdict::Failed,
            };
            let status = terminal_after_capture(false, capture, None);
            close_session(
                &sessions,
                session_id,
                status,
                &error.to_string(),
                user_id,
                device_id,
                enroll_type.as_str(),
            )
            .await;
            Err(error)
        }
    }
}

struct SavedCapture {
    card_number: Option<String>,
    card_type: Option<String>,
    identifier_type: Option<String>,
    identifier_unavailable: bool,
}

async fn run_capture(
    sessions: &EnrollmentSessionRepository,
    users: &UserRepository,
    devices: &DeviceRepository,
    device_users_repo: &DeviceUserRepository,
    vault: &SecretVault,
    matrix: &MatrixAdapter,
    session_id: Uuid,
    user_id: Uuid,
    device_id: Uuid,
    enroll_type: HardwareEnrollType,
) -> Result<SavedCapture, EnrollmentError> {
    let user = users
        .find_by_id(user_id)
        .await
        .map_err(map_db_error)?
        .ok_or(EnrollmentError::UserNotFound)?;
    let device = load_active_device(devices, device_id).await?;
    let password = decrypt_password(vault, &device.password_ciphertext)?;
    let port = device_port(device.port)?;

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

    if stores_card(enroll_type) {
        retry_if_reader_busy(|| {
            matrix.enable_card_and_face_access(&device.host, port, &device.username, &password)
        })
        .await?;
    }

    let before = matrix
        .credential_counts(
            &device.host,
            port,
            &device.username,
            &password,
            &mapping.matrix_user_id,
        )
        .await
        .map_err(map_matrix_error)?;
    let before_cards = if stores_card(enroll_type) {
        matrix
            .get_card_credential(
                &device.host,
                port,
                &device.username,
                &password,
                &mapping.matrix_user_id,
            )
            .await
            .map(|card| card.card_numbers)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if reader_in_menu(
        matrix,
        &device.host,
        port,
        &device.username,
        &password,
        &mapping.matrix_user_id,
    )
    .await
    {
        quiet_for_reader();
    }
    if !advance(
        sessions,
        session_id,
        EnrollmentSessionStatus::WaitingForCard,
    )
    .await?
    {
        return Err(EnrollmentError::Cancelled);
    }
    tracing::info!(
        enrollment_id = %session_id,
        user_id = %user_id,
        device_id = %device_id,
        credential_type = enroll_type.as_str(),
        "enrollment_waiting"
    );

    capture_on_reader(
        matrix,
        &device.host,
        port,
        &device.username,
        &password,
        &mapping.matrix_user_id,
        enroll_type,
        device_id,
        user_id,
        before,
        &before_cards,
    )
    .await?;

    if !advance(sessions, session_id, EnrollmentSessionStatus::Verifying).await? {
        return Err(EnrollmentError::Cancelled);
    }

    let mut parsed = ParsedCardCredential::default();
    let mut identifier_unavailable = false;
    if stores_card(enroll_type) {
        match matrix
            .get_card_credential(
                &device.host,
                port,
                &device.username,
                &password,
                &mapping.matrix_user_id,
            )
            .await
        {
            Ok(card) => {
                identifier_unavailable = card.card_number.is_none();
                parsed = card;
            }
            Err(error) => {
                tracing::warn!(
                    enrollment_id = %session_id,
                    device_id = %device_id,
                    error = %map_matrix_error(error),
                    "card identifier was not returned after enrollment"
                );
                identifier_unavailable = true;
            }
        }
    }

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
    record_online(devices, device_id).await;

    let card_number = parsed
        .card_number
        .as_deref()
        .and_then(|value| normalize_card_identifier(value).ok());
    if stores_card(enroll_type) && card_number.is_none() {
        identifier_unavailable = true;
    }
    Ok(SavedCapture {
        card_number,
        card_type: parsed.card_type.map(|value| value.as_str().to_string()),
        identifier_type: parsed
            .identifier_type
            .map(|value| value.as_str().to_string()),
        identifier_unavailable,
    })
}

async fn persist_capture(
    sessions: &EnrollmentSessionRepository,
    vault: &SecretVault,
    session_id: Uuid,
    user_id: Uuid,
    device_id: Uuid,
    enroll_type: HardwareEnrollType,
    saved: SavedCapture,
) -> Result<Uuid, EnrollmentError> {
    let still_open = advance(sessions, session_id, EnrollmentSessionStatus::Saving).await?;
    if !should_persist(!still_open, true) {
        return Err(EnrollmentError::Cancelled);
    }
    let credential_type = CredentialType::parse(enroll_type.credential_type())
        .map_err(|_| EnrollmentError::InvalidType)?;
    let reveal = saved.card_number.is_some();
    let secret = saved
        .card_number
        .clone()
        .unwrap_or_else(|| format!("enrolled:{session_id}"));
    let digest = saved
        .card_number
        .as_deref()
        .map(|number| card_identifier_digest(saved.identifier_type.as_deref(), number));
    let credential = hardware_credential_write(
        vault,
        user_id,
        HardwareCredentialDraft {
            credential_type,
            secret,
            card_type: saved.card_type.clone(),
            identifier_type: saved.identifier_type.clone(),
            reveal_hint: reveal,
        },
    )
    .map_err(map_credential_error)?;
    let now = Utc::now();
    let enrollment = EnrollmentWriteRecord {
        id: Uuid::new_v4(),
        user_id,
        credential_id: credential.id,
        device_id,
        status: EnrollmentStatus::Active.as_str().to_string(),
        cancelled_at: None,
        revoked_at: None,
        activated_at: Some(now),
        identifier_digest: digest,
        created_at: now,
        updated_at: now,
    };
    match sessions
        .finish_capture(
            session_id,
            &credential,
            &enrollment,
            saved.card_type.as_deref(),
            saved.identifier_type.as_deref(),
            saved.identifier_unavailable,
            now,
        )
        .await
    {
        Ok(FinishCapture::Saved(id)) => {
            debug_assert_eq!(
                terminal_after_capture(
                    false,
                    CaptureVerdict::Increased,
                    Some(PersistVerdict::Saved)
                ),
                EnrollmentSessionStatus::Success
            );
            tracing::info!(
                enrollment_id = %session_id,
                assignment_id = %id,
                user_id = %user_id,
                device_id = %device_id,
                credential_type = enroll_type.as_str(),
                "credential_saved"
            );
            tracing::info!(
                enrollment_id = %session_id,
                user_id = %user_id,
                device_id = %device_id,
                credential_type = enroll_type.as_str(),
                result = "success",
                "enrollment_completed"
            );
            Ok(id)
        }
        Ok(FinishCapture::Skipped) => Err(EnrollmentError::Cancelled),
        Err(error) if error.is_unique_violation() => {
            let status = terminal_after_capture(
                false,
                CaptureVerdict::Increased,
                Some(PersistVerdict::Duplicate),
            );
            close_session(
                sessions,
                session_id,
                status,
                "ENROLLMENT_DUPLICATE",
                user_id,
                device_id,
                enroll_type.as_str(),
            )
            .await;
            Err(EnrollmentError::Duplicate)
        }
        Err(error) => {
            tracing::error!(
                error = %error,
                enrollment_id = %session_id,
                user_id = %user_id,
                device_id = %device_id,
                credential_type = enroll_type.as_str(),
                "device enrollment needs reconciliation"
            );
            let status = terminal_after_capture(
                false,
                CaptureVerdict::Increased,
                Some(PersistVerdict::Failed),
            );
            close_session(
                sessions,
                session_id,
                status,
                "ENROLLMENT_PERSISTENCE_FAILED",
                user_id,
                device_id,
                enroll_type.as_str(),
            )
            .await;
            Err(EnrollmentError::PersistenceFailed)
        }
    }
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

fn stores_card(enroll_type: HardwareEnrollType) -> bool {
    matches!(
        enroll_type,
        HardwareEnrollType::ReadOnlyCard
            | HardwareEnrollType::SmartCard
            | HardwareEnrollType::BiometricThenCard
    )
}

fn reader_is_busy(error: &MatrixAdapterError) -> bool {
    matches!(error, MatrixAdapterError::ApiError { code: DEVICE_BUSY })
}

fn quiet_for_reader() {
    quiet_for(CAPTURE_QUIET_SECS);
}

fn quiet_for(seconds: u64) {
    std::thread::sleep(std::time::Duration::from_secs(seconds));
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
    before: crate::matrix::CredentialCounts,
    before_cards: &[String],
) -> Result<(), EnrollmentError> {
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
            // enrolluser often returns as soon as the reader opens. Another
            // request before the tap closes that screen, so a card held on
            // the reader is ignored.
            if stores_card(enroll_type) {
                quiet_for(CARD_PRESENT_QUIET_SECS);
            } else {
                quiet_for_reader();
            }
            confirm_capture(
                matrix,
                host,
                port,
                username,
                password,
                matrix_user_id,
                enroll_type,
                before,
                before_cards,
            )
            .await
        }
        Err(error) if reader_is_busy(&error) => {
            quiet_for_reader();
            if confirm_capture(
                matrix,
                host,
                port,
                username,
                password,
                matrix_user_id,
                enroll_type,
                before,
                before_cards,
            )
            .await
            .is_ok()
            {
                return Ok(());
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
                before,
                before_cards,
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
    before: crate::matrix::CredentialCounts,
    before_cards: &[String],
) -> Result<(), EnrollmentError> {
    if stores_card(enroll_type) {
        if let Ok(card) = matrix
            .get_card_credential(host, port, username, password, matrix_user_id)
            .await
        {
            let stored = card
                .card_numbers
                .iter()
                .any(|number| !before_cards.iter().any(|existing| existing == number));
            if stored {
                return Ok(());
            }
        }
    }
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
                if capture_increased(enroll_type, before, after) {
                    return Ok(());
                }
                // A credential that was already on the user is not this capture.
                let _already_present = crate::matrix::credential_present(enroll_type, after);
                last_error = None;
            }
            Err(error) => last_error = Some(error),
        }
    }
    if let Some(error) = last_error {
        if reader_is_busy(&error) {
            return Err(EnrollmentError::NotCaptured);
        }
        return Err(map_matrix_error(error));
    }
    Err(EnrollmentError::NotCaptured)
}

async fn advance(
    sessions: &EnrollmentSessionRepository,
    session_id: Uuid,
    status: EnrollmentSessionStatus,
) -> Result<bool, EnrollmentError> {
    let now = Utc::now();
    sessions
        .update_if_open(
            session_id,
            status.as_str(),
            None,
            None,
            None,
            false,
            None,
            None,
            now,
        )
        .await
        .map_err(map_db_error)
}

async fn close_session(
    sessions: &EnrollmentSessionRepository,
    session_id: Uuid,
    status: EnrollmentSessionStatus,
    error_code: &str,
    user_id: Uuid,
    device_id: Uuid,
    enroll_type: &str,
) {
    let now = Utc::now();
    if let Err(error) = sessions
        .update_if_open(
            session_id,
            status.as_str(),
            Some(error_code),
            None,
            None,
            false,
            None,
            Some(now),
            now,
        )
        .await
    {
        tracing::error!(
            error = %error,
            enrollment_id = %session_id,
            "could not record enrollment session"
        );
    }
    match status {
        EnrollmentSessionStatus::Timeout => tracing::info!(
            enrollment_id = %session_id,
            user_id = %user_id,
            device_id = %device_id,
            credential_type = enroll_type,
            result = error_code,
            "enrollment_timeout"
        ),
        EnrollmentSessionStatus::Cancelled => tracing::info!(
            enrollment_id = %session_id,
            user_id = %user_id,
            device_id = %device_id,
            credential_type = enroll_type,
            result = error_code,
            "enrollment_cancelled"
        ),
        EnrollmentSessionStatus::PersistenceFailed => tracing::info!(
            enrollment_id = %session_id,
            user_id = %user_id,
            device_id = %device_id,
            credential_type = enroll_type,
            result = error_code,
            "enrollment_failed"
        ),
        _ => tracing::info!(
            enrollment_id = %session_id,
            user_id = %user_id,
            device_id = %device_id,
            credential_type = enroll_type,
            result = error_code,
            "enrollment_failed"
        ),
    }
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

    #[test]
    fn card_type_label_round_trip() {
        use crate::matrix::{MatrixCardType, MatrixIdentifierType};
        let parsed = MatrixCardType::parse("mifare_4k").unwrap();
        assert_eq!(parsed, MatrixCardType::Mifare4K);
        assert_eq!(
            MatrixIdentifierType::parse("custom"),
            Some(MatrixIdentifierType::Custom)
        );
    }
}
