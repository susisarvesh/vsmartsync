//! Tauri commands exposed to the React frontend.
//!
//! Keep this surface small. Never return Matrix credentials, database
//! connection secrets, device passwords, or ciphertext to the UI.

use crate::common::{app_info, AppInfo, DevicePasswordVault};
use crate::database::repositories::{DeviceRepository, UserRepository};
use crate::database::{DatabaseRuntime, DatabaseStatus};
use crate::domains::devices::{self, Device, DeviceError};
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

fn user_error_to_command(error: UserError) -> String {
    error.to_string()
}

fn device_error_to_command(error: DeviceError) -> String {
    error.to_string()
}

fn secret_error_to_command(_: crate::common::SecretError) -> String {
    DeviceError::SecretUnavailable.to_string()
}

#[cfg(test)]
mod tests {
    use super::{device_error_to_command, get_app_info, user_error_to_command};
    use crate::common::AppInfo;
    use crate::domains::devices::{ConnectionStatus, Device, DeviceError};
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
