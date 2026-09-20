use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Persistence row used only for writes. Never serialize to React.
#[derive(Debug, Clone)]
pub struct CredentialWriteRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub credential_type: String,
    pub value_ciphertext: Vec<u8>,
    pub value_digest: Vec<u8>,
    pub display_hint: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Read model joined with user name. Omits ciphertext and digest.
#[derive(Debug, Clone, FromRow)]
pub struct CredentialRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    #[sqlx(rename = "type")]
    pub credential_type: String,
    pub display_hint: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
