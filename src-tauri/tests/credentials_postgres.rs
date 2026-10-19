//! Live PostgreSQL checks for the Credentials slice.
//!
//! Ignored by default so `cargo test` stays offline-friendly.

use uuid::Uuid;
use vsmart_sync_lib::database::repositories::{CredentialRepository, UserRepository};
use vsmart_sync_lib::database::run_isolated_test;
use vsmart_sync_lib::domains::credentials::{
    create_credential, get_credential, list_credentials, set_credential_status,
    update_credential_value, CredentialError, CredentialListFilter, CredentialStatus,
    CredentialType,
};
use vsmart_sync_lib::domains::users::create_user;
use vsmart_sync_lib::SecretVault;

fn test_vault() -> SecretVault {
    SecretVault::from_key([77u8; 32]).expect("vault")
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn credentials_create_list_update_deactivate() {
    run_isolated_test(|db| async move {
        let pool = db.pool();
        let users = UserRepository::new(pool.clone());
        let credentials = CredentialRepository::new(pool);
        let vault = test_vault();

        let user = create_user(&users, &format!("Cred User {}", Uuid::new_v4()))
            .await
            .expect("user");
        db.track_user(user.id);

        let card = create_credential(&credentials, &users, &vault, user.id, "card", "00 987654")
            .await
            .expect("card");
        assert_eq!(card.credential_type, CredentialType::Card);
        assert_eq!(card.masked_value.as_deref(), Some("••••7654"));
        assert_eq!(card.status, CredentialStatus::Active);

        let listed = list_credentials(
            &credentials,
            CredentialListFilter {
                user_id: Some(user.id),
                ..Default::default()
            },
        )
        .await
        .expect("list");
        assert!(listed.iter().any(|item| item.id == card.id));
        let json = serde_json::to_string(&listed).expect("json");
        assert!(!json.contains("987654"));
        assert!(!json.contains("ciphertext"));

        let pin = create_credential(&credentials, &users, &vault, user.id, "pin", "2468")
            .await
            .expect("pin");
        assert!(pin.masked_value.is_none());

        let updated = update_credential_value(&credentials, &vault, card.id, "11223344")
            .await
            .expect("update");
        assert_eq!(updated.masked_value.as_deref(), Some("••••3344"));

        let inactive = set_credential_status(&credentials, pin.id, "inactive")
            .await
            .expect("deactivate");
        assert_eq!(inactive.status, CredentialStatus::Inactive);

        let fetched = get_credential(&credentials, pin.id).await.expect("get");
        assert_eq!(fetched.status, CredentialStatus::Inactive);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn credentials_reject_invalid_user_and_duplicates() {
    run_isolated_test(|db| async move {
        let pool = db.pool();
        let users = UserRepository::new(pool.clone());
        let credentials = CredentialRepository::new(pool);
        let vault = test_vault();

        assert_eq!(
            create_credential(&credentials, &users, &vault, Uuid::nil(), "card", "5555")
                .await
                .unwrap_err(),
            CredentialError::UserNotFound
        );

        let user = create_user(&users, &format!("Dup User {}", Uuid::new_v4()))
            .await
            .expect("user");
        db.track_user(user.id);
        let card_number = format!("CARD{}", Uuid::new_v4().as_u128() % 1_000_000);
        create_credential(&credentials, &users, &vault, user.id, "card", &card_number)
            .await
            .expect("first");

        let other = create_user(&users, &format!("Other {}", Uuid::new_v4()))
            .await
            .expect("other");
        db.track_user(other.id);
        assert_eq!(
            create_credential(&credentials, &users, &vault, other.id, "card", &card_number)
                .await
                .unwrap_err(),
            CredentialError::Duplicate
        );

        create_credential(&credentials, &users, &vault, user.id, "pin", "1357")
            .await
            .expect("pin");
        assert_eq!(
            create_credential(&credentials, &users, &vault, user.id, "pin", "9999")
                .await
                .unwrap_err(),
            CredentialError::Duplicate
        );
    })
    .await;
}
