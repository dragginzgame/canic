//! Bounded Canic transports for managed component group validation.
//!
//! Keep command tables and reply decoding limited to the exercised methods.
//! Canonical compatibility checks cover every retained selector and payload.

use crate::dto::{
    fleet_admission::{FleetAdmissionProjectionStatusResponse, FleetAdmissionTargetReceipt},
    page::PageRequest,
    role::{OperationReceipt, OperationStatusRequest, RoleOverviewResponse},
    runtime::CanicRuntimeStatus,
    wire::projection::component_registry::CanisterOperationStatusFragment as ManagedOperationStatusResponse,
};

/// Bounded ManagedCommandResponse selectors for managed component group validation.
#[derive(ic_platform::candid::contract::CandidType, serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum ManagedCommandResponse {
    OperationAccepted(OperationReceipt),
    PrepareFleetAdmission(Box<FleetAdmissionTargetReceipt>),
}

/// Bounded ManagedStatusRequest selectors for managed component group validation.
#[derive(ic_platform::candid::contract::CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum ManagedStatusRequest {
    Admission(PageRequest),
    Binding,
    Operation(OperationStatusRequest),
    Overview,
    Runtime,
}

/// Bounded ManagedStatusResponse selectors for managed component group validation.
#[derive(ic_platform::candid::contract::CandidType, serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum ManagedStatusResponse {
    Admission(FleetAdmissionProjectionStatusResponse),
    Binding(crate::ids::ManagedCanisterBinding),
    Operation(Box<ManagedOperationStatusResponse>),
    Overview(RoleOverviewResponse),
    Runtime(CanicRuntimeStatus),
}
