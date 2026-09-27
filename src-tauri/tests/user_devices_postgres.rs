//! Live PostgreSQL checks for user-to-device assignment.
//!
//! Ignored by default so `cargo test` stays offline-friendly.

use uuid::Uuid;
use vsmart_sync_lib::database::repositories::{
    DeviceRepository, UserDeviceRepository, UserRepository,
};
use vsmart_sync_lib::database::{connect_and_migrate, DatabaseConfig};
use vsmart_sync_lib::domains::devices::create_device;
use vsmart_sync_lib::domains::user_devices::{
    assign_user_device, list_user_devices, list_users_for_device, remove_user_device,
    UserDeviceError,
};
use vsmart_sync_lib::domains::users::create_user;
use vsmart_sync_lib::DevicePasswordVault;

fn load_test_config() -> DatabaseConfig {
    DatabaseConfig::from_url(std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://vsmart_sync:change-me@127.0.0.1:5432/vsmart_sync".to_string()
    }))
    .expect("test database url")
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn user_can_be_assigned_to_multiple_devices() {
    let pool = connect_and_migrate(&load_test_config())
        .await
        .expect("postgresql should be reachable");
    let users = UserRepository::new(pool.clone());
    let devices = DeviceRepository::new(pool.clone());
    let assignments = UserDeviceRepository::new(pool);
    let vault = DevicePasswordVault::from_key([7u8; 32]).expect("vault");

    let user = create_user(&users, &format!("Assign {}", Uuid::new_v4()))
        .await
        .expect("user");
    let host_a = format!("10.80.{}.1", Uuid::new_v4().as_u128() % 200);
    let host_b = format!("10.80.{}.2", Uuid::new_v4().as_u128() % 200);
    let door_a = create_device(
        &devices,
        &vault,
        "Main Gate",
        &host_a,
        Some(80),
        None,
        "admin",
        "secret",
    )
    .await
    .expect("door a");
    let door_b = create_device(
        &devices,
        &vault,
        "Office Entrance",
        &host_b,
        Some(80),
        None,
        "admin",
        "secret",
    )
    .await
    .expect("door b");

    assign_user_device(&users, &devices, &assignments, user.id, door_a.id)
        .await
        .expect("assign a");
    assign_user_device(&users, &devices, &assignments, user.id, door_b.id)
        .await
        .expect("assign b");
    assert_eq!(
        assign_user_device(&users, &devices, &assignments, user.id, door_a.id)
            .await
            .unwrap_err(),
        UserDeviceError::Duplicate
    );

    let on_door_a = list_users_for_device(&devices, &assignments, door_a.id)
        .await
        .expect("users on door a");
    assert_eq!(on_door_a.len(), 1);
    assert_eq!(on_door_a[0].user_id, user.id);

    let listed = list_user_devices(&users, &assignments, user.id)
        .await
        .expect("list");
    assert_eq!(listed.len(), 2);
    assert!(listed.iter().any(|row| row.device_name == "Main Gate"));
    assert!(listed
        .iter()
        .any(|row| row.device_name == "Office Entrance"));

    remove_user_device(&assignments, user.id, door_a.id)
        .await
        .expect("remove");
    let listed = list_user_devices(&users, &assignments, user.id)
        .await
        .expect("list after remove");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].device_id, door_b.id);

    assert_eq!(
        assign_user_device(&users, &devices, &assignments, Uuid::nil(), door_a.id)
            .await
            .unwrap_err(),
        UserDeviceError::UserNotFound
    );
    assert_eq!(
        assign_user_device(&users, &devices, &assignments, user.id, Uuid::nil())
            .await
            .unwrap_err(),
        UserDeviceError::DeviceNotFound
    );
}
