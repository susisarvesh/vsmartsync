use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::models::{CredentialRecord, CredentialWriteRecord};
use crate::database::{DatabaseError, DbPool};

pub const CREDENTIAL_LIST_LIMIT: i64 = 500;

#[derive(Debug, Clone, Default)]
pub struct CredentialListQuery {
    pub user_id: Option<Uuid>,
    pub credential_type: Option<String>,
    pub status: Option<String>,
}

pub struct CredentialRepository {
    pool: DbPool,
}

impl CredentialRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, record: &CredentialWriteRecord) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO credentials (
                id, user_id, type, value_ciphertext, value_digest,
                display_hint, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(record.id)
        .bind(record.user_id)
        .bind(&record.credential_type)
        .bind(&record.value_ciphertext)
        .bind(&record.value_digest)
        .bind(&record.display_hint)
        .bind(&record.status)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<CredentialRecord>, DatabaseError> {
        sqlx::query_as::<_, CredentialRecord>(
            r#"
            SELECT
                c.id,
                c.user_id,
                u.name AS user_name,
                c.type,
                c.display_hint,
                c.status,
                c.created_at,
                c.updated_at
            FROM credentials c
            INNER JOIN users u ON u.id = c.user_id
            WHERE c.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn list(
        &self,
        query: &CredentialListQuery,
    ) -> Result<Vec<CredentialRecord>, DatabaseError> {
        sqlx::query_as::<_, CredentialRecord>(
            r#"
            SELECT
                c.id,
                c.user_id,
                u.name AS user_name,
                c.type,
                c.display_hint,
                c.status,
                c.created_at,
                c.updated_at
            FROM credentials c
            INNER JOIN users u ON u.id = c.user_id
            WHERE ($1::uuid IS NULL OR c.user_id = $1)
              AND ($2::text IS NULL OR c.type = $2)
              AND ($3::text IS NULL OR c.status = $3)
            ORDER BY c.created_at DESC
            LIMIT $4
            "#,
        )
        .bind(query.user_id)
        .bind(query.credential_type.as_deref())
        .bind(query.status.as_deref())
        .bind(CREDENTIAL_LIST_LIMIT)
        .fetch_all(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn update_value(
        &self,
        id: Uuid,
        value_ciphertext: &[u8],
        value_digest: &[u8],
        display_hint: Option<&str>,
        updated_at: DateTime<Utc>,
    ) -> Result<Option<CredentialRecord>, DatabaseError> {
        sqlx::query_as::<_, CredentialRecord>(
            r#"
            UPDATE credentials c
            SET
                value_ciphertext = $2,
                value_digest = $3,
                display_hint = $4,
                updated_at = $5
            FROM users u
            WHERE c.id = $1 AND u.id = c.user_id
            RETURNING
                c.id,
                c.user_id,
                u.name AS user_name,
                c.type,
                c.display_hint,
                c.status,
                c.created_at,
                c.updated_at
            "#,
        )
        .bind(id)
        .bind(value_ciphertext)
        .bind(value_digest)
        .bind(display_hint)
        .bind(updated_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn update_status(
        &self,
        id: Uuid,
        status: &str,
        updated_at: DateTime<Utc>,
    ) -> Result<Option<CredentialRecord>, DatabaseError> {
        sqlx::query_as::<_, CredentialRecord>(
            r#"
            UPDATE credentials c
            SET status = $2, updated_at = $3
            FROM users u
            WHERE c.id = $1 AND u.id = c.user_id
            RETURNING
                c.id,
                c.user_id,
                u.name AS user_name,
                c.type,
                c.display_hint,
                c.status,
                c.created_at,
                c.updated_at
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(updated_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }
}
