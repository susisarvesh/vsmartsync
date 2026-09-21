use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::common::{DevicePasswordVault, SecretError};
use crate::database::models::DeviceRecord;
use crate::database::repositories::DeviceRepository;
use crate::database::DatabaseError;
use crate::matrix::{MatrixAdapter, MatrixProbeError};

use super::{
    normalize_host, normalize_name, normalize_password, normalize_port, normalize_username,
    ConnectionStatus, Device, DeviceError, DEFAULT_DEVICE_PORT,
};

pub async fn create_device(
    repo: &DeviceRepository,
    vault: &DevicePasswordVault,
    name: &str,
    host: &str,
    port: Option<i32>,
    username: &str,
    password: &str,
) -> Result<Device, DeviceError> {
    let now = Utc::now();
    let name = normalize_name(name)?;
    let host = normalize_host(host)?;
    let port = normalize_port(port.unwrap_or(DEFAULT_DEVICE_PORT))?;
    let username = normalize_username(username)?;
    let password = normalize_password(password)?;
    let ciphertext = vault.encrypt(&password).map_err(map_secret_error)?;

    let record = DeviceRecord {
        id: Uuid::new_v4(),
        name,
        host,
        port,
        username,
        password_ciphertext: ciphertext,
        connection_status: ConnectionStatus::Unknown.as_str().to_string(),
        last_seen_at: None,
        created_at: now,
        updated_at: now,
    };

    repo.insert(&record).await.map_err(map_db_error)?;
    tracing::info!(device_id = %record.id, command = "create_device", "created device");
    from_record(record)
}

pub async fn list_devices(repo: &DeviceRepository) -> Result<Vec<Device>, DeviceError> {
    repo.list()
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(from_record)
        .collect()
}

pub async fn update_device(
    repo: &DeviceRepository,
    id: Uuid,
    name: &str,
    host: &str,
    port: i32,
    username: &str,
) -> Result<Device, DeviceError> {
    let mut record = load_record(repo, id).await?;
    record.name = normalize_name(name)?;
    record.host = normalize_host(host)?;
    record.port = normalize_port(port)?;
    record.username = normalize_username(username)?;
    record.updated_at = Utc::now();

    let saved = repo
        .update_metadata(&record)
        .await
        .map_err(map_db_error)?
        .ok_or(DeviceError::NotFound)?;
    tracing::info!(device_id = %id, command = "update_device", "updated device metadata");
    from_record(saved)
}

pub async fn set_device_password(
    repo: &DeviceRepository,
    vault: &DevicePasswordVault,
    id: Uuid,
    password: &str,
) -> Result<Device, DeviceError> {
    let _ = load_record(repo, id).await?;
    let password = normalize_password(password)?;
    let ciphertext = vault.encrypt(&password).map_err(map_secret_error)?;
    let updated_at = Utc::now();

    let saved = repo
        .update_password(id, &ciphertext, updated_at)
        .await
        .map_err(map_db_error)?
        .ok_or(DeviceError::NotFound)?;
    tracing::info!(device_id = %id, command = "set_device_password", "updated device password");
    from_record(saved)
}

pub async fn test_device_connection(
    repo: &DeviceRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    id: Uuid,
) -> Result<Device, DeviceError> {
    let record = load_record(repo, id).await?;
    let password = vault
        .decrypt(&record.password_ciphertext)
        .map_err(map_secret_error)?;
    let port = u16::try_from(record.port).map_err(|_| DeviceError::InvalidPort)?;

    tracing::info!(device_id = %id, command = "test_device_connection", "probing device");
    let probe = matrix
        .probe_basic_config(&record.host, port, &record.username, &password)
        .await;
    // Drop plaintext as soon as the probe returns.
    drop(password);

    let now = Utc::now();
    let outcome = apply_probe_result(probe, record.last_seen_at, now);
    let saved = repo
        .update_connection(id, outcome.status.as_str(), outcome.last_seen_at, now)
        .await
        .map_err(map_db_error)?
        .ok_or(DeviceError::NotFound)?;

    let device = from_record(saved)?;
    match outcome.error {
        Some(error) => Err(error),
        None => Ok(device),
    }
}

struct ProbeOutcome {
    status: ConnectionStatus,
    last_seen_at: Option<DateTime<Utc>>,
    error: Option<DeviceError>,
}

fn apply_probe_result(
    probe: Result<(), MatrixProbeError>,
    previous_last_seen: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> ProbeOutcome {
    match probe {
        Ok(()) => ProbeOutcome {
            status: ConnectionStatus::Online,
            last_seen_at: Some(now),
            error: None,
        },
        Err(MatrixProbeError::AuthFailed) => ProbeOutcome {
            status: ConnectionStatus::Offline,
            last_seen_at: previous_last_seen,
            error: Some(DeviceError::AuthFailed),
        },
        Err(MatrixProbeError::BadResponse) | Err(MatrixProbeError::InvalidTarget) => ProbeOutcome {
            status: ConnectionStatus::Offline,
            last_seen_at: previous_last_seen,
            error: Some(DeviceError::BadResponse),
        },
        Err(MatrixProbeError::Timeout) | Err(MatrixProbeError::Unreachable) => ProbeOutcome {
            status: ConnectionStatus::Offline,
            last_seen_at: previous_last_seen,
            error: Some(DeviceError::Offline),
        },
    }
}

async fn load_record(repo: &DeviceRepository, id: Uuid) -> Result<DeviceRecord, DeviceError> {
    repo.find_by_id(id)
        .await
        .map_err(map_db_error)?
        .ok_or(DeviceError::NotFound)
}

fn from_record(record: DeviceRecord) -> Result<Device, DeviceError> {
    Ok(Device {
        id: record.id,
        name: record.name,
        host: record.host,
        port: record.port,
        username: record.username,
        connection_status: ConnectionStatus::parse(&record.connection_status)?,
        last_seen_at: record.last_seen_at,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn map_db_error(error: DatabaseError) -> DeviceError {
    if error.is_unique_violation() {
        return DeviceError::Duplicate;
    }
    tracing::error!(error = %error, "devices persistence failed");
    DeviceError::Unavailable
}

fn map_secret_error(error: SecretError) -> DeviceError {
    match error {
        SecretError::Unavailable | SecretError::Corrupt => {
            tracing::error!(error = %error, "device secret vault failed");
            DeviceError::SecretUnavailable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_probe_result, from_record};
    use crate::database::models::DeviceRecord;
    use crate::domains::devices::{ConnectionStatus, DeviceError};
    use crate::matrix::MatrixProbeError;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn successful_probe_sets_online_and_last_seen() {
        let now = Utc::now();
        let outcome = apply_probe_result(Ok(()), None, now);
        assert_eq!(outcome.status, ConnectionStatus::Online);
        assert_eq!(outcome.last_seen_at, Some(now));
        assert!(outcome.error.is_none());
    }

    #[test]
    fn failed_probe_sets_offline_and_preserves_last_seen() {
        let now = Utc::now();
        let previous = Some(now - chrono::Duration::hours(1));
        let outcome = apply_probe_result(Err(MatrixProbeError::Timeout), previous, now);
        assert_eq!(outcome.status, ConnectionStatus::Offline);
        assert_eq!(outcome.last_seen_at, previous);
        assert_eq!(outcome.error, Some(DeviceError::Offline));
    }

    #[test]
    fn auth_failure_maps_without_exposing_secret() {
        let outcome = apply_probe_result(Err(MatrixProbeError::AuthFailed), None, Utc::now());
        assert_eq!(outcome.error, Some(DeviceError::AuthFailed));
        assert_eq!(outcome.error.unwrap().to_string(), "DEVICE_AUTH_FAILED");
    }

    #[test]
    fn public_device_view_omits_ciphertext() {
        let record = DeviceRecord {
            id: Uuid::new_v4(),
            name: "Door".into(),
            host: "192.168.1.10".into(),
            port: 80,
            username: "admin".into(),
            password_ciphertext: b"ciphertext-blob".to_vec(),
            connection_status: "unknown".into(),
            last_seen_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let device = from_record(record).unwrap();
        let json = serde_json::to_string(&device).unwrap();
        assert!(!json.contains("password"));
        assert!(!json.contains("ciphertext"));
        assert!(!json.contains("ciphertext-blob"));
    }
}
