//! Vsmart Sync application library.
//!
//! This crate is a modular monolith. React talks to this crate only through
//! Tauri commands — never to PostgreSQL or Matrix devices.

mod commands;
mod common;
pub mod database;
pub mod domains;
mod matrix;

pub use common::{DevicePasswordVault, SecretError, SecretVault};

use crate::common::load_env_files;
use tauri::Manager;

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,sqlx=warn"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .json()
        .try_init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    load_env_files();

    tracing::info!(stage = "foundation", "starting vsmart sync");

    tauri::Builder::default()
        .setup(|app| {
            app.manage(database::DatabaseRuntime::initialize());

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Regular);

            bring_main_window_forward(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_database_status,
            commands::connect_database,
            commands::create_user,
            commands::list_users,
            commands::update_user,
            commands::deactivate_user,
            commands::create_device,
            commands::list_devices,
            commands::update_device,
            commands::set_device_password,
            commands::test_device_connection,
            commands::create_credential,
            commands::list_credentials,
            commands::get_credential,
            commands::update_credential,
            commands::set_credential_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Show and focus the native window on macOS, Windows, and Linux.
///
/// IDE terminals often start the process without activating the GUI. macOS also
/// needs a regular activation policy so the app appears in the Dock.
fn bring_main_window_forward(app: &tauri::App) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.show() {
            tracing::warn!(error = %error, "failed to show main window");
        }
        if let Err(error) = window.unminimize() {
            tracing::warn!(error = %error, "failed to unminimize main window");
        }
        if let Err(error) = window.set_focus() {
            tracing::warn!(error = %error, "failed to focus main window");
        }
    } else {
        tracing::error!("main desktop window was not created");
    }
}
