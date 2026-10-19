//! Live PostgreSQL checks for the Devices slice.
//!
//! Ignored by default so `cargo test` stays offline-friendly.

use uuid::Uuid;
use vsmart_sync_lib::database::repositories::DeviceRepository;
use vsmart_sync_lib::database::run_isolated_test;
use vsmart_sync_lib::domains::devices::{
    activate_device, create_device, deactivate_device, list_devices, set_device_password,
    update_device, ConnectionStatus, DeviceError, DeviceStatus,
};
use vsmart_sync_lib::DevicePasswordVault;

fn test_vault() -> DevicePasswordVault {
    DevicePasswordVault::from_key([42u8; 32]).expect("vault")
}

#[tokio::test]
#[ignore = "requires an isolated PostgreSQL database named in TEST_DATABASE_URL"]
async fn devices_create_list_update_password() {
    run_isolated_test(|db| async move {
        let pool = db.pool();
        let repo = DeviceRepository::new(pool);
        let vault = test_vault();
        let suffix = Uuid::new_v4().simple().to_string();
        let host = format!("10.20.30.{}", (suffix.as_bytes()[0] % 200) + 20);

        let created = create_device(
            &repo,
            &vault,
            "  Lobby Door  ",
            &host,
            Some(8080),
            Some("aa-bb-cc-dd-ee-01"),
            "admin",
            "initial-secret",
        )
        .await
        .expect("create");
        db.track_device(created.id);
        assert_eq!(created.device_name, "Lobby Door");
        assert_eq!(created.host, host);
        assert_eq!(created.port, 8080);
        assert_eq!(created.mac_address.as_deref(), Some("AA:BB:CC:DD:EE:01"));
        assert!(created.device_model.is_none());
        assert_eq!(created.status, DeviceStatus::Active);
        assert_eq!(created.connection_status, ConnectionStatus::Unknown);
        assert!(created.last_seen_at.is_none());

        let listed = list_devices(&repo).await.expect("list");
        assert!(listed.iter().any(|device| device.id == created.id));
        let json = serde_json::to_string(&listed).expect("json");
        assert!(!json.contains("password"));
        assert!(!json.contains("ciphertext"));

        let updated = update_device(
            &repo,
            created.id,
            "Lobby Updated",
            &host,
            8080,
            None,
            "operator",
        )
        .await
        .expect("update");
        assert_eq!(updated.device_name, "Lobby Updated");
        assert_eq!(updated.username, "operator");
        assert!(updated.mac_address.is_none());

        let inactive = deactivate_device(&repo, created.id)
            .await
            .expect("deactivate");
        assert_eq!(inactive.status, DeviceStatus::Inactive);
        let active = activate_device(&repo, created.id).await.expect("activate");
        assert_eq!(active.status, DeviceStatus::Active);

        let with_password = set_device_password(&repo, &vault, created.id, "rotated-secret")
            .await
            .expect("password");
        assert_eq!(with_password.id, created.id);

        let record = repo
            .find_by_id(created.id)
            .await
            .expect("load")
            .expect("exists");
        assert_ne!(record.password_ciphertext, b"rotated-secret");
        assert_eq!(
            vault.decrypt(&record.password_ciphertext).expect("decrypt"),
            "rotated-secret"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires an isolated PostgreSQL database named in TEST_DATABASE_URL"]
async fn devices_reject_invalid_and_duplicate() {
    run_isolated_test(|db| async move {
        let pool = db.pool();
        let repo = DeviceRepository::new(pool);
        let vault = test_vault();
        let host = format!(
            "10.55.{}.{}",
            Uuid::new_v4().as_u128() % 200,
            Uuid::new_v4().as_u128() % 200
        );

        assert_eq!(
            create_device(&repo, &vault, "   ", &host, Some(80), None, "admin", "x")
                .await
                .unwrap_err(),
            DeviceError::InvalidName
        );
        assert_eq!(
            create_device(
                &repo,
                &vault,
                "Door",
                "http://evil",
                Some(80),
                None,
                "admin",
                "x",
            )
            .await
            .unwrap_err(),
            DeviceError::InvalidHost
        );
        assert_eq!(
            create_device(&repo, &vault, "Door", &host, Some(0), None, "admin", "x")
                .await
                .unwrap_err(),
            DeviceError::InvalidPort
        );
        assert_eq!(
            create_device(
                &repo,
                &vault,
                "Door",
                &host,
                Some(80),
                Some("zz"),
                "admin",
                "x"
            )
            .await
            .unwrap_err(),
            DeviceError::InvalidMac
        );

        let created = create_device(
            &repo,
            &vault,
            "Door A",
            &host,
            Some(90),
            None,
            "admin",
            "secret",
        )
        .await
        .expect("first");
        db.track_device(created.id);
        assert_eq!(
            create_device(
                &repo,
                &vault,
                "Door B",
                &host,
                Some(90),
                None,
                "admin",
                "secret",
            )
            .await
            .unwrap_err(),
            DeviceError::Duplicate
        );
        assert_eq!(
            update_device(&repo, Uuid::nil(), "Ada", "10.0.0.1", 80, None, "admin")
                .await
                .unwrap_err(),
            DeviceError::NotFound
        );
    })
    .await;
}
