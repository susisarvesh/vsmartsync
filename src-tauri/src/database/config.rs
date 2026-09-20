use std::env;

use thiserror::Error;

/// PostgreSQL connection settings loaded from the environment.
///
/// Prefer `DATABASE_URL`. `POSTGRES_*` variables are accepted as an alternative
/// for a local PostgreSQL install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseConfig {
    url: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DatabaseConfigError {
    #[error("DATABASE_URL is missing or empty")]
    MissingUrl,
    #[error("DATABASE_URL must start with postgres:// or postgresql://")]
    InvalidUrlScheme,
    #[error("POSTGRES_HOST is missing or empty")]
    MissingHost,
    #[error("POSTGRES_DB is missing or empty")]
    MissingDatabase,
    #[error("POSTGRES_USER is missing or empty")]
    MissingUser,
    #[error("POSTGRES_PASSWORD is missing or empty")]
    MissingPassword,
    #[error("POSTGRES_PORT is missing or invalid")]
    InvalidPort,
    #[error(
        "POSTGRES_USER or POSTGRES_PASSWORD contains URL-reserved characters; set DATABASE_URL instead"
    )]
    UnsafeUrlCredentials,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, DatabaseConfigError> {
        if let Ok(url) = env::var("DATABASE_URL") {
            if !url.trim().is_empty() {
                return Self::from_url(url);
            }
        }

        let host = required_env("POSTGRES_HOST").ok_or(DatabaseConfigError::MissingHost)?;
        let database = required_env("POSTGRES_DB").ok_or(DatabaseConfigError::MissingDatabase)?;
        let user = required_env("POSTGRES_USER").ok_or(DatabaseConfigError::MissingUser)?;
        let password =
            required_env("POSTGRES_PASSWORD").ok_or(DatabaseConfigError::MissingPassword)?;

        Self::from_parts(
            &host,
            parse_port(env::var("POSTGRES_PORT").ok().as_deref())?,
            &database,
            &user,
            &password,
        )
    }

    pub fn from_url(url: impl Into<String>) -> Result<Self, DatabaseConfigError> {
        let url = url.into();
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err(DatabaseConfigError::MissingUrl);
        }
        if !(trimmed.starts_with("postgres://") || trimmed.starts_with("postgresql://")) {
            return Err(DatabaseConfigError::InvalidUrlScheme);
        }
        Ok(Self {
            url: trimmed.to_string(),
        })
    }

    pub fn from_parts(
        host: &str,
        port: u16,
        database: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, DatabaseConfigError> {
        if host.trim().is_empty() {
            return Err(DatabaseConfigError::MissingHost);
        }
        if database.trim().is_empty() {
            return Err(DatabaseConfigError::MissingDatabase);
        }
        if user.trim().is_empty() {
            return Err(DatabaseConfigError::MissingUser);
        }
        if password.is_empty() {
            return Err(DatabaseConfigError::MissingPassword);
        }
        if port == 0 {
            return Err(DatabaseConfigError::InvalidPort);
        }
        if has_url_reserved_chars(user) || has_url_reserved_chars(password) {
            return Err(DatabaseConfigError::UnsafeUrlCredentials);
        }

        let url = format!(
            "postgres://{user}:{password}@{host}:{port}/{database}",
            host = host.trim(),
            database = database.trim(),
            user = user.trim(),
        );
        Self::from_url(url)
    }

    /// Connection string. Callers must not log this value; it contains secrets.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Host, port, database, and user for the UI. Never includes the password.
    pub fn public_target(&self) -> Option<PublicDatabaseTarget> {
        parse_public_target(&self.url)
    }
}

/// Non-secret connection target shown in the desktop UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicDatabaseTarget {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
}

fn parse_public_target(url: &str) -> Option<PublicDatabaseTarget> {
    let rest = url
        .strip_prefix("postgres://")
        .or_else(|| url.strip_prefix("postgresql://"))?;
    let (auth, host_and_db) = rest.split_once('@')?;
    let user = auth.split(':').next()?.to_string();
    if user.is_empty() {
        return None;
    }
    let (hostport, db_and_query) = host_and_db.split_once('/')?;
    let database = db_and_query.split(['?', '#']).next()?.trim().to_string();
    if database.is_empty() {
        return None;
    }
    let (host, port) = match hostport.rsplit_once(':') {
        Some((host, port)) => (host.to_string(), port.parse().ok()?),
        None => (hostport.to_string(), 5432),
    };
    if host.is_empty() || port == 0 {
        return None;
    }
    Some(PublicDatabaseTarget {
        host,
        port,
        database,
        user,
    })
}

fn required_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_port(raw: Option<&str>) -> Result<u16, DatabaseConfigError> {
    let raw = raw.map(str::trim).filter(|value| !value.is_empty());
    match raw {
        None => Ok(5432),
        Some(value) => value
            .parse::<u16>()
            .map_err(|_| DatabaseConfigError::InvalidPort)
            .and_then(|port| {
                if port == 0 {
                    Err(DatabaseConfigError::InvalidPort)
                } else {
                    Ok(port)
                }
            }),
    }
}

fn has_url_reserved_chars(value: &str) -> bool {
    value.contains(['@', ':', '/', '?', '#', '%', ' '])
}

#[cfg(test)]
mod tests {
    use super::{DatabaseConfig, DatabaseConfigError};

    #[test]
    fn from_url_accepts_postgres_schemes() {
        let config =
            DatabaseConfig::from_url("postgres://vsmart_sync:change-me@localhost:5432/vsmart_sync")
                .expect("url");
        assert!(config.url().starts_with("postgres://"));

        DatabaseConfig::from_url("postgresql://vsmart_sync:change-me@localhost:5432/vsmart_sync")
            .expect("postgresql scheme");
    }

    #[test]
    fn from_url_rejects_empty_and_non_postgres() {
        assert_eq!(
            DatabaseConfig::from_url("   ").unwrap_err(),
            DatabaseConfigError::MissingUrl
        );
        assert_eq!(
            DatabaseConfig::from_url("mysql://localhost/db").unwrap_err(),
            DatabaseConfigError::InvalidUrlScheme
        );
    }

    #[test]
    fn from_parts_builds_url() {
        let config = DatabaseConfig::from_parts(
            "localhost",
            5432,
            "vsmart_sync",
            "vsmart_sync",
            "change-me",
        )
        .expect("parts");
        assert_eq!(
            config.url(),
            "postgres://vsmart_sync:change-me@localhost:5432/vsmart_sync"
        );
    }

    #[test]
    fn from_parts_validates_inputs() {
        assert_eq!(
            DatabaseConfig::from_parts("", 5432, "db", "user", "pw").unwrap_err(),
            DatabaseConfigError::MissingHost
        );
        assert_eq!(
            DatabaseConfig::from_parts("localhost", 0, "db", "user", "pw").unwrap_err(),
            DatabaseConfigError::InvalidPort
        );
        assert_eq!(
            DatabaseConfig::from_parts("localhost", 5432, "", "user", "pw").unwrap_err(),
            DatabaseConfigError::MissingDatabase
        );
        assert_eq!(
            DatabaseConfig::from_parts("localhost", 5432, "db", "", "pw").unwrap_err(),
            DatabaseConfigError::MissingUser
        );
        assert_eq!(
            DatabaseConfig::from_parts("localhost", 5432, "db", "user", "").unwrap_err(),
            DatabaseConfigError::MissingPassword
        );
        assert_eq!(
            DatabaseConfig::from_parts("localhost", 5432, "db", "user:name", "pw").unwrap_err(),
            DatabaseConfigError::UnsafeUrlCredentials
        );
    }

    #[test]
    fn public_target_omits_password() {
        let config =
            DatabaseConfig::from_url("postgres://vsmart_sync:change-me@127.0.0.1:5432/vsmart_sync")
                .expect("url");
        let target = config.public_target().expect("target");
        assert_eq!(target.host, "127.0.0.1");
        assert_eq!(target.port, 5432);
        assert_eq!(target.database, "vsmart_sync");
        assert_eq!(target.user, "vsmart_sync");
        assert!(!target.host.contains("change-me"));
        assert_ne!(target.user, "change-me");
    }
}
