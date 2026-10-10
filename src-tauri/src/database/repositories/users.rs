use uuid::Uuid;

use crate::database::models::UserRecord;
use crate::database::{DatabaseError, DbPool};

pub const USER_LIST_LIMIT: i64 = 200;

const USER_COLUMNS: &str = "
    id, username, status, matrix_user_id, short_name, full_name, reference_id,
    created_at, updated_at
";

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
            INSERT INTO users (
                id, username, status, matrix_user_id, short_name, full_name, reference_id,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(record.id)
        .bind(&record.username)
        .bind(&record.status)
        .bind(&record.matrix_user_id)
        .bind(&record.short_name)
        .bind(&record.full_name)
        .bind(record.reference_id)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::Query)?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<UserRecord>, DatabaseError> {
        let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = $1");
        sqlx::query_as::<_, UserRecord>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::Query)
    }

    pub async fn list(&self) -> Result<Vec<UserRecord>, DatabaseError> {
        let sql = format!("SELECT {USER_COLUMNS} FROM users ORDER BY created_at DESC LIMIT $1");
        sqlx::query_as::<_, UserRecord>(&sql)
            .bind(USER_LIST_LIMIT)
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::Query)
    }

    pub async fn occupied_import_keys(
        &self,
        matrix_user_ids: &[String],
        reference_ids: &[i64],
    ) -> Result<Vec<(Option<String>, Option<i64>)>, DatabaseError> {
        sqlx::query_as(
            r#"
            SELECT matrix_user_id, reference_id
            FROM users
            WHERE matrix_user_id = ANY($1)
               OR reference_id = ANY($2)
            "#,
        )
        .bind(matrix_user_ids)
        .bind(reference_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(DatabaseError::Query)
    }

    pub async fn insert_all(&self, records: &[UserRecord]) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await.map_err(DatabaseError::Query)?;
        for record in records {
            sqlx::query(
                r#"
                INSERT INTO users (
                    id, username, status, matrix_user_id, short_name, full_name, reference_id,
                    created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(record.id)
            .bind(&record.username)
            .bind(&record.status)
            .bind(&record.matrix_user_id)
            .bind(&record.short_name)
            .bind(&record.full_name)
            .bind(record.reference_id)
            .bind(record.created_at)
            .bind(record.updated_at)
            .execute(&mut *tx)
            .await
            .map_err(DatabaseError::Query)?;
        }
        tx.commit().await.map_err(DatabaseError::Query)?;
        Ok(())
    }

    pub async fn save(&self, record: &UserRecord) -> Result<Option<UserRecord>, DatabaseError> {
        let sql = format!(
            r#"
            UPDATE users
            SET username = $2,
                status = $3,
                matrix_user_id = $4,
                short_name = $5,
                full_name = $6,
                reference_id = $7,
                updated_at = $8
            WHERE id = $1
            RETURNING {USER_COLUMNS}
            "#
        );
        sqlx::query_as::<_, UserRecord>(&sql)
            .bind(record.id)
            .bind(&record.username)
            .bind(&record.status)
            .bind(&record.matrix_user_id)
            .bind(&record.short_name)
            .bind(&record.full_name)
            .bind(record.reference_id)
            .bind(record.updated_at)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::Query)
    }

    pub async fn delete(&self, id: Uuid) -> Result<bool, DatabaseError> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::Query)?;
        Ok(result.rows_affected() > 0)
    }
}
