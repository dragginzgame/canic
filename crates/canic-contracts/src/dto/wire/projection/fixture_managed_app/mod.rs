//! Bounded Canic transports for managed app validation.
//!
//! Keep command tables and reply decoding limited to the exercised methods.
//! Canonical compatibility checks cover every retained selector and payload.

use crate::dto::{
    component_registry::ComponentRuntimeDirectoryPreparationRequest,
    fleet_admission::{FleetAdmissionPrepareTargetRequest, FleetAdmissionProjectionStatusResponse},
    page::PageRequest,
    role::OperationStatusRequest,
    wire::projection::component_registry::CanisterOperationStatusFragment as ManagedOperationStatusResponse,
};

/// Bounded ManagedCommand selectors for managed app validation.
#[derive(ic_platform::candid::contract::CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum ManagedCommand {
    ConfigureRuntime(Box<ComponentRuntimeDirectoryPreparationRequest>),
    PrepareFleetAdmission(Box<FleetAdmissionPrepareTargetRequest>),
}

/// Bounded ManagedStatusRequest selectors for managed app validation.
#[derive(ic_platform::candid::contract::CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum ManagedStatusRequest {
    Admission(PageRequest),
    Operation(OperationStatusRequest),
}

/// Bounded ManagedStatusResponse selectors for managed app validation.
#[derive(ic_platform::candid::contract::CandidType, serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
#[expect(
    clippy::large_enum_variant,
    reason = "the test decoder mirrors the generated managed status wire"
)]
pub enum ManagedStatusResponse {
    Admission(FleetAdmissionProjectionStatusResponse),
    Operation(Box<ManagedOperationStatusResponse>),
}
