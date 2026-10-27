//! Module: api::fleet_coordinator
//!
//! Responsibility: adapt Coordinator lifecycle and endpoint calls to workflow.
//! Does not own: authorization policy, stable state, or Registry validation.
//! Boundary: the dedicated facade macro is the only canister export authority.

use crate::{
    installation::FleetCoordinatorInitArgs, workflow::fleet_coordinator::FleetCoordinatorWorkflow,
};
use canic_contracts::dto::{
    component_provisioning::{
        FleetComponentProvisioningAdvanceRequest, FleetComponentProvisioningPrepareRequest,
        FleetComponentProvisioningStatusRequest, FleetComponentProvisioningStatusResponse,
    },
    error::Error,
    fleet_admission::{
        FleetAdmissionMutationRequest, FleetAdmissionMutationResponse, FleetAdmissionStatusRequest,
        FleetAdmissionStatusResponse,
    },
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
};
use canic_core::control_plane_support::ops::{ic::IcOps, runtime::env::EnvOps};
use ic_platform::cdk::api::{canister_self, is_controller, msg_caller};

///
/// FleetCoordinatorApi
///
/// Public adapter used by the built-in Fleet Coordinator canister exports.
///

pub struct FleetCoordinatorApi;

/// Existing Registry authority exposed as one endpoint-wide access predicate.
pub struct FleetCoordinatorRegistryCallerPredicate;

impl canic_core::access::expr::AccessCheck for FleetCoordinatorRegistryCallerPredicate {
    fn name(&self) -> &'static str {
        "fleet_coordinator::registry_caller"
    }

    fn eval(
        &self,
        context: &canic_core::access::expr::AccessContext,
    ) -> Result<(), canic_core::access::AccessError> {
        let caller = context.transport_caller();
        FleetCoordinatorWorkflow::authorize_registry_caller(caller, is_controller(&caller.into()))
            .map_err(canic_core::access::AccessError::Internal)
    }
}

impl FleetCoordinatorApi {
    /// Authorize an exact joining Root before command workflow dispatch.
    pub fn authorize_calling_root_snapshot() -> Result<(), Error> {
        FleetCoordinatorWorkflow::authorize_root_snapshot_caller(msg_caller().into())
            .map_err(Into::into)
    }

    /// Authorize the exact registered Root before any Coordinator treasury observation.
    pub fn authorize_calling_root_funding() -> Result<(), Error> {
        FleetCoordinatorWorkflow::authorize_root_funding_caller(msg_caller().into())
            .map_err(Into::into)
    }

    /// Restore memory invariants and synchronously commit fresh genesis during install.
    pub fn init(args: FleetCoordinatorInitArgs) {
        canic_core::api::timer::TimerApi::initialize_shared_runtime_required();
        canic_core::memory::bootstrap(crate::memory::framework())
            .unwrap_or_else(|error| IcOps::trap(format!("memory bootstrap failed: {error}")));
        EnvOps::initialize_fleet_coordinator_runtime();
        canic_core::api::authority_restore::AuthorityRestoreApi::initialize(canister_self().into())
            .unwrap_or_else(|error| {
                IcOps::trap(format!("authority restore fence init failed: {error}"))
            });
        let caller = msg_caller().into();
        FleetCoordinatorWorkflow::initialize(
            args,
            caller,
            is_controller(&caller.into()),
            canister_self().into(),
        )
        .unwrap_or_else(|error| IcOps::trap(format!("Fleet Coordinator init failed: {error}")));
        canic_core::api::ready::ReadyApi::mark_ready();
    }

    /// Observe one retained operation under its existing caller authority.
    pub fn admission_operation_status(
        operation_id: [u8; 32],
    ) -> Result<canic_contracts::dto::fleet_admission::FleetAdmissionOperationStatusResponse, Error>
    {
        let caller = msg_caller().into();
        FleetCoordinatorWorkflow::admission_operation_status(
            operation_id,
            caller,
            is_controller(&caller.into()),
        )
        .map_err(Into::into)
    }

    /// Observe one retained operation under its existing caller authority.
    pub fn funding_rotation_status(
        operation_id: [u8; 32],
    ) -> Result<
        canic_contracts::dto::fleet_coordinator::FleetFundingPolicyRotationStatusResponse,
        Error,
    > {
        let caller = msg_caller().into();
        FleetCoordinatorWorkflow::funding_rotation_status(
            operation_id,
            caller,
            is_controller(&caller.into()),
        )
        .map_err(Into::into)
    }

    /// Observe one retained operation under its existing caller authority.
    pub fn retirement_operation_status(
        operation_id: [u8; 32],
    ) -> Result<canic_contracts::dto::fleet_registry::FleetRetirementStatus, Error> {
        let caller = msg_caller().into();
        FleetCoordinatorWorkflow::retirement_operation_status(
            operation_id,
            caller,
            is_controller(&caller.into()),
        )
        .map_err(Into::into)
    }

    /// Observe retained Root-removal authority without advancing its effects.
    pub fn root_removal_operation_status(
        operation_id: [u8; 32],
    ) -> Result<canic_contracts::dto::fleet_coordinator::CoordinatorRootRemovalOperationStatus, Error>
    {
        let caller = msg_caller().into();
        FleetCoordinatorWorkflow::root_removal_operation_status(
            operation_id,
            caller,
            is_controller(&caller.into()),
        )
        .map_err(Into::into)
    }

    pub fn provisioning_operation_status(
        operation_id: [u8; 32],
    ) -> Result<
        canic_contracts::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
        Error,
    > {
        let caller = msg_caller().into();
        FleetCoordinatorWorkflow::provisioning_operation_status_for_caller(
            operation_id,
            caller,
            is_controller(&caller.into()),
        )
        .map_err(Into::into)
    }

    pub fn root_funding_status()
    -> Result<crate::dto::fleet_coordinator::CoordinatorFundingStatusResponse, Error> {
        FleetCoordinatorWorkflow::root_funding_status().map_err(Into::into)
    }

    /// Retain the reviewed plan before the existing autonomous workflow performs effects.
    pub fn provision_components(
        request: crate::dto::component_provisioning::FleetComponentProvisioningPrepareRequest,
    ) -> Result<crate::dto::role::OperationReceipt, Error> {
        FleetCoordinatorWorkflow::accept_component_provisioning(request).map_err(Into::into)
    }

    /// Accept the exact controller-reviewed removal without changing its durable reservation.
    pub async fn remove_root(
        request: crate::dto::fleet_registry::FleetSubnetRootDrainingReservationRequest,
    ) -> Result<crate::dto::role::OperationReceipt, Error> {
        FleetCoordinatorWorkflow::accept_root_removal(request)
            .await
            .map_err(Into::into)
    }

    /// Reconcile the original terminal Ledger transfer as the authenticated IC caller.
    pub async fn retire(
        request: crate::dto::fleet_registry::FleetRetirementRequest,
    ) -> Result<crate::dto::fleet_registry::FleetRetirementStatus, Error> {
        crate::workflow::fleet_coordinator::retirement::retire(msg_caller().into(), request)
            .await
            .map_err(Into::into)
    }

    pub async fn prepare_authority_snapshot(
        request: canic_contracts::dto::authority_restore::AuthoritySnapshotRequest,
    ) -> Result<canic_contracts::dto::authority_restore::AuthorityRestoreFenceStatusResponse, Error>
    {
        FleetCoordinatorWorkflow::require_root_funding_snapshot_resumable().map_err(Error::from)?;
        canic_core::api::authority_restore::AuthorityRestoreApi::prepare_coordinator_snapshot(
            request,
        )
        .await
    }

    pub async fn resume_authority_snapshot(
        request: canic_contracts::dto::authority_restore::AuthoritySnapshotRequest,
    ) -> Result<canic_contracts::dto::authority_restore::AuthorityRestoreFenceStatusResponse, Error>
    {
        canic_core::api::authority_restore::AuthorityRestoreApi::resume_coordinator_snapshot(
            request,
        )
        .await
    }

    pub fn begin_funding_policy_rotation(
        request: FleetFundingPolicyRotationBeginRequest,
    ) -> Result<canic_contracts::dto::role::OperationReceipt, Error> {
        FleetCoordinatorWorkflow::begin_funding_policy_rotation(request).map_err(Into::into)
    }

    pub fn mutate_admission(
        request: FleetAdmissionMutationRequest,
    ) -> Result<FleetAdmissionMutationResponse, Error> {
        FleetCoordinatorWorkflow::mutate_admission(request).map_err(Into::into)
    }

    pub fn admission_status(
        request: FleetAdmissionStatusRequest,
    ) -> Result<FleetAdmissionStatusResponse, Error> {
        FleetCoordinatorWorkflow::admission_status(request).map_err(Into::into)
    }

    pub fn stage_funding_policy_rotation_root(
        request: FleetFundingPolicyRotationStageRootRequest,
    ) -> Result<canic_contracts::dto::role::OperationReceipt, Error> {
        FleetCoordinatorWorkflow::stage_funding_policy_rotation_root(request).map_err(Into::into)
    }

    pub fn apply_funding_policy_rotation(
        request: FleetFundingPolicyRotationApplyRequest,
    ) -> Result<canic_contracts::dto::role::OperationReceipt, Error> {
        FleetCoordinatorWorkflow::apply_funding_policy_rotation(request).map_err(Into::into)
    }

    pub async fn request_calling_root_funding(
        request: FleetRootFundingRequest,
    ) -> Result<FleetRootFundingResponse, Error> {
        FleetCoordinatorWorkflow::request_root_funding(msg_caller().into(), request)
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
        let caller = msg_caller().into();
        FleetCoordinatorWorkflow::registry_for_caller(caller, is_controller(&caller.into()))
            .map_err(Into::into)
    }

    pub fn acknowledge_calling_root_snapshot(
        request: FleetSubnetRootSnapshotAcknowledgementRequest,
    ) -> Result<FleetSubnetRootSnapshotAcknowledgement, Error> {
        FleetCoordinatorWorkflow::acknowledge_root_snapshot(msg_caller().into(), request)
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
        let caller = msg_caller().into();
        FleetCoordinatorWorkflow::root_draining_reservation_status(
            caller,
            is_controller(&caller.into()),
            request,
        )
        .map_err(Into::into)
    }

    pub fn begin_root_deletion_execution(
        request: FleetSubnetRootDeletionExecutionRequest,
    ) -> Result<FleetSubnetRootDeletionExecutionResponse, Error> {
        FleetCoordinatorWorkflow::begin_root_deletion_execution(
            msg_caller().into(),
            canister_self().into(),
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
        FleetCoordinatorWorkflow::complete_root_deletion(
            msg_caller().into(),
            canister_self().into(),
            request,
        )
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
