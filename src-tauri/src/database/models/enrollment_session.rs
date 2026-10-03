use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Live capture session. Card numbers are not stored on this row.
#[derive(Debug, Clone, FromRow)]
pub struct EnrollmentSessionRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub device_id: Uuid,
    pub device_name: String,
    pub enroll_type: String,
    pub status: String,
    pub credential_id: Option<Uuid>,
    pub error_code: Option<String>,
    pub card_type: Option<String>,
    pub identifier_type: Option<String>,
    pub identifier_unavailable: bool,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct EnrollmentSessionWriteRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub enroll_type: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
