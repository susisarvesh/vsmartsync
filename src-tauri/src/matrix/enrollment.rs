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

    /// Local credential rows that can be assigned when this enroll type is available.
    ///
    /// An operator-entered `card` is included for card readers. PIN is not an
    /// `enrolluser` type and is not included.
    pub fn assignable_credential_types(self) -> &'static [&'static str] {
        match self {
            Self::ReadOnlyCard => &["read_only_card", "card"],
            Self::SmartCard => &["smart_card", "card"],
            Self::Biometric => &["finger"],
            Self::BiometricThenCard => &["biometric_card", "card"],
            Self::Face => &["face"],
            Self::DuressFinger => &["duress_finger"],
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

    /// Extra `enrolluser` arguments for this type.
    ///
    /// The guide encodes a count of one as `0`. A card enrollment sends
    /// `card-count=0` so the EM or smart-card reader asks for one card.
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
            3..=5 => smart = true,
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

/// Guide card-type values. Numeric codes stay in this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixCardType {
    IClass2K2,
    IClass16K2,
    IClass16K16,
    Mifare1K,
    Mifare4K,
    MifareDesfire2K,
    MifareDesfire4K,
    MifareDesfire8K,
    ReadOnly,
}

impl MatrixCardType {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "iclass_2k2" => Some(Self::IClass2K2),
            "iclass_16k2" => Some(Self::IClass16K2),
            "iclass_16k16" => Some(Self::IClass16K16),
            "mifare_1k" => Some(Self::Mifare1K),
            "mifare_4k" => Some(Self::Mifare4K),
            "mifare_desfire_2k" => Some(Self::MifareDesfire2K),
            "mifare_desfire_4k" => Some(Self::MifareDesfire4K),
            "mifare_desfire_8k" => Some(Self::MifareDesfire8K),
            "read_only" => Some(Self::ReadOnly),
            _ => None,
        }
    }

    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            1 => Some(Self::IClass2K2),
            2 => Some(Self::IClass16K2),
            3 => Some(Self::IClass16K16),
            4 => Some(Self::Mifare1K),
            5 => Some(Self::Mifare4K),
            6 => Some(Self::MifareDesfire2K),
            7 => Some(Self::MifareDesfire4K),
            8 => Some(Self::MifareDesfire8K),
            9 => Some(Self::ReadOnly),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::IClass2K2 => "iclass_2k2",
            Self::IClass16K2 => "iclass_16k2",
            Self::IClass16K16 => "iclass_16k16",
            Self::Mifare1K => "mifare_1k",
            Self::Mifare4K => "mifare_4k",
            Self::MifareDesfire2K => "mifare_desfire_2k",
            Self::MifareDesfire4K => "mifare_desfire_4k",
            Self::MifareDesfire8K => "mifare_desfire_8k",
            Self::ReadOnly => "read_only",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::IClass2K2 => "HID iCLASS 2K2",
            Self::IClass16K2 => "HID iCLASS 16K2",
            Self::IClass16K16 => "HID iCLASS 16K16",
            Self::Mifare1K => "MIFARE 1K",
            Self::Mifare4K => "MIFARE 4K",
            Self::MifareDesfire2K => "MIFARE DESFire 2K",
            Self::MifareDesfire4K => "MIFARE DESFire 4K",
            Self::MifareDesfire8K => "MIFARE DESFire 8K",
            Self::ReadOnly => "Read-only card",
        }
    }
}

/// How the device identified the card. Absent when the body does not say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixIdentifierType {
    Csn,
    Uid,
    Custom,
}

impl MatrixIdentifierType {
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Csn),
            1 => Some(Self::Uid),
            2 => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "0" | "csn" => Some(Self::Csn),
            "1" | "uid" => Some(Self::Uid),
            "2" | "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Csn => "csn",
            Self::Uid => "uid",
            Self::Custom => "custom",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Csn => "CSN",
            Self::Uid => "UID",
            Self::Custom => "Custom",
        }
    }
}

/// Card fields from `credential?action=get` when the body actually contains them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedCardCredential {
    pub card_number: Option<String>,
    /// `card1` and `card2` values that are present. `0` means that slot is empty.
    pub card_numbers: Vec<String>,
    pub card_type: Option<MatrixCardType>,
    pub identifier_type: Option<MatrixIdentifierType>,
}

/// Direct `card-read-write?action=read` result. This is not an enrollment.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedCardRead {
    pub card_number: Option<String>,
    pub card_type: Option<MatrixCardType>,
}

pub fn parse_card_credential(body: &str) -> ParsedCardCredential {
    let card_numbers = ["card1", "card2"]
        .into_iter()
        .filter_map(|key| config_field(body, key).filter(|value| !value.is_empty() && value != "0"))
        .collect::<Vec<_>>();
    let card_type = config_field(body, "card-type")
        .and_then(|raw| raw.parse::<i32>().ok())
        .and_then(MatrixCardType::from_code);
    let identifier_type =
        config_field(body, "identifier-type").and_then(|raw| MatrixIdentifierType::parse(&raw));
    ParsedCardCredential {
        card_number: card_numbers.first().cloned(),
        card_numbers,
        card_type,
        identifier_type,
    }
}

pub fn parse_card_read(body: &str) -> ParsedCardRead {
    ParsedCardRead {
        card_number: config_field(body, "card-no").filter(|value| !value.is_empty()),
        card_type: config_field(body, "card-type")
            .and_then(|raw| raw.parse::<i32>().ok())
            .and_then(MatrixCardType::from_code),
    }
}

/// Documented card-reader technology. Other reader codes stay unlabeled here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardReaderFamily {
    EmProx,
    HidProx,
    Mifare,
    HidIclass,
}

impl CardReaderFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmProx => "EM Prox",
            Self::HidProx => "HID Prox",
            Self::Mifare => "MIFARE",
            Self::HidIclass => "HID iCLASS",
        }
    }

    /// EM Prox and HID Prox are 125 kHz readers. `card-read-write` is the
    /// smart-card read and does not see a card on these antennas.
    pub fn is_proximity(self) -> bool {
        matches!(self, Self::EmProx | Self::HidProx)
    }

    /// Card types the guide associates with this reader. DESFire types are the
    /// documented MIFARE card-type codes. Read-only is EM Prox and HID Prox.
    pub fn accepts(self, card: MatrixCardType) -> bool {
        match self {
            Self::EmProx | Self::HidProx => card == MatrixCardType::ReadOnly,
            Self::Mifare => matches!(
                card,
                MatrixCardType::Mifare1K
                    | MatrixCardType::Mifare4K
                    | MatrixCardType::MifareDesfire2K
                    | MatrixCardType::MifareDesfire4K
                    | MatrixCardType::MifareDesfire8K
            ),
            Self::HidIclass => matches!(
                card,
                MatrixCardType::IClass2K2
                    | MatrixCardType::IClass16K2
                    | MatrixCardType::IClass16K16
            ),
        }
    }
}

/// First configured reader from `reader-config`. Reader 1 is preferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfiguredReader {
    pub code: i32,
    pub label: &'static str,
    pub family: Option<CardReaderFamily>,
}

pub fn configured_reader(reader_config: &str) -> Option<ConfiguredReader> {
    if let Some(code) = config_field(reader_config, "reader1").and_then(|raw| raw.parse().ok()) {
        if let Some(label) = reader1_label(code) {
            return Some(ConfiguredReader {
                code,
                label,
                family: reader1_family(code),
            });
        }
    }
    if let Some(code) = config_field(reader_config, "reader3").and_then(|raw| raw.parse().ok()) {
        if let Some(label) = reader3_label(code) {
            return Some(ConfiguredReader {
                code,
                label,
                family: reader3_family(code),
            });
        }
    }
    None
}

fn reader1_family(value: i32) -> Option<CardReaderFamily> {
    match value {
        1 => Some(CardReaderFamily::EmProx),
        2 => Some(CardReaderFamily::HidProx),
        3 => Some(CardReaderFamily::Mifare),
        4 | 5 => Some(CardReaderFamily::HidIclass),
        _ => None,
    }
}

fn reader3_family(value: i32) -> Option<CardReaderFamily> {
    match value {
        1 => Some(CardReaderFamily::EmProx),
        2 => Some(CardReaderFamily::HidProx),
        3 | 10 => Some(CardReaderFamily::Mifare),
        4 | 6 => Some(CardReaderFamily::HidIclass),
        _ => None,
    }
}

/// `smart-card-format?action=get`. `card-no` here is the identifier mode, not a card number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SmartCardFormat {
    pub card_type: Option<MatrixCardType>,
    pub identifier: Option<MatrixIdentifierType>,
}

pub fn parse_smart_card_format(body: &str) -> SmartCardFormat {
    SmartCardFormat {
        card_type: config_field(body, "card-type")
            .and_then(|raw| raw.parse::<i32>().ok())
            .and_then(MatrixCardType::from_code),
        identifier: config_field(body, "card-no").and_then(|raw| MatrixIdentifierType::parse(&raw)),
    }
}

/// Enable flags only. Key material is never copied out of the device body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CardKeyFlags {
    pub mifare_custom_key_enabled: bool,
    pub hid_iclass_custom_key_enabled: bool,
    pub card_custom_key_auto_update: bool,
}

pub fn parse_card_key_flags(body: &str) -> CardKeyFlags {
    CardKeyFlags {
        mifare_custom_key_enabled: flag_enabled(body, "mifare-custom-key-enable"),
        hid_iclass_custom_key_enabled: flag_enabled(body, "hid-iclass-custom-key-enable"),
        card_custom_key_auto_update: flag_enabled(body, "card-custom-key-auto-update"),
    }
}

fn flag_enabled(body: &str, key: &str) -> bool {
    config_field(body, key).is_some_and(|value| value.trim() == "1")
}

/// What a card-read attempt concluded. Numeric Matrix codes stay inside this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardReadAttempt {
    Read(ParsedCardRead),
    DeviceCode(i32),
    TimedOut,
    Unreachable,
    AuthFailed,
    BadResponse,
    /// Reader configuration already decided the test. The reader was not opened.
    NotAttempted,
    /// EM Prox or HID Prox. The smart-card read was not sent.
    ProximityReader,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardTestReport {
    pub outcome: &'static str,
    pub message: String,
    pub compatible: bool,
    pub reader: Option<String>,
    pub reader_label: Option<String>,
    pub reader_code: Option<i32>,
    pub card_type: Option<String>,
    pub card_type_label: Option<String>,
    pub card_number: Option<String>,
}

pub fn diagnose_card_read(reader_config: &str, attempt: CardReadAttempt) -> CardTestReport {
    let reader = configured_reader(reader_config);
    let reader_name = reader.map(|value| value.label.to_string());
    let family_name =
        reader.and_then(|value| value.family.map(|family| family.as_str().to_string()));
    let code = reader.map(|value| value.code);
    let base = |outcome: &'static str, message: String, compatible: bool| CardTestReport {
        outcome,
        message,
        compatible,
        reader: family_name.clone(),
        reader_label: reader_name.clone(),
        reader_code: code,
        card_type: None,
        card_type_label: None,
        card_number: None,
    };
    let Some(reader) = reader else {
        return base(
            "reader_not_configured",
            "No card reader is configured on this device.".to_string(),
            false,
        );
    };
    if reader.family.is_none() {
        return base(
            "reader_unsupported",
            "This reader is configured, but card compatibility is not documented for it."
                .to_string(),
            false,
        );
    }
    match attempt {
        CardReadAttempt::ProximityReader => base(
            "proximity",
            format!(
                "This is a {}. It reads a proximity card when enrollment starts. Click Enroll on device, then hold the card flat on the reader on the door.",
                reader.label
            ),
            true,
        ),
        CardReadAttempt::TimedOut => base(
            "timeout",
            "Card was not detected before the enrollment/read timeout.".to_string(),
            false,
        ),
        CardReadAttempt::Unreachable => base(
            "unreachable",
            "The Matrix device could not be reached.".to_string(),
            false,
        ),
        CardReadAttempt::AuthFailed => base(
            "auth_failed",
            "The device rejected the login.".to_string(),
            false,
        ),
        CardReadAttempt::BadResponse => base(
            "bad_response",
            "The device returned an unexpected response.".to_string(),
            false,
        ),
        CardReadAttempt::DeviceCode(16) => base(
            "device_busy",
            "The Matrix device is currently busy with another operation.".to_string(),
            false,
        ),
        CardReadAttempt::DeviceCode(26) => base(
            "parameters_not_applicable",
            "The card read parameters do not apply to this card type.".to_string(),
            false,
        ),
        CardReadAttempt::DeviceCode(27) => base(
            "timeout",
            "Card was not detected before the enrollment/read timeout.".to_string(),
            false,
        ),
        CardReadAttempt::DeviceCode(28) => base(
            "read_failed",
            "The device detected a card but could not read it. Check card placement, card technology, and card configuration.".to_string(),
            false,
        ),
        CardReadAttempt::DeviceCode(29) => base(
            "wrong_card_type",
            "Wrong card type. The card does not match the reader configured on this device.".to_string(),
            false,
        ),
        CardReadAttempt::DeviceCode(30) => base(
            "key_mismatch",
            "The card could not be read because its configured security key does not match the device configuration.".to_string(),
            false,
        ),
        CardReadAttempt::DeviceCode(_) | CardReadAttempt::NotAttempted => base(
            "bad_response",
            "The device returned an unexpected response.".to_string(),
            false,
        ),
        CardReadAttempt::Read(parsed) => {
            let card_label = parsed.card_type.map(|value| value.label().to_string());
            let card_type = parsed.card_type.map(|value| value.as_str().to_string());
            let Some(card) = parsed.card_type else {
                let mut report = base(
                    "read_failed",
                    "The device detected a card but did not report its card type.".to_string(),
                    false,
                );
                report.card_number = parsed.card_number;
                return report;
            };
            let family = reader.family.expect("checked above");
            if !family.accepts(card) {
                let mut report = base(
                    "incompatible",
                    format!(
                        "Incompatible card technology. The Matrix device is configured for a {} reader, but the card that was read is {}.",
                        family.as_str(),
                        card.label()
                    ),
                    false,
                );
                report.card_type = card_type;
                report.card_type_label = card_label;
                report.card_number = parsed.card_number;
                return report;
            }
            let mut report = base("success", "Card successfully detected.".to_string(), true);
            report.card_type = card_type;
            report.card_type_label = card_label;
            report.card_number = parsed.card_number;
            report
        }
    }
}

/// Human label for the first configured reader. `None` means no reader was reported.
pub fn reader_label(reader_config: &str) -> Option<String> {
    let reader1 = config_field(reader_config, "reader1")
        .and_then(|raw| raw.parse::<i32>().ok())
        .and_then(reader1_label);
    let reader3 = config_field(reader_config, "reader3")
        .and_then(|raw| raw.parse::<i32>().ok())
        .and_then(reader3_label);
    reader1.or(reader3).map(str::to_string)
}

fn reader1_label(value: i32) -> Option<&'static str> {
    match value {
        1 => Some("EM Prox Reader"),
        2 => Some("HID Prox Reader"),
        3 => Some("MiFare Reader"),
        4 => Some("HID iCLASS-U Reader"),
        5 => Some("HID iCLASS-W Reader"),
        _ => None,
    }
}

fn reader3_label(value: i32) -> Option<&'static str> {
    match value {
        1 => Some("EM Prox Reader"),
        2 => Some("HID Prox Reader"),
        3 => Some("MiFare U Reader"),
        4 => Some("HID iCLASS-U Reader"),
        5 => Some("Finger Reader"),
        6 => Some("HID iCLASS-W Reader"),
        8 => Some("UHF Reader"),
        9 => Some("Combo Exit Reader"),
        10 => Some("MiFare-W Reader"),
        11 => Some("PIN-W Reader"),
        12 => Some("Card and PIN-W Reader"),
        13 => Some("CB-U Reader"),
        14 => Some("CB-W Reader"),
        15 => Some("ATOM RD300"),
        16 => Some("ATOM RD200"),
        17 => Some("ATOM RD100"),
        _ => None,
    }
}

/// Types plus the reader name reported by `reader-config`.
pub struct ReportedEnrollment {
    pub types: Vec<HardwareEnrollType>,
    pub reader_label: Option<String>,
}

pub fn reported_enrollment(
    basic_config: &str,
    reader_config: &str,
    enroll_options: &str,
) -> ReportedEnrollment {
    ReportedEnrollment {
        types: supported_enroll_types(basic_config, reader_config, enroll_options),
        reader_label: reader_label(reader_config),
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
        assert!(super::credential_present(HardwareEnrollType::Face, after));
        assert!(!super::credential_present(HardwareEnrollType::Face, before));
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

    #[test]
    fn maps_card_reader_and_identifier_codes() {
        use super::{
            parse_card_credential, parse_card_read, reader_label, MatrixCardType,
            MatrixIdentifierType,
        };

        assert_eq!(MatrixCardType::from_code(4), Some(MatrixCardType::Mifare1K));
        assert_eq!(MatrixCardType::Mifare1K.as_str(), "mifare_1k");
        assert_eq!(MatrixCardType::Mifare1K.label(), "MIFARE 1K");
        assert_eq!(
            MatrixCardType::from_code(9).unwrap().label(),
            "Read-only card"
        );
        assert!(MatrixCardType::from_code(99).is_none());

        assert_eq!(
            MatrixIdentifierType::parse("uid"),
            Some(MatrixIdentifierType::Uid)
        );
        assert_eq!(
            MatrixIdentifierType::from_code(0),
            Some(MatrixIdentifierType::Csn)
        );
        assert_eq!(
            HardwareEnrollType::ReadOnlyCard.assignable_credential_types(),
            &["read_only_card", "card"]
        );
        assert_eq!(
            HardwareEnrollType::Face.assignable_credential_types(),
            &["face"]
        );
        assert_eq!(
            HardwareEnrollType::ReadOnlyCard.count_fields(),
            &[("card-count", "0")]
        );
        assert_eq!(
            HardwareEnrollType::SmartCard.count_fields(),
            &[("card-count", "0")]
        );
        assert_eq!(
            HardwareEnrollType::Face.count_fields(),
            &[("face-count", "0")]
        );
        assert_eq!(HardwareEnrollType::SmartCard.matrix_type(), 1);
        assert_eq!(HardwareEnrollType::ReadOnlyCard.matrix_type(), 0);
        assert_eq!(HardwareEnrollType::Face.matrix_type(), 7);
        assert_eq!(HardwareEnrollType::Biometric.matrix_type(), 2);

        assert_eq!(reader_label("reader1=3").as_deref(), Some("MiFare Reader"));
        assert_eq!(reader_label("reader3=1").as_deref(), Some("EM Prox Reader"));
        assert!(reader_label("reader1=0").is_none());

        let parsed =
            parse_card_credential("Response-Code=0 card1=12345678 card-type=4 identifier-type=csn");
        assert_eq!(parsed.card_number.as_deref(), Some("12345678"));
        assert_eq!(parsed.card_numbers, vec!["12345678".to_string()]);
        assert_eq!(parsed.card_type, Some(MatrixCardType::Mifare1K));
        assert_eq!(parsed.identifier_type, Some(MatrixIdentifierType::Csn));
        let empty_slot =
            parse_card_credential("<COSEC_API><card1>0</card1><card2>0</card2></COSEC_API>");
        assert!(empty_slot.card_number.is_none());
        assert!(empty_slot.card_numbers.is_empty());

        let read = parse_card_read("Response-Code=0 card-no=9988 card-type=9");
        assert_eq!(read.card_number.as_deref(), Some("9988"));
        assert_eq!(read.card_type, Some(MatrixCardType::ReadOnly));

        let types = supported_enroll_types("max-fingers=0", "reader1=0", "");
        assert!(!types.contains(&HardwareEnrollType::SmartCard));
        assert!(!types.contains(&HardwareEnrollType::ReadOnlyCard));
    }

    #[test]
    fn reader_family_and_card_compatibility() {
        use super::{
            diagnose_card_read, parse_card_key_flags, parse_smart_card_format, CardReadAttempt,
            CardReaderFamily, MatrixCardType, ParsedCardRead,
        };
        assert_eq!(
            super::configured_reader("reader1=3").unwrap().family,
            Some(CardReaderFamily::Mifare)
        );
        assert_eq!(
            super::configured_reader("reader1=0 reader3=4")
                .unwrap()
                .family,
            Some(CardReaderFamily::HidIclass)
        );
        assert!(super::configured_reader("reader1=0 reader3=0").is_none());
        assert!(CardReaderFamily::Mifare.accepts(MatrixCardType::Mifare1K));
        assert!(CardReaderFamily::Mifare.accepts(MatrixCardType::MifareDesfire4K));
        assert!(!CardReaderFamily::Mifare.accepts(MatrixCardType::IClass2K2));
        assert!(CardReaderFamily::HidIclass.accepts(MatrixCardType::IClass16K16));
        assert!(!CardReaderFamily::HidIclass.accepts(MatrixCardType::Mifare1K));
        assert!(CardReaderFamily::EmProx.accepts(MatrixCardType::ReadOnly));
        assert!(!CardReaderFamily::HidProx.accepts(MatrixCardType::Mifare1K));

        let format = parse_smart_card_format("card-type=4 card-no=0");
        assert_eq!(format.card_type, Some(MatrixCardType::Mifare1K));
        assert_eq!(format.identifier, Some(super::MatrixIdentifierType::Csn));
        let flags = parse_card_key_flags(
            "mifare-custom-key-enable=1 hid-iclass-custom-key-enable=0 card-custom-key-auto-update=1 mifare-custom-key=SECRET",
        );
        assert!(flags.mifare_custom_key_enabled);
        assert!(!flags.hid_iclass_custom_key_enabled);
        assert!(flags.card_custom_key_auto_update);

        let matched = diagnose_card_read(
            "reader1=3",
            CardReadAttempt::Read(ParsedCardRead {
                card_number: Some("12345678".to_string()),
                card_type: Some(MatrixCardType::Mifare1K),
            }),
        );
        assert_eq!(matched.outcome, "success");
        assert!(matched.compatible);

        let crossed = diagnose_card_read(
            "reader1=3",
            CardReadAttempt::Read(ParsedCardRead {
                card_number: Some("1".to_string()),
                card_type: Some(MatrixCardType::IClass2K2),
            }),
        );
        assert_eq!(crossed.outcome, "incompatible");
        assert!(crossed.message.contains("MIFARE"));
        assert!(crossed.message.contains("HID iCLASS 2K2"));

        let reverse = diagnose_card_read(
            "reader1=4",
            CardReadAttempt::Read(ParsedCardRead {
                card_number: None,
                card_type: Some(MatrixCardType::Mifare4K),
            }),
        );
        assert_eq!(reverse.outcome, "incompatible");

        assert_eq!(
            diagnose_card_read("reader1=1", CardReadAttempt::DeviceCode(27)).outcome,
            "timeout"
        );
        assert_eq!(
            diagnose_card_read("reader1=1", CardReadAttempt::DeviceCode(16)).outcome,
            "device_busy"
        );
        assert_eq!(
            diagnose_card_read("reader1=1", CardReadAttempt::DeviceCode(26)).outcome,
            "parameters_not_applicable"
        );
        assert_eq!(
            diagnose_card_read("reader1=1", CardReadAttempt::DeviceCode(28)).outcome,
            "read_failed"
        );
        assert_eq!(
            diagnose_card_read("reader1=1", CardReadAttempt::DeviceCode(29)).outcome,
            "wrong_card_type"
        );
        assert_eq!(
            diagnose_card_read("reader1=1", CardReadAttempt::DeviceCode(30)).outcome,
            "key_mismatch"
        );
        assert_eq!(
            diagnose_card_read("reader1=1", CardReadAttempt::TimedOut).outcome,
            "timeout"
        );
        assert_eq!(
            diagnose_card_read("reader1=1", CardReadAttempt::Unreachable).outcome,
            "unreachable"
        );
        assert_eq!(
            diagnose_card_read("reader1=1", CardReadAttempt::AuthFailed).outcome,
            "auth_failed"
        );
        assert_eq!(
            diagnose_card_read("reader1=1", CardReadAttempt::BadResponse).outcome,
            "bad_response"
        );
        assert_eq!(
            diagnose_card_read("reader1=0", CardReadAttempt::NotAttempted).outcome,
            "reader_not_configured"
        );
        assert_eq!(
            diagnose_card_read("reader3=8", CardReadAttempt::NotAttempted).outcome,
            "reader_unsupported"
        );
        let prox = diagnose_card_read(
            "reader1=1",
            CardReadAttempt::Read(ParsedCardRead {
                card_number: Some("445566".to_string()),
                card_type: Some(MatrixCardType::ReadOnly),
            }),
        );
        assert_eq!(prox.outcome, "success");
        assert_eq!(prox.reader.as_deref(), Some("EM Prox"));
        assert!(CardReaderFamily::EmProx.is_proximity());
        assert!(CardReaderFamily::HidProx.is_proximity());
        assert!(!CardReaderFamily::Mifare.is_proximity());
        let proximity = diagnose_card_read("reader1=1", CardReadAttempt::ProximityReader);
        assert_eq!(proximity.outcome, "proximity");
        assert!(proximity.compatible);
        assert!(proximity.message.contains("Enroll on device"));
    }
}
