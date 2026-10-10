//! Bounded Canic transports for managed app validation.
//!
//! Keep command tables and reply decoding limited to the exercised methods.
//! Canonical compatibility checks cover every retained selector and payload.

use crate::dto::component_registry::ComponentRuntimeDirectoryPreparationRequest;
use crate::dto::fleet_admission::FleetAdmissionPrepareTargetRequest;
use crate::dto::fleet_admission::FleetAdmissionProjectionStatusResponse;
use crate::dto::page::PageRequest;
use crate::dto::role::OperationStatusRequest;
use crate::dto::wire::projection::component_registry::CanisterOperationStatusFragment as ManagedOperationStatusResponse;

/// Bounded ManagedCommand selectors for managed app validation.
#[derive(candid::CandidType)]
pub enum ManagedCommand {
    ConfigureRuntime(Box<ComponentRuntimeDirectoryPreparationRequest>),
    PrepareFleetAdmission(Box<FleetAdmissionPrepareTargetRequest>),
}

/// Bounded ManagedStatusRequest selectors for managed app validation.
#[derive(candid::CandidType)]
pub enum ManagedStatusRequest {
    Admission(PageRequest),
    Operation(OperationStatusRequest),
}

/// Bounded ManagedStatusResponse selectors for managed app validation.
#[derive(candid::CandidType, serde::Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the test decoder mirrors the generated managed status wire"
)]
pub enum ManagedStatusResponse {
    Admission(FleetAdmissionProjectionStatusResponse),
    Operation(Box<ManagedOperationStatusResponse>),
}
