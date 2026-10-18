//! Passive admission transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    fleet_admission::{
        FleetAdmissionMutationRequest, FleetAdmissionMutationResponse,
        FleetAdmissionRootStatusResponse, FleetAdmissionStatusRequest,
        FleetAdmissionStatusResponse,
    },
    fleet_registry::{FleetRegistry, FleetRegistryVersion},
    page::PageRequest,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded RemoteCoordinatorStatusRequest wire projection for admission.

#[derive(CandidType)]
pub enum RemoteCoordinatorStatusRequest {
    Admission(FleetAdmissionStatusRequest),
    Registry,
    RegistryVersion,
}

/// Bounded RemoteCoordinatorStatusResponse wire projection for admission.
#[derive(CandidType, Deserialize)]
#[allow(
    clippy::large_enum_variant,
    reason = "layout differs across host/Wasm builds, so expect is unfulfilled in the host build; retain this public projection without adding allocation or changing its Rust API"
)]
pub enum RemoteCoordinatorStatusResponse {
    Admission(FleetAdmissionStatusResponse),
    Registry(FleetRegistry),
    RegistryVersion(FleetRegistryVersion),
}

/// Bounded RemoteCoordinatorCommand wire projection for admission.
#[derive(CandidType)]
pub enum RemoteCoordinatorCommand {
    MutateAdmission(FleetAdmissionMutationRequest),
}

/// Bounded RemoteCoordinatorCommandResponse wire projection for admission.
#[derive(CandidType, Deserialize)]
pub enum RemoteCoordinatorCommandResponse {
    MutateAdmission(FleetAdmissionMutationResponse),
}

/// Bounded RemoteRootStatusRequest wire projection for admission.
#[derive(CandidType)]
pub enum RemoteRootStatusRequest {
    Admission(PageRequest),
}

/// Bounded RemoteRootStatusResponse wire projection for admission.
#[derive(CandidType, Deserialize)]
pub enum RemoteRootStatusResponse {
    Admission(FleetAdmissionRootStatusResponse),
}
