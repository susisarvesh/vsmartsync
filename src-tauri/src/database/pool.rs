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
}

pub async fn connect(config: &DatabaseConfig) -> Result<DbPool, DatabaseError> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
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
