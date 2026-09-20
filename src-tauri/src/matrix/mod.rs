//! Matrix hardware integration boundary.
//!
//! All communication with Matrix devices must stay inside this module.
//! Do not call Matrix HTTP APIs from domain services, repositories, or React.
//!
//! Matrix API behavior is not implemented yet. Do not invent endpoints here.

pub mod adapter;
pub mod client;
pub mod models;
