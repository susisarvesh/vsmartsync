//! Tauri commands exposed to the React frontend.
//!
//! Keep this surface small. Never return Matrix credentials, database
//! connection secrets, device passwords, or ciphertext to the UI.

use crate::common::{app_info, AppInfo, DevicePasswordVault, SecretVault};
use crate::database::repositories::{
    CredentialRepository, DeviceRepository, EnrollmentRepository, UserRepository,
};
use crate::database::{DatabaseRuntime, DatabaseStatus};
use crate::domains::credentials::{self, Credential, CredentialError, CredentialListFilter};
use crate::domains::devices::{self, Device, DeviceError};
use crate::domains::enrollments::{self, Enrollment, EnrollmentError, EnrollmentListFilter};
use crate::domains::users::{self, User, UserError};
use crate::matrix::MatrixAdapter;
use uuid::Uuid;

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    tracing::info!(command = "get_app_info", "frontend invoked rust");
    app_info()
}

#[tauri::command]
pub async fn get_database_status(
    runtime: tauri::State<'_, DatabaseRuntime>,
) -> Result<DatabaseStatus, String> {
    tracing::info!(command = "get_database_status", "frontend invoked rust");
    let runtime = DatabaseRuntime::clone(&runtime);
    Ok(runtime.status().await)
}

#[tauri::command]
pub async fn connect_database(
    runtime: tauri::State<'_, DatabaseRuntime>,
) -> Result<DatabaseStatus, String> {
    tracing::info!(command = "connect_database", "frontend invoked rust");
    let runtime = DatabaseRuntime::clone(&runtime);
    Ok(runtime.connect().await)
}

#[tauri::command]
pub async fn create_user(
    runtime: tauri::State<'_, DatabaseRuntime>,
    name: String,
) -> Result<User, String> {
    tracing::info!(command = "create_user", "frontend invoked rust");
    let repo = users_repo(&runtime)?;
    users::create_user(&repo, &name)
        .await
        .map_err(user_error_to_command)
}

#[tauri::command]
pub async fn list_users(runtime: tauri::State<'_, DatabaseRuntime>) -> Result<Vec<User>, String> {
    tracing::info!(command = "list_users", "frontend invoked rust");
    let repo = users_repo(&runtime)?;
    users::list_users(&repo)
        .await
        .map_err(user_error_to_command)
}

#[tauri::command]
pub async fn update_user(
    runtime: tauri::State<'_, DatabaseRuntime>,
    id: Uuid,
    name: String,
) -> Result<User, String> {
    tracing::info!(command = "update_user", user_id = %id, "frontend invoked rust");
    let repo = users_repo(&runtime)?;
    users::update_user_name(&repo, id, &name)
        .await
        .map_err(user_error_to_command)
}

#[tauri::command]
pub async fn deactivate_user(
    runtime: tauri::State<'_, DatabaseRuntime>,
    id: Uuid,
) -> Result<User, String> {
    tracing::info!(command = "deactivate_user", user_id = %id, "frontend invoked rust");
    let repo = users_repo(&runtime)?;
    users::deactivate_user(&repo, id)
        .await
        .map_err(user_error_to_command)
}

#[tauri::command]
pub async fn create_device(
    runtime: tauri::State<'_, DatabaseRuntime>,
    name: String,
    host: String,
    port: Option<i32>,
    username: String,
    password: String,
) -> Result<Device, String> {
    tracing::info!(command = "create_device", "frontend invoked rust");
    let repo = devices_repo(&runtime)?;
    let vault = DevicePasswordVault::open().map_err(secret_error_to_command)?;
    devices::create_device(&repo, &vault, &name, &host, port, &username, &password)
        .await
        .map_err(device_error_to_command)
}

#[tauri::command]
pub async fn list_devices(
    runtime: tauri::State<'_, DatabaseRuntime>,
) -> Result<Vec<Device>, String> {
    tracing::info!(command = "list_devices", "frontend invoked rust");
    let repo = devices_repo(&runtime)?;
    devices::list_devices(&repo)
        .await
        .map_err(device_error_to_command)
}

#[tauri::command]
pub async fn update_device(
    runtime: tauri::State<'_, DatabaseRuntime>,
    id: Uuid,
    name: String,
    host: String,
    port: i32,
    username: String,
) -> Result<Device, String> {
    tracing::info!(command = "update_device", device_id = %id, "frontend invoked rust");
    let repo = devices_repo(&runtime)?;
    devices::update_device(&repo, id, &name, &host, port, &username)
        .await
        .map_err(device_error_to_command)
}

#[tauri::command]
pub async fn set_device_password(
    runtime: tauri::State<'_, DatabaseRuntime>,
    id: Uuid,
    password: String,
) -> Result<Device, String> {
    tracing::info!(command = "set_device_password", device_id = %id, "frontend invoked rust");
    let repo = devices_repo(&runtime)?;
    let vault = DevicePasswordVault::open().map_err(secret_error_to_command)?;
    devices::set_device_password(&repo, &vault, id, &password)
        .await
        .map_err(device_error_to_command)
}

#[tauri::command]
pub async fn test_device_connection(
    runtime: tauri::State<'_, DatabaseRuntime>,
    id: Uuid,
) -> Result<Device, String> {
    tracing::info!(
        command = "test_device_connection",
        device_id = %id,
        "frontend invoked rust"
    );
    let repo = devices_repo(&runtime)?;
    let vault = DevicePasswordVault::open().map_err(secret_error_to_command)?;
    let matrix = MatrixAdapter::new().map_err(|_| "DEVICE_OFFLINE".to_string())?;
    devices::test_device_connection(&repo, &vault, &matrix, id)
        .await
        .map_err(device_error_to_command)
}

#[tauri::command]
pub async fn create_credential(
    runtime: tauri::State<'_, DatabaseRuntime>,
    user_id: Uuid,
    credential_type: String,
    value: String,
) -> Result<Credential, String> {
    tracing::info!(
        command = "create_credential",
        user_id = %user_id,
        credential_type = %credential_type,
        "frontend invoked rust"
    );
    let credentials = credentials_repo(&runtime)?;
    let users = users_repo(&runtime)?;
    let vault = SecretVault::open().map_err(credential_secret_error)?;
    credentials::create_credential(
        &credentials,
        &users,
        &vault,
        user_id,
        &credential_type,
        &value,
    )
    .await
    .map_err(credential_error_to_command)
}

#[tauri::command]
pub async fn list_credentials(
    runtime: tauri::State<'_, DatabaseRuntime>,
    user_id: Option<Uuid>,
    credential_type: Option<String>,
    status: Option<String>,
) -> Result<Vec<Credential>, String> {
    tracing::info!(command = "list_credentials", "frontend invoked rust");
    let credentials = credentials_repo(&runtime)?;
    let filter = CredentialListFilter {
        user_id,
        credential_type: credential_type
            .as_deref()
            .map(credentials::CredentialType::parse)
            .transpose()
            .map_err(credential_error_to_command)?,
        status: status
            .as_deref()
            .map(credentials::CredentialStatus::parse)
            .transpose()
            .map_err(credential_error_to_command)?,
    };
    credentials::list_credentials(&credentials, filter)
        .await
        .map_err(credential_error_to_command)
}

#[tauri::command]
pub async fn get_credential(
    runtime: tauri::State<'_, DatabaseRuntime>,
    id: Uuid,
) -> Result<Credential, String> {
    tracing::info!(command = "get_credential", credential_id = %id, "frontend invoked rust");
    let credentials = credentials_repo(&runtime)?;
    credentials::get_credential(&credentials, id)
        .await
        .map_err(credential_error_to_command)
}

#[tauri::command]
pub async fn update_credential(
    runtime: tauri::State<'_, DatabaseRuntime>,
    id: Uuid,
    value: Option<String>,
) -> Result<Credential, String> {
    tracing::info!(command = "update_credential", credential_id = %id, "frontend invoked rust");
    let credentials = credentials_repo(&runtime)?;
    match value {
        Some(value) if !value.is_empty() => {
            let vault = SecretVault::open().map_err(credential_secret_error)?;
            credentials::update_credential_value(&credentials, &vault, id, &value)
                .await
                .map_err(credential_error_to_command)
        }
        Some(_) => Err(credential_error_to_command(CredentialError::InvalidValue)),
        None => credentials::get_credential(&credentials, id)
            .await
            .map_err(credential_error_to_command),
    }
}

#[tauri::command]
pub async fn set_credential_status(
    runtime: tauri::State<'_, DatabaseRuntime>,
    id: Uuid,
    status: String,
) -> Result<Credential, String> {
    tracing::info!(
        command = "set_credential_status",
        credential_id = %id,
        "frontend invoked rust"
    );
    let credentials = credentials_repo(&runtime)?;
    credentials::set_credential_status(&credentials, id, &status)
        .await
        .map_err(credential_error_to_command)
}

#[tauri::command]
pub async fn create_enrollment(
    runtime: tauri::State<'_, DatabaseRuntime>,
    user_id: Uuid,
    credential_id: Uuid,
    device_id: Uuid,
) -> Result<Enrollment, String> {
    tracing::info!(
        command = "create_enrollment",
        user_id = %user_id,
        credential_id = %credential_id,
        device_id = %device_id,
        "frontend invoked rust"
    );
    let enrollments = enrollments_repo(&runtime)?;
    let users = users_repo(&runtime)?;
    let credentials = credentials_repo(&runtime)?;
    let devices = devices_repo(&runtime)?;
    enrollments::create_enrollment(
        &enrollments,
        &users,
        &credentials,
        &devices,
        user_id,
        credential_id,
        device_id,
    )
    .await
    .map_err(enrollment_error_to_command)
}

#[tauri::command]
pub async fn list_enrollments(
    runtime: tauri::State<'_, DatabaseRuntime>,
    user_id: Option<Uuid>,
    device_id: Option<Uuid>,
    credential_id: Option<Uuid>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Enrollment>, String> {
    tracing::info!(command = "list_enrollments", "frontend invoked rust");
    let enrollments = enrollments_repo(&runtime)?;
    let filter = EnrollmentListFilter {
        user_id,
        device_id,
        credential_id,
        status: status
            .as_deref()
            .map(enrollments::EnrollmentStatus::parse)
            .transpose()
            .map_err(enrollment_error_to_command)?,
        limit,
        offset,
    };
    enrollments::list_enrollments(&enrollments, filter)
        .await
        .map_err(enrollment_error_to_command)
}

#[tauri::command]
pub async fn get_enrollment(
    runtime: tauri::State<'_, DatabaseRuntime>,
    id: Uuid,
) -> Result<Enrollment, String> {
    tracing::info!(command = "get_enrollment", enrollment_id = %id, "frontend invoked rust");
    let enrollments = enrollments_repo(&runtime)?;
    enrollments::get_enrollment(&enrollments, id)
        .await
        .map_err(enrollment_error_to_command)
}

#[tauri::command]
pub async fn cancel_enrollment(
    runtime: tauri::State<'_, DatabaseRuntime>,
    id: Uuid,
) -> Result<Enrollment, String> {
    tracing::info!(command = "cancel_enrollment", enrollment_id = %id, "frontend invoked rust");
    let enrollments = enrollments_repo(&runtime)?;
    enrollments::cancel_enrollment(&enrollments, id)
        .await
        .map_err(enrollment_error_to_command)
}

#[tauri::command]
pub async fn revoke_enrollment(
    runtime: tauri::State<'_, DatabaseRuntime>,
    id: Uuid,
) -> Result<Enrollment, String> {
    tracing::info!(command = "revoke_enrollment", enrollment_id = %id, "frontend invoked rust");
    let enrollments = enrollments_repo(&runtime)?;
    enrollments::revoke_enrollment(&enrollments, id)
        .await
        .map_err(enrollment_error_to_command)
}

#[tauri::command]
pub async fn retry_enrollment(
    runtime: tauri::State<'_, DatabaseRuntime>,
    id: Uuid,
) -> Result<Enrollment, String> {
    tracing::info!(command = "retry_enrollment", enrollment_id = %id, "frontend invoked rust");
    let enrollments = enrollments_repo(&runtime)?;
    enrollments::retry_enrollment(&enrollments, id)
        .await
        .map_err(enrollment_error_to_command)
}

fn users_repo(runtime: &DatabaseRuntime) -> Result<UserRepository, String> {
    let pool = runtime
        .pool()
        .ok_or_else(|| user_error_to_command(UserError::Unavailable))?;
    Ok(UserRepository::new(pool))
}

fn devices_repo(runtime: &DatabaseRuntime) -> Result<DeviceRepository, String> {
    let pool = runtime
        .pool()
        .ok_or_else(|| device_error_to_command(DeviceError::Unavailable))?;
    Ok(DeviceRepository::new(pool))
}

fn credentials_repo(runtime: &DatabaseRuntime) -> Result<CredentialRepository, String> {
    let pool = runtime
        .pool()
        .ok_or_else(|| credential_error_to_command(CredentialError::Unavailable))?;
    Ok(CredentialRepository::new(pool))
}

fn enrollments_repo(runtime: &DatabaseRuntime) -> Result<EnrollmentRepository, String> {
    let pool = runtime
        .pool()
        .ok_or_else(|| enrollment_error_to_command(EnrollmentError::Unavailable))?;
    Ok(EnrollmentRepository::new(pool))
}

fn user_error_to_command(error: UserError) -> String {
    error.to_string()
}

fn device_error_to_command(error: DeviceError) -> String {
    error.to_string()
}

fn credential_error_to_command(error: CredentialError) -> String {
    error.to_string()
}

fn enrollment_error_to_command(error: EnrollmentError) -> String {
    error.to_string()
}

fn secret_error_to_command(_: crate::common::SecretError) -> String {
    DeviceError::SecretUnavailable.to_string()
}

fn credential_secret_error(_: crate::common::SecretError) -> String {
    CredentialError::SecretUnavailable.to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        credential_error_to_command, device_error_to_command, enrollment_error_to_command,
        get_app_info, user_error_to_command,
    };
    use crate::common::AppInfo;
    use crate::domains::credentials::{
        Credential, CredentialError, CredentialStatus, CredentialType,
    };
    use crate::domains::devices::{ConnectionStatus, Device, DeviceError};
    use crate::domains::enrollments::EnrollmentError;
    use crate::domains::users::UserError;
    use chrono::Utc;
    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
    use tauri::webview::InvokeRequest;
    use tauri::WebviewWindowBuilder;
    use uuid::Uuid;

    #[test]
    fn user_errors_are_stable_codes_not_sql() {
        assert_eq!(
            user_error_to_command(UserError::InvalidName),
            "USER_INVALID_NAME"
        );
        assert_eq!(user_error_to_command(UserError::NotFound), "USER_NOT_FOUND");
        assert_eq!(
            user_error_to_command(UserError::Unavailable),
            "DATABASE_UNAVAILABLE"
        );
        assert!(!user_error_to_command(UserError::Unavailable).contains("postgres://"));
    }

    #[test]
    fn device_errors_are_stable_codes_without_secrets() {
        assert_eq!(
            device_error_to_command(DeviceError::InvalidHost),
            "DEVICE_INVALID_HOST"
        );
        assert_eq!(
            device_error_to_command(DeviceError::Duplicate),
            "DEVICE_DUPLICATE"
        );
        assert_eq!(
            device_error_to_command(DeviceError::AuthFailed),
            "DEVICE_AUTH_FAILED"
        );
        let message = device_error_to_command(DeviceError::SecretUnavailable);
        assert_eq!(message, "DEVICE_SECRET_UNAVAILABLE");
        assert!(!message.to_lowercase().contains("password"));
        assert!(!message.contains("ciphertext"));
    }

    #[test]
    fn credential_errors_are_stable_and_omit_secrets() {
        assert_eq!(
            credential_error_to_command(CredentialError::InvalidType),
            "CREDENTIAL_INVALID_TYPE"
        );
        assert_eq!(
            credential_error_to_command(CredentialError::UserNotFound),
            "USER_NOT_FOUND"
        );
        assert_eq!(
            credential_error_to_command(CredentialError::Duplicate),
            "CREDENTIAL_DUPLICATE"
        );
        let message = credential_error_to_command(CredentialError::SecretUnavailable);
        assert_eq!(message, "CREDENTIAL_SECRET_UNAVAILABLE");
        assert!(!message.to_lowercase().contains("pin"));
        assert!(!message.contains("ciphertext"));
    }

    #[test]
    fn enrollment_errors_are_stable_codes() {
        assert_eq!(
            enrollment_error_to_command(EnrollmentError::Duplicate),
            "ENROLLMENT_DUPLICATE"
        );
        assert_eq!(
            enrollment_error_to_command(EnrollmentError::InvalidTransition),
            "ENROLLMENT_INVALID_TRANSITION"
        );
        assert_eq!(
            enrollment_error_to_command(EnrollmentError::CredentialOwnership),
            "ENROLLMENT_CREDENTIAL_OWNERSHIP"
        );
        assert_eq!(
            enrollment_error_to_command(EnrollmentError::DeviceNotFound),
            "DEVICE_NOT_FOUND"
        );
    }

    #[test]
    fn credential_serde_never_includes_secret_fields() {
        let credential = Credential {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            user_name: "Ada".into(),
            credential_type: CredentialType::Pin,
            status: CredentialStatus::Active,
            masked_value: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&credential).expect("json");
        assert!(!json.contains("password"));
        assert!(!json.contains("ciphertext"));
        assert!(!json.contains("digest"));
        assert!(!json.contains("\"value\":"));
        assert!(json.contains("credentialType"));
        assert!(json.contains("maskedValue") || !json.contains("1234"));
    }

    #[test]
    fn device_serde_never_includes_password_fields() {
        let device = Device {
            id: Uuid::new_v4(),
            name: "Lobby".into(),
            host: "10.0.0.5".into(),
            port: 80,
            username: "admin".into(),
            connection_status: ConnectionStatus::Unknown,
            last_seen_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&device).expect("json");
        assert!(!json.contains("password"));
        assert!(!json.contains("ciphertext"));
        assert!(json.contains("connectionStatus"));
    }

    #[test]
    fn get_app_info_returns_foundation_metadata() {
        let info = get_app_info();
        assert_eq!(info.name, "Vsmart Sync");
        assert_eq!(info.version, "0.1.0");
        assert_eq!(info.stage, "foundation");
        assert!(!info.description.is_empty());
    }

    #[test]
    fn tauri_can_invoke_get_app_info() {
        let app = mock_builder()
            .invoke_handler(tauri::generate_handler![get_app_info])
            .build(mock_context(noop_assets()))
            .expect("failed to build test app");

        let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("failed to build test webview");

        let response = get_ipc_response(
            &webview,
            InvokeRequest {
                cmd: "get_app_info".into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: "tauri://localhost".parse().expect("webview url"),
                body: InvokeBody::default(),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .expect("invoke get_app_info");

        let info: AppInfo = response.deserialize().expect("deserialize app info");
        assert_eq!(info.name, "Vsmart Sync");
        assert_eq!(info.stage, "foundation");
    }

    #[test]
    fn database_status_ipc_omits_secrets() {
        use crate::database::{DatabaseRuntime, DatabaseStatus};

        let app = mock_builder()
            .manage(DatabaseRuntime::initialize())
            .invoke_handler(tauri::generate_handler![super::get_database_status])
            .build(mock_context(noop_assets()))
            .expect("failed to build test app");

        let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("failed to build test webview");

        let response = get_ipc_response(
            &webview,
            InvokeRequest {
                cmd: "get_database_status".into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: "tauri://localhost".parse().expect("webview url"),
                body: InvokeBody::default(),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .expect("invoke get_database_status");

        let status: DatabaseStatus = response.deserialize().expect("deserialize status");
        let json = serde_json::to_string(&status).expect("json");
        assert!(!json.contains("password"));
        assert!(!json.contains("postgres://"));
        assert!(!status.user.is_empty());
        assert!(status.port > 0);
    }
}
