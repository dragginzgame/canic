//! One bounded management observation bracketed by certified physical custody reads.

use crate::fleet_ensure::{
    model::capacity_import::{CapacityImportSourceBinding, survey::CapacityImportSampleRecord},
    ops::{
        capacity_import::{admission::declarations::hash, journal::CapacityImportJournalError},
        reinstall::terminal::inventory::custody,
    },
};
use candid::{CandidType, Nat, Principal};
use canic_core::{
    dto::{
        canister::{CanisterInspectionRequest, CanisterStatusResponse, CanisterStatusType},
        error::Error,
    },
    protocol,
};
use ic_agent::Agent;
use serde::Deserialize;
use std::time::Duration;

#[derive(CandidType)]
struct Request {
    canister_id: Principal,
}

#[derive(CandidType, Deserialize)]
struct Status {
    version: u64,
    status: RunState,
    settings: Settings,
    module_hash: Option<Vec<u8>>,
    memory_metrics: Memory,
    cycles: Nat,
    reserved_cycles: Nat,
}

#[derive(CandidType, Deserialize)]
enum RunState {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "stopped")]
    Stopped,
}

#[derive(CandidType, Deserialize)]
struct Settings {
    controllers: Vec<Principal>,
}

#[derive(CandidType, Deserialize)]
struct Memory {
    snapshots_size: Nat,
}

#[derive(CandidType)]
enum RootRequest {
    InspectCanister(CanisterInspectionRequest),
}

#[derive(CandidType, Deserialize)]
enum RootResponse {
    InspectCanister(Box<CanisterStatusResponse>),
    InspectionReserveRequired(canic_core::dto::canister::CanisterInspectionReserveResponse),
}

/// Inspect a held child through current Root, bracketed by certified custody.
/// The caller reserves the observation attempt before entering this paid boundary.
pub(in crate::fleet_ensure) async fn observe_root_owned(
    agent: &Agent,
    root: Principal,
    canister: Principal,
) -> Result<CapacityImportSampleRecord, CapacityImportJournalError> {
    let invalid = || CapacityImportJournalError::ObservationUnavailable { canister };
    let before = custody::observe_one(agent, canister)
        .await
        .map_err(|_| invalid())?;
    if !before.controllers.contains(&root) {
        return Err(invalid());
    }
    let argument = candid::encode_one(RootRequest::InspectCanister(CanisterInspectionRequest {
        canister_id: canister,
    }))
    .map_err(|_| invalid())?;
    let bytes = tokio::time::timeout(
        Duration::from_secs(45),
        agent
            .update(&root, protocol::CANIC_ROOT_COMMAND)
            .with_arg(argument)
            .call_and_wait(),
    )
    .await
    .map_err(|_| invalid())?
    .map_err(|_| invalid())?;
    let response: Result<RootResponse, Error> =
        candid::decode_one(&bytes).map_err(|_| invalid())?;
    let RootResponse::InspectCanister(mut status) =
        response.map_err(CapacityImportJournalError::RootRejected)?
    else {
        return Err(invalid());
    };
    status.settings.controllers.sort_unstable();
    let module_sha256 = status
        .module_hash
        .map(|value| value.try_into().map_err(|_| invalid()))
        .transpose()?;
    let after = custody::observe_one(agent, canister)
        .await
        .map_err(|_| invalid())?;
    let expected = (
        before.subnet,
        &before.controllers,
        before.module_sha256.as_deref().map(hash).transpose()?,
    );
    let actual = (after.subnet, &status.settings.controllers, module_sha256);
    if expected != actual
        || after.controllers != status.settings.controllers
        || after.module_sha256.as_deref().map(hash).transpose()? != module_sha256
        || status.status == CanisterStatusType::Stopping
    {
        return Err(invalid());
    }
    Ok(CapacityImportSampleRecord {
        binding: CapacityImportSourceBinding {
            canister_id: canister,
            subnet: after.subnet,
            controllers: status.settings.controllers,
            module_sha256,
            canister_version: status.version,
            stopped: status.status == CanisterStatusType::Stopped,
            snapshots_size_bytes: status
                .memory_metrics
                .snapshots_size
                .0
                .try_into()
                .map_err(|_| invalid())?,
        },
        cycles: status.cycles.0.try_into().map_err(|_| invalid())?,
        reserved_cycles: status.reserved_cycles.0.try_into().map_err(|_| invalid())?,
    })
}

pub(in crate::fleet_ensure) async fn observe(
    agent: &Agent,
    canister: Principal,
) -> Result<CapacityImportSampleRecord, CapacityImportJournalError> {
    let invalid = || CapacityImportJournalError::ObservationUnavailable { canister };
    let before = custody::observe_one(agent, canister)
        .await
        .map_err(|_| invalid())?;
    let bytes = tokio::time::timeout(
        Duration::from_secs(45),
        agent
            .update(&Principal::management_canister(), "canister_status")
            .with_effective_canister_id(canister)
            .with_arg(
                candid::encode_one(Request {
                    canister_id: canister,
                })
                .map_err(|_| invalid())?,
            )
            .call_and_wait(),
    )
    .await
    .map_err(|_| invalid())?
    .map_err(|_| invalid())?;
    let mut status: Status = candid::decode_one(&bytes).map_err(|_| invalid())?;
    status.settings.controllers.sort_unstable();
    let module_sha256 = status
        .module_hash
        .map(|value| value.try_into().map_err(|_| invalid()))
        .transpose()?;
    let after = custody::observe_one(agent, canister)
        .await
        .map_err(|_| invalid())?;
    let before_module = before.module_sha256.as_deref().map(hash).transpose()?;
    let after_module = after.module_sha256.as_deref().map(hash).transpose()?;
    let expected = (before.subnet, &before.controllers, before_module);
    let actual = (after.subnet, &status.settings.controllers, module_sha256);
    if expected != actual
        || after.controllers != status.settings.controllers
        || after_module != module_sha256
        || matches!(status.status, RunState::Stopping)
    {
        return Err(invalid());
    }
    Ok(CapacityImportSampleRecord {
        binding: CapacityImportSourceBinding {
            canister_id: canister,
            subnet: after.subnet,
            controllers: status.settings.controllers,
            module_sha256,
            canister_version: status.version,
            stopped: matches!(status.status, RunState::Stopped),
            snapshots_size_bytes: status
                .memory_metrics
                .snapshots_size
                .0
                .try_into()
                .map_err(|_| invalid())?,
        },
        cycles: status.cycles.0.try_into().map_err(|_| invalid())?,
        reserved_cycles: status.reserved_cycles.0.try_into().map_err(|_| invalid())?,
    })
}
