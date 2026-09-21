use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Read model with joined display fields. Never includes secrets.
#[derive(Debug, Clone, FromRow)]
pub struct EnrollmentRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub credential_id: Uuid,
    pub credential_type: String,
    pub display_hint: Option<String>,
    pub device_id: Uuid,
    pub device_name: String,
    pub status: String,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub activated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Persistence payload for insert (no joined names).
#[derive(Debug, Clone)]
pub struct EnrollmentWriteRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub credential_id: Uuid,
    pub device_id: Uuid,
    pub status: String,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub activated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
