use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct DeviceRecord {
    pub id: Uuid,
    pub device_name: String,
    pub host: String,
    pub port: i32,
    pub mac_address: Option<String>,
    pub device_model: Option<String>,
    pub username: String,
    pub password_ciphertext: Vec<u8>,
    pub status: String,
    pub connection_status: String,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fields the poll scheduler needs. The password stays out of this row.
#[derive(Debug, Clone, FromRow)]
pub struct DevicePollTarget {
    pub id: Uuid,
    pub device_name: String,
    pub status: String,
}
