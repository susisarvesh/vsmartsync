//! Matrix hardware integration boundary.
//!
//! All communication with Matrix devices must stay inside this module.
//! Do not call Matrix HTTP APIs from repositories or React.
//!
//! **Implemented:** Client foundation (`Response-Code`), connectivity probe,
//! Adapter `set_user`, `set_pin`, enrollment capability reads, `enroll_user`,
//! `credential?action=get` for card fields, direct `card-read-write?action=read`,
//! read-only `smart-card-format?action=get`, and read-only
//! `internal-card-format?action=get`.
//!
//! **Not in this slice:** `set_card`, credential template download, retries,
//! `/check-enrollment`, smart-card-format changes, internal-card-format changes,
//! HTTP event fetch (`geteventcount`, `events?action=getevent`).
//! **Not in this slice:** `set_card`, a TCP event daemon, and `RPL_EVT` parsing.

pub mod adapter;
pub mod client;
mod enrollment;
pub(crate) mod events;
pub mod models;

pub use adapter::{MatrixAdapter, MatrixAdapterError, MatrixProbeError, SetUserParams};
pub use enrollment::{
    capture_enroll_type, capture_increased, configured_reader, credential_present,
    diagnose_card_read, door_access_mode, parse_card_key_flags, parse_internal_card_format,
    parse_smart_card_format, reader_slots, CardReadAttempt, CardReaderFamily, CredentialCounts,
    HardwareEnrollType, MatrixCardType, MatrixIdentifierType, ParsedCardCredential,
    ReportedReaderSlot,
};
