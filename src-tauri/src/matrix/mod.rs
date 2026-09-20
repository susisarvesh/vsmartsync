//! Matrix hardware integration boundary.
//!
//! All communication with Matrix devices must stay inside this module.
//! Do not call Matrix HTTP APIs from repositories or React.
//!
//! Only the documented connectivity probe is implemented in this slice.

pub mod adapter;
pub mod client;
pub mod models;

pub use adapter::{MatrixAdapter, MatrixProbeError};
