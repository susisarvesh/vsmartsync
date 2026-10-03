//! Matrix hardware integration boundary.
//!
//! All communication with Matrix devices must stay inside this module.
//! Do not call Matrix HTTP APIs from repositories or React.
//!
//! **Implemented:** Client foundation (`Response-Code`), connectivity probe,
//! Adapter `set_user`, `set_pin`, enrollment capability reads, `enroll_user`,
//! `credential?action=get` for card fields, direct `card-read-write?action=read`,
//! and read-only `smart-card-format?action=get`.
//!
//! **Not in this slice:** `set_card`, credential template download, retries,
//! `/check-enrollment`, smart-card-format changes, and a TCP event daemon.

pub mod adapter;
pub mod client;
mod enrollment;
pub mod models;

pub use adapter::{MatrixAdapter, MatrixAdapterError, MatrixProbeError, SetUserParams};
pub use enrollment::{
    capture_increased, configured_reader, credential_present, diagnose_card_read,
    parse_card_key_flags, parse_smart_card_format, CardReadAttempt, CredentialCounts,
    HardwareEnrollType, MatrixCardType, MatrixIdentifierType, ParsedCardCredential,
};
