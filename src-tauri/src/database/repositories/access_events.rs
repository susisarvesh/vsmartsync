use chrono::{DateTime, Utc};
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

    /// Insert one HTTP batch. A sequence already stored for this device is skipped.
    pub async fn insert_ignore_batch(
        &self,
        records: &[AccessEventWrite],
    ) -> Result<u64, DatabaseError> {
        let mut tx = self.pool.begin().await.map_err(DatabaseError::Query)?;
        let inserted = insert_rows(&mut tx, records).await?;
        tx.commit().await.map_err(DatabaseError::Query)?;
        Ok(inserted)
    }

    /// Store a batch and its checkpoint together. The checkpoint does not move
    /// when the insert fails or the process stops before commit.
    pub async fn persist_batch_and_cursor(
        &self,
        records: &[AccessEventWrite],
        device_id: Uuid,
        roll_over_count: i32,
        seq_number: i64,
        updated_at: DateTime<Utc>,
    ) -> Result<u64, DatabaseError> {
        let mut tx = self.pool.begin().await.map_err(DatabaseError::Query)?;
        let inserted = insert_rows(&mut tx, records).await?;
        write_cursor(&mut tx, device_id, roll_over_count, seq_number, updated_at).await?;
        tx.commit().await.map_err(DatabaseError::Query)?;
        Ok(inserted)
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

async fn insert_rows(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    records: &[AccessEventWrite],
) -> Result<u64, DatabaseError> {
    if records.is_empty() {
        return Ok(0);
    }
    let ids: Vec<Uuid> = records.iter().map(|record| record.id).collect();
    let device_ids: Vec<Uuid> = records.iter().map(|record| record.device_id).collect();
    let device_names: Vec<&str> = records
        .iter()
        .map(|record| record.device_name.as_str())
        .collect();
    let roll_overs: Vec<i32> = records
        .iter()
        .map(|record| record.roll_over_count)
        .collect();
    let sequences: Vec<i64> = records.iter().map(|record| record.seq_number).collect();
    let event_ids: Vec<Option<&str>> = records
        .iter()
        .map(|record| record.event_id.as_deref())
        .collect();
    let device_dates: Vec<&str> = records
        .iter()
        .map(|record| record.device_date.as_str())
        .collect();
    let device_times: Vec<&str> = records
        .iter()
        .map(|record| record.device_time.as_str())
        .collect();
    let user_ids: Vec<Option<Uuid>> = records.iter().map(|record| record.user_id).collect();
    let person_names: Vec<Option<&str>> = records
        .iter()
        .map(|record| record.person_name.as_deref())
        .collect();
    let ref_user_ids: Vec<Option<i64>> = records.iter().map(|record| record.ref_user_id).collect();
    let detail_1: Vec<Option<&str>> = records
        .iter()
        .map(|record| record.detail_1.as_deref())
        .collect();
    let detail_2: Vec<Option<&str>> = records
        .iter()
        .map(|record| record.detail_2.as_deref())
        .collect();
    let detail_3: Vec<Option<&str>> = records
        .iter()
        .map(|record| record.detail_3.as_deref())
        .collect();
    let detail_4: Vec<Option<&str>> = records
        .iter()
        .map(|record| record.detail_4.as_deref())
        .collect();
    let detail_5: Vec<Option<&str>> = records
        .iter()
        .map(|record| record.detail_5.as_deref())
        .collect();
    let ingested_at: Vec<DateTime<Utc>> = records.iter().map(|record| record.ingested_at).collect();
    let inserted = sqlx::query(
        r#"
            INSERT INTO events (
                id, device_id, device_name, roll_over_count, seq_number, event_id,
                device_date, device_time, user_id, person_name, ref_user_id,
                detail_1, detail_2, detail_3, detail_4, detail_5, ingested_at
            )
            SELECT * FROM UNNEST(
                $1::uuid[],
                $2::uuid[],
                $3::text[],
                $4::int4[],
                $5::int8[],
                $6::text[],
                $7::text[],
                $8::text[],
                $9::uuid[],
                $10::text[],
                $11::int8[],
                $12::text[],
                $13::text[],
                $14::text[],
                $15::text[],
                $16::text[],
                $17::timestamptz[]
            )
            ON CONFLICT (device_id, roll_over_count, seq_number) DO NOTHING
            "#,
    )
    .bind(&ids)
    .bind(&device_ids)
    .bind(&device_names)
    .bind(&roll_overs)
    .bind(&sequences)
    .bind(&event_ids)
    .bind(&device_dates)
    .bind(&device_times)
    .bind(&user_ids)
    .bind(&person_names)
    .bind(&ref_user_ids)
    .bind(&detail_1)
    .bind(&detail_2)
    .bind(&detail_3)
    .bind(&detail_4)
    .bind(&detail_5)
    .bind(&ingested_at)
    .execute(&mut **tx)
    .await
    .map_err(DatabaseError::Query)?;
    Ok(inserted.rows_affected())
}

async fn write_cursor(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    device_id: Uuid,
    roll_over_count: i32,
    seq_number: i64,
    updated_at: DateTime<Utc>,
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
    .execute(&mut **tx)
    .await
    .map_err(DatabaseError::Query)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::AccessEventRepository;
    use crate::database::models::AccessEventWrite;
    use chrono::Utc;
    use uuid::Uuid;

    /// A failed insert must not leave a newer checkpoint. The row created here is deleted
    /// before the test returns. Not part of `cargo test`.
    #[tokio::test]
    #[ignore = "requires an isolated PostgreSQL database named in TEST_DATABASE_URL"]
    async fn failed_event_insert_does_not_advance_the_cursor() {
        crate::database::run_isolated_test(|db| async move {
        let pool = db.pool();
        let device_id = Uuid::new_v4();
        db.track_device(device_id);
        let now = Utc::now();
        let port = i32::from(device_id.as_bytes()[0]) + 20_000;
        sqlx::query(
            r#"
            INSERT INTO devices (
                id, device_name, host, port, username, password_ciphertext,
                status, connection_status, created_at, updated_at
            )
            VALUES ($1, 'Checkpoint Probe', '203.0.113.10', $2, 'probe', $3, 'inactive', 'unknown', $4, $4)
            "#,
        )
        .bind(device_id)
        .bind(port)
        .bind(vec![0_u8; 32])
        .bind(now)
        .execute(&pool)
        .await
        .unwrap_or_else(|_| panic!("could not insert the probe device"));

        let events = AccessEventRepository::new(pool.clone());
        let outcome: Result<(), String> = async {
            events
                .save_cursor(device_id, 1, 10, now)
                .await
                .map_err(|error| error.to_string())?;
            let missing_user = AccessEventWrite {
                id: Uuid::new_v4(),
                device_id,
                device_name: "Checkpoint Probe".to_string(),
                roll_over_count: 1,
                seq_number: 11,
                event_id: None,
                device_date: "01012026".to_string(),
                device_time: "000000".to_string(),
                user_id: Some(Uuid::new_v4()),
                person_name: None,
                ref_user_id: None,
                detail_1: None,
                detail_2: None,
                detail_3: None,
                detail_4: None,
                detail_5: None,
                ingested_at: now,
            };
            let failed = events
                .persist_batch_and_cursor(&[missing_user], device_id, 1, 11, now)
                .await;
            if failed.is_ok() {
                return Err("a missing user must roll the write back".to_string());
            }
            let cursor = events
                .cursor(device_id)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "cursor missing".to_string())?;
            if cursor.roll_over_count != 1 || cursor.seq_number != 10 {
                return Err("cursor advanced after a failed insert".to_string());
            }
            let stored: i64 =
                sqlx::query_scalar("SELECT count(*) FROM events WHERE device_id = $1")
                    .bind(device_id)
                    .fetch_one(&pool)
                    .await
                    .map_err(|error| error.to_string())?;
            if stored != 0 {
                return Err("failed insert left an event".to_string());
            }

            let event = AccessEventWrite {
                id: Uuid::new_v4(),
                user_id: None,
                seq_number: 11,
                ..missing_user_placeholder(device_id, now)
            };
            let inserted = events
                .persist_batch_and_cursor(std::slice::from_ref(&event), device_id, 1, 11, now)
                .await
                .map_err(|error| error.to_string())?;
            if inserted != 1 {
                return Err("first event was not stored".to_string());
            }
            let again = events
                .persist_batch_and_cursor(std::slice::from_ref(&event), device_id, 1, 11, now)
                .await
                .map_err(|error| error.to_string())?;
            if again != 0 {
                return Err("duplicate event was stored again".to_string());
            }
            let cursor = events
                .cursor(device_id)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "cursor missing after insert".to_string())?;
            if cursor.seq_number != 11 {
                return Err("cursor did not stay on the stored event".to_string());
            }
            let stored: i64 =
                sqlx::query_scalar("SELECT count(*) FROM events WHERE device_id = $1")
                    .bind(device_id)
                    .fetch_one(&pool)
                    .await
                    .map_err(|error| error.to_string())?;
            if stored != 1 {
                return Err("duplicate changed the stored event count".to_string());
            }
            Ok(())
        }
        .await;

        outcome.unwrap_or_else(|error| panic!("checkpoint check failed: {error}"));
        })
        .await;
    }

    fn missing_user_placeholder(device_id: Uuid, now: chrono::DateTime<Utc>) -> AccessEventWrite {
        AccessEventWrite {
            id: Uuid::new_v4(),
            device_id,
            device_name: "Checkpoint Probe".to_string(),
            roll_over_count: 1,
            seq_number: 11,
            event_id: None,
            device_date: "01012026".to_string(),
            device_time: "000000".to_string(),
            user_id: None,
            person_name: None,
            ref_user_id: None,
            detail_1: None,
            detail_2: None,
            detail_3: None,
            detail_4: None,
            detail_5: None,
            ingested_at: now,
        }
    }
}
