//! Live PostgreSQL checks for the device_users slice.
//!
//! Ignored by default so `cargo test` stays offline-friendly.

use std::sync::Arc;

use uuid::Uuid;
use vsmart_sync_lib::database::repositories::{
    DeviceRepository, DeviceUserRepository, UserRepository,
};
use vsmart_sync_lib::database::{connect_and_migrate, DatabaseConfig};
use vsmart_sync_lib::domains::device_users::{
    ensure_mapping, get_device_user, list_device_users_for_device, mark_provisioned,
    DeviceUserError,
};
use vsmart_sync_lib::domains::devices::create_device;
use vsmart_sync_lib::domains::users::create_user;
use vsmart_sync_lib::SecretVault;

fn load_test_config() -> DatabaseConfig {
    DatabaseConfig::from_url(std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://vsmart_sync:change-me@127.0.0.1:5432/vsmart_sync".to_string()
    }))
    .expect("test database url")
}

fn test_vault() -> SecretVault {
    SecretVault::from_key([91u8; 32]).expect("vault")
}

struct Fixture {
    users: UserRepository,
    devices: DeviceRepository,
    device_users: DeviceUserRepository,
    user_id: Uuid,
    device_id: Uuid,
}

async fn setup() -> Fixture {
    let pool = connect_and_migrate(&load_test_config())
        .await
        .expect("postgresql should be reachable");
    let users = UserRepository::new(pool.clone());
    let devices = DeviceRepository::new(pool.clone());
    let device_users = DeviceUserRepository::new(pool);
    let vault = test_vault();

    let user = create_user(&users, &format!("Map User {}", Uuid::new_v4()))
        .await
        .expect("user");
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
        "admin",
        "secret",
    )
    .await
    .expect("device");

    Fixture {
        users,
        devices,
        device_users,
        user_id: user.id,
        device_id: device.id,
    }
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn device_users_allocate_idempotent_and_provision() {
    let fx = setup().await;

    let first = ensure_mapping(
        &fx.device_users,
        &fx.users,
        &fx.devices,
        fx.user_id,
        fx.device_id,
    )
    .await
    .expect("ensure");
    assert_eq!(first.matrix_user_id, "VS000001");
    assert_eq!(first.matrix_ref_user_id, 10_000_001);
    assert!(first.provisioned_at.is_none());

    let again = ensure_mapping(
        &fx.device_users,
        &fx.users,
        &fx.devices,
        fx.user_id,
        fx.device_id,
    )
    .await
    .expect("idempotent");
    assert_eq!(again.id, first.id);
    assert_eq!(again.matrix_user_id, first.matrix_user_id);
    assert_eq!(again.matrix_ref_user_id, first.matrix_ref_user_id);

    let provisioned = mark_provisioned(&fx.device_users, first.id)
        .await
        .expect("provision");
    assert!(provisioned.provisioned_at.is_some());

    let second_mark = mark_provisioned(&fx.device_users, first.id)
        .await
        .expect("second provision");
    assert_eq!(second_mark.provisioned_at, provisioned.provisioned_at);

    let fetched = get_device_user(&fx.device_users, first.id)
        .await
        .expect("get");
    assert_eq!(fetched.matrix_user_id, "VS000001");

    let listed = list_device_users_for_device(&fx.device_users, fx.device_id)
        .await
        .expect("list");
    assert!(listed.iter().any(|row| row.id == first.id));
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn device_users_allocate_independently_per_device() {
    let fx = setup().await;
    let vault = test_vault();
    let host = format!(
        "10.{}.{}.{}",
        (Uuid::new_v4().as_u128() % 200) + 20,
        Uuid::new_v4().as_u128() % 200,
        Uuid::new_v4().as_u128() % 200
    );
    let device_b = create_device(
        &fx.devices,
        &vault,
        &format!("Door B {}", Uuid::new_v4()),
        &host,
        Some(80),
        "admin",
        "secret",
    )
    .await
    .expect("device b");

    let on_a = ensure_mapping(
        &fx.device_users,
        &fx.users,
        &fx.devices,
        fx.user_id,
        fx.device_id,
    )
    .await
    .expect("a");
    let on_b = ensure_mapping(
        &fx.device_users,
        &fx.users,
        &fx.devices,
        fx.user_id,
        device_b.id,
    )
    .await
    .expect("b");

    assert_eq!(on_a.matrix_user_id, "VS000001");
    assert_eq!(on_b.matrix_user_id, "VS000001");
    assert_eq!(on_a.matrix_ref_user_id, 10_000_001);
    assert_eq!(on_b.matrix_ref_user_id, 10_000_001);
    assert_ne!(on_a.id, on_b.id);
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn device_users_concurrent_allocation_is_unique() {
    let fx = setup().await;
    let device_users = Arc::new(fx.device_users);
    let users = Arc::new(fx.users);
    let devices = Arc::new(fx.devices);
    let device_id = fx.device_id;

    let mut handles = Vec::new();
    for _ in 0..8 {
        let user = create_user(&users, &format!("Concurrent {}", Uuid::new_v4()))
            .await
            .expect("user");
        let device_users = Arc::clone(&device_users);
        let users = Arc::clone(&users);
        let devices = Arc::clone(&devices);
        handles.push(tokio::spawn(async move {
            ensure_mapping(&device_users, &users, &devices, user.id, device_id).await
        }));
    }

    let mut refs = Vec::new();
    let mut matrix_ids = Vec::new();
    for handle in handles {
        let mapping = handle.await.expect("join").expect("ensure");
        refs.push(mapping.matrix_ref_user_id);
        matrix_ids.push(mapping.matrix_user_id);
    }

    refs.sort_unstable();
    matrix_ids.sort();
    refs.dedup();
    matrix_ids.dedup();
    assert_eq!(refs.len(), 8);
    assert_eq!(matrix_ids.len(), 8);
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn device_users_reject_missing_user_or_device() {
    let fx = setup().await;
    let err = ensure_mapping(
        &fx.device_users,
        &fx.users,
        &fx.devices,
        Uuid::new_v4(),
        fx.device_id,
    )
    .await
    .unwrap_err();
    assert_eq!(err, DeviceUserError::UserNotFound);

    let err = ensure_mapping(
        &fx.device_users,
        &fx.users,
        &fx.devices,
        fx.user_id,
        Uuid::new_v4(),
    )
    .await
    .unwrap_err();
    assert_eq!(err, DeviceUserError::DeviceNotFound);
}
