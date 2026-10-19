//! Bounded Canic transports for fleet_registry baseline tests funding_deadline validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::state::{FleetStateCommandResult, SetCyclesFundingRequest};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by fleet_registry baseline tests funding_deadline validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum Command {
    SetCyclesFunding(SetCyclesFundingRequest),
}

/// Bounded transport selectors used by fleet_registry baseline tests funding_deadline validation.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum Response {
    SetCyclesFunding(FleetStateCommandResult<bool>),
}
