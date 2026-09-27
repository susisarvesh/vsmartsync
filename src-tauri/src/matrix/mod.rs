//! Matrix hardware integration boundary.
//!
//! All communication with Matrix devices must stay inside this module.
//! Do not call Matrix HTTP APIs from repositories or React.
//!
//! **Implemented:** Client foundation (`Response-Code`), connectivity probe,
//! Adapter `set_user`, `set_pin`, enrollment capability reads, and `enroll_user`.
//!
//! **Not in this slice:** `set_card`, credential template download, retries.

pub mod adapter;
pub mod client;
mod enrollment;
pub mod models;

pub use adapter::{MatrixAdapter, MatrixAdapterError, MatrixProbeError, SetUserParams};
pub use enrollment::{credential_present, HardwareEnrollType};
