//! Matrixcosec application library.
//!
//! This crate is a modular monolith. Domain modules are placeholders until
//! their services are implemented. React talks to this crate only through
//! Tauri commands — never to PostgreSQL or Matrix devices.

mod audit;
mod auth;
mod commands;
mod common;
mod credentials;
pub mod database;
mod devices;
mod enrollments;
mod events;
mod licensing;
mod matrix;
mod synchronization;
mod users;

use database::DatabaseConfig;
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

fn load_env_files() {
    // `tauri dev` typically uses src-tauri as the working directory.
    let _ = dotenvy::from_filename("../.env");
    let _ = dotenvy::dotenv();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    load_env_files();

    tracing::info!(stage = "foundation", "starting matrixcosec");

    tauri::Builder::default()
        .setup(|app| {
            let config = DatabaseConfig::from_env().map_err(|error| {
                tracing::error!(error = %error, "database configuration is invalid");
                error
            })?;

            tracing::info!("connecting to postgresql and running migrations");
            let pool = tauri::async_runtime::block_on(database::connect_and_migrate(&config))
                .map_err(|error| {
                    tracing::error!(error = %error, "failed to initialize postgresql");
                    error
                })?;

            app.manage(pool);
            tracing::info!("postgresql pool is ready");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
