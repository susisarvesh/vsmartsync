use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub username: String,
    pub status: String,
    pub matrix_user_id: Option<String>,
    pub short_name: Option<String>,
    pub full_name: Option<String>,
    pub reference_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
