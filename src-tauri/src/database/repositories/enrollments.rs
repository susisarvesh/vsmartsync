use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::models::{EnrollmentRecord, EnrollmentWriteRecord};
use crate::database::{DatabaseError, DbPool};

pub const ENROLLMENT_LIST_LIMIT: i64 = 200;
pub const ENROLLMENT_LIST_MAX_LIMIT: i64 = 500;

#[derive(Debug, Clone, Default)]
pub struct EnrollmentListQuery {
    pub user_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub credential_id: Option<Uuid>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub struct EnrollmentRepository {
    pool: DbPool,
}

impl EnrollmentRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, record: &EnrollmentWriteRecord) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO enrollments (
                id, user_id, credential_id, device_id, status,
                cancelled_at, revoked_at, activated_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(record.id)
        .bind(record.user_id)
        .bind(record.credential_id)
        .bind(record.device_id)
        .bind(&record.status)
        .bind(record.cancelled_at)
        .bind(record.revoked_at)
        .bind(record.activated_at)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<EnrollmentRecord>, DatabaseError> {
        sqlx::query_as::<_, EnrollmentRecord>(
            r#"
            SELECT
                e.id,
                e.user_id,
                u.name AS user_name,
                e.credential_id,
                c.type AS credential_type,
                c.display_hint,
                e.device_id,
                d.name AS device_name,
                e.status,
                e.cancelled_at,
                e.revoked_at,
                e.activated_at,
                e.created_at,
                e.updated_at
            FROM enrollments e
            INNER JOIN users u ON u.id = e.user_id
            INNER JOIN credentials c ON c.id = e.credential_id
            INNER JOIN devices d ON d.id = e.device_id
            WHERE e.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn list(
        &self,
        query: &EnrollmentListQuery,
    ) -> Result<Vec<EnrollmentRecord>, DatabaseError> {
        let limit = query
            .limit
            .unwrap_or(ENROLLMENT_LIST_LIMIT)
            .clamp(1, ENROLLMENT_LIST_MAX_LIMIT);
        let offset = query.offset.unwrap_or(0).max(0);

        sqlx::query_as::<_, EnrollmentRecord>(
            r#"
            SELECT
                e.id,
                e.user_id,
                u.name AS user_name,
                e.credential_id,
                c.type AS credential_type,
                c.display_hint,
                e.device_id,
                d.name AS device_name,
                e.status,
                e.cancelled_at,
                e.revoked_at,
                e.activated_at,
                e.created_at,
                e.updated_at
            FROM enrollments e
            INNER JOIN users u ON u.id = e.user_id
            INNER JOIN credentials c ON c.id = e.credential_id
            INNER JOIN devices d ON d.id = e.device_id
            WHERE ($1::uuid IS NULL OR e.user_id = $1)
              AND ($2::uuid IS NULL OR e.device_id = $2)
              AND ($3::uuid IS NULL OR e.credential_id = $3)
              AND ($4::text IS NULL OR e.status = $4)
            ORDER BY e.created_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(query.user_id)
        .bind(query.device_id)
        .bind(query.credential_id)
        .bind(query.status.as_deref())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn update_status(
        &self,
        id: Uuid,
        status: &str,
        activated_at: Option<DateTime<Utc>>,
        cancelled_at: Option<DateTime<Utc>>,
        revoked_at: Option<DateTime<Utc>>,
        updated_at: DateTime<Utc>,
    ) -> Result<Option<EnrollmentRecord>, DatabaseError> {
        sqlx::query(
            r#"
            UPDATE enrollments
            SET
                status = $2,
                activated_at = $3,
                cancelled_at = $4,
                revoked_at = $5,
                updated_at = $6
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(activated_at)
        .bind(cancelled_at)
        .bind(revoked_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;

        self.find_by_id(id).await
    }
}
