//! Live PostgreSQL checks for the Users slice.
//!
//! Ignored by default so `cargo test` stays offline-friendly.

use uuid::Uuid;
use vsmart_sync_lib::database::repositories::UserRepository;
use vsmart_sync_lib::database::run_isolated_test;
use vsmart_sync_lib::domains::users::{
    activate_user, create_user, deactivate_user, list_users, update_user, UserError, UserStatus,
};

#[tokio::test]
#[ignore = "requires an isolated PostgreSQL database named in TEST_DATABASE_URL"]
async fn users_create_list_update_activate_deactivate() {
    run_isolated_test(|db| async move {
        let pool = db.pool();
        let repo = UserRepository::new(pool);

        let created = create_user(&repo, "  Integration User  ")
            .await
            .expect("create");
        db.track_user(created.id);
        assert_eq!(created.username, "Integration User");
        assert_eq!(created.status, UserStatus::Active);

        let listed = list_users(&repo).await.expect("list");
        assert!(listed.iter().any(|user| user.id == created.id));

        let renamed = update_user(&repo, created.id, "Renamed User")
            .await
            .expect("update");
        assert_eq!(renamed.username, "Renamed User");
        assert_eq!(renamed.status, UserStatus::Active);

        let deactivated = deactivate_user(&repo, created.id)
            .await
            .expect("deactivate");
        assert_eq!(deactivated.status, UserStatus::Inactive);

        let again = deactivate_user(&repo, created.id)
            .await
            .expect("idempotent");
        assert_eq!(again.status, UserStatus::Inactive);

        let activated = activate_user(&repo, created.id).await.expect("activate");
        assert_eq!(activated.status, UserStatus::Active);
        assert_eq!(activated.username, "Renamed User");

        let still_active = activate_user(&repo, created.id)
            .await
            .expect("activate idempotent");
        assert_eq!(still_active.status, UserStatus::Active);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires an isolated PostgreSQL database named in TEST_DATABASE_URL"]
async fn users_reject_invalid_username_and_missing_id() {
    run_isolated_test(|db| async move {
        let pool = db.pool();
        let repo = UserRepository::new(pool);

        assert_eq!(
            create_user(&repo, "   ").await.unwrap_err(),
            UserError::InvalidUsername
        );
        assert_eq!(
            update_user(&repo, Uuid::nil(), "Ada").await.unwrap_err(),
            UserError::NotFound
        );
        assert_eq!(
            deactivate_user(&repo, Uuid::nil()).await.unwrap_err(),
            UserError::NotFound
        );
        assert_eq!(
            activate_user(&repo, Uuid::nil()).await.unwrap_err(),
            UserError::NotFound
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires an isolated PostgreSQL database named in TEST_DATABASE_URL"]
async fn users_update_does_not_accept_status_from_caller() {
    run_isolated_test(|db| async move {
        let pool = db.pool();
        let repo = UserRepository::new(pool);
        let created = create_user(&repo, &format!("Status Guard {}", Uuid::new_v4()))
            .await
            .expect("create");
        db.track_user(created.id);
        let updated = update_user(&repo, created.id, "Still Active")
            .await
            .expect("update");
        assert_eq!(updated.status, UserStatus::Active);
    })
    .await;
}
