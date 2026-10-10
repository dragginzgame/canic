//! Bounded Canic transports for framework validation.
//!
//! Retain only exercised selectors and reply payloads, checked against canonical contracts.

use crate::dto::{
    fleet_activation::FleetActivationResumeRequest,
    fleet_subnet_root::FleetSubnetWasmStoreAdoptionRequest, role::OperationStatusRequest,
    root::RootOperationStatusResponse, runtime::CanicReadinessStatus,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by framework validation.
#[derive(CandidType)]
#[expect(
    clippy::large_enum_variant,
    reason = "the test decoder mirrors the direct Root command wire without a parallel DTO"
)]
pub enum RootCommandFragment {
    AdoptStore(FleetSubnetWasmStoreAdoptionRequest),
    PrepareFleetActivation,
    ResumeFleetActivation(FleetActivationResumeRequest),
}

/// Bounded transport selectors used by framework validation.
#[derive(CandidType)]
pub enum RootStatusRequestFragment {
    Operation(OperationStatusRequest),
    Readiness,
}

/// Bounded transport selectors used by framework validation.
#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the test decoder mirrors the direct Root status wire without a parallel DTO"
)]
pub enum RootStatusResponseFragment {
    Operation(RootOperationStatusResponse),
    Readiness(CanicReadinessStatus),
}

/// Bounded transport selectors used by framework validation.
#[derive(CandidType)]
pub enum CanisterStatusRequestFragment {
    Readiness,
}

/// Bounded transport selectors used by framework validation.
#[derive(CandidType, Deserialize)]
pub enum CanisterStatusResponseFragment {
    Readiness(CanicReadinessStatus),
}
