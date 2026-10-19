//! Bounded Canic transports for fleet_registry baseline tests application_initialization validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::component_registry::RootComponentInitializationRequest;
use ic_platform::candid::contract::CandidType;

/// Bounded transport selectors used by fleet_registry baseline tests application_initialization validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum Command {
    BindComponentInitialization(RootComponentInitializationRequest),
}
