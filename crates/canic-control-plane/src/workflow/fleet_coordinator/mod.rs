//! Module: workflow::fleet_coordinator
//!
//! Responsibility: orchestrate fresh Coordinator genesis and read-only Registry projections.
//! Does not own: stable encoding, canonical Registry validation, or endpoint transport.
//! Boundary: lifecycle and endpoint APIs delegate here after transport authentication.

#[cfg(test)]
mod tests;

use crate::{
    dto::fleet_coordinator::FleetCoordinatorInitArgs, ops::fleet_coordinator::FleetCoordinatorOps,
};
use candid::Principal;
use canic_core::{
    control_plane_support::error::InternalError,
    dto::fleet_registry::{
        FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
        FleetRegistryManifest, FleetRegistrySnapshotResponse, FleetRegistryVersion,
        FleetSubnetRootJoinRequest, FleetSubnetRootJoinResponse,
        FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootSnapshotAcknowledgementRequest,
    },
};

///
/// FleetCoordinatorWorkflow
///
/// Coordinator lifecycle and query orchestration.
///

pub struct FleetCoordinatorWorkflow;

impl FleetCoordinatorWorkflow {
    pub(crate) fn initialize(
        args: FleetCoordinatorInitArgs,
        caller: Principal,
        caller_is_controller: bool,
        coordinator_canister: Principal,
    ) -> Result<(), InternalError> {
        if !caller_is_controller {
            return Err(InternalError::forbidden(format!(
                "Fleet Coordinator init caller {caller} is not a controller"
            )));
        }
        let record = FleetCoordinatorOps::compile_genesis(args, coordinator_canister)?;
        FleetCoordinatorOps::commit_genesis(record)?;
        Ok(())
    }

    pub(crate) fn registry() -> Result<FleetRegistry, InternalError> {
        FleetCoordinatorOps::registry()
    }

    pub(crate) fn join_root(
        request: FleetSubnetRootJoinRequest,
    ) -> Result<FleetSubnetRootJoinResponse, InternalError> {
        FleetCoordinatorOps::join_root(request)
    }

    pub(crate) fn manifest() -> Result<FleetRegistryManifest, InternalError> {
        FleetCoordinatorOps::manifest()
    }

    pub(crate) fn snapshot_for_root(
        caller: Principal,
    ) -> Result<FleetRegistrySnapshotResponse, InternalError> {
        FleetCoordinatorOps::snapshot_for_root(caller)
    }

    pub(crate) fn acknowledge_root_snapshot(
        caller: Principal,
        request: FleetSubnetRootSnapshotAcknowledgementRequest,
    ) -> Result<FleetSubnetRootSnapshotAcknowledgement, InternalError> {
        FleetCoordinatorOps::acknowledge_root_snapshot(caller, request)
    }

    pub(crate) fn root_snapshot_acknowledgements()
    -> Result<Vec<FleetSubnetRootSnapshotAcknowledgement>, InternalError> {
        FleetCoordinatorOps::root_snapshot_acknowledgements()
    }

    pub(crate) fn activate_registry(
        request: FleetRegistryActivationRequest,
    ) -> Result<FleetRegistryActivationResponse, InternalError> {
        FleetCoordinatorOps::activate_registry(request)
    }

    pub(crate) fn version() -> Result<FleetRegistryVersion, InternalError> {
        FleetCoordinatorOps::version()
    }
}
