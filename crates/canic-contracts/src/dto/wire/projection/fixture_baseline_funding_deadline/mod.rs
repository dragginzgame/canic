//! Bounded Canic transports for fleet_registry baseline tests funding_deadline validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::state::FleetStateCommandResult;
use crate::dto::state::SetCyclesFundingRequest;
use candid::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by fleet_registry baseline tests funding_deadline validation.
#[derive(CandidType)]
pub enum Command {
    SetCyclesFunding(SetCyclesFundingRequest),
}

/// Bounded transport selectors used by fleet_registry baseline tests funding_deadline validation.
#[derive(CandidType, Deserialize)]
pub enum Response {
    SetCyclesFunding(FleetStateCommandResult<bool>),
}
