//! Database access layer.
//!
//! All PostgreSQL access must go through this module. Domain repositories
//! are not implemented yet. SQL migration files live in the repository-root
//! `migrations/` directory (SQLx standard), not under this Rust source tree.

mod config;
mod pool;

pub use config::{DatabaseConfig, DatabaseConfigError};
pub use pool::{connect, connect_and_migrate, ping, run_migrations, DatabaseError, DbPool};

pub mod models;
pub mod repositories;
