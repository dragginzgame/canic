//! Bounded Canic transports for fleet_registry baseline tests funding_inventory validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::{
    component_registry::{
        ComponentDirectoryHead, ComponentDirectoryHeadRequest, ComponentDirectoryPageRequest,
        ComponentDirectoryPageResponse,
    },
    root::RootFundingReleaseResponse,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by fleet_registry baseline tests funding_inventory validation.
#[derive(CandidType)]
pub enum Request {
    ComponentDirectoryHead(ComponentDirectoryHeadRequest),
    ComponentDirectoryPage(Box<ComponentDirectoryPageRequest>),
    FundingRelease(Option<u64>),
}

/// Bounded transport selectors used by fleet_registry baseline tests funding_inventory validation.
#[derive(CandidType, Deserialize)]
pub enum Response {
    ComponentDirectoryHead(ComponentDirectoryHead),
    ComponentDirectoryPage(ComponentDirectoryPageResponse),
    FundingRelease(Box<RootFundingReleaseResponse>),
}

/// Bounded transport selectors used by fleet_registry baseline tests funding_inventory validation.
#[derive(CandidType, Deserialize)]
pub enum CommandResponse {
    InspectCanister(Box<crate::dto::canister::CanisterStatusResponse>),
    ObserveCanister(crate::dto::observability::CanisterObservabilityResponse),
}
