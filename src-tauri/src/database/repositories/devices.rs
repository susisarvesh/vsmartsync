use uuid::Uuid;

use crate::database::models::DeviceRecord;
use crate::database::{DatabaseError, DbPool};

pub const DEVICE_LIST_LIMIT: i64 = 200;

pub struct DeviceRepository {
    pool: DbPool,
}

impl DeviceRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, record: &DeviceRecord) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO devices (
                id, name, host, port, username, password_ciphertext,
                connection_status, last_seen_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(record.id)
        .bind(&record.name)
        .bind(&record.host)
        .bind(record.port)
        .bind(&record.username)
        .bind(&record.password_ciphertext)
        .bind(&record.connection_status)
        .bind(record.last_seen_at)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<DeviceRecord>, DatabaseError> {
        sqlx::query_as::<_, DeviceRecord>(
            r#"
            SELECT
                id, name, host, port, username, password_ciphertext,
                connection_status, last_seen_at, created_at, updated_at
            FROM devices
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn list(&self) -> Result<Vec<DeviceRecord>, DatabaseError> {
        sqlx::query_as::<_, DeviceRecord>(
            r#"
            SELECT
                id, name, host, port, username, password_ciphertext,
                connection_status, last_seen_at, created_at, updated_at
            FROM devices
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(DEVICE_LIST_LIMIT)
        .fetch_all(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn update_metadata(
        &self,
        record: &DeviceRecord,
    ) -> Result<Option<DeviceRecord>, DatabaseError> {
        sqlx::query_as::<_, DeviceRecord>(
            r#"
            UPDATE devices
            SET
                name = $2,
                host = $3,
                port = $4,
                username = $5,
                updated_at = $6
            WHERE id = $1
            RETURNING
                id, name, host, port, username, password_ciphertext,
                connection_status, last_seen_at, created_at, updated_at
            "#,
        )
        .bind(record.id)
        .bind(&record.name)
        .bind(&record.host)
        .bind(record.port)
        .bind(&record.username)
        .bind(record.updated_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn update_password(
        &self,
        id: Uuid,
        password_ciphertext: &[u8],
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<DeviceRecord>, DatabaseError> {
        sqlx::query_as::<_, DeviceRecord>(
            r#"
            UPDATE devices
            SET password_ciphertext = $2, updated_at = $3
            WHERE id = $1
            RETURNING
                id, name, host, port, username, password_ciphertext,
                connection_status, last_seen_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(password_ciphertext)
        .bind(updated_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn update_connection(
        &self,
        id: Uuid,
        connection_status: &str,
        last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<DeviceRecord>, DatabaseError> {
        sqlx::query_as::<_, DeviceRecord>(
            r#"
            UPDATE devices
            SET
                connection_status = $2,
                last_seen_at = $3,
                updated_at = $4
            WHERE id = $1
            RETURNING
                id, name, host, port, username, password_ciphertext,
                connection_status, last_seen_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(connection_status)
        .bind(last_seen_at)
        .bind(updated_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }
}
