//! Live PostgreSQL checks for the database foundation.
//!
//! These tests require a running local PostgreSQL instance. They are ignored by
//! default so `cargo test` stays offline-friendly.

use vsmart_sync_lib::database::{ping, run_isolated_test};

#[tokio::test]
#[ignore = "requires an isolated PostgreSQL database named in TEST_DATABASE_URL"]
async fn connects_and_runs_migrations() {
    run_isolated_test(|db| async move {
        ping(&db.pool()).await.expect("select 1");
    })
    .await;
}
