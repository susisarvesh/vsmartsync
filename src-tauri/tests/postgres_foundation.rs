//! Live PostgreSQL checks for the database foundation.
//!
//! These tests require a running local PostgreSQL instance. They are ignored by
//! default so `cargo test` stays offline-friendly.

use vsmart_sync_lib::database::{connect_and_migrate, ping, DatabaseConfig};

fn load_test_config() -> DatabaseConfig {
    DatabaseConfig::from_url(std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://vsmart_sync:change-me@127.0.0.1:5432/vsmart_sync".to_string()
    }))
    .expect("test database url")
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn connects_and_runs_migrations() {
    let config = load_test_config();
    let pool = connect_and_migrate(&config)
        .await
        .expect("postgresql should be reachable");
    ping(&pool).await.expect("select 1");
}
