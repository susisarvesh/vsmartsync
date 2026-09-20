//! Tauri commands exposed to the React frontend.
//!
//! Keep this surface small. Never return Matrix credentials, database
//! connection secrets, or other sensitive configuration to the UI.

use crate::common::{app_info, AppInfo};
use crate::database::{DatabaseRuntime, DatabaseStatus};

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

#[cfg(test)]
mod tests {
    use super::get_app_info;
    use crate::common::AppInfo;
    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
    use tauri::webview::InvokeRequest;
    use tauri::WebviewWindowBuilder;

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
