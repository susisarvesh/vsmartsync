use uuid::Uuid;

use crate::database::models::DeviceRecord;
use crate::database::{DatabaseError, DbPool};

pub const DEVICE_LIST_LIMIT: i64 = 200;

const DEVICE_COLUMNS: &str = "
    id, device_name, host, port, mac_address, device_model, username,
    password_ciphertext, status, connection_status, last_seen_at, created_at, updated_at
";

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
                id, device_name, host, port, mac_address, device_model, username,
                password_ciphertext, status, connection_status, last_seen_at,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(record.id)
        .bind(&record.device_name)
        .bind(&record.host)
        .bind(record.port)
        .bind(&record.mac_address)
        .bind(&record.device_model)
        .bind(&record.username)
        .bind(&record.password_ciphertext)
        .bind(&record.status)
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
        let sql = format!("SELECT {DEVICE_COLUMNS} FROM devices WHERE id = $1");
        sqlx::query_as::<_, DeviceRecord>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::Query)
    }

    pub async fn list(&self) -> Result<Vec<DeviceRecord>, DatabaseError> {
        let sql = format!("SELECT {DEVICE_COLUMNS} FROM devices ORDER BY created_at DESC LIMIT $1");
        sqlx::query_as::<_, DeviceRecord>(&sql)
            .bind(DEVICE_LIST_LIMIT)
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::Query)
    }

    pub async fn update_metadata(
        &self,
        record: &DeviceRecord,
    ) -> Result<Option<DeviceRecord>, DatabaseError> {
        let sql = format!(
            r#"
            UPDATE devices
            SET
                device_name = $2,
                host = $3,
                port = $4,
                mac_address = $5,
                username = $6,
                updated_at = $7
            WHERE id = $1
            RETURNING {DEVICE_COLUMNS}
            "#
        );
        sqlx::query_as::<_, DeviceRecord>(&sql)
            .bind(record.id)
            .bind(&record.device_name)
            .bind(&record.host)
            .bind(record.port)
            .bind(&record.mac_address)
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
        let sql = format!(
            r#"
            UPDATE devices
            SET password_ciphertext = $2, updated_at = $3
            WHERE id = $1
            RETURNING {DEVICE_COLUMNS}
            "#
        );
        sqlx::query_as::<_, DeviceRecord>(&sql)
            .bind(id)
            .bind(password_ciphertext)
            .bind(updated_at)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::Query)
    }

    pub async fn update_status(
        &self,
        id: Uuid,
        status: &str,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<DeviceRecord>, DatabaseError> {
        let sql = format!(
            r#"
            UPDATE devices
            SET status = $2, updated_at = $3
            WHERE id = $1
            RETURNING {DEVICE_COLUMNS}
            "#
        );
        sqlx::query_as::<_, DeviceRecord>(&sql)
            .bind(id)
            .bind(status)
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
        let sql = format!(
            r#"
            UPDATE devices
            SET
                connection_status = $2,
                last_seen_at = $3,
                updated_at = $4
            WHERE id = $1
            RETURNING {DEVICE_COLUMNS}
            "#
        );
        sqlx::query_as::<_, DeviceRecord>(&sql)
            .bind(id)
            .bind(connection_status)
            .bind(last_seen_at)
            .bind(updated_at)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::Query)
    }
}
