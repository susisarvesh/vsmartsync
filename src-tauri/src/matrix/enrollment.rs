//! Documented enrollment capability checks.
//!
//! Field names and type codes come from the COSEC Devices API guide:
//! `device-basic-config` (`max-fingers`, `max-faces`, `name`),
//! `reader-config` (`reader1`, `reader3`), `enroll-options` (`enroll-card-count`),
//! and `enrolluser` `type`. A missing field means that capacity was not reported.
//! `enroll-on-device` is special-function enrollment and is not used here;
//! the guide says `enrolluser` is not tied to that setting.

/// `enrolluser` type codes from the guide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareEnrollType {
    ReadOnlyCard,
    SmartCard,
    Biometric,
    BiometricThenCard,
    Face,
    DuressFinger,
}

impl HardwareEnrollType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnlyCard => "read_only_card",
            Self::SmartCard => "smart_card",
            Self::Biometric => "biometric",
            Self::BiometricThenCard => "biometric_then_card",
            Self::Face => "face",
            Self::DuressFinger => "duress_finger",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ReadOnlyCard => "Read-only card",
            Self::SmartCard => "Smart card",
            Self::Biometric => "Fingerprint",
            Self::BiometricThenCard => "Fingerprint then card",
            Self::Face => "Face",
            Self::DuressFinger => "Duress finger",
        }
    }

    /// Local credential type stored after the device accepts enrollment.
    /// Fingerprint enrollment is stored as `finger`. No template is stored.
    pub fn credential_type(self) -> &'static str {
        match self {
            Self::ReadOnlyCard => "read_only_card",
            Self::SmartCard => "smart_card",
            Self::Biometric => "finger",
            Self::BiometricThenCard => "biometric_card",
            Self::Face => "face",
            Self::DuressFinger => "duress_finger",
        }
    }

    pub fn matrix_type(self) -> u8 {
        match self {
            Self::ReadOnlyCard => 0,
            Self::SmartCard => 1,
            Self::Biometric => 2,
            Self::BiometricThenCard => 3,
            Self::Face => 7,
            Self::DuressFinger => 8,
        }
    }

    /// Guide count arguments. `0` means one credential. Omitted counts are
    /// defined as one, but this unit only opens card or face capture when the
    /// matching count is sent.
    pub fn count_fields(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::ReadOnlyCard | Self::SmartCard => &[("card-count", "0")],
            Self::Biometric | Self::DuressFinger => &[("finger-count", "0")],
            Self::BiometricThenCard => &[("finger-count", "0"), ("card-count", "0")],
            Self::Face => &[("face-count", "0")],
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "read_only_card" => Some(Self::ReadOnlyCard),
            "smart_card" => Some(Self::SmartCard),
            "biometric" => Some(Self::Biometric),
            "biometric_then_card" => Some(Self::BiometricThenCard),
            "face" => Some(Self::Face),
            "duress_finger" => Some(Self::DuressFinger),
            _ => None,
        }
    }
}

/// Types the device configuration reports as available, in display order.
pub fn supported_enroll_types(
    basic_config: &str,
    reader_config: &str,
    enroll_options: &str,
) -> Vec<HardwareEnrollType> {
    let face200t = config_field(basic_config, "name")
        .is_some_and(|name| name.to_ascii_lowercase().contains("face200t"));
    let finger = config_field(basic_config, "max-fingers").is_some();
    let face_capacity = config_field(basic_config, "max-faces").is_some();
    let access_mode =
        config_field(reader_config, "door-access-mode").and_then(|raw| raw.parse::<i32>().ok());
    let face = face_capacity || matches!(access_mode, Some(6 | 15 | 16 | 17 | 18));
    let (mut read_only_card, smart_card) = if face200t {
        (false, false)
    } else {
        card_support(reader_config, enroll_options)
    };
    if !face200t
        && (face_capacity || matches!(access_mode, Some(0 | 2 | 4 | 5 | 6 | 9 | 10 | 12 | 16 | 20)))
    {
        read_only_card = true;
    }

    let mut types = Vec::new();
    if face {
        types.push(HardwareEnrollType::Face);
    }
    if finger {
        types.push(HardwareEnrollType::Biometric);
        types.push(HardwareEnrollType::DuressFinger);
    }
    if read_only_card {
        types.push(HardwareEnrollType::ReadOnlyCard);
    }
    if smart_card {
        types.push(HardwareEnrollType::SmartCard);
    }
    if finger && (read_only_card || smart_card) {
        types.push(HardwareEnrollType::BiometricThenCard);
    }
    types
}

fn card_support(reader_config: &str, enroll_options: &str) -> (bool, bool) {
    let mut read_only = config_field(enroll_options, "enroll-card-count").is_some();
    let mut smart = false;
    if let Some(value) =
        config_field(reader_config, "reader1").and_then(|raw| raw.parse::<i32>().ok())
    {
        match value {
            1 | 2 => read_only = true,
            3 | 4 | 5 => smart = true,
            _ => {}
        }
    }
    if let Some(value) =
        config_field(reader_config, "reader3").and_then(|raw| raw.parse::<i32>().ok())
    {
        match value {
            1 | 2 | 8 | 12 | 15 | 16 | 17 => read_only = true,
            3 | 4 | 6 | 10 | 13 | 14 => smart = true,
            _ => {}
        }
    }
    (read_only, smart)
}

/// Enrolled counts from `command?action=getcount`.
///
/// The guide encodes a present value as count-minus-one (`0` means one).
/// A missing field means that credential is not on the user.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CredentialCounts {
    pub cards: u32,
    pub faces: u32,
    pub fingers: u32,
}

pub fn credential_counts(body: &str) -> CredentialCounts {
    CredentialCounts {
        cards: decode_enrolled_count(config_field(body, "card-count")),
        faces: decode_enrolled_count(config_field(body, "face-count")),
        fingers: decode_enrolled_count(config_field(body, "finger-count")),
    }
}

/// Guide door-access-mode 15 is Face only, so a card prompt does not start.
pub fn face_only_door(reader_config: &str) -> bool {
    config_field(reader_config, "door-access-mode").is_some_and(|value| value.trim() == "15")
}

/// True when `getcount` already shows this credential on the user.
pub fn credential_present(enroll_type: HardwareEnrollType, counts: CredentialCounts) -> bool {
    match enroll_type {
        HardwareEnrollType::Face => counts.faces > 0,
        HardwareEnrollType::ReadOnlyCard | HardwareEnrollType::SmartCard => counts.cards > 0,
        HardwareEnrollType::Biometric | HardwareEnrollType::DuressFinger => counts.fingers > 0,
        HardwareEnrollType::BiometricThenCard => counts.fingers > 0 || counts.cards > 0,
    }
}

pub fn capture_increased(
    enroll_type: HardwareEnrollType,
    before: CredentialCounts,
    after: CredentialCounts,
) -> bool {
    match enroll_type {
        HardwareEnrollType::Face => after.faces > before.faces,
        HardwareEnrollType::ReadOnlyCard | HardwareEnrollType::SmartCard => {
            after.cards > before.cards
        }
        HardwareEnrollType::Biometric | HardwareEnrollType::DuressFinger => {
            after.fingers > before.fingers
        }
        HardwareEnrollType::BiometricThenCard => {
            after.fingers > before.fingers || after.cards > before.cards
        }
    }
}

fn decode_enrolled_count(raw: Option<String>) -> u32 {
    let Some(raw) = raw else {
        return 0;
    };
    raw.trim()
        .parse::<u32>()
        .map(|value| value.saturating_add(1))
        .unwrap_or(1)
}

pub fn config_field(body: &str, key: &str) -> Option<String> {
    xml_field(body, key).or_else(|| text_field(body, key))
}

fn xml_field(body: &str, key: &str) -> Option<String> {
    let lower = body.to_ascii_lowercase();
    let start_tag = format!("<{key}>");
    let end_tag = format!("</{key}>");
    let start = lower.find(&start_tag)?;
    let after = start + start_tag.len();
    let end_rel = lower[after..].find(&end_tag)?;
    Some(body[after..after + end_rel].trim().to_string())
}

fn text_field(body: &str, key: &str) -> Option<String> {
    let lower = body.to_ascii_lowercase();
    let needle = format!("{key}=");
    let mut search = 0;
    while let Some(rel) = lower[search..].find(&needle) {
        let start = search + rel;
        let boundary = start == 0
            || lower.as_bytes()[start - 1].is_ascii_whitespace()
            || lower.as_bytes()[start - 1] == b'>';
        if boundary {
            let value_start = start + needle.len();
            return Some(take_assignment_value(&body[value_start..]).to_string());
        }
        search = start + needle.len();
    }
    None
}

fn take_assignment_value(rest: &str) -> &str {
    let mut index = 0;
    for (offset, ch) in rest.char_indices() {
        if ch.is_whitespace() {
            let after = rest[offset..].trim_start();
            if looks_like_assignment(after) {
                return rest[..index].trim();
            }
        }
        index = offset + ch.len_utf8();
    }
    rest.trim()
}

fn looks_like_assignment(value: &str) -> bool {
    let Some((key, _)) = value.split_once('=') else {
        return false;
    };
    let key = key.trim();
    !key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}

#[cfg(test)]
mod tests {
    use super::{supported_enroll_types, HardwareEnrollType};

    #[test]
    fn ngt_sample_reports_finger_and_not_face() {
        let basic = "app=1 name=NGT Direct Door-Device-11 asc-code=0 generate-invalid-user-events=0 generate-exit-switch-events=0 max-fingers=1 finger-format=0";
        let reader = "reader1=2 reader2=1";
        let options = "enroll-mode=2 temp-per-finger=0";
        let types = supported_enroll_types(basic, reader, options);
        assert!(types.contains(&HardwareEnrollType::Biometric));
        assert!(types.contains(&HardwareEnrollType::DuressFinger));
        assert!(types.contains(&HardwareEnrollType::ReadOnlyCard));
        assert!(!types.contains(&HardwareEnrollType::Face));
        assert!(!types.contains(&HardwareEnrollType::SmartCard));
    }

    #[test]
    fn xml_max_faces_enables_face_without_treating_zero_as_absent() {
        let basic = "<COSEC_API><name>Argo Face</name><max-faces>0</max-faces><max-fingers>1</max-fingers></COSEC_API>";
        let types = supported_enroll_types(basic, "", "enroll-card-count=0");
        assert!(types.contains(&HardwareEnrollType::Face));
        assert!(types.contains(&HardwareEnrollType::Biometric));
        assert!(types.contains(&HardwareEnrollType::ReadOnlyCard));
        assert!(types.contains(&HardwareEnrollType::BiometricThenCard));
    }

    #[test]
    fn face200t_does_not_offer_card_enrollment() {
        let basic = "name=ARGO FACE200T max-faces=2 max-fingers=0";
        let reader = "reader1=3 reader3=1";
        let options = "enroll-card-count=1";
        let types = supported_enroll_types(basic, reader, options);
        assert!(types.contains(&HardwareEnrollType::Face));
        assert!(types.contains(&HardwareEnrollType::Biometric));
        assert!(!types.contains(&HardwareEnrollType::ReadOnlyCard));
        assert!(!types.contains(&HardwareEnrollType::SmartCard));
        assert!(!types.contains(&HardwareEnrollType::BiometricThenCard));
    }

    #[test]
    fn mifare_reader_is_smart_card() {
        let types = supported_enroll_types("max-fingers=0", "reader1=3", "");
        assert!(types.contains(&HardwareEnrollType::SmartCard));
        assert!(!types.contains(&HardwareEnrollType::ReadOnlyCard));
        assert!(types.contains(&HardwareEnrollType::BiometricThenCard));
    }

    #[test]
    fn door_access_mode_card_and_face_offers_both() {
        let types = supported_enroll_types("name=Lobby", "door-access-mode=16", "");
        assert!(types.contains(&HardwareEnrollType::Face));
        assert!(types.contains(&HardwareEnrollType::ReadOnlyCard));
        assert!(!types.contains(&HardwareEnrollType::Biometric));
    }

    #[test]
    fn face_capacity_offers_card_on_non_face200t() {
        let types = supported_enroll_types("<max-faces>0</max-faces>", "", "");
        assert!(types.contains(&HardwareEnrollType::Face));
        assert!(types.contains(&HardwareEnrollType::ReadOnlyCard));
    }

    #[test]
    fn face_only_door_is_access_mode_15() {
        assert!(super::face_only_door("door-access-mode=15"));
        assert!(super::face_only_door(
            "<door-access-mode>15</door-access-mode>"
        ));
        assert!(!super::face_only_door("door-access-mode=16"));
    }

    #[test]
    fn capture_count_zero_means_one_enrolled() {
        let before = super::credential_counts("Response-Code=0");
        let after = super::credential_counts("Response-Code=0 face-count=0 card-count=0");
        assert!(super::capture_increased(
            HardwareEnrollType::Face,
            before,
            after
        ));
        assert!(super::capture_increased(
            HardwareEnrollType::ReadOnlyCard,
            before,
            after
        ));
        assert!(!super::capture_increased(
            HardwareEnrollType::Biometric,
            before,
            after
        ));
    }
}
