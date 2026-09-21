//! Product domain modules.
//!
//! Implement one service at a time. These modules must not call Matrix HTTP
//! or talk to PostgreSQL from the UI.

mod audit;
mod auth;
pub mod credentials;
pub mod device_users;
pub mod devices;
pub mod enrollments;
mod events;
mod licensing;
mod synchronization;
pub mod users;
