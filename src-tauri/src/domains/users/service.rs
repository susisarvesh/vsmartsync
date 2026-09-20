use chrono::Utc;
use uuid::Uuid;

use crate::database::models::UserRecord;
use crate::database::repositories::UserRepository;
use crate::database::DatabaseError;

use super::{deactivate, new_user, rename_user, User, UserError, UserStatus};

pub async fn create_user(repo: &UserRepository, name: &str) -> Result<User, UserError> {
    let user = new_user(name, Uuid::new_v4(), Utc::now())?;
    repo.insert(&to_record(&user)).await.map_err(map_db_error)?;
    tracing::info!(user_id = %user.id, command = "create_user", "created user");
    Ok(user)
}

pub async fn list_users(repo: &UserRepository) -> Result<Vec<User>, UserError> {
    repo.list()
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(from_record)
        .collect()
}

pub async fn update_user_name(
    repo: &UserRepository,
    id: Uuid,
    name: &str,
) -> Result<User, UserError> {
    let user = load_user(repo, id).await?;
    let user = rename_user(user, name, Utc::now())?;
    save_user(repo, &user).await
}

pub async fn deactivate_user(repo: &UserRepository, id: Uuid) -> Result<User, UserError> {
    let user = load_user(repo, id).await?;
    let user = deactivate(user, Utc::now());
    save_user(repo, &user).await
}

async fn load_user(repo: &UserRepository, id: Uuid) -> Result<User, UserError> {
    let record = repo
        .find_by_id(id)
        .await
        .map_err(map_db_error)?
        .ok_or(UserError::NotFound)?;
    from_record(record)
}

async fn save_user(repo: &UserRepository, user: &User) -> Result<User, UserError> {
    let saved = repo
        .save(&to_record(user))
        .await
        .map_err(map_db_error)?
        .ok_or(UserError::NotFound)?;
    tracing::info!(user_id = %user.id, "saved user");
    from_record(saved)
}

fn to_record(user: &User) -> UserRecord {
    UserRecord {
        id: user.id,
        name: user.name.clone(),
        status: user.status.as_str().to_string(),
        created_at: user.created_at,
        updated_at: user.updated_at,
    }
}

fn from_record(record: UserRecord) -> Result<User, UserError> {
    Ok(User {
        id: record.id,
        name: record.name,
        status: UserStatus::parse(&record.status)?,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn map_db_error(error: DatabaseError) -> UserError {
    tracing::error!(error = %error, "users persistence failed");
    UserError::Unavailable
}
