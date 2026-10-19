//! Writing tests must use a database that is not the application database.
//!
//! `TEST_DATABASE_URL` is the only accepted target. The name must end with
//! `_test` and must differ from the application database. Cleanup deletes only
//! the user and device ids the test registered.

use std::future::Future;
use std::sync::{Arc, Mutex};

use thiserror::Error;
use uuid::Uuid;

use super::{connect_and_migrate, DatabaseConfig, DatabaseConfigError, DatabaseError, DbPool};

const APPLICATION_DATABASE: &str = "vsmart_sync";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TestDatabaseError {
    #[error("TEST_DATABASE_URL is missing. Set it to a database whose name ends with _test")]
    Missing,
    #[error("TEST_DATABASE_URL is not a postgres connection URL")]
    Invalid,
    #[error(
        "TEST_DATABASE_URL names the application database. Use a different database whose name ends with _test"
    )]
    ApplicationDatabase,
    #[error("the test database name must end with _test")]
    NameNotIsolated,
}

/// Decide whether a URL is safe for a writing test. `application_names` are
/// compared without the connection secret. The returned error never includes
/// a URL, user, or password.
pub fn assess_test_target(
    test_url: &str,
    application_names: &[&str],
) -> Result<DatabaseConfig, TestDatabaseError> {
    let trimmed = test_url.trim();
    if trimmed.is_empty() {
        return Err(TestDatabaseError::Missing);
    }
    let config = DatabaseConfig::from_url(trimmed).map_err(invalid_config)?;
    let Some(target) = config.public_target() else {
        return Err(TestDatabaseError::Invalid);
    };
    let name = target.database.to_ascii_lowercase();
    if name == APPLICATION_DATABASE
        || application_names
            .iter()
            .any(|app| app.eq_ignore_ascii_case(&name))
    {
        return Err(TestDatabaseError::ApplicationDatabase);
    }
    if !name.ends_with("_test") {
        return Err(TestDatabaseError::NameNotIsolated);
    }
    Ok(config)
}

fn invalid_config(error: DatabaseConfigError) -> TestDatabaseError {
    match error {
        DatabaseConfigError::MissingUrl => TestDatabaseError::Missing,
        _ => TestDatabaseError::Invalid,
    }
}

/// Load `TEST_DATABASE_URL` and refuse the application database.
pub fn writable_test_config() -> Result<DatabaseConfig, TestDatabaseError> {
    let url = std::env::var("TEST_DATABASE_URL").unwrap_or_default();
    let names = application_database_names();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    assess_test_target(&url, &names)
}

fn application_database_names() -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(url) = std::env::var("DATABASE_URL") {
        if let Ok(config) = DatabaseConfig::from_url(url) {
            if let Some(target) = config.public_target() {
                names.push(target.database);
            }
        }
    }
    if let Ok(name) = std::env::var("POSTGRES_DB") {
        let name = name.trim();
        if !name.is_empty() {
            names.push(name.to_string());
        }
    }
    names
}

/// Connection plus the ids this test is allowed to delete.
#[derive(Clone)]
pub struct IsolatedDatabase {
    pool: DbPool,
    devices: Arc<Mutex<Vec<Uuid>>>,
    users: Arc<Mutex<Vec<Uuid>>>,
}

impl IsolatedDatabase {
    pub fn pool(&self) -> DbPool {
        self.pool.clone()
    }

    pub fn track_device(&self, id: Uuid) {
        self.devices
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(id);
    }

    pub fn track_user(&self, id: Uuid) {
        self.users
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(id);
    }

    async fn remove_tracked_rows(&self) {
        let devices = self
            .devices
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let users = self
            .users
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        if devices.is_empty() && users.is_empty() {
            return;
        }
        // Each statement binds only the placeholders it uses. A query that
        // mentions $2 without $1 is rejected by PostgreSQL, which previously
        // left tracked devices behind.
        let paired = [
            "DELETE FROM enrollment_sessions WHERE user_id = ANY($1) OR device_id = ANY($2)",
            "DELETE FROM enrollments WHERE user_id = ANY($1) OR device_id = ANY($2)",
            "DELETE FROM device_users WHERE user_id = ANY($1) OR device_id = ANY($2)",
            "DELETE FROM user_devices WHERE user_id = ANY($1) OR device_id = ANY($2)",
        ];
        for statement in paired {
            Self::finish_cleanup(
                sqlx::query(statement)
                    .bind(&users)
                    .bind(&devices)
                    .execute(&self.pool)
                    .await,
            );
        }
        Self::finish_cleanup(
            sqlx::query("DELETE FROM credentials WHERE user_id = ANY($1)")
                .bind(&users)
                .execute(&self.pool)
                .await,
        );
        Self::finish_cleanup(
            sqlx::query("DELETE FROM device_id_sequences WHERE device_id = ANY($1)")
                .bind(&devices)
                .execute(&self.pool)
                .await,
        );
        Self::finish_cleanup(
            sqlx::query("DELETE FROM devices WHERE id = ANY($1)")
                .bind(&devices)
                .execute(&self.pool)
                .await,
        );
        Self::finish_cleanup(
            sqlx::query("DELETE FROM users WHERE id = ANY($1)")
                .bind(&users)
                .execute(&self.pool)
                .await,
        );
    }

    fn finish_cleanup(result: Result<sqlx::postgres::PgQueryResult, sqlx::Error>) {
        if let Err(error) = result {
            panic!(
                "isolated test cleanup failed: {}",
                DatabaseError::Query(error)
            );
        }
    }
}

/// Run a writing test on the isolated database. Rows registered with
/// `track_device` and `track_user` are deleted after the test, including when
/// the test panics. Unregistered rows are left in the test database.
pub async fn run_isolated_test<F, Fut>(test: F)
where
    F: FnOnce(IsolatedDatabase) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let config = writable_test_config().unwrap_or_else(|error| panic!("{error}"));
    let pool = connect_and_migrate(&config)
        .await
        .unwrap_or_else(|error| panic!("isolated test database connection failed: {error}"));
    let db = IsolatedDatabase {
        pool,
        devices: Arc::new(Mutex::new(Vec::new())),
        users: Arc::new(Mutex::new(Vec::new())),
    };
    let cleanup = db.clone();
    let outcome = tokio::spawn(test(db)).await;
    cleanup.remove_tracked_rows().await;
    if let Err(error) = outcome {
        if error.is_panic() {
            std::panic::resume_unwind(error.into_panic());
        }
        panic!("isolated test was cancelled");
    }
}

#[cfg(test)]
mod tests {
    use super::{assess_test_target, TestDatabaseError};

    const APP_URL: &str = "postgres://vsmart_sync:change-me@127.0.0.1:5432/vsmart_sync";
    const SECRET_URL: &str = "postgres://operator:secret-pass@127.0.0.1:5432/vsmart_sync";

    #[test]
    fn the_application_database_is_refused() {
        let error = assess_test_target(APP_URL, &["vsmart_sync"]).expect_err("refused");
        assert_eq!(error, TestDatabaseError::ApplicationDatabase);
        let message = error.to_string();
        assert!(!message.contains("change-me"));
        assert!(!message.contains("postgres://"));
        assert!(!message.contains("secret-pass"));

        let same = assess_test_target(SECRET_URL, &["vsmart_sync"]).expect_err("refused");
        assert_eq!(same, TestDatabaseError::ApplicationDatabase);
        assert!(!same.to_string().contains("secret-pass"));
    }

    #[test]
    fn a_database_that_matches_the_application_url_is_refused_even_with_a_test_suffix() {
        let error = assess_test_target(
            "postgres://operator:secret-pass@127.0.0.1:5432/custom_test",
            &["custom_test"],
        )
        .expect_err("refused");
        assert_eq!(error, TestDatabaseError::ApplicationDatabase);
        assert!(!error.to_string().contains("secret-pass"));
    }

    #[test]
    fn a_distinct_test_database_is_accepted() {
        let config = assess_test_target(
            "postgres://operator:secret-pass@127.0.0.1:5432/vsmart_sync_test",
            &["vsmart_sync"],
        )
        .expect("accepted");
        let target = config.public_target().expect("target");
        assert_eq!(target.database, "vsmart_sync_test");
        assert_ne!(target.database, "vsmart_sync");
    }

    #[test]
    fn a_name_without_the_test_suffix_is_refused() {
        let error = assess_test_target(
            "postgres://operator:secret-pass@127.0.0.1:5432/matrix_lab",
            &["vsmart_sync"],
        )
        .expect_err("refused");
        assert_eq!(error, TestDatabaseError::NameNotIsolated);
        assert!(!error.to_string().contains("secret-pass"));
    }

    #[test]
    fn a_missing_url_is_refused_before_any_connection() {
        assert_eq!(
            assess_test_target("   ", &["vsmart_sync"]),
            Err(TestDatabaseError::Missing)
        );
    }
}
