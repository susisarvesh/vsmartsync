use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::models::DeviceUserRecord;
use crate::database::{DatabaseError, DbPool};

pub struct DeviceUserRepository {
    pool: DbPool,
}

impl DeviceUserRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_user_and_device(
        &self,
        user_id: Uuid,
        device_id: Uuid,
    ) -> Result<Option<DeviceUserRecord>, DatabaseError> {
        sqlx::query_as::<_, DeviceUserRecord>(
            r#"
            SELECT
                id, user_id, device_id, matrix_user_id, matrix_ref_user_id,
                provisioned_at, created_at, updated_at
            FROM device_users
            WHERE user_id = $1 AND device_id = $2
            "#,
        )
        .bind(user_id)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<DeviceUserRecord>, DatabaseError> {
        sqlx::query_as::<_, DeviceUserRecord>(
            r#"
            SELECT
                id, user_id, device_id, matrix_user_id, matrix_ref_user_id,
                provisioned_at, created_at, updated_at
            FROM device_users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn list_for_device(
        &self,
        device_id: Uuid,
    ) -> Result<Vec<DeviceUserRecord>, DatabaseError> {
        sqlx::query_as::<_, DeviceUserRecord>(
            r#"
            SELECT
                id, user_id, device_id, matrix_user_id, matrix_ref_user_id,
                provisioned_at, created_at, updated_at
            FROM device_users
            WHERE device_id = $1
            ORDER BY matrix_ref_user_id ASC
            "#,
        )
        .bind(device_id)
        .fetch_all(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    /// Atomically: return existing mapping, or lock sequence, allocate ids, insert.
    pub async fn ensure_mapping(
        &self,
        user_id: Uuid,
        device_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<DeviceUserRecord, DatabaseError> {
        let mut tx = self.pool.begin().await.map_err(DatabaseError::Query)?;

        if let Some(existing) = sqlx::query_as::<_, DeviceUserRecord>(
            r#"
            SELECT
                id, user_id, device_id, matrix_user_id, matrix_ref_user_id,
                provisioned_at, created_at, updated_at
            FROM device_users
            WHERE user_id = $1 AND device_id = $2
            "#,
        )
        .bind(user_id)
        .bind(device_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(DatabaseError::Query)?
        {
            tx.commit().await.map_err(DatabaseError::Query)?;
            return Ok(existing);
        }

        sqlx::query(
            r#"
            INSERT INTO device_id_sequences (device_id, next_matrix_user_seq, next_ref_user_id, updated_at)
            VALUES ($1, 1, 10000001, $2)
            ON CONFLICT (device_id) DO NOTHING
            "#,
        )
        .bind(device_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(DatabaseError::Query)?;

        let (user_seq, ref_id): (i64, i64) = sqlx::query_as(
            r#"
            SELECT next_matrix_user_seq, next_ref_user_id
            FROM device_id_sequences
            WHERE device_id = $1
            FOR UPDATE
            "#,
        )
        .bind(device_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(DatabaseError::Query)?;

        let matrix_user_id = format_matrix_user_id(user_seq).ok_or_else(|| {
            DatabaseError::Query(sqlx::Error::Protocol(
                "matrix user id sequence exhausted".into(),
            ))
        })?;
        if !(0..=99_999_999).contains(&ref_id) {
            return Err(DatabaseError::Query(sqlx::Error::Protocol(
                "matrix ref user id sequence exhausted".into(),
            )));
        }

        let next_user_seq = user_seq.checked_add(1).ok_or_else(|| {
            DatabaseError::Query(sqlx::Error::Protocol(
                "matrix user id sequence overflow".into(),
            ))
        })?;
        let next_ref = ref_id
            .checked_add(1)
            .filter(|v| *v <= 99_999_999)
            .ok_or_else(|| {
                DatabaseError::Query(sqlx::Error::Protocol(
                    "matrix ref user id sequence exhausted".into(),
                ))
            })?;

        sqlx::query(
            r#"
            UPDATE device_id_sequences
            SET next_matrix_user_seq = $2,
                next_ref_user_id = $3,
                updated_at = $4
            WHERE device_id = $1
            "#,
        )
        .bind(device_id)
        .bind(next_user_seq)
        .bind(next_ref)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(DatabaseError::Query)?;

        let id = Uuid::new_v4();
        let insert_result = sqlx::query(
            r#"
            INSERT INTO device_users (
                id, user_id, device_id, matrix_user_id, matrix_ref_user_id,
                provisioned_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, NULL, $6, $6)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(device_id)
        .bind(&matrix_user_id)
        .bind(ref_id)
        .bind(now)
        .execute(&mut *tx)
        .await;

        if let Err(error) = insert_result {
            let db_err = DatabaseError::Query(error);
            if db_err.is_unique_violation() {
                let existing = sqlx::query_as::<_, DeviceUserRecord>(
                    r#"
                    SELECT
                        id, user_id, device_id, matrix_user_id, matrix_ref_user_id,
                        provisioned_at, created_at, updated_at
                    FROM device_users
                    WHERE user_id = $1 AND device_id = $2
                    "#,
                )
                .bind(user_id)
                .bind(device_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(DatabaseError::Query)?;
                tx.commit().await.map_err(DatabaseError::Query)?;
                if let Some(row) = existing {
                    return Ok(row);
                }
                return Err(db_err);
            }
            return Err(db_err);
        }

        tx.commit().await.map_err(DatabaseError::Query)?;

        Ok(DeviceUserRecord {
            id,
            user_id,
            device_id,
            matrix_user_id,
            matrix_ref_user_id: ref_id,
            provisioned_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Sets `provisioned_at` on first successful provision; leaves existing value if already set.
    pub async fn mark_provisioned(
        &self,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Option<DeviceUserRecord>, DatabaseError> {
        sqlx::query_as::<_, DeviceUserRecord>(
            r#"
            UPDATE device_users
            SET provisioned_at = COALESCE(provisioned_at, $2),
                updated_at = $2
            WHERE id = $1
            RETURNING
                id, user_id, device_id, matrix_user_id, matrix_ref_user_id,
                provisioned_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }
}

/// `VS` + zero-padded decimal sequence, max 15 characters total.
pub fn format_matrix_user_id(seq: i64) -> Option<String> {
    if seq < 1 {
        return None;
    }
    let id = if seq <= 999_999 {
        format!("VS{seq:06}")
    } else {
        format!("VS{seq}")
    };
    if id.len() > 15 || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(id)
}

#[cfg(test)]
mod format_tests {
    use super::format_matrix_user_id;

    #[test]
    fn formats_vs_padded_ids() {
        assert_eq!(format_matrix_user_id(1).as_deref(), Some("VS000001"));
        assert_eq!(format_matrix_user_id(42).as_deref(), Some("VS000042"));
        assert_eq!(format_matrix_user_id(999_999).as_deref(), Some("VS999999"));
        assert_eq!(
            format_matrix_user_id(1_000_000).as_deref(),
            Some("VS1000000")
        );
        assert_eq!(format_matrix_user_id(0), None);
    }
}
