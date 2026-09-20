use uuid::Uuid;

use crate::database::models::UserRecord;
use crate::database::{DatabaseError, DbPool};

pub const USER_LIST_LIMIT: i64 = 200;

pub struct UserRepository {
    pool: DbPool,
}

impl UserRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, record: &UserRecord) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO users (id, name, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(record.id)
        .bind(&record.name)
        .bind(&record.status)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<UserRecord>, DatabaseError> {
        sqlx::query_as::<_, UserRecord>(
            r#"
            SELECT id, name, status, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn list(&self) -> Result<Vec<UserRecord>, DatabaseError> {
        sqlx::query_as::<_, UserRecord>(
            r#"
            SELECT id, name, status, created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(USER_LIST_LIMIT)
        .fetch_all(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn save(&self, record: &UserRecord) -> Result<Option<UserRecord>, DatabaseError> {
        sqlx::query_as::<_, UserRecord>(
            r#"
            UPDATE users
            SET name = $2, status = $3, updated_at = $4
            WHERE id = $1
            RETURNING id, name, status, created_at, updated_at
            "#,
        )
        .bind(record.id)
        .bind(&record.name)
        .bind(&record.status)
        .bind(record.updated_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }
}
