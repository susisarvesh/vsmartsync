use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use thiserror::Error;

use super::DatabaseConfig;

pub type DbPool = PgPool;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("failed to connect to postgresql")]
    Connect(#[source] sqlx::Error),
    #[error("failed to run postgresql migrations")]
    Migrate(#[source] sqlx::migrate::MigrateError),
    #[error("failed to query postgresql")]
    Query(#[source] sqlx::Error),
}

impl DatabaseError {
    pub fn is_unique_violation(&self) -> bool {
        match self {
            Self::Query(sqlx::Error::Database(db)) => {
                db.code().as_deref() == Some("23505")
                    || db.constraint() == Some("devices_host_port_unique")
                    || db.constraint() == Some("credentials_card_value_digest_unique")
                    || db.constraint() == Some("credentials_one_pin_per_user")
                    || db.constraint() == Some("enrollments_open_credential_device_unique")
                    || db.constraint() == Some("device_users_user_device_unique")
                    || db.constraint() == Some("device_users_device_matrix_user_unique")
                    || db.constraint() == Some("device_users_device_matrix_ref_unique")
            }
            _ => false,
        }
    }

    pub fn is_foreign_key_violation(&self) -> bool {
        match self {
            Self::Query(sqlx::Error::Database(db)) => db.code().as_deref() == Some("23503"),
            _ => false,
        }
    }
}

pub async fn connect(config: &DatabaseConfig) -> Result<DbPool, DatabaseError> {
    connect_with_timeout(config, Duration::from_secs(3)).await
}

async fn connect_with_timeout(
    config: &DatabaseConfig,
    timeout: Duration,
) -> Result<DbPool, DatabaseError> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(timeout)
        .connect(config.url())
        .await
        .map_err(DatabaseError::Connect)
}

pub async fn run_migrations(pool: &DbPool) -> Result<(), DatabaseError> {
    // Path is relative to CARGO_MANIFEST_DIR (src-tauri/).
    sqlx::migrate!("../migrations")
        .run(pool)
        .await
        .map_err(DatabaseError::Migrate)
}

pub async fn connect_and_migrate(config: &DatabaseConfig) -> Result<DbPool, DatabaseError> {
    let pool = connect(config).await?;
    run_migrations(&pool).await?;
    Ok(pool)
}

pub async fn ping(pool: &DbPool) -> Result<(), DatabaseError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .map_err(DatabaseError::Connect)?;
    Ok(())
}
