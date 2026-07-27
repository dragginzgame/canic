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
        FleetRegistry, FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootJoinRequest,
        FleetSubnetRootJoinResponse,
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

    pub(crate) fn version() -> Result<FleetRegistryVersion, InternalError> {
        FleetCoordinatorOps::version()
    }
}
