//! Live PostgreSQL plus a local HTTP stand-in for adding assigned users on a device.
//!
//! Ignored by default so `cargo test` stays offline-friendly.

use uuid::Uuid;
use wiremock::matchers::{basic_auth, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::database::repositories::{
    DeviceRepository, DeviceUserRepository, UserDeviceRepository, UserRepository,
};
use crate::database::{connect_and_migrate, DatabaseConfig};
use crate::domains::devices::{create_device, deactivate_device};
use crate::domains::synchronization::{sync_assigned_users, SyncError};
use crate::domains::user_devices::assign_user_device;
use crate::domains::users::create_user;
use crate::matrix::MatrixAdapter;
use crate::DevicePasswordVault;

fn load_test_config() -> DatabaseConfig {
    DatabaseConfig::from_url(std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://vsmart_sync:change-me@127.0.0.1:5432/vsmart_sync".to_string()
    }))
    .expect("test database url")
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn assigned_user_is_added_on_the_device() {
    let server = MockServer::start().await;
    let username = format!("Door{}", &Uuid::new_v4().simple().to_string()[..8]);
    Mock::given(method("GET"))
        .and(path("/device.cgi/users"))
        .and(query_param("action", "set"))
        .and(query_param("user-id", "VS000001"))
        .and(query_param("ref-user-id", "10000001"))
        .and(query_param("name", username.as_str()))
        .and(query_param("user-active", "1"))
        .and(basic_auth("admin", "secret"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Response-Code=0"))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/device.cgi/users"))
        .respond_with(
            ResponseTemplate::new(401).insert_header("WWW-Authenticate", "Basic realm=\"device\""),
        )
        .with_priority(2)
        .mount(&server)
        .await;

    let pool = connect_and_migrate(&load_test_config())
        .await
        .expect("postgresql should be reachable");
    let users = UserRepository::new(pool.clone());
    let devices = DeviceRepository::new(pool.clone());
    let assignments = UserDeviceRepository::new(pool.clone());
    let mappings = DeviceUserRepository::new(pool);
    let vault = DevicePasswordVault::from_key([9u8; 32]).expect("vault");
    let matrix = MatrixAdapter::new().expect("matrix client");

    let user = create_user(&users, &username).await.expect("user");
    let device = create_device(
        &devices,
        &vault,
        "Sync Door",
        "127.0.0.1",
        Some(i32::from(server.address().port())),
        None,
        "admin",
        "secret",
    )
    .await
    .expect("device");
    assign_user_device(&users, &devices, &assignments, user.id, device.id)
        .await
        .expect("assign");

    let result = sync_assigned_users(
        &users,
        &devices,
        &assignments,
        &mappings,
        &vault,
        &matrix,
        device.id,
        &[user.id],
    )
    .await
    .expect("sync");

    assert_eq!(result.failed, vec![]);
    assert_eq!(result.synced.len(), 1);
    assert_eq!(result.synced[0].user_id, user.id);
    assert_eq!(result.synced[0].matrix_user_id, "VS000001");
    assert_eq!(result.synced[0].matrix_ref_user_id, 10_000_001);
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn inactive_device_is_not_contacted() {
    let pool = connect_and_migrate(&load_test_config())
        .await
        .expect("postgresql should be reachable");
    let users = UserRepository::new(pool.clone());
    let devices = DeviceRepository::new(pool.clone());
    let assignments = UserDeviceRepository::new(pool.clone());
    let mappings = DeviceUserRepository::new(pool);
    let vault = DevicePasswordVault::from_key([9u8; 32]).expect("vault");
    let matrix = MatrixAdapter::new().expect("matrix client");

    let user = create_user(&users, &format!("Off{}", Uuid::new_v4().simple()))
        .await
        .expect("user");
    let device = create_device(
        &devices,
        &vault,
        "Inactive Door",
        "127.0.0.1",
        Some(20_000 + (Uuid::new_v4().as_u128() % 20_000) as i32),
        None,
        "admin",
        "secret",
    )
    .await
    .expect("device");
    deactivate_device(&devices, device.id)
        .await
        .expect("deactivate");
    assign_user_device(&users, &devices, &assignments, user.id, device.id)
        .await
        .expect("assign");

    let error = sync_assigned_users(
        &users,
        &devices,
        &assignments,
        &mappings,
        &vault,
        &matrix,
        device.id,
        &[user.id],
    )
    .await
    .unwrap_err();
    assert_eq!(error, SyncError::DeviceInactive);
}
