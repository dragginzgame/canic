//! Typed inter-Canister client for the Coordinator's role-owned command surface.
//!
//! This module carries only the exact Root-owned command fragments. It does not
//! own Coordinator state, operation advancement, fallback decoding, or protocol
//! negotiation.

use crate::dto::fleet_coordinator::CoordinatorRootRemovalOperationStatus;
use candid::{CandidType, Principal};
use canic_core::{
    control_plane_support::{error::InternalError, ops::ic::call::CallOps},
    dto::{
        error::Error,
        fleet_funding::{FleetRootFundingRequest, FleetRootFundingResponse},
        fleet_registry::{
            FleetRegistry, FleetSubnetRootSnapshotAcknowledgement,
            FleetSubnetRootSnapshotAcknowledgementRequest,
        },
        role::OperationStatusRequest,
    },
    protocol,
};
use serde::Deserialize;

#[derive(CandidType)]
enum CoordinatorCommandFragment {
    AcknowledgeRootSnapshot(FleetSubnetRootSnapshotAcknowledgementRequest),
    RequestRootFunding(FleetRootFundingRequest),
}

#[derive(CandidType, Deserialize)]
enum CoordinatorCommandResponseFragment {
    AcknowledgeRootSnapshot(FleetSubnetRootSnapshotAcknowledgement),
    RequestRootFunding(FleetRootFundingResponse),
}

#[derive(CandidType)]
enum CoordinatorStatusRequestFragment {
    Operation(OperationStatusRequest),
    Registry,
}

#[derive(CandidType, Deserialize)]
enum CoordinatorStatusResponseFragment {
    Operation(Box<CoordinatorOperationStatusFragment>),
    Registry(Box<FleetRegistry>),
}

#[derive(CandidType, Deserialize)]
enum CoordinatorOperationStatusFragment {
    RootRemoval(CoordinatorRootRemovalOperationStatus),
}

pub(super) async fn registry(coordinator: Principal) -> Result<FleetRegistry, InternalError> {
    let call = CallOps::unbounded_wait(coordinator, protocol::CANIC_STATUS)
        .with_arg(CoordinatorStatusRequestFragment::Registry)?
        .execute()
        .await?;
    let result: Result<CoordinatorStatusResponseFragment, Error> = call.candid()?;
    match result.map_err(InternalError::observed_public)? {
        CoordinatorStatusResponseFragment::Registry(registry) => Ok(*registry),
        CoordinatorStatusResponseFragment::Operation(_) => Err(InternalError::conflict()),
    }
}

pub(super) async fn root_removal_status(
    coordinator: Principal,
    operation_id: [u8; 32],
) -> Result<CoordinatorRootRemovalOperationStatus, InternalError> {
    let call = CallOps::unbounded_wait(coordinator, protocol::CANIC_STATUS)
        .with_arg(CoordinatorStatusRequestFragment::Operation(
            OperationStatusRequest { operation_id },
        ))?
        .execute()
        .await?;
    let result: Result<CoordinatorStatusResponseFragment, Error> = call.candid()?;
    match result.map_err(InternalError::observed_public)? {
        CoordinatorStatusResponseFragment::Operation(status) => match *status {
            CoordinatorOperationStatusFragment::RootRemoval(status)
                if status.operation_id == operation_id =>
            {
                Ok(status)
            }
            CoordinatorOperationStatusFragment::RootRemoval(_) => Err(InternalError::conflict()),
        },
        CoordinatorStatusResponseFragment::Registry(_) => Err(InternalError::conflict()),
    }
}

pub(super) async fn acknowledge_root_snapshot(
    coordinator: Principal,
    request: FleetSubnetRootSnapshotAcknowledgementRequest,
) -> Result<FleetSubnetRootSnapshotAcknowledgement, InternalError> {
    match call(
        coordinator,
        CoordinatorCommandFragment::AcknowledgeRootSnapshot(request),
    )
    .await?
    {
        CoordinatorCommandResponseFragment::AcknowledgeRootSnapshot(response) => Ok(response),
        CoordinatorCommandResponseFragment::RequestRootFunding(_) => Err(InternalError::conflict()),
    }
}

pub(super) async fn request_root_funding(
    coordinator: Principal,
    request: FleetRootFundingRequest,
) -> Result<FleetRootFundingResponse, InternalError> {
    let call = CallOps::bounded_wait(coordinator, protocol::CANIC_COORDINATOR_COMMAND)
        .with_arg(CoordinatorCommandFragment::RequestRootFunding(request))?
        .execute()
        .await?;
    let result: Result<CoordinatorCommandResponseFragment, Error> = call.candid()?;
    match result.map_err(InternalError::observed_public)? {
        CoordinatorCommandResponseFragment::RequestRootFunding(response) => Ok(response),
        CoordinatorCommandResponseFragment::AcknowledgeRootSnapshot(_) => {
            Err(InternalError::conflict())
        }
    }
}

async fn call(
    coordinator: Principal,
    command: CoordinatorCommandFragment,
) -> Result<CoordinatorCommandResponseFragment, InternalError> {
    let call = CallOps::unbounded_wait(coordinator, protocol::CANIC_COORDINATOR_COMMAND)
        .with_arg(command)?
        .execute()
        .await?;
    let result: Result<CoordinatorCommandResponseFragment, Error> = call.candid()?;
    result.map_err(InternalError::observed_public)
}
