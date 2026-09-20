//! Runtime PostgreSQL connection for the desktop app.
//!
//! The window must start even when PostgreSQL is missing. Secrets never go to
//! the frontend — only host, port, database name, user, and a safe message.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::{connect_and_migrate, ping, DatabaseConfig, DatabaseError, DbPool};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatus {
    pub connected: bool,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub platform: String,
    pub message: String,
}

struct DatabaseRuntimeInner {
    config: Result<DatabaseConfig, String>,
    pool: Mutex<Option<DbPool>>,
}

#[derive(Clone)]
pub struct DatabaseRuntime {
    inner: Arc<DatabaseRuntimeInner>,
}

impl DatabaseRuntime {
    pub fn initialize() -> Self {
        let config = DatabaseConfig::from_env().map_err(|error| error.to_string());
        let pool = match &config {
            Ok(config) => match tauri::async_runtime::block_on(connect_and_migrate(config)) {
                Ok(pool) => {
                    tracing::info!("postgresql pool is ready");
                    Some(pool)
                }
                Err(error) => {
                    tracing::warn!("postgresql is not available; starting in setup mode");
                    tracing::debug!(error = %error, "postgresql connect failed");
                    None
                }
            },
            Err(error) => {
                tracing::warn!(error = %error, "database configuration is invalid; starting in setup mode");
                None
            }
        };

        Self {
            inner: Arc::new(DatabaseRuntimeInner {
                config,
                pool: Mutex::new(pool),
            }),
        }
    }

    pub async fn status(&self) -> DatabaseStatus {
        if self.ping_existing().await {
            return self.connected_status();
        }
        self.connect().await
    }

    pub async fn connect(&self) -> DatabaseStatus {
        let config = match &self.inner.config {
            Ok(config) => config,
            Err(error) => return self.disconnected_status(config_message(error)),
        };

        match connect_and_migrate(config).await {
            Ok(pool) => {
                if let Ok(mut guard) = self.inner.pool.lock() {
                    *guard = Some(pool);
                }
                tracing::info!("postgresql pool is ready");
                self.connected_status()
            }
            Err(error) => {
                if let Ok(mut guard) = self.inner.pool.lock() {
                    *guard = None;
                }
                tracing::warn!("postgresql is not available; showing setup steps");
                tracing::debug!(error = %error, "postgresql connect failed");
                self.disconnected_status(public_connect_message(&error))
            }
        }
    }

    async fn ping_existing(&self) -> bool {
        let pool = {
            let Ok(guard) = self.inner.pool.lock() else {
                return false;
            };
            guard.clone()
        };
        match pool {
            Some(pool) => ping(&pool).await.is_ok(),
            None => false,
        }
    }

    fn connected_status(&self) -> DatabaseStatus {
        let mut status = self.base_status();
        status.connected = true;
        status.message = format!(
            "Connected to PostgreSQL at {}:{} / {}",
            status.host, status.port, status.database
        );
        status
    }

    fn disconnected_status(&self, message: String) -> DatabaseStatus {
        let mut status = self.base_status();
        status.connected = false;
        status.message = message;
        status
    }

    fn base_status(&self) -> DatabaseStatus {
        let (host, port, database, user) = self
            .inner
            .config
            .as_ref()
            .ok()
            .and_then(DatabaseConfig::public_target)
            .map(|target| (target.host, target.port, target.database, target.user))
            .unwrap_or_else(|| {
                (
                    "127.0.0.1".to_string(),
                    5432,
                    "vsmart_sync".to_string(),
                    "vsmart_sync".to_string(),
                )
            });

        DatabaseStatus {
            connected: false,
            host,
            port,
            database,
            user,
            platform: std::env::consts::OS.to_string(),
            message: String::new(),
        }
    }
}

fn config_message(error: &str) -> String {
    if error.contains("DATABASE_URL") || error.contains("POSTGRES_") {
        "Database settings in .env are incomplete. Copy .env.example to .env and set a local PostgreSQL URL on port 5432.".to_string()
    } else {
        "Database settings could not be loaded. Check .env in the project folder.".to_string()
    }
}

fn public_connect_message(error: &DatabaseError) -> String {
    let detail = format!("{error:?}");
    if detail.to_lowercase().contains("password")
        || detail.to_lowercase().contains("authentication")
    {
        return "PostgreSQL rejected the login. Check POSTGRES_USER and POSTGRES_PASSWORD in .env."
            .to_string();
    }
    if detail.contains("does not exist") {
        return "The vsmart_sync database or role is missing. Start PostgreSQL, then click Try again (npm start can create them when psql is available).".to_string();
    }
    "PostgreSQL is not running locally. Install it on this computer (not Docker) and start the service on port 5432.".to_string()
}

#[cfg(test)]
mod tests {
    use super::{config_message, DatabaseStatus};

    #[test]
    fn status_json_has_no_password_field() {
        let status = DatabaseStatus {
            connected: false,
            host: "127.0.0.1".into(),
            port: 5432,
            database: "vsmart_sync".into(),
            user: "vsmart_sync".into(),
            platform: "macos".into(),
            message: "PostgreSQL is not running locally.".into(),
        };
        let json = serde_json::to_string(&status).expect("json");
        assert!(json.contains("127.0.0.1"));
        assert!(!json.contains("password"));
        assert!(!json.contains("DATABASE_URL"));
        assert!(!json.contains("postgres://"));
    }

    #[test]
    fn config_message_is_actionable() {
        let message = config_message("DATABASE_URL is missing or empty");
        assert!(message.contains(".env"));
        assert!(!message.contains("postgres://"));
    }
}
