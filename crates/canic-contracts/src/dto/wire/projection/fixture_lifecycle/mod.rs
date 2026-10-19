//! Bounded Canic transports for framework validation.
//!
//! Retain only exercised selectors and reply payloads, checked against canonical contracts.

use crate::dto::{
    caller_authority::{
        CallerAuthorityCommand, CallerAuthorityPublication, CallerAuthorityReceipt,
    },
    component_registry::ComponentRuntimeDirectoryPreparationRequest,
    fleet_admission::{
        FleetAdmissionActivateTargetRequest, FleetAdmissionOpenTargetRequest,
        FleetAdmissionPrepareTargetRequest, FleetAdmissionTargetReceipt,
    },
    role::OperationReceipt,
};

/// Bounded transport selectors used by framework validation.
#[derive(ic_platform::candid::contract::CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum ManagedCommand {
    ActivateFleetAdmission(FleetAdmissionActivateTargetRequest),
    CallerAuthority(CallerAuthorityCommand),
    ConfigureRuntime(Box<ComponentRuntimeDirectoryPreparationRequest>),
    OpenFleetAdmission(FleetAdmissionOpenTargetRequest),
    PrepareFleetAdmission(Box<FleetAdmissionPrepareTargetRequest>),
    ReleaseApplicationStartup(CallerAuthorityPublication),
}

/// Bounded transport selectors used by framework validation.
#[derive(ic_platform::candid::contract::CandidType, Debug, serde::Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub enum ManagedCommandResponse {
    ActivateFleetAdmission(FleetAdmissionTargetReceipt),
    CallerAuthority(Box<CallerAuthorityReceipt>),
    OpenFleetAdmission(FleetAdmissionTargetReceipt),
    OperationAccepted(OperationReceipt),
    PrepareFleetAdmission(FleetAdmissionTargetReceipt),
}
