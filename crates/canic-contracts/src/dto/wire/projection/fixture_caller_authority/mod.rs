//! Bounded Canic transports for caller_authority validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::caller_authority::{CallerAuthorityCommand, CallerAuthorityReceipt};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by caller_authority validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum Command {
    CallerAuthority(CallerAuthorityCommand),
}

/// Bounded transport selectors used by caller_authority validation.
#[derive(CandidType, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum Response {
    CallerAuthority(CallerAuthorityReceipt),
}
