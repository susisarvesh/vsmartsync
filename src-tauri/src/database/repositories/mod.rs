//! Data-access repositories.
//!
//! PostgreSQL access for domain modules lives here, not in Tauri commands
//! or React.

mod credentials;
mod devices;
mod users;

pub use credentials::{CredentialListQuery, CredentialRepository, CREDENTIAL_LIST_LIMIT};
pub use devices::{DeviceRepository, DEVICE_LIST_LIMIT};
pub use users::{UserRepository, USER_LIST_LIMIT};
