//! Bounded Canic transports for fleet_registry baseline tests application_initialization validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::component_registry::RootComponentInitializationRequest;
use candid::CandidType;

/// Bounded transport selectors used by fleet_registry baseline tests application_initialization validation.
#[derive(CandidType)]
pub enum Command {
    BindComponentInitialization(RootComponentInitializationRequest),
}
