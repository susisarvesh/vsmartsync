//! Thin Matrix HTTP client — transport only.
//!
//! Owns: HTTP auth handshake, timeouts, no redirects, CGI URL construction
//! for documented paths, and `Response-Code` evaluation.
//!
//! The first request is unauthenticated. A Digest challenge (`MD5`, `qop=auth`)
//! is answered with Digest. A Basic challenge is answered with Basic.
//! At most two follow-up requests: the challenge response, then one `stale=true`
//! Digest refresh. That handshake is not a Sync retry.
//!
//! Must not own: Sync jobs, domain rules, capability tables, or React-facing
//! types. Do not invent CGI endpoints. Never log passwords, `Authorization`,
//! or Digest nonce material.

mod response;
pub mod response_codes;

use std::time::Duration;

use digest_auth::{AlgorithmType, AuthContext, Qop};
use reqwest::header::{HeaderMap, AUTHORIZATION, WWW_AUTHENTICATE};
use reqwest::{Client, StatusCode, Url};
use thiserror::Error;

use self::response::parse_response_code;
use self::response_codes::{documented_label, SUCCESS};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Unauthenticated request, then at most two challenge responses.
const MAX_AUTH_FOLLOWUPS: u8 = 2;
const BASIC_CONFIG_PATH: &str = "/device.cgi/device-basic-config";
const READER_CONFIG_PATH: &str = "/device.cgi/reader-config";
const ENROLL_OPTIONS_PATH: &str = "/device.cgi/enroll-options";
const ENROLL_USER_PATH: &str = "/device.cgi/enrolluser";
const COMMAND_PATH: &str = "/device.cgi/command";
const USERS_PATH: &str = "/device.cgi/users";
const ENROLL_TIMEOUT: Duration = Duration::from_secs(90);

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
            .connect_timeout(CONNECT_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| MatrixClientError::Unreachable)?;
        Ok(Self { client })
    }

    /// GET /device.cgi/device-basic-config?action=get&format=xml
    ///
    /// Host and port must already be validated by the devices domain.
    /// Success is HTTP 2xx with `Response-Code=0`, or HTTP 2xx whose body is
    /// the documented `<COSEC_API>` configuration document.
    pub async fn get_device_basic_config(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        let url = build_cgi_get_url(
            host,
            port,
            BASIC_CONFIG_PATH,
            &[("action", "get"), ("format", "xml")],
        )?;
        let (status, body) = self
            .exchange(url, username, password, REQUEST_TIMEOUT)
            .await?;
        evaluate_basic_config_response(status, body)
    }

    /// GET /device.cgi/reader-config?action=get&format=xml
    pub async fn get_reader_config(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        self.get_config_document(host, port, username, password, READER_CONFIG_PATH)
            .await
    }

    /// GET /device.cgi/reader-config?action=set&door-access-mode=
    ///
    /// Sends only the access mode so other reader fields stay as they are.
    pub async fn set_door_access_mode(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        mode: &str,
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        let url = build_cgi_get_url(
            host,
            port,
            READER_CONFIG_PATH,
            &[("action", "set"), ("door-access-mode", mode)],
        )?;
        self.get_documented(url, username, password, REQUEST_TIMEOUT)
            .await
    }

    /// GET /device.cgi/enroll-options?action=get&format=xml
    pub async fn get_enroll_options(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        self.get_config_document(host, port, username, password, ENROLL_OPTIONS_PATH)
            .await
    }

    /// GET /device.cgi/enrolluser?action=enroll&type=&user-id=&count=
    ///
    /// Starts capture on the device and waits longer than a normal config read
    /// so the person can present a card or face while the device holds the call.
    /// The guide says this response is not the credential itself.
    pub async fn enroll_user(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        matrix_user_id: &str,
        enroll_type: &str,
        counts: &[(&str, &str)],
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        let mut query = vec![
            ("action", "enroll"),
            ("type", enroll_type),
            ("user-id", matrix_user_id),
        ];
        query.extend_from_slice(counts);
        let url = build_cgi_get_url(host, port, ENROLL_USER_PATH, &query)?;
        self.get_documented(url, username, password, ENROLL_TIMEOUT)
            .await
    }

    /// GET /device.cgi/command?action=getcount&user-id=
    ///
    /// Returns enrolled card, face, and finger counts for that user.
    pub async fn get_credential_counts(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        matrix_user_id: &str,
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        let url = build_cgi_get_url(
            host,
            port,
            COMMAND_PATH,
            &[
                ("action", "getcount"),
                ("user-id", matrix_user_id),
                ("format", "text"),
            ],
        )?;
        let (status, body) = self
            .exchange(url, username, password, REQUEST_TIMEOUT)
            .await?;
        evaluate_count_response(status, body)
    }

    async fn get_config_document(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        path: &str,
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        let url = build_cgi_get_url(host, port, path, &[("action", "get"), ("format", "xml")])?;
        let (status, body) = self
            .exchange(url, username, password, REQUEST_TIMEOUT)
            .await?;
        evaluate_config_response(status, body)
    }

    /// GET /device.cgi/users?action=get&user-id=
    ///
    /// The guide sample is a field list (`user-index=`) rather than only
    /// `Response-Code=0`. Card numbers in that body are not logged.
    pub async fn get_user(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        matrix_user_id: &str,
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        let url = build_cgi_get_url(
            host,
            port,
            USERS_PATH,
            &[
                ("action", "get"),
                ("user-id", matrix_user_id),
                ("format", "text"),
            ],
        )?;
        let (status, body) = self
            .exchange(url, username, password, REQUEST_TIMEOUT)
            .await?;
        if status.is_success() && user_record_body(&body) {
            return Ok(MatrixHttpSuccess {
                body,
                response_code: SUCCESS,
            });
        }
        evaluate_matrix_response(status, body)
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
        self.get_documented(url, username, password, REQUEST_TIMEOUT)
            .await
    }

    /// GET /device.cgi/users?action=delete&user-id=
    ///
    /// Guide: deleting a user also removes that user's credentials on the device.
    pub async fn users_delete(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        matrix_user_id: &str,
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        let url = build_cgi_get_url(
            host,
            port,
            USERS_PATH,
            &[("action", "delete"), ("user-id", matrix_user_id)],
        )?;
        self.get_documented(url, username, password, REQUEST_TIMEOUT)
            .await
    }

    async fn get_documented(
        &self,
        url: Url,
        username: &str,
        password: &str,
        timeout: Duration,
    ) -> Result<MatrixHttpSuccess, MatrixClientError> {
        let (status, body) = self.exchange(url, username, password, timeout).await?;
        evaluate_matrix_response(status, body)
    }

    async fn exchange(
        &self,
        url: Url,
        username: &str,
        password: &str,
        timeout: Duration,
    ) -> Result<(StatusCode, String), MatrixClientError> {
        let mut authorization = None;
        let mut followups = 0u8;
        loop {
            let response = self
                .send_get(&url, username, password, authorization.as_ref(), timeout)
                .await?;
            let status = response.status();
            if status != StatusCode::UNAUTHORIZED {
                let body = response
                    .text()
                    .await
                    .map_err(|_| MatrixClientError::BadResponse)?;
                return Ok((status, body));
            }
            if followups >= MAX_AUTH_FOLLOWUPS {
                return Err(MatrixClientError::AuthFailed);
            }
            let challenge = www_authenticate(response.headers());
            authorization = Some(authorization_for_challenge(
                challenge.as_deref(),
                username,
                password,
                &url,
                followups,
            )?);
            followups += 1;
        }
    }

    async fn send_get(
        &self,
        url: &Url,
        username: &str,
        password: &str,
        authorization: Option<&Authorization>,
        timeout: Duration,
    ) -> Result<reqwest::Response, MatrixClientError> {
        let mut request = self.client.get(url.clone()).timeout(timeout);
        match authorization {
            None => {}
            Some(Authorization::Basic) => {
                request = request.basic_auth(username, Some(password));
            }
            Some(Authorization::Digest(header)) => {
                request = request.header(AUTHORIZATION, header);
            }
        }
        request.send().await.map_err(map_transport_error)
    }
}

enum Authorization {
    Basic,
    Digest(String),
}

fn www_authenticate(headers: &HeaderMap) -> Option<String> {
    headers
        .get(WWW_AUTHENTICATE)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn authorization_for_challenge(
    challenge: Option<&str>,
    username: &str,
    password: &str,
    url: &Url,
    followup: u8,
) -> Result<Authorization, MatrixClientError> {
    let Some(raw) = challenge else {
        return Err(MatrixClientError::AuthFailed);
    };
    if starts_with_scheme(raw, "digest") {
        return digest_authorization(raw, username, password, url, followup);
    }
    if followup == 0 && starts_with_scheme(raw, "basic") {
        return Ok(Authorization::Basic);
    }
    Err(MatrixClientError::AuthFailed)
}

fn digest_authorization(
    raw: &str,
    username: &str,
    password: &str,
    url: &Url,
    followup: u8,
) -> Result<Authorization, MatrixClientError> {
    let mut prompt = digest_auth::parse(raw).map_err(|_| MatrixClientError::AuthFailed)?;
    if prompt.algorithm.algo != AlgorithmType::MD5 || prompt.algorithm.sess {
        return Err(MatrixClientError::AuthFailed);
    }
    match &prompt.qop {
        Some(options) if options.contains(&Qop::AUTH) && !options.contains(&Qop::AUTH_INT) => {}
        Some(_) => return Err(MatrixClientError::AuthFailed),
        None => {}
    }
    if followup > 0 && !prompt.stale {
        return Err(MatrixClientError::AuthFailed);
    }
    let context = AuthContext::new(username, password, digest_uri(url));
    let header = prompt
        .respond(&context)
        .map_err(|_| MatrixClientError::AuthFailed)?;
    Ok(Authorization::Digest(header.to_string()))
}

fn digest_uri(url: &Url) -> String {
    match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_string(),
    }
}

fn starts_with_scheme(value: &str, scheme: &str) -> bool {
    let rest = value.get(scheme.len()..);
    value.len() >= scheme.len()
        && value[..scheme.len()].eq_ignore_ascii_case(scheme)
        && rest.is_some_and(|rest| rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace()))
}

/// Probe success: `Response-Code=0`, or a 2xx `<COSEC_API>` document.
///
/// The ARGO FACE `format=xml` sample has no `Response-Code` element.
pub fn evaluate_basic_config_response(
    status: StatusCode,
    body: String,
) -> Result<MatrixHttpSuccess, MatrixClientError> {
    match evaluate_matrix_response(status, body.clone()) {
        Ok(success) => Ok(success),
        Err(MatrixClientError::BadResponse) if is_cosec_basic_config_xml(status, &body) => {
            Ok(MatrixHttpSuccess {
                body,
                response_code: SUCCESS,
            })
        }
        Err(error) => Err(error),
    }
}

fn is_cosec_basic_config_xml(status: StatusCode, body: &str) -> bool {
    if !status.is_success() {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    lower.contains("<cosec_api") && lower.contains("<name>") && lower.contains("</name>")
}

/// Config get success: `Response-Code=0`, or a 2xx `<COSEC_API>` document.
pub fn evaluate_config_response(
    status: StatusCode,
    body: String,
) -> Result<MatrixHttpSuccess, MatrixClientError> {
    match evaluate_matrix_response(status, body.clone()) {
        Ok(success) => Ok(success),
        Err(MatrixClientError::BadResponse) if is_cosec_config_document(status, &body) => {
            Ok(MatrixHttpSuccess {
                body,
                response_code: SUCCESS,
            })
        }
        Err(error) => Err(error),
    }
}

fn user_record_body(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("user-index=") || lower.contains("<user-index>")
}

/// `getcount` often returns the count fields, or a `<COSEC_API>` document,
/// without `Response-Code`. A non-zero code is still a failure.
pub fn evaluate_count_response(
    status: StatusCode,
    body: String,
) -> Result<MatrixHttpSuccess, MatrixClientError> {
    match evaluate_matrix_response(status, body.clone()) {
        Ok(success) => Ok(success),
        Err(MatrixClientError::BadResponse)
            if status.is_success() && count_fields_present(&body) =>
        {
            Ok(MatrixHttpSuccess {
                body,
                response_code: SUCCESS,
            })
        }
        Err(error) => Err(error),
    }
}

fn count_fields_present(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("face-count") || lower.contains("card-count") || lower.contains("finger-count")
}

fn is_cosec_config_document(status: StatusCode, body: &str) -> bool {
    status.is_success() && body.to_ascii_lowercase().contains("<cosec_api")
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
        build_cgi_get_url, evaluate_basic_config_response, evaluate_count_response,
        evaluate_matrix_response, MatrixClientError, MatrixHttpClient, BASIC_CONFIG_PATH,
        USERS_PATH,
    };
    use crate::matrix::client::response_codes::{FAILURE, REFERENCE_USER_ID_EXISTS, SUCCESS};
    use reqwest::StatusCode;
    use wiremock::matchers::{basic_auth, method, path, query_param};
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

    struct DigestAuthorization;

    impl Match for DigestAuthorization {
        fn matches(&self, request: &Request) -> bool {
            request
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.starts_with("Digest "))
        }
    }

    fn digest_authorization() -> DigestAuthorization {
        DigestAuthorization
    }

    struct HeaderContains(&'static str);

    impl Match for HeaderContains {
        fn matches(&self, request: &Request) -> bool {
            request
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.contains(self.0))
        }
    }

    fn header_contains(fragment: &'static str) -> HeaderContains {
        HeaderContains(fragment)
    }

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

    #[test]
    fn count_document_without_response_code_is_success() {
        let body = "<COSEC_API><face-count>0</face-count><card-count>0</card-count></COSEC_API>";
        let ok = evaluate_count_response(StatusCode::OK, body.to_string()).unwrap();
        assert_eq!(ok.response_code, SUCCESS);
        assert_eq!(
            evaluate_count_response(StatusCode::OK, "face-count=0".into())
                .unwrap()
                .response_code,
            SUCCESS
        );
        assert_eq!(
            evaluate_count_response(StatusCode::OK, "Response-Code=16".into()),
            Err(MatrixClientError::ApiError { code: 16 })
        );
    }

    #[test]
    fn xml_basic_config_without_response_code_is_success() {
        let body = "<COSEC_API><app>1</app><name>Door</name><max-faces>9</max-faces></COSEC_API>";
        let ok = evaluate_basic_config_response(StatusCode::OK, body.to_string()).unwrap();
        assert_eq!(ok.response_code, SUCCESS);
        assert_eq!(
            evaluate_basic_config_response(StatusCode::OK, "ok".into()),
            Err(MatrixClientError::BadResponse)
        );
        assert_eq!(
            evaluate_basic_config_response(StatusCode::UNAUTHORIZED, body.to_string()),
            Err(MatrixClientError::AuthFailed)
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
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/device-basic-config"))
            .and(query_param("action", "get"))
            .respond_with(
                ResponseTemplate::new(401)
                    .insert_header("WWW-Authenticate", "Basic realm=\"device\""),
            )
            .with_priority(2)
            .mount(&server)
            .await;

        let client = MatrixHttpClient::new().unwrap();
        let port = server.address().port();
        let result = client
            .get_device_basic_config("127.0.0.1", port, "admin", "secret")
            .await
            .unwrap();
        assert_eq!(result.response_code, SUCCESS);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].headers.get("authorization").is_none());
        let second = requests[1]
            .headers
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(second.starts_with("Basic "));
        assert!(!second.contains("secret"));
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
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/users"))
            .respond_with(
                ResponseTemplate::new(401)
                    .insert_header("WWW-Authenticate", "Basic realm=\"device\""),
            )
            .with_priority(2)
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

    #[test]
    fn argo_face_digest_challenge_is_accepted() {
        let challenge = concat!(
            "Digest realm=\"Authenticate Yourself\", domain=\"127.0.1.1\", qop=\"auth\", ",
            "nonce=\"testnonce\", opaque=\"5ccc069c403ebaf9f0171e9517f40e41\", ",
            "algorithm=\"MD5\", stale=\"FALSE\""
        );
        let url = reqwest::Url::parse(
            "http://192.168.0.11/device.cgi/device-basic-config?action=get&format=xml",
        )
        .unwrap();
        let authorization =
            super::authorization_for_challenge(Some(challenge), "admin", "secret", &url, 0)
                .unwrap();
        match authorization {
            super::Authorization::Digest(header) => {
                assert!(header.starts_with("Digest "));
                assert!(header
                    .contains("uri=\"/device.cgi/device-basic-config?action=get&format=xml\""));
                assert!(header.contains("qop=auth"));
                assert!(!header.contains("secret"));
            }
            super::Authorization::Basic => panic!("expected digest"),
        }
    }

    #[tokio::test]
    async fn digest_challenge_is_answered_without_sending_basic() {
        let server = MockServer::start().await;
        let challenge = concat!(
            "Digest realm=\"Authenticate Yourself\", qop=\"auth\", ",
            "nonce=\"testnonce\", opaque=\"abc\", algorithm=\"MD5\", stale=\"false\""
        );
        Mock::given(method("GET"))
            .and(path("/device.cgi/device-basic-config"))
            .and(query_param("action", "get"))
            .and(digest_authorization())
            .respond_with(ResponseTemplate::new(200).set_body_string("Response-Code=0"))
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/device-basic-config"))
            .and(query_param("action", "get"))
            .respond_with(ResponseTemplate::new(401).insert_header("WWW-Authenticate", challenge))
            .with_priority(2)
            .mount(&server)
            .await;

        let client = MatrixHttpClient::new().unwrap();
        client
            .get_device_basic_config("127.0.0.1", server.address().port(), "admin", "secret")
            .await
            .unwrap();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].headers.get("authorization").is_none());
        let header = requests[1]
            .headers
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(header.starts_with("Digest "));
        assert!(header.contains("username=\"admin\""));
        assert!(header.contains("uri=\"/device.cgi/device-basic-config?action=get&format=xml\""));
        assert!(header.contains("qop=auth"));
        assert!(header.contains("algorithm=MD5"));
        assert!(!header.contains("secret"));
        assert!(!header.to_ascii_lowercase().contains("basic "));
    }

    #[tokio::test]
    async fn stale_digest_challenge_is_refreshed_once() {
        let server = MockServer::start().await;
        let first = concat!(
            "Digest realm=\"Authenticate Yourself\", qop=\"auth\", ",
            "nonce=\"nonce-1\", opaque=\"abc\", algorithm=\"MD5\", stale=\"false\""
        );
        let stale = concat!(
            "Digest realm=\"Authenticate Yourself\", qop=\"auth\", ",
            "nonce=\"nonce-2\", opaque=\"abc\", algorithm=\"MD5\", stale=\"true\""
        );
        Mock::given(method("GET"))
            .and(path("/device.cgi/device-basic-config"))
            .and(header_contains("nonce=\"nonce-2\""))
            .respond_with(ResponseTemplate::new(200).set_body_string("Response-Code=0"))
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/device-basic-config"))
            .and(header_contains("nonce=\"nonce-1\""))
            .respond_with(ResponseTemplate::new(401).insert_header("WWW-Authenticate", stale))
            .with_priority(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/device-basic-config"))
            .respond_with(ResponseTemplate::new(401).insert_header("WWW-Authenticate", first))
            .with_priority(3)
            .mount(&server)
            .await;

        let client = MatrixHttpClient::new().unwrap();
        client
            .get_device_basic_config("127.0.0.1", server.address().port(), "admin", "secret")
            .await
            .unwrap();
        assert_eq!(server.received_requests().await.unwrap().len(), 3);
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

    #[tokio::test]
    async fn enroll_user_sends_documented_type_and_user_id() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/enrolluser"))
            .and(query_param("action", "enroll"))
            .and(query_param("type", "7"))
            .and(query_param("user-id", "VS000001"))
            .and(query_param("face-count", "0"))
            .and(basic_auth("admin", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Response-Code=0"))
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/enrolluser"))
            .respond_with(
                ResponseTemplate::new(401)
                    .insert_header("WWW-Authenticate", "Basic realm=\"device\""),
            )
            .with_priority(2)
            .mount(&server)
            .await;

        let client = MatrixHttpClient::new().unwrap();
        client
            .enroll_user(
                "127.0.0.1",
                server.address().port(),
                "admin",
                "secret",
                "VS000001",
                "7",
                &[("face-count", "0")],
            )
            .await
            .unwrap();
    }
}
