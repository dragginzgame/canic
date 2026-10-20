//! Bounded Canic transports for fleet_registry baseline tests state_cascade validation.
//!
//! Keep fixture command tables and selected reply decoding bounded to the
//! tested operation. Selectors may dispatch to several methods or roles in
//! authorization cases; compatibility tests name those canonical contracts.
//! Application probe payloads and external IC protocols stay with the fixture.

use crate::dto::{
    cascade::StateSnapshotInput,
    fleet_admission::FleetAdmissionOpenTargetRequest,
    state::{
        FleetStateCommandResult, FleetStateResponse, FleetStatus, SetCyclesFundingRequest,
        SetFleetStatusRequest,
    },
    template::WasmStoreGcRequest,
};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by fleet_registry baseline tests state_cascade validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum StateCommand {
    MaintainPool,
    SetCyclesFunding(SetCyclesFundingRequest),
    SetFleetStatus(SetFleetStatusRequest),
}

/// Bounded transport selectors used by fleet_registry baseline tests state_cascade validation.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum StateResponse {
    SetCyclesFunding(FleetStateCommandResult<bool>),
    SetFleetStatus(FleetStateCommandResult<FleetStatus>),
}

/// Bounded transport selectors used by fleet_registry baseline tests state_cascade validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum StateQuery {
    FleetState,
}

/// Bounded transport selectors used by fleet_registry baseline tests state_cascade validation.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum StateQueryResponse {
    FleetState(FleetStateResponse),
}

/// Bounded transport selectors used by fleet_registry baseline tests state_cascade validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum SnapshotCommand {
    SynchronizeState(StateSnapshotInput),
}

/// Bounded transport selectors used by fleet_registry baseline tests state_cascade validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum ManagedAdminCommand {
    OpenFleetAdmission(FleetAdmissionOpenTargetRequest),
}

/// Bounded transport selectors used by fleet_registry baseline tests state_cascade validation.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum StoreAdminCommand {
    RunGc(WasmStoreGcRequest),
}
