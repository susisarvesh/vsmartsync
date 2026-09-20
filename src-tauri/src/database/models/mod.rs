//! Persistence models.
//!
//! Row types stay in this layer. Domain modules map them to application types.
//! Do not send these structs to React.

mod credential;
mod device;
mod user;

pub use credential::{CredentialRecord, CredentialWriteRecord};
pub use device::DeviceRecord;
pub use user::UserRecord;
