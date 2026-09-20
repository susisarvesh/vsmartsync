//! Shared types and helpers that do not belong to a single domain module.

mod app_info;
mod env_files;

pub use app_info::{app_info, AppInfo};
pub(crate) use env_files::load as load_env_files;
