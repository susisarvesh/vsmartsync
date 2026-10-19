//! Live PostgreSQL checks for the Enrollments slice.
//!
//! Ignored by default so `cargo test` stays offline-friendly.

use uuid::Uuid;
use vsmart_sync_lib::database::repositories::{
    CredentialRepository, DeviceRepository, EnrollmentRepository, UserRepository,
};
use vsmart_sync_lib::database::{run_isolated_test, IsolatedDatabase};
use vsmart_sync_lib::domains::credentials::create_credential;
use vsmart_sync_lib::domains::devices::create_device;
use vsmart_sync_lib::domains::enrollments::{
    cancel_enrollment, create_enrollment, get_enrollment, list_enrollments, mark_enrollment_active,
    mark_enrollment_failed, retry_enrollment, revoke_enrollment, EnrollmentError,
    EnrollmentListFilter, EnrollmentStatus,
};
use vsmart_sync_lib::domains::users::{create_user, deactivate_user};
use vsmart_sync_lib::SecretVault;

fn test_vault() -> SecretVault {
    SecretVault::from_key([91u8; 32]).expect("vault")
}

struct Fixture {
    users: UserRepository,
    credentials: CredentialRepository,
    devices: DeviceRepository,
    enrollments: EnrollmentRepository,
    user_id: Uuid,
    credential_id: Uuid,
    device_id: Uuid,
}

async fn setup(db: &IsolatedDatabase) -> Fixture {
    let pool = db.pool();
    let users = UserRepository::new(pool.clone());
    let credentials = CredentialRepository::new(pool.clone());
    let devices = DeviceRepository::new(pool.clone());
    let enrollments = EnrollmentRepository::new(pool);
    let vault = test_vault();

    let user = create_user(&users, &format!("Enroll User {}", Uuid::new_v4()))
        .await
        .expect("user");
    let card = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000);
    let credential = create_credential(&credentials, &users, &vault, user.id, "card", &card)
        .await
        .expect("credential");
    let host = format!(
        "10.{}.{}.{}",
        (Uuid::new_v4().as_u128() % 200) + 20,
        Uuid::new_v4().as_u128() % 200,
        Uuid::new_v4().as_u128() % 200
    );
    let device = create_device(
        &devices,
        &vault,
        &format!("Door {}", Uuid::new_v4()),
        &host,
        Some(80),
        None,
        "admin",
        "secret",
    )
    .await
    .expect("device");
    db.track_user(user.id);
    db.track_device(device.id);

    Fixture {
        users,
        credentials,
        devices,
        enrollments,
        user_id: user.id,
        credential_id: credential.id,
        device_id: device.id,
    }
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn enrollments_create_list_cancel_retry_revoke() {
    run_isolated_test(|db| async move {
        let fx = setup(&db).await;

        let created = create_enrollment(
            &fx.enrollments,
            &fx.users,
            &fx.credentials,
            &fx.devices,
            fx.user_id,
            fx.credential_id,
            fx.device_id,
        )
        .await
        .expect("create");
        assert_eq!(created.status, EnrollmentStatus::Pending);
        assert_eq!(created.user_id, fx.user_id);

        let listed = list_enrollments(
            &fx.enrollments,
            EnrollmentListFilter {
                user_id: Some(fx.user_id),
                limit: Some(50),
                ..Default::default()
            },
        )
        .await
        .expect("list");
        assert!(listed.iter().any(|row| row.id == created.id));
        let json = serde_json::to_string(&listed).expect("json");
        assert!(!json.contains("ciphertext"));
        assert!(!json.contains("password"));

        let failed = mark_enrollment_failed(&fx.enrollments, created.id)
            .await
            .expect("failed");
        assert_eq!(failed.status, EnrollmentStatus::Failed);
        assert_eq!(failed.id, created.id);

        let retried = retry_enrollment(&fx.enrollments, created.id)
            .await
            .expect("retry");
        assert_eq!(retried.status, EnrollmentStatus::Pending);
        assert_eq!(retried.id, created.id);

        let active = mark_enrollment_active(&fx.enrollments, created.id)
            .await
            .expect("active");
        assert_eq!(active.status, EnrollmentStatus::Active);
        assert!(active.activated_at.is_some());

        let revoked = revoke_enrollment(&fx.enrollments, created.id)
            .await
            .expect("revoke");
        assert_eq!(revoked.status, EnrollmentStatus::Revoked);
        assert!(revoked.revoked_at.is_some());

        assert_eq!(
            retry_enrollment(&fx.enrollments, created.id)
                .await
                .unwrap_err(),
            EnrollmentError::InvalidTransition
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn enrollments_reject_invalid_and_duplicate() {
    run_isolated_test(|db| async move {
        let fx = setup(&db).await;

        assert_eq!(
            create_enrollment(
                &fx.enrollments,
                &fx.users,
                &fx.credentials,
                &fx.devices,
                Uuid::nil(),
                fx.credential_id,
                fx.device_id,
            )
            .await
            .unwrap_err(),
            EnrollmentError::UserNotFound
        );

        let inactive = create_user(&fx.users, &format!("Inactive {}", Uuid::new_v4()))
            .await
            .expect("inactive user");
        db.track_user(inactive.id);
        deactivate_user(&fx.users, inactive.id)
            .await
            .expect("deactivate");
        assert_eq!(
            create_enrollment(
                &fx.enrollments,
                &fx.users,
                &fx.credentials,
                &fx.devices,
                inactive.id,
                fx.credential_id,
                fx.device_id,
            )
            .await
            .unwrap_err(),
            EnrollmentError::UserInactive
        );

        let other = create_user(&fx.users, &format!("Other {}", Uuid::new_v4()))
            .await
            .expect("other");
        db.track_user(other.id);
        assert_eq!(
            create_enrollment(
                &fx.enrollments,
                &fx.users,
                &fx.credentials,
                &fx.devices,
                other.id,
                fx.credential_id,
                fx.device_id,
            )
            .await
            .unwrap_err(),
            EnrollmentError::CredentialOwnership
        );

        create_enrollment(
            &fx.enrollments,
            &fx.users,
            &fx.credentials,
            &fx.devices,
            fx.user_id,
            fx.credential_id,
            fx.device_id,
        )
        .await
        .expect("first");

        assert_eq!(
            create_enrollment(
                &fx.enrollments,
                &fx.users,
                &fx.credentials,
                &fx.devices,
                fx.user_id,
                fx.credential_id,
                fx.device_id,
            )
            .await
            .unwrap_err(),
            EnrollmentError::Duplicate
        );

        let pending = get_enrollment(
            &fx.enrollments,
            list_enrollments(
                &fx.enrollments,
                EnrollmentListFilter {
                    credential_id: Some(fx.credential_id),
                    device_id: Some(fx.device_id),
                    ..Default::default()
                },
            )
            .await
            .expect("list")[0]
                .id,
        )
        .await
        .expect("get");
        let cancelled = cancel_enrollment(&fx.enrollments, pending.id)
            .await
            .expect("cancel");
        assert_eq!(cancelled.status, EnrollmentStatus::Cancelled);
        assert!(cancelled.cancelled_at.is_some());

        // After cancel, a new enrollment may be created.
        let again = create_enrollment(
            &fx.enrollments,
            &fx.users,
            &fx.credentials,
            &fx.devices,
            fx.user_id,
            fx.credential_id,
            fx.device_id,
        )
        .await
        .expect("recreate");
        assert_ne!(again.id, cancelled.id);
        assert_eq!(again.status, EnrollmentStatus::Pending);
    })
    .await;
}
