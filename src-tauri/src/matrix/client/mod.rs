//! Thin Matrix HTTP client for documented CGI paths only.

use std::time::Duration;

use reqwest::{Client, StatusCode};
use thiserror::Error;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const BASIC_CONFIG_PATH: &str = "/device.cgi/device-basic-config";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MatrixClientError {
    #[error("MATRIX_TIMEOUT")]
    Timeout,
    #[error("MATRIX_UNREACHABLE")]
    Unreachable,
    #[error("MATRIX_AUTH_FAILED")]
    AuthFailed,
    #[error("MATRIX_BAD_RESPONSE")]
    BadResponse,
    #[error("MATRIX_INVALID_TARGET")]
    InvalidTarget,
}

#[derive(Debug, Clone)]
pub struct MatrixHttpClient {
    client: Client,
}

impl MatrixHttpClient {
    pub fn new() -> Result<Self, MatrixClientError> {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| MatrixClientError::Unreachable)?;
        Ok(Self { client })
    }

    /// GET /device.cgi/device-basic-config?action=get
    ///
    /// Documented in COSEC Devices API User Guide v28. Host and port must
    /// already be validated by the devices domain (host-only, no URL input).
    pub async fn get_device_basic_config(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<(), MatrixClientError> {
        let url = build_basic_config_url(host, port)?;
        let response = self
            .client
            .get(url)
            .basic_auth(username, Some(password))
            .send()
            .await
            .map_err(map_transport_error)?;

        match response.status() {
            StatusCode::OK
            | StatusCode::CREATED
            | StatusCode::ACCEPTED
            | StatusCode::NO_CONTENT => {
                let _body = response
                    .text()
                    .await
                    .map_err(|_| MatrixClientError::BadResponse)?;
                Ok(())
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(MatrixClientError::AuthFailed),
            _ => Err(MatrixClientError::BadResponse),
        }
    }
}

fn build_basic_config_url(host: &str, port: u16) -> Result<reqwest::Url, MatrixClientError> {
    if host.is_empty() || host.contains(['/', '?', '#', '@', ':', ' ', '\\']) {
        return Err(MatrixClientError::InvalidTarget);
    }
    if port == 0 {
        return Err(MatrixClientError::InvalidTarget);
    }

    let mut url = reqwest::Url::parse("http://127.0.0.1/").map_err(|_| MatrixClientError::InvalidTarget)?;
    url.set_scheme("http")
        .map_err(|_| MatrixClientError::InvalidTarget)?;
    url.set_host(Some(host))
        .map_err(|_| MatrixClientError::InvalidTarget)?;
    url.set_port(Some(port))
        .map_err(|_| MatrixClientError::InvalidTarget)?;
    url.set_path(BASIC_CONFIG_PATH);
    url.set_query(Some("action=get"));

    if url.scheme() != "http" {
        return Err(MatrixClientError::InvalidTarget);
    }
    if url.username() != "" || url.password().is_some() {
        return Err(MatrixClientError::InvalidTarget);
    }
    if url.path() != BASIC_CONFIG_PATH {
        return Err(MatrixClientError::InvalidTarget);
    }
    Ok(url)
}

fn map_transport_error(error: reqwest::Error) -> MatrixClientError {
    if error.is_timeout() {
        MatrixClientError::Timeout
    } else {
        MatrixClientError::Unreachable
    }
}

#[cfg(test)]
mod tests {
    use super::{build_basic_config_url, MatrixClientError, MatrixHttpClient};
    use wiremock::matchers::{basic_auth, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn builds_documented_http_url_only() {
        let url = build_basic_config_url("192.168.1.10", 80).unwrap();
        assert_eq!(
            url.as_str(),
            "http://192.168.1.10/device.cgi/device-basic-config?action=get"
        );
        assert!(build_basic_config_url("http://evil", 80).is_err());
        assert!(build_basic_config_url("192.168.1.10/x", 80).is_err());
    }

    #[tokio::test]
    async fn success_on_200() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/device-basic-config"))
            .and(query_param("action", "get"))
            .and(basic_auth("admin", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;

        let client = MatrixHttpClient::new().unwrap();
        let port = server.address().port();
        client
            .get_device_basic_config("127.0.0.1", port, "admin", "secret")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn maps_auth_failure() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/device-basic-config"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = MatrixHttpClient::new().unwrap();
        let err = client
            .get_device_basic_config("127.0.0.1", server.address().port(), "admin", "bad")
            .await
            .unwrap_err();
        assert_eq!(err, MatrixClientError::AuthFailed);
    }

    #[tokio::test]
    async fn maps_unexpected_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/device-basic-config"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = MatrixHttpClient::new().unwrap();
        let err = client
            .get_device_basic_config("127.0.0.1", server.address().port(), "admin", "secret")
            .await
            .unwrap_err();
        assert_eq!(err, MatrixClientError::BadResponse);
    }

    #[tokio::test]
    async fn connection_refused_is_unreachable() {
        let client = MatrixHttpClient::new().unwrap();
        let err = client
            .get_device_basic_config("127.0.0.1", 1, "admin", "secret")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            MatrixClientError::Unreachable | MatrixClientError::Timeout
        ));
    }
}
