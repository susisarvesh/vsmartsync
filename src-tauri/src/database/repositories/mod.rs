//! Data-access repositories.
//!
//! PostgreSQL access for domain modules lives here, not in Tauri commands
//! or React.

mod devices;
mod users;

pub use devices::{DeviceRepository, DEVICE_LIST_LIMIT};
pub use users::{UserRepository, USER_LIST_LIMIT};
