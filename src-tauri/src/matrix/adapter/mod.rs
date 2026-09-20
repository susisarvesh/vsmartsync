//! Matrix adapter.
//!
//! Translates application reachability checks into the thin documented CGI
//! call. Domain code must not construct `device.cgi` URLs.

use crate::matrix::client::{MatrixClientError, MatrixHttpClient};

#[derive(Debug, PartialEq, Eq)]
pub enum MatrixProbeError {
    Timeout,
    Unreachable,
    AuthFailed,
    BadResponse,
    InvalidTarget,
}

pub struct MatrixAdapter {
    client: MatrixHttpClient,
}

impl MatrixAdapter {
    pub fn new() -> Result<Self, MatrixProbeError> {
        Ok(Self {
            client: MatrixHttpClient::new().map_err(|_| MatrixProbeError::Unreachable)?,
        })
    }

    /// Confirms the application can authenticate to the device via the
    /// documented basic-config get action.
    pub async fn probe_basic_config(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<(), MatrixProbeError> {
        self.client
            .get_device_basic_config(host, port, username, password)
            .await
            .map_err(map_client_error)
    }
}

fn map_client_error(error: MatrixClientError) -> MatrixProbeError {
    match error {
        MatrixClientError::Timeout => MatrixProbeError::Timeout,
        MatrixClientError::Unreachable => MatrixProbeError::Unreachable,
        MatrixClientError::AuthFailed => MatrixProbeError::AuthFailed,
        MatrixClientError::BadResponse => MatrixProbeError::BadResponse,
        MatrixClientError::InvalidTarget => MatrixProbeError::InvalidTarget,
    }
}
