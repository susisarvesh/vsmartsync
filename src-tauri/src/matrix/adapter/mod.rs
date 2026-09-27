//! Matrix adapter — application-facing Matrix operations.
//!
//! Translates domain-oriented calls into documented CGI via the HTTP client.
//! Domain code must not construct `device.cgi` URLs.
//!
//! **This slice:** `probe_basic_config`, `set_user`, `set_pin`.
//! **Deferred:** `set_card` (Card path not hardware-/example-verified), Sync,
//! `device_users`, capability framework, retries, orchestration.

use thiserror::Error;

use crate::matrix::client::{MatrixClientError, MatrixHttpClient};
use crate::matrix::enrollment::{
    config_field, credential_counts, face_only_door, supported_enroll_types, CredentialCounts,
    HardwareEnrollType,
};

/// Reachability probe errors (Devices domain). Subset of transport failures.
#[derive(Debug, PartialEq, Eq)]
pub enum MatrixProbeError {
    Timeout,
    Unreachable,
    AuthFailed,
    BadResponse,
    InvalidTarget,
}

/// Errors from application-facing adapter operations (`set_user`, `set_pin`).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MatrixAdapterError {
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
    #[error("MATRIX_INVALID_ARGUMENT")]
    InvalidArgument,
    #[error("MATRIX_API_ERROR:{code}")]
    ApiError { code: i32 },
}

/// Establish or update a Matrix user via `/device.cgi/users?action=set`.
///
/// Callers supply Matrix device-side identifiers (not application UUIDs).
/// Create requires both `user_id` and `ref_user_id` per the guide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetUserParams {
    pub user_id: String,
    pub ref_user_id: u32,
    pub name: Option<String>,
    pub user_active: Option<bool>,
    /// Guide: `enable-fr` on `/users`. `1` turns face recognition on for this user.
    pub enable_fr: Option<bool>,
}

/// Set or clear PIN for an **existing** Matrix user (`user-pin` on `/users`).
///
/// Does not create users and does not send `ref-user-id`. Empty `pin` clears
/// the PIN (`user-pin=`), which the guide documents as allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetPinParams {
    pub user_id: String,
    pub pin: String,
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
    /// documented basic-config get action (`Response-Code=0`).
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
            .map(|_| ())
            .map_err(map_probe_error)
    }

    /// Ensure Matrix user configuration (`/users?action=set`).
    ///
    /// Does not set PIN or Card. Does not retry. Does not orchestrate Sync.
    pub async fn set_user(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        params: &SetUserParams,
    ) -> Result<(), MatrixAdapterError> {
        let user_id = validate_user_id(&params.user_id)?;
        let ref_user_id = validate_ref_user_id(params.ref_user_id)?;
        let ref_str = ref_user_id.to_string();

        let mut owned: Vec<(String, String)> =
            vec![("user-id".into(), user_id), ("ref-user-id".into(), ref_str)];
        if let Some(raw_name) = &params.name {
            owned.push(("name".into(), validate_user_name(raw_name)?));
        }
        if let Some(active) = params.user_active {
            owned.push((
                "user-active".into(),
                if active { "1".into() } else { "0".into() },
            ));
        }
        if let Some(enabled) = params.enable_fr {
            owned.push((
                "enable-fr".into(),
                if enabled { "1".into() } else { "0".into() },
            ));
        }

        let fields: Vec<(&str, &str)> = owned
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        self.client
            .users_set(host, port, username, password, &fields)
            .await
            .map(|_| ())
            .map_err(map_adapter_error)
    }

    /// Delete a user on the device (`/users?action=delete`).
    ///
    /// The guide says this also removes that user's credentials on the device.
    /// Response codes 10 and 13 mean the user is already absent.
    pub async fn delete_user(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        matrix_user_id: &str,
    ) -> Result<(), MatrixAdapterError> {
        let user_id = validate_user_id(matrix_user_id)?;
        match self
            .client
            .users_delete(host, port, username, password, user_id.as_str())
            .await
        {
            Ok(_) => Ok(()),
            Err(crate::matrix::client::MatrixClientError::ApiError { code })
                if device_user_already_gone(code) =>
            {
                Ok(())
            }
            Err(error) => Err(map_adapter_error(error)),
        }
    }

    /// Set PIN for an existing Matrix user (`user-pin` on `/users?action=set`).
    ///
    /// Requires a Matrix `user-id` that already exists on the device (Sync
    /// responsibility). Does not send Card fields. Does not retry.
    pub async fn set_pin(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        params: &SetPinParams,
    ) -> Result<(), MatrixAdapterError> {
        let user_id = validate_user_id(&params.user_id)?;
        let pin = validate_user_pin(&params.pin)?;

        self.client
            .users_set(
                host,
                port,
                username,
                password,
                &[("user-id", user_id.as_str()), ("user-pin", pin.as_str())],
            )
            .await
            .map(|_| ())
            .map_err(map_adapter_error)
    }

    /// Read basic config, reader config, and enroll options, then return the
    /// enrollment types those documents report.
    pub async fn enrollment_capabilities(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<Vec<HardwareEnrollType>, MatrixAdapterError> {
        let basic = self
            .client
            .get_device_basic_config(host, port, username, password)
            .await
            .map_err(map_adapter_error)?;
        let reader = self
            .client
            .get_reader_config(host, port, username, password)
            .await
            .map_err(map_adapter_error)?;
        let options = self
            .client
            .get_enroll_options(host, port, username, password)
            .await
            .map_err(map_adapter_error)?;
        Ok(supported_enroll_types(
            &basic.body,
            &reader.body,
            &options.body,
        ))
    }

    /// Turn face recognition on or off for an existing Matrix user (`enable-fr`).
    ///
    /// The guide allows an update with only the alphanumeric user id.
    pub async fn set_face_recognition_enabled(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        matrix_user_id: &str,
        enabled: bool,
    ) -> Result<(), MatrixAdapterError> {
        let user_id = validate_user_id(matrix_user_id)?;
        let flag = if enabled { "1" } else { "0" };
        self.client
            .users_set(
                host,
                port,
                username,
                password,
                &[("user-id", user_id.as_str()), ("enable-fr", flag)],
            )
            .await
            .map(|_| ())
            .map_err(map_adapter_error)
    }

    /// Face-only doors do not open a card prompt. Card & Face keeps face and
    /// allows read-only card enrollment.
    pub async fn enable_card_and_face_access(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<(), MatrixAdapterError> {
        let reader = self
            .client
            .get_reader_config(host, port, username, password)
            .await
            .map_err(map_adapter_error)?;
        if !face_only_door(&reader.body) {
            return Ok(());
        }
        tracing::info!("door access mode is face only; setting card and face for card enrollment");
        self.client
            .set_door_access_mode(host, port, username, password, "16")
            .await
            .map(|_| ())
            .map_err(map_adapter_error)
    }

    /// Start physical enrollment for a Matrix user that already exists on the device.
    pub async fn enroll_user(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        matrix_user_id: &str,
        enroll_type: HardwareEnrollType,
    ) -> Result<(), MatrixAdapterError> {
        let user_id = validate_user_id(matrix_user_id)?;
        let type_code = enroll_type.matrix_type().to_string();
        self.client
            .enroll_user(
                host,
                port,
                username,
                password,
                user_id.as_str(),
                type_code.as_str(),
                enroll_type.count_fields(),
            )
            .await
            .map(|_| ())
            .map_err(map_adapter_error)
    }

    /// `command?action=getcount` for the Matrix user id used by `enrolluser`.
    pub async fn credential_counts(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        matrix_user_id: &str,
    ) -> Result<CredentialCounts, MatrixAdapterError> {
        let user_id = validate_user_id(matrix_user_id)?;
        let primary = self
            .client
            .get_credential_counts(host, port, username, password, user_id.as_str())
            .await;
        if let Ok(body) = &primary {
            let counts = credential_counts(&body.body);
            if counts_present(counts) {
                return Ok(counts);
            }
        }
        if let Ok(user) = self
            .client
            .get_user(host, port, username, password, user_id.as_str())
            .await
        {
            if let Some(index) = config_field(&user.body, "user-index") {
                let index = index.trim();
                if !index.is_empty() && index != user_id.as_str() {
                    if let Ok(body) = self
                        .client
                        .get_credential_counts(host, port, username, password, index)
                        .await
                    {
                        let counts = credential_counts(&body.body);
                        if counts_present(counts) {
                            return Ok(counts);
                        }
                    }
                }
            }
        }
        primary
            .map(|body| credential_counts(&body.body))
            .map_err(map_adapter_error)
    }
}

/// Guide: 10 is no record found, 13 is user id not found.
pub fn device_user_already_gone(code: i32) -> bool {
    code == 10 || code == 13
}

fn counts_present(counts: CredentialCounts) -> bool {
    counts.cards > 0 || counts.faces > 0 || counts.fingers > 0
}

/// Guide: alphanumeric user-id, max 15 characters.
fn validate_user_id(raw: &str) -> Result<String, MatrixAdapterError> {
    let value = raw.trim();
    if value.is_empty() || value.chars().count() > 15 {
        return Err(MatrixAdapterError::InvalidArgument);
    }
    if !is_matrix_alphanumeric(value) || contains_forbidden_cgi_chars(value) {
        return Err(MatrixAdapterError::InvalidArgument);
    }
    Ok(value.to_string())
}

/// Guide: numeric ref-user-id, max 8 digits.
fn validate_ref_user_id(value: u32) -> Result<u32, MatrixAdapterError> {
    if value > 99_999_999 {
        return Err(MatrixAdapterError::InvalidArgument);
    }
    Ok(value)
}

/// Guide: alphanumeric name, max 15 characters.
fn validate_user_name(raw: &str) -> Result<String, MatrixAdapterError> {
    let value = raw.trim();
    if value.is_empty() || value.chars().count() > 15 {
        return Err(MatrixAdapterError::InvalidArgument);
    }
    if !is_matrix_alphanumeric(value) || contains_forbidden_cgi_chars(value) {
        return Err(MatrixAdapterError::InvalidArgument);
    }
    Ok(value.to_string())
}

/// Guide: user-pin is 1–15 digits, or blank to clear.
fn validate_user_pin(raw: &str) -> Result<String, MatrixAdapterError> {
    if raw.is_empty() {
        return Ok(String::new());
    }
    if !(1..=15).contains(&raw.len()) {
        return Err(MatrixAdapterError::InvalidArgument);
    }
    if !raw.chars().all(|c| c.is_ascii_digit()) || contains_forbidden_cgi_chars(raw) {
        return Err(MatrixAdapterError::InvalidArgument);
    }
    Ok(raw.to_string())
}

fn is_matrix_alphanumeric(value: &str) -> bool {
    value.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Guide: `& ' " < > # % ;` are not allowed inside argument values.
fn contains_forbidden_cgi_chars(value: &str) -> bool {
    value
        .chars()
        .any(|c| matches!(c, '&' | '\'' | '"' | '<' | '>' | '#' | '%' | ';'))
}

fn map_probe_error(error: MatrixClientError) -> MatrixProbeError {
    match error {
        MatrixClientError::Timeout => MatrixProbeError::Timeout,
        MatrixClientError::Unreachable => MatrixProbeError::Unreachable,
        MatrixClientError::AuthFailed => MatrixProbeError::AuthFailed,
        MatrixClientError::BadResponse | MatrixClientError::ApiError { .. } => {
            MatrixProbeError::BadResponse
        }
        MatrixClientError::InvalidTarget => MatrixProbeError::InvalidTarget,
    }
}

fn map_adapter_error(error: MatrixClientError) -> MatrixAdapterError {
    match error {
        MatrixClientError::Timeout => MatrixAdapterError::Timeout,
        MatrixClientError::Unreachable => MatrixAdapterError::Unreachable,
        MatrixClientError::AuthFailed => MatrixAdapterError::AuthFailed,
        MatrixClientError::BadResponse => MatrixAdapterError::BadResponse,
        MatrixClientError::InvalidTarget => MatrixAdapterError::InvalidTarget,
        MatrixClientError::ApiError { code } => MatrixAdapterError::ApiError { code },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        device_user_already_gone, validate_ref_user_id, validate_user_id, validate_user_name,
        validate_user_pin, MatrixAdapter, MatrixAdapterError, SetPinParams, SetUserParams,
    };
    use wiremock::matchers::{basic_auth, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn rejects_invalid_user_identity_fields() {
        assert!(validate_user_id("").is_err());
        assert!(validate_user_id("this-is-way-too-long").is_err());
        assert!(validate_user_id("bad id").is_err());
        assert!(validate_user_id("VS000001").is_ok());
        assert!(validate_ref_user_id(100_000_000).is_err());
        assert!(validate_ref_user_id(10000001).is_ok());
        assert!(validate_user_name("Ravi Kumar!!").is_err());
        assert!(validate_user_name("Ravi").is_ok());
    }

    #[test]
    fn missing_user_on_device_is_already_gone() {
        assert!(device_user_already_gone(10));
        assert!(device_user_already_gone(13));
        assert!(!device_user_already_gone(0));
        assert!(!device_user_already_gone(16));
    }

    #[tokio::test]
    async fn delete_user_sends_documented_users_delete() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/users"))
            .and(query_param("action", "delete"))
            .and(query_param("user-id", "VS000001"))
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

        let adapter = MatrixAdapter::new().unwrap();
        adapter
            .delete_user(
                "127.0.0.1",
                server.address().port(),
                "admin",
                "secret",
                "VS000001",
            )
            .await
            .unwrap();
    }

    #[test]
    fn pin_allows_blank_clear_and_digit_bounds() {
        assert_eq!(validate_user_pin("").unwrap(), "");
        assert!(validate_user_pin("1234").is_ok());
        assert!(validate_user_pin("123456789012345").is_ok());
        assert!(validate_user_pin("1234567890123456").is_err());
        assert!(validate_user_pin("12ab").is_err());
    }

    #[tokio::test]
    async fn set_user_sends_documented_users_set() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/users"))
            .and(query_param("action", "set"))
            .and(query_param("user-id", "VS000001"))
            .and(query_param("ref-user-id", "10000001"))
            .and(query_param("name", "Ravi"))
            .and(query_param("user-active", "1"))
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

        let adapter = MatrixAdapter::new().unwrap();
        adapter
            .set_user(
                "127.0.0.1",
                server.address().port(),
                "admin",
                "secret",
                &SetUserParams {
                    user_id: "VS000001".into(),
                    ref_user_id: 10000001,
                    name: Some("Ravi".into()),
                    user_active: Some(true),
                    enable_fr: None,
                },
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn set_user_nonzero_response_code_is_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/users"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Response-Code=21"))
            .mount(&server)
            .await;

        let adapter = MatrixAdapter::new().unwrap();
        let err = adapter
            .set_user(
                "127.0.0.1",
                server.address().port(),
                "admin",
                "secret",
                &SetUserParams {
                    user_id: "VS000001".into(),
                    ref_user_id: 10000001,
                    name: None,
                    user_active: None,
                    enable_fr: None,
                },
            )
            .await
            .unwrap_err();
        assert_eq!(err, MatrixAdapterError::ApiError { code: 21 });
    }

    #[tokio::test]
    async fn set_pin_sends_user_pin_without_ref_user_id() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/users"))
            .and(query_param("action", "set"))
            .and(query_param("user-id", "VS000001"))
            .and(query_param("user-pin", "4321"))
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

        let adapter = MatrixAdapter::new().unwrap();
        adapter
            .set_pin(
                "127.0.0.1",
                server.address().port(),
                "admin",
                "secret",
                &SetPinParams {
                    user_id: "VS000001".into(),
                    pin: "4321".into(),
                },
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn set_pin_nonzero_response_code_is_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/device.cgi/users"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Response-Code=33"))
            .mount(&server)
            .await;

        let adapter = MatrixAdapter::new().unwrap();
        let err = adapter
            .set_pin(
                "127.0.0.1",
                server.address().port(),
                "admin",
                "secret",
                &SetPinParams {
                    user_id: "VS000001".into(),
                    pin: "1234".into(),
                },
            )
            .await
            .unwrap_err();
        assert_eq!(err, MatrixAdapterError::ApiError { code: 33 });
    }

    #[tokio::test]
    async fn set_pin_rejects_invalid_pin_before_http() {
        let adapter = MatrixAdapter::new().unwrap();
        let err = adapter
            .set_pin(
                "127.0.0.1",
                9,
                "admin",
                "secret",
                &SetPinParams {
                    user_id: "VS000001".into(),
                    pin: "12ab".into(),
                },
            )
            .await
            .unwrap_err();
        assert_eq!(err, MatrixAdapterError::InvalidArgument);
    }
}
