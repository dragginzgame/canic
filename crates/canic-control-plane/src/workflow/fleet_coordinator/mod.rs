//! Module: workflow::fleet_coordinator
//!
//! Responsibility: orchestrate Coordinator genesis, Registry transitions, and root provisioning.
//! Does not own: stable encoding, canonical validation, root effects, or endpoint transport.
//! Boundary: lifecycle and endpoint APIs delegate here after transport authentication.

#[cfg(test)]
mod tests;

use crate::{
    dto::fleet_coordinator::FleetCoordinatorInitArgs,
    ops::fleet_coordinator::FleetCoordinatorOps,
    view::fleet_coordinator::{
        FleetComponentProvisioningRootAcceptanceCallView,
        FleetComponentProvisioningRootAcceptanceDisposition,
        FleetComponentProvisioningRootProvisionCallView,
        FleetComponentProvisioningRootProvisionDisposition,
    },
};
use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::InternalError,
        ops::ic::{IcOps, call::CallOps},
    },
    dto::{
        component_provisioning::{
            FleetComponentProvisioningAdvanceRequest, FleetComponentProvisioningPhase,
            FleetComponentProvisioningPrepareRequest, FleetComponentProvisioningStatusRequest,
            FleetComponentProvisioningStatusResponse, RootComponentProvisioningStatusResponse,
        },
        error::Error,
        fleet_registry::{
            FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
            FleetRegistryManifest, FleetRegistrySnapshotResponse, FleetRegistryVersion,
            FleetSubnetRootDeletionCompletionRequest, FleetSubnetRootDeletionExecutionRequest,
            FleetSubnetRootDeletionExecutionResponse,
            FleetSubnetRootDeletionReadinessIntentRequest,
            FleetSubnetRootDeletionReadinessIntentResponse,
            FleetSubnetRootDeletionReadinessRequest, FleetSubnetRootDeletionReadinessResponse,
            FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionStatusRequest,
            FleetSubnetRootDrainingPublicationRequest, FleetSubnetRootDrainingPublicationResponse,
            FleetSubnetRootJoinRequest, FleetSubnetRootJoinResponse,
            FleetSubnetRootRemovalPublicationRequest, FleetSubnetRootRemovalPublicationResponse,
            FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootSnapshotAcknowledgementRequest,
        },
    },
    protocol,
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

    pub(crate) fn prepare_component_provisioning(
        request: FleetComponentProvisioningPrepareRequest,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        FleetCoordinatorOps::prepare_component_provisioning(request, IcOps::now_nanos())
    }

    pub(crate) fn component_provisioning_status(
        request: FleetComponentProvisioningStatusRequest,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        FleetCoordinatorOps::component_provisioning_status(request)
    }

    pub(crate) async fn advance_component_provisioning(
        request: FleetComponentProvisioningAdvanceRequest,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let disposition = FleetCoordinatorOps::advance_component_provisioning_root_acceptance(
            request,
            IcOps::now_nanos(),
        )?;
        let acceptance_status = match disposition {
            FleetComponentProvisioningRootAcceptanceDisposition::Current(status) => status,
            FleetComponentProvisioningRootAcceptanceDisposition::Invoke(call)
            | FleetComponentProvisioningRootAcceptanceDisposition::Reconcile(call) => {
                let response = accept_root_component_provisioning(call).await?;
                return FleetCoordinatorOps::record_component_provisioning_root_acceptance(
                    request,
                    response,
                    IcOps::now_nanos(),
                );
            }
        };
        if acceptance_status.accepted_root_count != request.expected_accepted_root_count
            || !matches!(
                acceptance_status.phase,
                FleetComponentProvisioningPhase::RootsAccepted
                    | FleetComponentProvisioningPhase::ProvisioningRoots
                    | FleetComponentProvisioningPhase::ComponentsProvisioned
                    | FleetComponentProvisioningPhase::ServiceTopologyPublished
            )
        {
            return Ok(acceptance_status);
        }
        let disposition =
            FleetCoordinatorOps::advance_component_provisioning_root(&request, IcOps::now_nanos())?;
        let call = match disposition {
            FleetComponentProvisioningRootProvisionDisposition::Current(status) => {
                return Ok(*status);
            }
            FleetComponentProvisioningRootProvisionDisposition::Invoke(call)
            | FleetComponentProvisioningRootProvisionDisposition::Reconcile(call) => call,
            FleetComponentProvisioningRootProvisionDisposition::Publish => {
                return FleetCoordinatorOps::publish_component_provisioning_services(
                    &request,
                    IcOps::now_nanos(),
                );
            }
        };
        let response = advance_root_component_provisioning(call).await?;
        FleetCoordinatorOps::record_component_provisioning_root(
            &request,
            response,
            IcOps::now_nanos(),
        )
    }

    pub(crate) fn publish_root_draining(
        request: FleetSubnetRootDrainingPublicationRequest,
    ) -> Result<FleetSubnetRootDrainingPublicationResponse, InternalError> {
        FleetCoordinatorOps::publish_root_draining(request)
    }

    pub(crate) fn publish_root_removed(
        caller: Principal,
        request: FleetSubnetRootRemovalPublicationRequest,
    ) -> Result<FleetSubnetRootRemovalPublicationResponse, InternalError> {
        FleetCoordinatorOps::publish_root_removed(caller, request)
    }

    pub(crate) fn prepare_root_deletion_readiness(
        caller: Principal,
        coordinator: Principal,
        request: FleetSubnetRootDeletionReadinessIntentRequest,
    ) -> Result<FleetSubnetRootDeletionReadinessIntentResponse, InternalError> {
        FleetCoordinatorOps::prepare_root_deletion_readiness(
            caller,
            coordinator,
            request,
            IcOps::now_nanos(),
        )
    }

    pub(crate) fn record_root_deletion_readiness(
        caller: Principal,
        coordinator: Principal,
        request: FleetSubnetRootDeletionReadinessRequest,
    ) -> Result<FleetSubnetRootDeletionReadinessResponse, InternalError> {
        FleetCoordinatorOps::record_root_deletion_readiness(
            caller,
            coordinator,
            request,
            IcOps::now_nanos(),
        )
    }

    pub(crate) fn begin_root_deletion_execution(
        executor: Principal,
        coordinator: Principal,
        request: FleetSubnetRootDeletionExecutionRequest,
    ) -> Result<FleetSubnetRootDeletionExecutionResponse, InternalError> {
        FleetCoordinatorOps::begin_root_deletion_execution(
            executor,
            coordinator,
            request,
            IcOps::now_nanos(),
        )
    }

    pub(crate) fn root_deletion_execution_status(
        request: FleetSubnetRootDeletionStatusRequest,
    ) -> Result<FleetSubnetRootDeletionExecutionResponse, InternalError> {
        FleetCoordinatorOps::root_deletion_execution_status(request)
    }

    pub(crate) fn complete_root_deletion(
        executor: Principal,
        coordinator: Principal,
        request: FleetSubnetRootDeletionCompletionRequest,
    ) -> Result<FleetSubnetRootDeletionResponse, InternalError> {
        FleetCoordinatorOps::complete_root_deletion(
            executor,
            coordinator,
            request,
            IcOps::now_nanos(),
        )
    }

    pub(crate) fn root_deletion_status(
        request: FleetSubnetRootDeletionStatusRequest,
    ) -> Result<FleetSubnetRootDeletionResponse, InternalError> {
        FleetCoordinatorOps::root_deletion_status(request)
    }

    pub(crate) fn version() -> Result<FleetRegistryVersion, InternalError> {
        FleetCoordinatorOps::version()
    }
}

async fn accept_root_component_provisioning(
    call: FleetComponentProvisioningRootAcceptanceCallView,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let result = CallOps::unbounded_wait(
        call.fleet_subnet_root,
        protocol::CANIC_ROOT_COMPONENT_PROVISIONING_ACCEPT,
    )
    .with_arg(call.request)?
    .execute()
    .await?;
    let response: Result<RootComponentProvisioningStatusResponse, Error> = result.candid()?;
    response.map_err(InternalError::public)
}

async fn advance_root_component_provisioning(
    call: FleetComponentProvisioningRootProvisionCallView,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let result = CallOps::unbounded_wait(
        call.fleet_subnet_root,
        protocol::CANIC_ROOT_COMPONENT_PROVISIONING_ADVANCE,
    )
    .with_arg(call.request)?
    .execute()
    .await?;
    let response: Result<RootComponentProvisioningStatusResponse, Error> = result.candid()?;
    response.map_err(InternalError::public)
}
