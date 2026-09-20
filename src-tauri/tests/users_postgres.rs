//! Live PostgreSQL checks for the Users slice.
//!
//! Ignored by default so `cargo test` stays offline-friendly.

use uuid::Uuid;
use vsmart_sync_lib::database::repositories::UserRepository;
use vsmart_sync_lib::database::{connect_and_migrate, DatabaseConfig};
use vsmart_sync_lib::domains::users::{
    create_user, deactivate_user, list_users, update_user_name, UserError, UserStatus,
};

fn load_test_config() -> DatabaseConfig {
    DatabaseConfig::from_url(std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://vsmart_sync:change-me@127.0.0.1:5432/vsmart_sync".to_string()
    }))
    .expect("test database url")
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn users_create_list_update_deactivate() {
    let pool = connect_and_migrate(&load_test_config())
        .await
        .expect("postgresql should be reachable");
    let repo = UserRepository::new(pool);

    let created = create_user(&repo, "  Integration User  ")
        .await
        .expect("create");
    assert_eq!(created.name, "Integration User");
    assert_eq!(created.status, UserStatus::Active);

    let listed = list_users(&repo).await.expect("list");
    assert!(listed.iter().any(|user| user.id == created.id));

    let renamed = update_user_name(&repo, created.id, "Renamed User")
        .await
        .expect("update");
    assert_eq!(renamed.name, "Renamed User");
    assert_eq!(renamed.status, UserStatus::Active);

    let deactivated = deactivate_user(&repo, created.id)
        .await
        .expect("deactivate");
    assert_eq!(deactivated.status, UserStatus::Inactive);

    let again = deactivate_user(&repo, created.id)
        .await
        .expect("idempotent");
    assert_eq!(again.status, UserStatus::Inactive);
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn users_reject_invalid_name_and_missing_id() {
    let pool = connect_and_migrate(&load_test_config())
        .await
        .expect("postgresql should be reachable");
    let repo = UserRepository::new(pool);

    assert_eq!(
        create_user(&repo, "   ").await.unwrap_err(),
        UserError::InvalidName
    );
    assert_eq!(
        update_user_name(&repo, Uuid::nil(), "Ada")
            .await
            .unwrap_err(),
        UserError::NotFound
    );
    assert_eq!(
        deactivate_user(&repo, Uuid::nil()).await.unwrap_err(),
        UserError::NotFound
    );
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn users_update_does_not_accept_status_from_caller() {
    let pool = connect_and_migrate(&load_test_config())
        .await
        .expect("postgresql should be reachable");
    let repo = UserRepository::new(pool);
    let created = create_user(&repo, &format!("Status Guard {}", Uuid::new_v4()))
        .await
        .expect("create");
    let updated = update_user_name(&repo, created.id, "Still Active")
        .await
        .expect("update");
    assert_eq!(updated.status, UserStatus::Active);
}
