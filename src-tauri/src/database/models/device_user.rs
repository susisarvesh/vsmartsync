use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Row from `device_users` (optionally joined names for diagnostics).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeviceUserRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub matrix_user_id: String,
    pub matrix_ref_user_id: i64,
    pub provisioned_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceUserWriteRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub matrix_user_id: String,
    pub matrix_ref_user_id: i64,
    pub provisioned_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
