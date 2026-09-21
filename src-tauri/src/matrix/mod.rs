//! Matrix hardware integration boundary.
//!
//! All communication with Matrix devices must stay inside this module.
//! Do not call Matrix HTTP APIs from repositories or React.
//!
//! **Implemented:** Client foundation (`Response-Code`), connectivity probe,
//! Adapter `set_user` and `set_pin` via `/device.cgi/users`.
//!
//! **Not in this slice:** `set_card`, `/credential`, Sync, `device_users`,
//! capability framework, retries, React/domain SoR changes.

pub mod adapter;
pub mod client;
pub mod models;

pub use adapter::{MatrixAdapter, MatrixProbeError};

// Application-facing set_user / set_pin types: `matrix::adapter::{SetUserParams,
// SetPinParams, MatrixAdapterError}` — re-export when a domain/Sync caller lands.
