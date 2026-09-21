//! Documented Matrix API `Response-Code` values from COSEC Devices API User Guide v28.
//!
//! Catalog for the HTTP client and later adapter mapping. Unknown codes are still
//! returned as `MatrixClientError::ApiError { code }` — do not invent meanings.

/// Matrix operation succeeded (`Response-Code=0`).
pub const SUCCESS: i32 = 0;

/// Reference User ID already exists on the device.
pub const REFERENCE_USER_ID_EXISTS: i32 = 21;

/// Wrong selection.
pub const WRONG_SELECTION: i32 = 22;

/// Palm template mode mismatch.
pub const PALM_TEMPLATE_MODE_MISMATCH: i32 = 23;

/// Feature not enabled on this device/configuration.
pub const FEATURE_NOT_ENABLED: i32 = 24;

/// Invalid value in the request.
pub const INVALID_VALUE: i32 = 33;

/// Credential does not match.
pub const CREDENTIAL_DOES_NOT_MATCH: i32 = 34;

/// Generic failure.
pub const FAILURE: i32 = 35;

/// Face not detected.
pub const FACE_NOT_DETECTED: i32 = 36;

/// User conflict.
pub const USER_CONFLICT: i32 = 37;

/// Enroll conflict.
pub const ENROLL_CONFLICT: i32 = 38;

/// Stable snake_case label for known guide codes; `None` if not in this catalog.
pub fn documented_label(code: i32) -> Option<&'static str> {
    match code {
        SUCCESS => Some("success"),
        REFERENCE_USER_ID_EXISTS => Some("reference_user_id_exists"),
        WRONG_SELECTION => Some("wrong_selection"),
        PALM_TEMPLATE_MODE_MISMATCH => Some("palm_template_mode_mismatch"),
        FEATURE_NOT_ENABLED => Some("feature_not_enabled"),
        INVALID_VALUE => Some("invalid_value"),
        CREDENTIAL_DOES_NOT_MATCH => Some("credential_does_not_match"),
        FAILURE => Some("failure"),
        FACE_NOT_DETECTED => Some("face_not_detected"),
        USER_CONFLICT => Some("user_conflict"),
        ENROLL_CONFLICT => Some("enroll_conflict"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_matches_guide_values() {
        assert_eq!(SUCCESS, 0);
        assert_eq!(REFERENCE_USER_ID_EXISTS, 21);
        assert_eq!(WRONG_SELECTION, 22);
        assert_eq!(PALM_TEMPLATE_MODE_MISMATCH, 23);
        assert_eq!(FEATURE_NOT_ENABLED, 24);
        assert_eq!(INVALID_VALUE, 33);
        assert_eq!(CREDENTIAL_DOES_NOT_MATCH, 34);
        assert_eq!(FAILURE, 35);
        assert_eq!(FACE_NOT_DETECTED, 36);
        assert_eq!(USER_CONFLICT, 37);
        assert_eq!(ENROLL_CONFLICT, 38);
    }

    #[test]
    fn labels_cover_catalog_and_unknown() {
        assert_eq!(documented_label(SUCCESS), Some("success"));
        assert_eq!(
            documented_label(REFERENCE_USER_ID_EXISTS),
            Some("reference_user_id_exists")
        );
        assert_eq!(documented_label(999), None);
    }
}
