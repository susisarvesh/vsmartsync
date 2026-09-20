//! Live PostgreSQL checks for the Devices slice.
//!
//! Ignored by default so `cargo test` stays offline-friendly.

use uuid::Uuid;
use vsmart_sync_lib::database::repositories::DeviceRepository;
use vsmart_sync_lib::database::{connect_and_migrate, DatabaseConfig};
use vsmart_sync_lib::domains::devices::{
    create_device, list_devices, set_device_password, update_device, ConnectionStatus,
    DeviceError,
};
use vsmart_sync_lib::DevicePasswordVault;

fn load_test_config() -> DatabaseConfig {
    DatabaseConfig::from_url(std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://vsmart_sync:change-me@127.0.0.1:5432/vsmart_sync".to_string()
    }))
    .expect("test database url")
}

fn test_vault() -> DevicePasswordVault {
    DevicePasswordVault::from_key([42u8; 32]).expect("vault")
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn devices_create_list_update_password() {
    let pool = connect_and_migrate(&load_test_config())
        .await
        .expect("postgresql should be reachable");
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
        "admin",
        "initial-secret",
    )
    .await
    .expect("create");
    assert_eq!(created.name, "Lobby Door");
    assert_eq!(created.host, host);
    assert_eq!(created.port, 8080);
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
        "operator",
    )
    .await
    .expect("update");
    assert_eq!(updated.name, "Lobby Updated");
    assert_eq!(updated.username, "operator");

    let with_password = set_device_password(&repo, &vault, created.id, "rotated-secret")
        .await
        .expect("password");
    assert_eq!(with_password.id, created.id);

    let record = repo.find_by_id(created.id).await.expect("load").expect("exists");
    assert_ne!(record.password_ciphertext, b"rotated-secret");
    assert_eq!(
        vault.decrypt(&record.password_ciphertext).expect("decrypt"),
        "rotated-secret"
    );
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn devices_reject_invalid_and_duplicate() {
    let pool = connect_and_migrate(&load_test_config())
        .await
        .expect("postgresql should be reachable");
    let repo = DeviceRepository::new(pool);
    let vault = test_vault();
    let host = format!("10.55.{}.{}", Uuid::new_v4().as_u128() % 200, Uuid::new_v4().as_u128() % 200);

    assert_eq!(
        create_device(&repo, &vault, "   ", &host, Some(80), "admin", "x")
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
            "admin",
            "x"
        )
        .await
        .unwrap_err(),
        DeviceError::InvalidHost
    );
    assert_eq!(
        create_device(&repo, &vault, "Door", &host, Some(0), "admin", "x")
            .await
            .unwrap_err(),
        DeviceError::InvalidPort
    );

    create_device(&repo, &vault, "Door A", &host, Some(90), "admin", "secret")
        .await
        .expect("first");
    assert_eq!(
        create_device(&repo, &vault, "Door B", &host, Some(90), "admin", "secret")
            .await
            .unwrap_err(),
        DeviceError::Duplicate
    );
    assert_eq!(
        update_device(&repo, Uuid::nil(), "Ada", "10.0.0.1", 80, "admin")
            .await
            .unwrap_err(),
        DeviceError::NotFound
    );
}
