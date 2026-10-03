use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::models::{
    CredentialWriteRecord, EnrollmentSessionRecord, EnrollmentSessionWriteRecord,
    EnrollmentWriteRecord,
};
use crate::database::{DatabaseError, DbPool};

const SESSION_COLUMNS: &str = r#"
    s.id,
    s.user_id,
    u.username AS user_name,
    s.device_id,
    d.device_name AS device_name,
    s.enroll_type,
    s.status,
    s.credential_id,
    s.error_code,
    s.card_type,
    s.identifier_type,
    s.identifier_unavailable,
    s.started_at,
    s.completed_at,
    s.created_at,
    s.updated_at
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishCapture {
    Saved(Uuid),
    Skipped,
}

pub struct EnrollmentSessionRepository {
    pool: DbPool,
}

impl EnrollmentSessionRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, record: &EnrollmentSessionWriteRecord) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO enrollment_sessions (
                id, user_id, device_id, enroll_type, status,
                started_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(record.id)
        .bind(record.user_id)
        .bind(record.device_id)
        .bind(&record.enroll_type)
        .bind(&record.status)
        .bind(record.started_at)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(())
    }

    pub async fn find_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<EnrollmentSessionRecord>, DatabaseError> {
        let sql = format!(
            r#"
            SELECT {SESSION_COLUMNS}
            FROM enrollment_sessions s
            INNER JOIN users u ON u.id = s.user_id
            INNER JOIN devices d ON d.id = s.device_id
            WHERE s.id = $1
            "#
        );
        sqlx::query_as::<_, EnrollmentSessionRecord>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::Query)
    }

    /// Move a session that is still open. A cancel or terminal status wins.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_if_open(
        &self,
        id: Uuid,
        status: &str,
        error_code: Option<&str>,
        card_type: Option<&str>,
        identifier_type: Option<&str>,
        identifier_unavailable: bool,
        credential_id: Option<Uuid>,
        completed_at: Option<DateTime<Utc>>,
        updated_at: DateTime<Utc>,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query(
            r#"
            UPDATE enrollment_sessions
            SET
                status = $2,
                error_code = $3,
                card_type = COALESCE($4, card_type),
                identifier_type = COALESCE($5, identifier_type),
                identifier_unavailable = $6,
                credential_id = COALESCE($7, credential_id),
                completed_at = COALESCE($8, completed_at),
                updated_at = $9
            WHERE id = $1
              AND status IN (
                  'starting',
                  'waiting_for_card',
                  'processing',
                  'verifying',
                  'saving'
              )
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(error_code)
        .bind(card_type)
        .bind(identifier_type)
        .bind(identifier_unavailable)
        .bind(credential_id)
        .bind(completed_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(result.rows_affected() > 0)
    }

    /// Insert the credential and assignment only if the session was not cancelled.
    ///
    /// The transaction stays closed around the physical card wait. It covers
    /// the save that follows a completed device capture.
    #[allow(clippy::too_many_arguments)]
    pub async fn finish_capture(
        &self,
        session_id: Uuid,
        credential: &CredentialWriteRecord,
        enrollment: &EnrollmentWriteRecord,
        card_type: Option<&str>,
        identifier_type: Option<&str>,
        identifier_unavailable: bool,
        updated_at: DateTime<Utc>,
    ) -> Result<FinishCapture, DatabaseError> {
        let mut tx = self.pool.begin().await.map_err(DatabaseError::Query)?;
        let status = sqlx::query_scalar::<_, String>(
            "SELECT status FROM enrollment_sessions WHERE id = $1 FOR UPDATE",
        )
        .bind(session_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(DatabaseError::Query)?;
        if !status.as_deref().is_some_and(session_is_open) {
            tx.rollback().await.map_err(DatabaseError::Query)?;
            return Ok(FinishCapture::Skipped);
        }

        sqlx::query(
            r#"
            INSERT INTO credentials (
                id, user_id, type, value_ciphertext, value_digest,
                display_hint, card_type, identifier_type, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(credential.id)
        .bind(credential.user_id)
        .bind(&credential.credential_type)
        .bind(&credential.value_ciphertext)
        .bind(&credential.value_digest)
        .bind(&credential.display_hint)
        .bind(&credential.card_type)
        .bind(&credential.identifier_type)
        .bind(&credential.status)
        .bind(credential.created_at)
        .bind(credential.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(DatabaseError::Query)?;

        sqlx::query(
            r#"
            INSERT INTO enrollments (
                id, user_id, credential_id, device_id, status,
                cancelled_at, revoked_at, activated_at, identifier_digest,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(enrollment.id)
        .bind(enrollment.user_id)
        .bind(enrollment.credential_id)
        .bind(enrollment.device_id)
        .bind(&enrollment.status)
        .bind(enrollment.cancelled_at)
        .bind(enrollment.revoked_at)
        .bind(enrollment.activated_at)
        .bind(&enrollment.identifier_digest)
        .bind(enrollment.created_at)
        .bind(enrollment.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(DatabaseError::Query)?;

        sqlx::query(
            r#"
            UPDATE enrollment_sessions
            SET
                status = 'success',
                credential_id = $2,
                error_code = NULL,
                card_type = $3,
                identifier_type = $4,
                identifier_unavailable = $5,
                completed_at = $6,
                updated_at = $6
            WHERE id = $1
            "#,
        )
        .bind(session_id)
        .bind(credential.id)
        .bind(card_type)
        .bind(identifier_type)
        .bind(identifier_unavailable)
        .bind(updated_at)
        .execute(&mut *tx)
        .await
        .map_err(DatabaseError::Query)?;

        tx.commit().await.map_err(DatabaseError::Query)?;
        Ok(FinishCapture::Saved(enrollment.id))
    }

    pub async fn delete_for_user(&self, user_id: Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM enrollment_sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::Query)?;
        Ok(())
    }
}

fn session_is_open(status: &str) -> bool {
    matches!(
        status,
        "starting" | "waiting_for_card" | "processing" | "verifying" | "saving"
    )
}
