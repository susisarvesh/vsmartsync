//! Data-access repositories.
//!
//! PostgreSQL access for domain modules lives here, not in Tauri commands
//! or React.

mod credentials;
mod device_users;
mod devices;
mod enrollments;
mod users;

pub use credentials::{CredentialListQuery, CredentialRepository, CREDENTIAL_LIST_LIMIT};
pub use device_users::{format_matrix_user_id, DeviceUserRepository};
pub use devices::{DeviceRepository, DEVICE_LIST_LIMIT};
pub use enrollments::{
    EnrollmentListQuery, EnrollmentRepository, ENROLLMENT_LIST_LIMIT, ENROLLMENT_LIST_MAX_LIMIT,
};
pub use users::{UserRepository, USER_LIST_LIMIT};
