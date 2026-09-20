//! Product domain modules.
//!
//! Each folder is a future service. Implement one at a time. These modules
//! must not call Matrix HTTP or talk to PostgreSQL from the UI.

mod audit;
mod auth;
mod credentials;
mod devices;
mod enrollments;
mod events;
mod licensing;
mod synchronization;
mod users;
