//! Thin Matrix HTTP client — transport only.
//!
//! Owns: HTTP basic auth to the device, timeouts, no redirects, CGI URL
//! construction for documented paths, and `Response-Code` evaluation.
//!
//! Must not own: retries, Sync jobs, domain rules, capability tables, or
//! React-facing types. Do not invent CGI endpoints.

mod response;
pub mod response_codes;

use std::time::Duration;

use reqwest::{Client, StatusCode};
use thiserror::Error;

use self::response::parse_response_code;
use self::response_codes::{documented_label, SUCCESS};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const BASIC_CONFIG_PATH: &str = "/device.cgi/device-basic-config";
const USERS_PATH: &str = "/device.cgi/users";

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
    /// HTTP succeeded but Matrix `Response-Code` was missing, unparsable, or non-zero.
    #[error("MATRIX_API_ERROR:{code}")]
    ApiError { code: i32 },
}

/// Successful CGI exchange after HTTP + `Response-Code=0` checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixHttpSuccess {
    pub body: String,
    pub response_code: i32,
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
    ///
    /// Success requires HTTP 2xx **and** body `Response-Code=0`.
    pub async fn get_device_basic_config(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        let url = build_cgi_get_url(host, port, BASIC_CONFIG_PATH, &[("action", "get")])?;
        self.get_documented(url, username, password).await
    }

    /// GET /device.cgi/users?action=set&…
    ///
    /// Query pairs must already be validated by the adapter (guide limits and
    /// forbidden characters). Does not retry. Does not log credentials or PIN.
    pub async fn users_set(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        fields: &[(&str, &str)],
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        let mut query: Vec<(&str, &str)> = Vec::with_capacity(fields.len() + 1);
        query.push(("action", "set"));
        query.extend_from_slice(fields);
        let url = build_cgi_get_url(host, port, USERS_PATH, &query)?;
        self.get_documented(url, username, password).await
    }

    async fn get_documented(
        &self,
        url: reqwest::Url,
        username: &str,
        password: &str,
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        let response = self
            .client
            .get(url)
            .basic_auth(username, Some(password))
            .send()
            .await
            .map_err(map_transport_error)?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|_| MatrixClientError::BadResponse)?;

        evaluate_matrix_response(status, body)
    }
}

/// HTTP 2xx + `Response-Code=0` → success. Any other Matrix code → `ApiError`.
///
/// Auth failures stay on HTTP status. Missing/unparsable codes are `BadResponse`.
pub fn evaluate_matrix_response(
    status: StatusCode,
    body: String,
) -> Result<MatrixHttpSuccess, MatrixClientError> {
    match status {
        StatusCode::OK | StatusCode::CREATED | StatusCode::ACCEPTED | StatusCode::NO_CONTENT => {
            match parse_response_code(&body) {
                Some(SUCCESS) => Ok(MatrixHttpSuccess {
                    body,
                    response_code: SUCCESS,
                }),
                Some(code) => {
                    tracing::debug!(
                        code,
                        label = documented_label(code).unwrap_or("unknown"),
                        "matrix Response-Code is not success"
                    );
                    Err(MatrixClientError::ApiError { code })
                }
                None => Err(MatrixClientError::BadResponse),
            }
        }
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(MatrixClientError::AuthFailed),
        _ => Err(MatrixClientError::BadResponse),
    }
}

fn build_cgi_get_url(
    host: &str,
    port: u16,
    path: &str,
    query: &[(&str, &str)],
) -> Result<reqwest::Url, MatrixClientError> {
    if host.is_empty() || host.contains(['/', '?', '#', '@', ':', ' ', '\\']) {
        return Err(MatrixClientError::InvalidTarget);
    }
    if port == 0 {
        return Err(MatrixClientError::InvalidTarget);
    }
    if !path.starts_with("/device.cgi/") {
        return Err(MatrixClientError::InvalidTarget);
    }

    let mut url =
        reqwest::Url::parse("http://127.0.0.1/").map_err(|_| MatrixClientError::InvalidTarget)?;
    url.set_scheme("http")
        .map_err(|_| MatrixClientError::InvalidTarget)?;
    url.set_host(Some(host))
        .map_err(|_| MatrixClientError::InvalidTarget)?;
    url.set_port(Some(port))
        .map_err(|_| MatrixClientError::InvalidTarget)?;
    url.set_path(path);

    {
        let mut pairs = url.query_pairs_mut();
        pairs.clear();
        for (key, value) in query {
            pairs.append_pair(key, value);
        }
    }

    if url.scheme() != "http" {
        return Err(MatrixClientError::InvalidTarget);
    }
    if url.username() != "" || url.password().is_some() {
        return Err(MatrixClientError::InvalidTarget);
    }
    if url.path() != path {
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
    use super::{
        build_cgi_get_url, evaluate_matrix_response, MatrixClientError, MatrixHttpClient,
        BASIC_CONFIG_PATH, USERS_PATH,
    };
    use crate::matrix::client::response_codes::{FAILURE, REFERENCE_USER_ID_EXISTS, SUCCESS};
    use reqwest::StatusCode;
    use wiremock::matchers::{basic_auth, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn builds_documented_http_url_only() {
        let url =
            build_cgi_get_url("192.168.1.10", 80, BASIC_CONFIG_PATH, &[("action", "get")]).unwrap();
        assert_eq!(
            url.as_str(),
            "http://192.168.1.10/device.cgi/device-basic-config?action=get"
        );
        assert!(
            build_cgi_get_url("http://evil", 80, BASIC_CONFIG_PATH, &[("action", "get")]).is_err()
        );
        assert!(build_cgi_get_url(
            "192.168.1.10/x",
            80,
            BASIC_CONFIG_PATH,
            &[("action", "get")]
        )
        .is_err());
    }

    #[test]
    fn builds_users_set_query_with_encoding() {
        let url = build_cgi_get_url(
            "192.168.1.10",
            80,
            USERS_PATH,
            &[
                ("action", "set"),
                ("user-id", "VS000001"),
                ("ref-user-id", "10000001"),
                ("user-pin", ""),
            ],
        )
        .unwrap();
        assert_eq!(url.path(), USERS_PATH);
        let q = url.query().unwrap();
        assert!(q.contains("action=set"));
        assert!(q.contains("user-id=VS000001"));
        assert!(q.contains("ref-user-id=10000001"));
        assert!(q.contains("user-pin="));
    }

    #[test]
    fn evaluate_requires_response_code_zero() {
        let ok = evaluate_matrix_response(StatusCode::OK, "Response-Code=0".into()).unwrap();
        assert_eq!(ok.response_code, SUCCESS);

        assert_eq!(
            evaluate_matrix_response(
                StatusCode::OK,
                format!("Response-Code={REFERENCE_USER_ID_EXISTS}")
            ),
            Err(MatrixClientError::ApiError {
                code: REFERENCE_USER_ID_EXISTS
            })
        );
        assert_eq!(
            evaluate_matrix_response(StatusCode::OK, format!("Response-Code={FAILURE}")),
            Err(MatrixClientError::ApiError { code: FAILURE })
        );
        assert_eq!(
            evaluate_matrix_response(StatusCode::OK, "ok".into()),
            Err(MatrixClientError::BadResponse)
        );
    }

    #[tokio::test]
    async fn success_on_200_with_response_code_zero() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/device-basic-config"))
            .and(query_param("action", "get"))
            .and(basic_auth("admin", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Response-Code=0"))
            .mount(&server)
            .await;

        let client = MatrixHttpClient::new().unwrap();
        let port = server.address().port();
        let result = client
            .get_device_basic_config("127.0.0.1", port, "admin", "secret")
            .await
            .unwrap();
        assert_eq!(result.response_code, SUCCESS);
    }

    #[tokio::test]
    async fn users_set_success_and_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/users"))
            .and(query_param("action", "set"))
            .and(query_param("user-id", "VS000001"))
            .and(query_param("ref-user-id", "10000001"))
            .and(basic_auth("admin", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Response-Code=0"))
            .mount(&server)
            .await;

        let client = MatrixHttpClient::new().unwrap();
        let port = server.address().port();
        client
            .users_set(
                "127.0.0.1",
                port,
                "admin",
                "secret",
                &[("user-id", "VS000001"), ("ref-user-id", "10000001")],
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn http_200_with_nonzero_response_code_is_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/device-basic-config"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Response-Code=24"))
            .mount(&server)
            .await;

        let client = MatrixHttpClient::new().unwrap();
        let err = client
            .get_device_basic_config("127.0.0.1", server.address().port(), "admin", "secret")
            .await
            .unwrap_err();
        assert_eq!(err, MatrixClientError::ApiError { code: 24 });
    }

    #[tokio::test]
    async fn http_200_without_response_code_is_bad_response() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/device-basic-config"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
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
