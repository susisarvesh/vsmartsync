use uuid::Uuid;

use crate::database::models::{AccessEventRecord, AccessEventWrite, DeviceEventCursorRecord};
use crate::database::{DatabaseError, DbPool};

pub const ACCESS_EVENT_LIST_LIMIT: i64 = 200;

const EVENT_COLUMNS: &str = "
    id, device_id, device_name, roll_over_count, seq_number, event_id,
    device_date, device_time, user_id, person_name, ref_user_id,
    detail_1, detail_2, detail_3, detail_4, detail_5, ingested_at
";

pub struct AccessEventRepository {
    pool: DbPool,
}

impl AccessEventRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn cursor(
        &self,
        device_id: Uuid,
    ) -> Result<Option<DeviceEventCursorRecord>, DatabaseError> {
        sqlx::query_as::<_, DeviceEventCursorRecord>(
            r#"
            SELECT device_id, roll_over_count, seq_number
            FROM device_event_cursors
            WHERE device_id = $1
            "#,
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn save_cursor(
        &self,
        device_id: Uuid,
        roll_over_count: i32,
        seq_number: i64,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO device_event_cursors (device_id, roll_over_count, seq_number, updated_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (device_id) DO UPDATE
            SET roll_over_count = EXCLUDED.roll_over_count,
                seq_number = EXCLUDED.seq_number,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(device_id)
        .bind(roll_over_count)
        .bind(seq_number)
        .bind(updated_at)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(())
    }

    /// Insert one event. `false` means this device sequence was already stored.
    pub async fn insert_ignore(&self, record: &AccessEventWrite) -> Result<bool, DatabaseError> {
        let inserted = sqlx::query(
            r#"
            INSERT INTO events (
                id, device_id, device_name, roll_over_count, seq_number, event_id,
                device_date, device_time, user_id, person_name, ref_user_id,
                detail_1, detail_2, detail_3, detail_4, detail_5, ingested_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10, $11,
                $12, $13, $14, $15, $16, $17
            )
            ON CONFLICT (device_id, roll_over_count, seq_number) DO NOTHING
            "#,
        )
        .bind(record.id)
        .bind(record.device_id)
        .bind(&record.device_name)
        .bind(record.roll_over_count)
        .bind(record.seq_number)
        .bind(&record.event_id)
        .bind(&record.device_date)
        .bind(&record.device_time)
        .bind(record.user_id)
        .bind(&record.person_name)
        .bind(record.ref_user_id)
        .bind(&record.detail_1)
        .bind(&record.detail_2)
        .bind(&record.detail_3)
        .bind(&record.detail_4)
        .bind(&record.detail_5)
        .bind(record.ingested_at)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(inserted.rows_affected() > 0)
    }

    /// Copy the local username onto user events that stored the reference id in field 1.
    /// A name already saved on the row is left as it was.
    pub async fn fill_missing_person_names(
        &self,
        device_id: Option<Uuid>,
        event_ids: &[String],
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE events AS stored
            SET ref_user_id = mapping.matrix_ref_user_id,
                user_id = mapping.user_id,
                person_name = person.username
            FROM device_users AS mapping
            JOIN users AS person ON person.id = mapping.user_id
            WHERE stored.person_name IS NULL
              AND stored.device_id = mapping.device_id
              AND stored.event_id = ANY($2::text[])
              AND stored.detail_1 ~ '^[0-9]{1,8}$'
              AND mapping.matrix_ref_user_id = stored.detail_1::bigint
              AND ($1::uuid IS NULL OR stored.device_id = $1)
            "#,
        )
        .bind(device_id)
        .bind(event_ids)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(())
    }

    pub async fn list(
        &self,
        device_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<AccessEventRecord>, DatabaseError> {
        let limit = limit.clamp(1, ACCESS_EVENT_LIST_LIMIT);
        let sql = format!(
            r#"
            SELECT {EVENT_COLUMNS}
            FROM events
            WHERE ($1::uuid IS NULL OR device_id = $1)
            ORDER BY roll_over_count DESC, seq_number DESC
            LIMIT $2
            "#
        );
        sqlx::query_as::<_, AccessEventRecord>(&sql)
            .bind(device_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::Query)
    }
}
