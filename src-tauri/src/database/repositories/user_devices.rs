use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::models::{UserDeviceRecord, UserOnDeviceRecord};
use crate::database::{DatabaseError, DbPool};

pub const USER_DEVICE_LIST_LIMIT: i64 = 200;

const ASSIGNMENT_COLUMNS: &str = "
    a.id, a.user_id, a.device_id, d.device_name, d.host, d.port, a.created_at
";

pub struct UserDeviceRepository {
    pool: DbPool,
}

impl UserDeviceRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        id: Uuid,
        user_id: Uuid,
        device_id: Uuid,
        created_at: DateTime<Utc>,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO user_devices (id, user_id, device_id, created_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(device_id)
        .bind(created_at)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(())
    }

    pub async fn list_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserDeviceRecord>, DatabaseError> {
        let sql = format!(
            r#"
            SELECT {ASSIGNMENT_COLUMNS}
            FROM user_devices a
            INNER JOIN devices d ON d.id = a.device_id
            WHERE a.user_id = $1
            ORDER BY d.device_name, d.host, d.port
            LIMIT $2
            "#
        );
        sqlx::query_as::<_, UserDeviceRecord>(&sql)
            .bind(user_id)
            .bind(USER_DEVICE_LIST_LIMIT)
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::Query)
    }

    pub async fn list_for_device(
        &self,
        device_id: Uuid,
    ) -> Result<Vec<UserOnDeviceRecord>, DatabaseError> {
        let sql = format!(
            r#"
            SELECT
                a.id,
                a.user_id,
                a.device_id,
                u.username,
                u.status,
                a.created_at,
                du.provisioned_at
            FROM user_devices a
            INNER JOIN users u ON u.id = a.user_id
            LEFT JOIN device_users du
                ON du.user_id = a.user_id AND du.device_id = a.device_id
            WHERE a.device_id = $1
            ORDER BY u.username
            LIMIT {USER_DEVICE_LIST_LIMIT}
            "#
        );
        sqlx::query_as::<_, UserOnDeviceRecord>(&sql)
            .bind(device_id)
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::Query)
    }

    pub async fn contains_pair(
        &self,
        user_id: Uuid,
        device_id: Uuid,
    ) -> Result<bool, DatabaseError> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM user_devices WHERE user_id = $1 AND device_id = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(device_id)
        .fetch_one(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(exists)
    }

    pub async fn delete_pair(&self, user_id: Uuid, device_id: Uuid) -> Result<bool, DatabaseError> {
        let result = sqlx::query(
            r#"
            DELETE FROM user_devices
            WHERE user_id = $1 AND device_id = $2
            "#,
        )
        .bind(user_id)
        .bind(device_id)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_for_user(&self, user_id: Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM user_devices WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::Query)?;
        Ok(())
    }
}
