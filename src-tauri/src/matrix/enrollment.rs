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

/// Command actually sent to `enrolluser`.
///
/// An HID iCLASS reader on this firmware reads the card serial. `enrolluser`
/// type 1 waits for a smart-card capture that does not start, and the call
/// times out. Type 0 is the documented card enrollment and matches that read.
/// A MIFARE reader keeps type 1.
pub fn capture_enroll_type(
    requested: HardwareEnrollType,
    reader_config: &str,
) -> HardwareEnrollType {
    let iclass = configured_reader(reader_config).and_then(|reader| reader.family)
        == Some(CardReaderFamily::HidIclass);
    if requested == HardwareEnrollType::SmartCard && iclass {
        HardwareEnrollType::ReadOnlyCard
    } else {
        requested
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
    // Reader1 3, 4, and 5 are smart-card readers. Door mode must not turn
    // that into a read-only (EM/HID Prox) enrollment.
    let reader1_is_smart = config_field(reader_config, "reader1")
        .and_then(|raw| raw.parse::<i32>().ok())
        .is_some_and(|code| matches!(code, 3..=5));
    if reader1_is_smart {
        read_only_card = false;
    } else if !face200t
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

    /// `card-read-write` is the smart-card read. EM Prox and HID Prox are not
    /// those readers, and this firmware answers that call with response code 27.
    pub fn uses_card_read(self) -> bool {
        matches!(self, Self::Mifare | Self::HidIclass)
    }

    /// Card types the guide associates with this reader. DESFire types are the
    /// documented MIFARE card-type codes. Read-only is EM Prox and HID Prox.
    /// An HID iCLASS reader also returns card-type 9 when it reads the card
    /// serial (CSN). That card enrolls, so it is a successful read.
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
                    | MatrixCardType::ReadOnly
            ),
        }
    }
}

/// One configured interface from `reader-config?action=get`.
///
/// Reader 1 and reader 2 use the same type codes: `2` is HID, `3` is MIFARE.
/// Reader 3 uses its own table. Code `0` is not a configured reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportedReaderSlot {
    pub slot: &'static str,
    pub code: i32,
    pub label: &'static str,
    pub family: Option<CardReaderFamily>,
}

/// `door-access-mode` from `reader-config`. Absent when the device did not report it.
pub fn door_access_mode(reader_config: &str) -> Option<i32> {
    config_field(reader_config, "door-access-mode").and_then(|raw| raw.parse().ok())
}

/// Card-format view of `reader-config`. Order is reader 1, reader 2, reader 3.
pub fn reader_slots(reader_config: &str) -> Vec<ReportedReaderSlot> {
    let mut slots = Vec::new();
    push_entry_reader(&mut slots, reader_config, "reader1");
    push_entry_reader(&mut slots, reader_config, "reader2");
    if let Some(code) = config_field(reader_config, "reader3").and_then(|raw| raw.parse().ok()) {
        if let Some(label) = reader3_label(code) {
            slots.push(ReportedReaderSlot {
                slot: "reader3",
                code,
                label,
                family: reader3_family(code),
            });
        }
    }
    slots
}

fn push_entry_reader(slots: &mut Vec<ReportedReaderSlot>, reader_config: &str, slot: &'static str) {
    let Some(code) = config_field(reader_config, slot).and_then(|raw| raw.parse().ok()) else {
        return;
    };
    let Some(label) = reader1_label(code) else {
        return;
    };
    slots.push(ReportedReaderSlot {
        slot,
        code,
        label,
        family: reader1_family(code),
    });
}

/// First card reader from `reader-config`. Reader 1 is preferred, then reader 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfiguredReader {
    pub code: i32,
    pub label: &'static str,
    pub family: Option<CardReaderFamily>,
}

pub fn configured_reader(reader_config: &str) -> Option<ConfiguredReader> {
    let slots = reader_slots(reader_config);
    let chosen = slots
        .iter()
        .find(|slot| slot.family.is_some())
        .or_else(|| slots.first());
    chosen.map(|slot| ConfiguredReader {
        code: slot.code,
        label: slot.label,
        family: slot.family,
    })
}

fn reader1_family(value: i32) -> Option<CardReaderFamily> {
    ReaderType::from_reader1(value).and_then(ReaderType::family)
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

/// Documented `reader1` codes. `None` is code 0 and is not a configured reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderType {
    None,
    EmProx,
    HidProx,
    Mifare,
    HidIclassU,
    HidIclassW,
}

impl ReaderType {
    pub fn from_reader1(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::None),
            1 => Some(Self::EmProx),
            2 => Some(Self::HidProx),
            3 => Some(Self::Mifare),
            4 => Some(Self::HidIclassU),
            5 => Some(Self::HidIclassW),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::EmProx => "EM Prox Reader",
            Self::HidProx => "HID Prox Reader",
            Self::Mifare => "MiFare Reader",
            Self::HidIclassU => "HID iCLASS-U Reader",
            Self::HidIclassW => "HID iCLASS-W Reader",
        }
    }

    pub fn family(self) -> Option<CardReaderFamily> {
        match self {
            Self::None => None,
            Self::EmProx => Some(CardReaderFamily::EmProx),
            Self::HidProx => Some(CardReaderFamily::HidProx),
            Self::Mifare => Some(CardReaderFamily::Mifare),
            Self::HidIclassU | Self::HidIclassW => Some(CardReaderFamily::HidIclass),
        }
    }
}

/// MIFARE card types returned by `card-read-write` and `smart-card-format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MifareCardType {
    Mifare1K,
    Mifare4K,
}

impl MifareCardType {
    pub fn from_matrix(card: MatrixCardType) -> Option<Self> {
        match card {
            MatrixCardType::Mifare1K => Some(Self::Mifare1K),
            MatrixCardType::Mifare4K => Some(Self::Mifare4K),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Mifare1K => "Mifare 1K",
            Self::Mifare4K => "Mifare 4K",
        }
    }
}

/// `smart-card-format` `card-no`: how the reader builds the card number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardNumberMode {
    Csn,
    Uid,
    Custom,
}

impl From<MatrixIdentifierType> for CardNumberMode {
    fn from(value: MatrixIdentifierType) -> Self {
        match value {
            MatrixIdentifierType::Csn => Self::Csn,
            MatrixIdentifierType::Uid => Self::Uid,
            MatrixIdentifierType::Custom => Self::Custom,
        }
    }
}

impl CardNumberMode {
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

/// `card-read-write` response codes used by the card diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixCardReadResponseCode {
    Success,
    DeviceBusy,
    Timeout,
    ReadWriteFailed,
    WrongCardType,
    KeyMismatch,
}

impl MatrixCardReadResponseCode {
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            16 => Some(Self::DeviceBusy),
            27 => Some(Self::Timeout),
            28 => Some(Self::ReadWriteFailed),
            29 => Some(Self::WrongCardType),
            30 => Some(Self::KeyMismatch),
            _ => None,
        }
    }

    pub fn code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::DeviceBusy => 16,
            Self::Timeout => 27,
            Self::ReadWriteFailed => 28,
            Self::WrongCardType => 29,
            Self::KeyMismatch => 30,
        }
    }

    pub fn message(self, reader: Option<CardReaderFamily>) -> &'static str {
        match self {
            Self::Success => "Card successfully detected.",
            Self::DeviceBusy => "The Matrix reader is currently busy.",
            Self::Timeout => "Matrix did not detect a card before timeout.",
            Self::ReadWriteFailed => "Matrix failed to read the card.",
            Self::WrongCardType => {
                "The presented card type does not match the configured Matrix reader."
            }
            Self::KeyMismatch if reader == Some(CardReaderFamily::Mifare) => "MIFARE key mismatch.",
            Self::KeyMismatch => "Key mismatch.",
        }
    }
}

/// `smart-card-format?action=get`. `card-no` here is the identifier mode, not a card number.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SmartCardFormat {
    pub card_type: Option<MatrixCardType>,
    pub identifier: Option<CardNumberMode>,
    /// Device `read-csn` value when it is a short numeric flag. Not a card number.
    pub read_csn: Option<String>,
}

pub fn parse_smart_card_format(body: &str) -> SmartCardFormat {
    SmartCardFormat {
        card_type: config_field(body, "card-type")
            .and_then(|raw| raw.parse::<i32>().ok())
            .and_then(MatrixCardType::from_code),
        identifier: config_field(body, "card-no")
            .and_then(|raw| MatrixIdentifierType::parse(&raw))
            .map(CardNumberMode::from),
        read_csn: short_numeric_field(body, "read-csn"),
    }
}

/// `internal-card-format?action=get`. Values are what the device reports.
/// Nothing here is written back, and bit width is not turned into a card rule.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InternalCardFormat {
    pub format_id: Option<String>,
    pub max_no_of_bits: Option<u32>,
    pub card_structure: Option<String>,
    pub card_read_order: Option<String>,
    pub include_facility_code: Option<String>,
    pub seq_of_operation: Option<String>,
}

pub fn parse_internal_card_format(body: &str) -> InternalCardFormat {
    InternalCardFormat {
        format_id: short_config_value(body, "format-id"),
        max_no_of_bits: config_field(body, "max-no-of-bits").and_then(|raw| raw.parse().ok()),
        card_structure: short_config_value(body, "card-structure"),
        card_read_order: short_config_value(body, "card-read-order"),
        include_facility_code: short_config_value(body, "include-facility-code"),
        seq_of_operation: short_config_value(body, "seq-of-operation"),
    }
}

fn short_numeric_field(body: &str, key: &str) -> Option<String> {
    let value = config_field(body, key)?;
    let trimmed = value.trim();
    if !trimmed.is_empty() && trimmed.len() <= 8 && trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn short_config_value(body: &str, key: &str) -> Option<String> {
    let value = config_field(body, key)?;
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > 32
        || trimmed.to_ascii_lowercase().contains("key")
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return None;
    }
    Some(trimmed.to_string())
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
    /// EM Prox or HID Prox. `card-read-write` was not sent.
    DirectEnrollment,
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
    pub response_code: Option<i32>,
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
        response_code: None,
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
        CardReadAttempt::DirectEnrollment => base(
            "enroll_on_device",
            format!(
                "This device reports {}. The smart-card read does not detect a card on that setting. Click Enroll on device, then hold the card on the reader.",
                reader.label
            ),
            true,
        ),
        CardReadAttempt::TimedOut => base(
            "timeout",
            "Matrix did not detect a card before timeout.".to_string(),
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
        CardReadAttempt::DeviceCode(26) => {
            let mut report = base(
                "parameters_not_applicable",
                "The card read parameters do not apply to this card type.".to_string(),
                false,
            );
            report.response_code = Some(26);
            report
        }
        CardReadAttempt::DeviceCode(code) => {
            let (outcome, message) = match MatrixCardReadResponseCode::from_code(code) {
                Some(known) => (
                    match known {
                        MatrixCardReadResponseCode::DeviceBusy => "device_busy",
                        MatrixCardReadResponseCode::Timeout => "timeout",
                        MatrixCardReadResponseCode::ReadWriteFailed => "read_failed",
                        MatrixCardReadResponseCode::WrongCardType => "wrong_card_type",
                        MatrixCardReadResponseCode::KeyMismatch => "key_mismatch",
                        MatrixCardReadResponseCode::Success => "bad_response",
                    },
                    known.message(reader.family).to_string(),
                ),
                None => (
                    "bad_response",
                    format!("The device returned Matrix response code {code}."),
                ),
            };
            let mut report = base(outcome, message, false);
            report.response_code = Some(code);
            report
        }
        CardReadAttempt::NotAttempted => base(
            "bad_response",
            "The device returned an unexpected response.".to_string(),
            false,
        ),
        CardReadAttempt::Read(parsed) => {
            let card_label = parsed.card_type.map(|value| {
                MifareCardType::from_matrix(value)
                    .map(|kind| kind.label().to_string())
                    .unwrap_or_else(|| value.label().to_string())
            });
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
            report.response_code = Some(MatrixCardReadResponseCode::Success.code());
            report
        }
    }
}

/// Human label for the first configured reader. `None` means no reader was reported.
pub fn reader_label(reader_config: &str) -> Option<String> {
    reader_slots(reader_config)
        .into_iter()
        .next()
        .map(|slot| slot.label.to_string())
}

fn reader1_label(value: i32) -> Option<&'static str> {
    ReaderType::from_reader1(value)
        .filter(|kind| *kind != ReaderType::None)
        .map(ReaderType::label)
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
    fn iclass_card_enrollment_uses_the_read_only_command() {
        assert_eq!(
            super::capture_enroll_type(
                HardwareEnrollType::SmartCard,
                "reader1=4 door-access-mode=6"
            ),
            HardwareEnrollType::ReadOnlyCard
        );
        assert_eq!(HardwareEnrollType::ReadOnlyCard.matrix_type(), 0);
        assert_eq!(
            super::capture_enroll_type(HardwareEnrollType::SmartCard, "reader1=3"),
            HardwareEnrollType::SmartCard
        );
        assert_eq!(
            super::capture_enroll_type(HardwareEnrollType::Face, "reader1=4"),
            HardwareEnrollType::Face
        );
    }

    #[test]
    fn mifare_reader_is_smart_card() {
        let types = supported_enroll_types("max-fingers=0", "reader1=3", "");
        assert!(types.contains(&HardwareEnrollType::SmartCard));
        assert!(!types.contains(&HardwareEnrollType::ReadOnlyCard));
        assert!(types.contains(&HardwareEnrollType::BiometricThenCard));
        let with_face = supported_enroll_types(
            "<max-faces>9</max-faces>",
            "reader1=3 door-access-mode=6",
            "enroll-card-count=0",
        );
        assert!(with_face.contains(&HardwareEnrollType::SmartCard));
        assert!(with_face.contains(&HardwareEnrollType::Face));
        assert!(!with_face.contains(&HardwareEnrollType::ReadOnlyCard));
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

        let slots = super::reader_slots("reader1=3 reader2=2 reader3=1 door-access-mode=6");
        assert_eq!(slots.len(), 3);
        assert_eq!(slots[0].slot, "reader1");
        assert_eq!(slots[0].code, 3);
        assert_eq!(slots[0].family, Some(super::CardReaderFamily::Mifare));
        assert_eq!(slots[1].slot, "reader2");
        assert_eq!(slots[1].code, 2);
        assert_eq!(slots[1].family, Some(super::CardReaderFamily::HidProx));
        assert_eq!(slots[2].slot, "reader3");
        assert_eq!(slots[2].code, 1);
        assert_eq!(
            super::configured_reader("reader1=0 reader2=2")
                .unwrap()
                .family,
            Some(super::CardReaderFamily::HidProx)
        );

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
            configured_reader, diagnose_card_read, parse_card_key_flags,
            parse_internal_card_format, parse_smart_card_format, CardNumberMode, CardReadAttempt,
            CardReaderFamily, MatrixCardType, ParsedCardRead, ReaderType,
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
        assert!(CardReaderFamily::HidIclass.accepts(MatrixCardType::ReadOnly));
        assert!(!CardReaderFamily::HidIclass.accepts(MatrixCardType::Mifare1K));
        assert!(CardReaderFamily::EmProx.accepts(MatrixCardType::ReadOnly));
        assert!(!CardReaderFamily::HidProx.accepts(MatrixCardType::Mifare1K));

        let format = parse_smart_card_format("card-type=4 card-no=0");
        assert_eq!(format.card_type, Some(MatrixCardType::Mifare1K));
        assert_eq!(format.identifier, Some(CardNumberMode::Csn));
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

        let iclass_csn = diagnose_card_read(
            "reader1=4 door-access-mode=6",
            CardReadAttempt::Read(ParsedCardRead {
                card_number: Some("51579".to_string()),
                card_type: Some(MatrixCardType::ReadOnly),
            }),
        );
        assert_eq!(iclass_csn.outcome, "success");
        assert!(iclass_csn.compatible);
        assert_eq!(
            iclass_csn.card_type_label.as_deref(),
            Some("Read-only card")
        );

        assert_eq!(
            diagnose_card_read("reader1=3", CardReadAttempt::DeviceCode(27)).message,
            "Matrix did not detect a card before timeout."
        );
        assert_eq!(
            diagnose_card_read("reader1=3", CardReadAttempt::DeviceCode(27)).response_code,
            Some(27)
        );
        assert_eq!(
            diagnose_card_read("reader1=3", CardReadAttempt::DeviceCode(28)).message,
            "Matrix failed to read the card."
        );
        assert_eq!(
            diagnose_card_read("reader1=3", CardReadAttempt::DeviceCode(29)).message,
            "The presented card type does not match the configured Matrix reader."
        );
        assert_eq!(
            diagnose_card_read("reader1=3", CardReadAttempt::DeviceCode(30)).message,
            "MIFARE key mismatch."
        );
        assert_eq!(
            diagnose_card_read("reader1=3", CardReadAttempt::DeviceCode(16)).message,
            "The Matrix reader is currently busy."
        );
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
        assert!(!CardReaderFamily::EmProx.uses_card_read());
        assert!(CardReaderFamily::Mifare.uses_card_read());
        let direct = diagnose_card_read("reader1=1", CardReadAttempt::DirectEnrollment);
        assert_eq!(direct.outcome, "enroll_on_device");
        assert!(direct.message.contains("Enroll on device"));
        assert_eq!(ReaderType::from_reader1(3), Some(ReaderType::Mifare));
        assert_eq!(ReaderType::Mifare.label(), "MiFare Reader");
        assert_eq!(
            configured_reader("reader1=3").unwrap().family,
            Some(CardReaderFamily::Mifare)
        );
        let mifare = diagnose_card_read(
            "reader1=3",
            CardReadAttempt::Read(ParsedCardRead {
                card_number: Some("12345678".to_string()),
                card_type: Some(MatrixCardType::Mifare1K),
            }),
        );
        assert_eq!(mifare.outcome, "success");
        assert_eq!(mifare.card_type_label.as_deref(), Some("Mifare 1K"));
        assert_eq!(mifare.response_code, Some(0));
        assert_eq!(mifare.reader_label.as_deref(), Some("MiFare Reader"));
        let mifare_format = parse_smart_card_format(
            "<card-type>5</card-type><card-no>1</card-no><read-csn>0</read-csn>",
        );
        assert_eq!(mifare_format.card_type, Some(MatrixCardType::Mifare4K));
        assert_eq!(mifare_format.identifier, Some(CardNumberMode::Uid));
        assert_eq!(mifare_format.read_csn.as_deref(), Some("0"));
        let bits = parse_internal_card_format(
            "<format-id>2</format-id><max-no-of-bits>56</max-no-of-bits><card-structure>1</card-structure><mifare-custom-key>secret</mifare-custom-key>",
        );
        assert_eq!(bits.max_no_of_bits, Some(56));
        assert_eq!(bits.format_id.as_deref(), Some("2"));
        assert!(bits.card_structure.is_some());
    }
}
