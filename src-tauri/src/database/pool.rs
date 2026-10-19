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
                    || db.constraint() == Some("enrollments_active_card_identifier_unique")
                    || db.constraint() == Some("device_users_user_device_unique")
                    || db.constraint() == Some("device_users_device_matrix_user_unique")
                    || db.constraint() == Some("device_users_device_matrix_ref_unique")
                    || db.constraint() == Some("user_devices_pair_unique")
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
    connect_with_limit(config, Duration::from_secs(3), 5).await
}

pub async fn connect_with_limit(
    config: &DatabaseConfig,
    timeout: Duration,
    max_connections: u32,
) -> Result<DbPool, DatabaseError> {
    let max_connections = max_connections.clamp(1, 48);
    PgPoolOptions::new()
        .max_connections(max_connections)
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
    connect_and_migrate_with_limit(config, 5).await
}

pub async fn connect_and_migrate_with_limit(
    config: &DatabaseConfig,
    max_connections: u32,
) -> Result<DbPool, DatabaseError> {
    let pool = connect_with_limit(config, Duration::from_secs(3), max_connections).await?;
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

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::connect_with_limit;

    fn percentile(samples: &[u128], percent: u8) -> u128 {
        if samples.is_empty() {
            return 0;
        }
        let index = (samples.len() - 1) * usize::from(percent) / 100;
        samples[index]
    }

    /// Times connection use against the application database. Read-only.
    /// Not part of `cargo test`.
    #[tokio::test]
    #[ignore = "requires local postgres"]
    async fn local_pool_acquire_times() {
        crate::common::load_env_files();
        let config = crate::database::DatabaseConfig::from_env()
            .unwrap_or_else(|_| panic!("local postgres settings are missing"));
        for size in [7_u32, 43] {
            let pool = connect_with_limit(&config, std::time::Duration::from_secs(3), size)
                .await
                .unwrap_or_else(|_| panic!("postgres connect failed"));
            let max_connections: String = sqlx::query_scalar("SHOW max_connections")
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|_| panic!("could not read max_connections"));
            let activity: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity")
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|_| panic!("could not count sessions"));
            let devices: i64 = sqlx::query_scalar("SELECT count(*) FROM devices")
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|_| panic!("could not count devices"));
            let events: i64 = sqlx::query_scalar("SELECT count(*) FROM events")
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|_| panic!("could not count events"));
            let mut samples = Vec::with_capacity(40);
            let mut failed = 0_u32;
            let wave = Instant::now();
            let mut tasks = Vec::with_capacity(40);
            for _ in 0..40 {
                let pool = pool.clone();
                tasks.push(tokio::spawn(async move {
                    let started = Instant::now();
                    let outcome = sqlx::query("SELECT 1").execute(&pool).await;
                    (outcome.is_ok(), started.elapsed())
                }));
            }
            for task in tasks {
                match task.await {
                    Ok((true, elapsed)) => samples.push(elapsed.as_micros()),
                    _ => failed += 1,
                }
            }
            samples.sort_unstable();
            eprintln!(
                "pool_probe pool={size} server_max={max_connections} other_sessions={activity} devices={devices} events={events} queries=40 failed={failed} wave_ms={} p50_us={} p95_us={} p99_us={} pool_size={} pool_idle={}",
                wave.elapsed().as_millis(),
                percentile(&samples, 50),
                percentile(&samples, 95),
                percentile(&samples, 99),
                pool.size(),
                pool.num_idle()
            );
            assert_eq!(failed, 0, "pool {size} dropped queries");
        }
    }

    /// Counts decrypt outcomes for stored device secrets. Prints lengths and
    /// counts only. Read-only. Not part of `cargo test`.
    #[tokio::test]
    #[ignore = "requires local postgres and the workstation key"]
    async fn local_device_secret_outcomes() {
        crate::common::load_env_files();
        let config = crate::database::DatabaseConfig::from_env()
            .unwrap_or_else(|_| panic!("local postgres settings are missing"));
        let pool = connect_with_limit(&config, std::time::Duration::from_secs(3), 1)
            .await
            .unwrap_or_else(|_| panic!("postgres connect failed"));
        let rows: Vec<(Vec<u8>, bool, String, bool, bool)> = sqlx::query_as(
            r#"
            SELECT password_ciphertext,
                   device_name = 'Sync Door',
                   status,
                   created_at < timestamptz '2026-09-27 16:23:27+05:30',
                   updated_at < timestamptz '2026-09-27 16:23:27+05:30'
            FROM devices
            "#,
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_else(|_| panic!("could not read device secrets"));
        let home = crate::common::SecretVault::open();
        let known_keys: [([u8; 32], &str); 5] = [
            ([9u8; 32], "k9"),
            ([42u8; 32], "k42"),
            ([7u8; 32], "k7"),
            ([91u8; 32], "k91"),
            ([77u8; 32], "k77"),
        ];
        let probes: Vec<(&str, crate::common::SecretVault)> = known_keys
            .into_iter()
            .map(|(key, label)| {
                (
                    label,
                    crate::common::SecretVault::from_key(key).expect("probe vault"),
                )
            })
            .collect();
        let mut short = 0_u32;
        let mut home_ok = 0_u32;
        let mut home_corrupt = 0_u32;
        let mut known_ok = [0_u32; 5];
        let mut neither = 0_u32;
        let mut sync_door = 0_u32;
        let mut active = 0_u32;
        let mut created_before_key = 0_u32;
        let mut updated_before_key = 0_u32;
        let mut home_ok_before = 0_u32;
        let mut min_len = usize::MAX;
        let mut max_len = 0_usize;
        for (blob, is_sync_door, status, before_key, updated_before) in &rows {
            min_len = min_len.min(blob.len());
            max_len = max_len.max(blob.len());
            if *is_sync_door {
                sync_door += 1;
            }
            if status == "active" {
                active += 1;
            }
            if *before_key {
                created_before_key += 1;
            }
            if *updated_before {
                updated_before_key += 1;
            }
            if blob.len() <= 12 {
                short += 1;
            }
            let home_result = home.as_ref().map(|vault| vault.decrypt(blob));
            let home_success = matches!(home_result, Ok(Ok(_)));
            if home_success {
                home_ok += 1;
                if *before_key {
                    home_ok_before += 1;
                }
            } else {
                home_corrupt += 1;
            }
            let mut matched_known = false;
            for (index, (_, vault)) in probes.iter().enumerate() {
                if vault.decrypt(blob).is_ok() {
                    known_ok[index] += 1;
                    matched_known = true;
                }
            }
            if !home_success && !matched_known {
                neither += 1;
            }
        }
        let key_meta = std::env::var_os("HOME").and_then(|home| {
            std::fs::metadata(
                std::path::PathBuf::from(home)
                    .join(".vsmart-sync")
                    .join("master.key"),
            )
            .ok()
        });
        let (key_bytes, key_mtime) = match key_meta {
            Some(meta) => {
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|time| time.as_secs())
                    .unwrap_or(0);
                (meta.len(), modified)
            }
            None => (0, 0),
        };
        eprintln!(
            "secret_probe devices={} active={} sync_door={} short={} min_len={} max_len={} home_open={} home_ok={} home_ok_before_key={} home_corrupt={} k9={} k42={} k7={} k91={} k77={} neither={} created_before_key={} updated_before_key={} key_bytes={} key_mtime={}",
            rows.len(),
            active,
            sync_door,
            short,
            if rows.is_empty() { 0 } else { min_len },
            max_len,
            home.is_ok(),
            home_ok,
            home_ok_before,
            home_corrupt,
            known_ok[0],
            known_ok[1],
            known_ok[2],
            known_ok[3],
            known_ok[4],
            neither,
            created_before_key,
            updated_before_key,
            key_bytes,
            key_mtime
        );
    }

    /// Contacts devices that decrypt with the workstation key. Prints outcome
    /// counts only. Does not insert or update rows.
    #[tokio::test]
    #[ignore = "requires local postgres; read-only device contact"]
    async fn local_home_key_event_count_is_read_only() {
        crate::common::load_env_files();
        let config = crate::database::DatabaseConfig::from_env()
            .unwrap_or_else(|_| panic!("local postgres settings are missing"));
        let pool = connect_with_limit(&config, std::time::Duration::from_secs(3), 1)
            .await
            .unwrap_or_else(|_| panic!("postgres connect failed"));
        let rows: Vec<(String, i32, String, Vec<u8>)> = sqlx::query_as(
            "SELECT host, port, username, password_ciphertext FROM devices WHERE status = 'active'",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_else(|_| panic!("could not read devices"));
        let before_devices: i64 = sqlx::query_scalar("SELECT count(*) FROM devices")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
        let before_events: i64 = sqlx::query_scalar("SELECT count(*) FROM events")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
        let vault = crate::common::SecretVault::open().expect("workstation key");
        let matrix = crate::matrix::MatrixAdapter::new().expect("matrix client");
        let mut decrypt_ok = 0_u32;
        let mut count_ok = 0_u32;
        let mut timeout = 0_u32;
        let mut unreachable = 0_u32;
        let mut auth_failed = 0_u32;
        let mut other = 0_u32;
        for (host, port, username, blob) in &rows {
            let Ok(password) = vault.decrypt(blob) else {
                continue;
            };
            decrypt_ok += 1;
            let Ok(port) = u16::try_from(*port) else {
                other += 1;
                continue;
            };
            match matrix.event_count(host, port, username, &password).await {
                Ok(_) => count_ok += 1,
                Err(error) => match error {
                    crate::matrix::MatrixAdapterError::Timeout => timeout += 1,
                    crate::matrix::MatrixAdapterError::Unreachable
                    | crate::matrix::MatrixAdapterError::InvalidTarget => unreachable += 1,
                    crate::matrix::MatrixAdapterError::AuthFailed => auth_failed += 1,
                    _ => other += 1,
                },
            }
        }
        let after_devices: i64 = sqlx::query_scalar("SELECT count(*) FROM devices")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
        let after_events: i64 = sqlx::query_scalar("SELECT count(*) FROM events")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
        eprintln!(
            "physical_probe active_rows={} decrypt_ok={} event_count_ok={} timeout={} unreachable={} auth_failed={} other={} devices_before={} devices_after={} events_before={} events_after={}",
            rows.len(),
            decrypt_ok,
            count_ok,
            timeout,
            unreachable,
            auth_failed,
            other,
            before_devices,
            after_devices,
            before_events,
            after_events
        );
        assert_eq!(before_devices, after_devices);
        assert_eq!(before_events, after_events);
    }
}
