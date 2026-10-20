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
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded RemoteCoordinatorStatusRequest wire projection for admission.

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum RemoteCoordinatorStatusRequest {
    Admission(FleetAdmissionStatusRequest),
    Registry,
    RegistryVersion,
}

/// Bounded RemoteCoordinatorStatusResponse wire projection for admission.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RemoteCoordinatorStatusResponse {
    Admission(FleetAdmissionStatusResponse),
    Registry(FleetRegistry),
    RegistryVersion(FleetRegistryVersion),
}

/// Bounded RemoteCoordinatorCommand wire projection for admission.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum RemoteCoordinatorCommand {
    MutateAdmission(FleetAdmissionMutationRequest),
}

/// Bounded RemoteCoordinatorCommandResponse wire projection for admission.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RemoteCoordinatorCommandResponse {
    MutateAdmission(FleetAdmissionMutationResponse),
}

/// Bounded RemoteRootStatusRequest wire projection for admission.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum RemoteRootStatusRequest {
    Admission(PageRequest),
}

/// Bounded RemoteRootStatusResponse wire projection for admission.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RemoteRootStatusResponse {
    Admission(FleetAdmissionRootStatusResponse),
}
