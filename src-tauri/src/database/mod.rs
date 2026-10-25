//! Database access layer.
//!
//! All PostgreSQL access must go through this module. SQL migration files live
//! in the repository-root `migrations/` directory (SQLx standard), not under
//! this Rust source tree.

mod config;
mod pool;
#[cfg(test)]
mod reader_burst;
mod runtime;
mod test_isolation;

pub use config::{DatabaseConfig, DatabaseConfigError, PublicDatabaseTarget};
pub use pool::{
    connect, connect_and_migrate, connect_and_migrate_with_limit, ping, run_migrations,
    DatabaseError, DbPool,
};
pub use runtime::{DatabaseRuntime, DatabaseStatus};
pub use test_isolation::{
    assess_test_target, run_isolated_test, writable_test_config, IsolatedDatabase,
    TestDatabaseError,
};

pub mod models;
pub mod repositories;
