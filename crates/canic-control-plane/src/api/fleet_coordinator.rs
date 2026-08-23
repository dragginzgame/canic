//! Module: api::fleet_coordinator
//!
//! Responsibility: adapt Coordinator lifecycle and endpoint calls to workflow.
//! Does not own: authorization policy, stable state, or Registry validation.
//! Boundary: the dedicated facade macro is the only canister export authority.

use crate::{
    dto::fleet_coordinator::{
        CoordinatorCommand, CoordinatorCommandResponse, CoordinatorOperationStatusResponse,
        FleetCoordinatorInitArgs,
    },
    workflow::fleet_coordinator::FleetCoordinatorWorkflow,
};
use canic_core::{
    api::runtime::MemoryRuntimeApi,
    control_plane_support::ops::runtime::env::EnvOps,
    dto::{
        component_provisioning::{
            FleetComponentProvisioningAdvanceRequest, FleetComponentProvisioningPrepareRequest,
            FleetComponentProvisioningStatusRequest, FleetComponentProvisioningStatusResponse,
        },
        error::Error,
        fleet_funding::{
            FleetFundingPolicyRotationApplyRequest, FleetFundingPolicyRotationBeginRequest,
            FleetFundingPolicyRotationStageRootRequest, FleetRootFundingRequest,
            FleetRootFundingResponse,
        },
        fleet_registry::{
            FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
            FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootDeletionCompletionRequest,
            FleetSubnetRootDeletionExecutionRequest, FleetSubnetRootDeletionExecutionResponse,
            FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionStatusRequest,
            FleetSubnetRootDrainingReservationRequest, FleetSubnetRootDrainingReservationResponse,
            FleetSubnetRootDrainingReservationStatusRequest, FleetSubnetRootJoinRequest,
            FleetSubnetRootJoinResponse, FleetSubnetRootSnapshotAcknowledgement,
            FleetSubnetRootSnapshotAcknowledgementRequest,
        },
        state::{SetCyclesFundingRequest, SetStateResponse},
    },
};
use ic_cdk::api::{canister_self, is_controller, msg_caller};

///
/// FleetCoordinatorApi
///
/// Public adapter used by the built-in Fleet Coordinator canister exports.
///

pub struct FleetCoordinatorApi;

impl FleetCoordinatorApi {
    /// Authorize an exact joining Root before command workflow dispatch.
    pub fn authorize_calling_root_snapshot() -> Result<(), Error> {
        FleetCoordinatorWorkflow::authorize_root_snapshot_caller(msg_caller()).map_err(Into::into)
    }

    /// Authorize the exact registered Root before any Coordinator treasury observation.
    pub fn authorize_calling_root_funding() -> Result<(), Error> {
        FleetCoordinatorWorkflow::authorize_root_funding_caller(msg_caller()).map_err(Into::into)
    }

    /// Authorize a controller or exact snapshot Root before status workflow dispatch.
    pub fn authorize_calling_registry_status() -> Result<(), Error> {
        let caller = msg_caller();
        FleetCoordinatorWorkflow::authorize_registry_caller(caller, is_controller(&caller))
            .map_err(Into::into)
    }

    /// Restore memory invariants and synchronously commit fresh genesis during install.
    pub fn init(args: FleetCoordinatorInitArgs) {
        canic_core::api::timer::TimerApi::initialize_shared_runtime_required();
        MemoryRuntimeApi::bootstrap_registry()
            .unwrap_or_else(|error| ic_cdk::trap(format!("memory bootstrap failed: {error}")));
        EnvOps::initialize_fleet_coordinator_runtime();
        canic_core::api::authority_restore::AuthorityRestoreApi::initialize(canister_self())
            .unwrap_or_else(|error| {
                ic_cdk::trap(format!("authority restore fence init failed: {error}"))
            });
        let caller = msg_caller();
        FleetCoordinatorWorkflow::initialize(args, caller, is_controller(&caller), canister_self())
            .unwrap_or_else(|error| {
                ic_cdk::trap(format!("Fleet Coordinator init failed: {error}"))
            });
        canic_core::api::ready::ReadyApi::mark_ready();
    }

    pub fn operation_status(
        operation_id: [u8; 32],
    ) -> Result<CoordinatorOperationStatusResponse, Error> {
        let caller = msg_caller();
        FleetCoordinatorWorkflow::operation_status_for_caller(
            operation_id,
            caller,
            is_controller(&caller),
        )
        .map_err(Into::into)
    }

    pub fn root_funding_status()
    -> Result<crate::dto::fleet_coordinator::CoordinatorFundingStatusResponse, Error> {
        FleetCoordinatorWorkflow::root_funding_status().map_err(Into::into)
    }

    /// Dispatch one closed Coordinator command and preserve its exact response correlation.
    pub async fn command(command: CoordinatorCommand) -> Result<CoordinatorCommandResponse, Error> {
        match command {
            CoordinatorCommand::AcknowledgeRootSnapshot(request) => {
                Self::acknowledge_calling_root_snapshot(request)
                    .map(CoordinatorCommandResponse::AcknowledgeRootSnapshot)
            }
            CoordinatorCommand::ActivateRegistry(request) => {
                Self::activate_registry(request).map(CoordinatorCommandResponse::ActivateRegistry)
            }
            CoordinatorCommand::ApplyFundingPolicyRotation(request) => {
                Self::apply_funding_policy_rotation(request)
                    .map(CoordinatorCommandResponse::OperationAccepted)
            }
            CoordinatorCommand::BeginFundingPolicyRotation(request) => {
                Self::begin_funding_policy_rotation(request)
                    .map(CoordinatorCommandResponse::OperationAccepted)
            }
            CoordinatorCommand::CompleteRootDeletion(request) => {
                Self::complete_root_deletion(request)
                    .map(CoordinatorCommandResponse::CompleteRootDeletion)
            }
            CoordinatorCommand::JoinRoot(request) => {
                Self::join_root(request).map(CoordinatorCommandResponse::JoinRoot)
            }
            CoordinatorCommand::PrepareAuthoritySnapshot(request) => {
                FleetCoordinatorWorkflow::require_root_funding_snapshot_resumable()
                    .map_err(Error::from)?;
                canic_core::api::authority_restore::AuthorityRestoreApi::prepare_coordinator_snapshot(
                    request,
                )
                .await
                .map(CoordinatorCommandResponse::PrepareAuthoritySnapshot)
            }
            CoordinatorCommand::PrepareRootDeletionExecution(request) => {
                Self::begin_root_deletion_execution(request)
                    .map(CoordinatorCommandResponse::PrepareRootDeletionExecution)
            }
            CoordinatorCommand::ProvisionComponents(request) => {
                FleetCoordinatorWorkflow::accept_component_provisioning(request)
                    .map(CoordinatorCommandResponse::OperationAccepted)
                    .map_err(Into::into)
            }
            CoordinatorCommand::RemoveRoot(request) => {
                FleetCoordinatorWorkflow::accept_root_removal(request)
                    .await
                    .map(CoordinatorCommandResponse::OperationAccepted)
                    .map_err(Into::into)
            }
            CoordinatorCommand::RequestRootFunding(request) => {
                Self::request_calling_root_funding(request)
                    .await
                    .map(CoordinatorCommandResponse::RequestRootFunding)
            }
            CoordinatorCommand::ResumeAuthoritySnapshot(request) => {
                canic_core::api::authority_restore::AuthorityRestoreApi::resume_coordinator_snapshot(
                    request,
                )
                .await
                .map(CoordinatorCommandResponse::ResumeAuthoritySnapshot)
            }
            CoordinatorCommand::SetRootFunding(request) => Self::set_root_funding(request)
                .map(CoordinatorCommandResponse::SetRootFunding),
            CoordinatorCommand::StageFundingPolicyRotationRoot(request) => {
                Self::stage_funding_policy_rotation_root(request)
                    .map(CoordinatorCommandResponse::OperationAccepted)
            }
        }
    }

    pub fn begin_funding_policy_rotation(
        request: FleetFundingPolicyRotationBeginRequest,
    ) -> Result<canic_core::dto::role::OperationReceipt, Error> {
        FleetCoordinatorWorkflow::begin_funding_policy_rotation(request).map_err(Into::into)
    }

    pub fn stage_funding_policy_rotation_root(
        request: FleetFundingPolicyRotationStageRootRequest,
    ) -> Result<canic_core::dto::role::OperationReceipt, Error> {
        FleetCoordinatorWorkflow::stage_funding_policy_rotation_root(request).map_err(Into::into)
    }

    pub fn apply_funding_policy_rotation(
        request: FleetFundingPolicyRotationApplyRequest,
    ) -> Result<canic_core::dto::role::OperationReceipt, Error> {
        FleetCoordinatorWorkflow::apply_funding_policy_rotation(request).map_err(Into::into)
    }

    pub async fn request_calling_root_funding(
        request: FleetRootFundingRequest,
    ) -> Result<FleetRootFundingResponse, Error> {
        FleetCoordinatorWorkflow::request_root_funding(msg_caller(), request)
            .await
            .map_err(Into::into)
    }

    pub fn set_root_funding(
        request: SetCyclesFundingRequest,
    ) -> Result<SetStateResponse<bool>, Error> {
        FleetCoordinatorWorkflow::set_root_funding_enabled(request.enabled).map_err(Into::into)
    }

    pub fn registry() -> Result<FleetRegistry, Error> {
        FleetCoordinatorWorkflow::registry().map_err(Into::into)
    }

    pub fn join_root(
        request: FleetSubnetRootJoinRequest,
    ) -> Result<FleetSubnetRootJoinResponse, Error> {
        FleetCoordinatorWorkflow::join_root(request).map_err(Into::into)
    }

    pub fn manifest() -> Result<FleetRegistryManifest, Error> {
        FleetCoordinatorWorkflow::manifest().map_err(Into::into)
    }

    pub fn registry_for_calling_status() -> Result<FleetRegistry, Error> {
        let caller = msg_caller();
        FleetCoordinatorWorkflow::registry_for_caller(caller, is_controller(&caller))
            .map_err(Into::into)
    }

    pub fn acknowledge_calling_root_snapshot(
        request: FleetSubnetRootSnapshotAcknowledgementRequest,
    ) -> Result<FleetSubnetRootSnapshotAcknowledgement, Error> {
        FleetCoordinatorWorkflow::acknowledge_root_snapshot(msg_caller(), request)
            .map_err(Into::into)
    }

    pub fn root_snapshot_acknowledgements()
    -> Result<Vec<FleetSubnetRootSnapshotAcknowledgement>, Error> {
        FleetCoordinatorWorkflow::root_snapshot_acknowledgements().map_err(Into::into)
    }

    pub fn activate_registry(
        request: FleetRegistryActivationRequest,
    ) -> Result<FleetRegistryActivationResponse, Error> {
        FleetCoordinatorWorkflow::activate_registry(request).map_err(Into::into)
    }

    pub fn prepare_component_provisioning(
        request: FleetComponentProvisioningPrepareRequest,
    ) -> Result<FleetComponentProvisioningStatusResponse, Error> {
        FleetCoordinatorWorkflow::prepare_component_provisioning(request).map_err(Into::into)
    }

    pub fn component_provisioning_status(
        request: FleetComponentProvisioningStatusRequest,
    ) -> Result<FleetComponentProvisioningStatusResponse, Error> {
        FleetCoordinatorWorkflow::component_provisioning_status(request).map_err(Into::into)
    }

    pub async fn advance_component_provisioning(
        request: FleetComponentProvisioningAdvanceRequest,
    ) -> Result<FleetComponentProvisioningStatusResponse, Error> {
        FleetCoordinatorWorkflow::advance_component_provisioning(&request)
            .await
            .map_err(Into::into)
    }

    pub fn prepare_root_draining_reservation(
        request: FleetSubnetRootDrainingReservationRequest,
    ) -> Result<FleetSubnetRootDrainingReservationResponse, Error> {
        FleetCoordinatorWorkflow::prepare_root_draining_reservation(request).map_err(Into::into)
    }

    pub fn root_draining_reservation_status(
        request: FleetSubnetRootDrainingReservationStatusRequest,
    ) -> Result<FleetSubnetRootDrainingReservationResponse, Error> {
        let caller = msg_caller();
        FleetCoordinatorWorkflow::root_draining_reservation_status(
            caller,
            is_controller(&caller),
            request,
        )
        .map_err(Into::into)
    }

    pub fn begin_root_deletion_execution(
        request: FleetSubnetRootDeletionExecutionRequest,
    ) -> Result<FleetSubnetRootDeletionExecutionResponse, Error> {
        FleetCoordinatorWorkflow::begin_root_deletion_execution(
            msg_caller(),
            canister_self(),
            request,
        )
        .map_err(Into::into)
    }

    pub fn root_deletion_execution_status(
        request: FleetSubnetRootDeletionStatusRequest,
    ) -> Result<FleetSubnetRootDeletionExecutionResponse, Error> {
        FleetCoordinatorWorkflow::root_deletion_execution_status(request).map_err(Into::into)
    }

    pub fn complete_root_deletion(
        request: FleetSubnetRootDeletionCompletionRequest,
    ) -> Result<FleetSubnetRootDeletionResponse, Error> {
        FleetCoordinatorWorkflow::complete_root_deletion(msg_caller(), canister_self(), request)
            .map_err(Into::into)
    }

    pub fn root_deletion_status(
        request: FleetSubnetRootDeletionStatusRequest,
    ) -> Result<FleetSubnetRootDeletionResponse, Error> {
        FleetCoordinatorWorkflow::root_deletion_status(request).map_err(Into::into)
    }

    pub fn version() -> Result<FleetRegistryVersion, Error> {
        FleetCoordinatorWorkflow::version().map_err(Into::into)
    }
}
