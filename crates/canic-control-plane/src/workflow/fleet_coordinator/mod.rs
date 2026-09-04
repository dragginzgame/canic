//! Module: workflow::fleet_coordinator
//!
//! Responsibility: orchestrate Coordinator genesis, Registry transitions, and root provisioning.
//! Does not own: stable encoding, canonical validation, root effects, or endpoint transport.
//! Boundary: lifecycle and endpoint APIs delegate here after transport authentication.

#[cfg(test)]
mod tests;

use crate::{
    dto::{
        fleet_coordinator::{
            CoordinatorFundingStatusResponse, CoordinatorOperationStatusResponse,
            FleetCoordinatorInitArgs, FleetFundingPolicyRotationStatusPhase,
        },
        root::RootRemovalOperationStatus,
    },
    ops::{
        fleet_admission::{FleetAdmissionCoordinatorStep, FleetAdmissionOps},
        fleet_coordinator::FleetCoordinatorOps,
    },
    view::fleet_coordinator::{
        FleetComponentDirectoryConfirmationCallView,
        FleetComponentDirectoryConfirmationDisposition,
        FleetComponentProvisioningRootAcceptanceCallView,
        FleetComponentProvisioningRootAcceptanceDisposition,
        FleetComponentProvisioningRootProvisionCallView,
        FleetComponentProvisioningRootProvisionDisposition,
        FleetComponentRuntimeActivationCallView, FleetComponentRuntimeActivationDisposition,
        FleetFundingPolicyRotationStep, FleetRootFundingCallView, FleetRootFundingDisposition,
    },
};
use candid::{CandidType, Principal};
use canic_core::{
    api::timer::TimerApi,
    cdk::utils::hash::hex_bytes,
    control_plane_support::{
        error::InternalError,
        ops::ic::{IcOps, call::CallOps},
    },
    dto::{
        component_provisioning::{
            FleetComponentProvisioningAdvanceRequest, FleetComponentProvisioningOperation,
            FleetComponentProvisioningPhase, FleetComponentProvisioningPrepareRequest,
            FleetComponentProvisioningRootProgress, FleetComponentProvisioningStatusRequest,
            FleetComponentProvisioningStatusResponse, RootComponentDirectorySynchronizationRequest,
            RootComponentDirectorySynchronizationResponse,
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningStatusResponse,
        },
        error::Error,
        fleet_admission::{
            FleetAdmissionActivateRootRequest, FleetAdmissionMutationOutcome,
            FleetAdmissionMutationRequest, FleetAdmissionMutationResponse,
            FleetAdmissionOpenRootRequest, FleetAdmissionOperationPhase,
            FleetAdmissionPrepareRootRequest, FleetAdmissionRootReceipt,
            FleetAdmissionStatusRequest, FleetAdmissionStatusResponse,
        },
        fleet_funding::{
            FleetFundingPolicyRotationApplyRequest, FleetFundingPolicyRotationBeginRequest,
            FleetFundingPolicyRotationRootActivateRequest,
            FleetFundingPolicyRotationRootPrepareRequest, FleetFundingPolicyRotationRootReceipt,
            FleetFundingPolicyRotationStageRootRequest, FleetRootFundingAcceptanceReceipt,
            FleetRootFundingRequest, FleetRootFundingResponse,
        },
        fleet_registry::{
            FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
            FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootDeletionCompletionRequest,
            FleetSubnetRootDeletionExecutionRequest, FleetSubnetRootDeletionExecutionResponse,
            FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionStatusRequest,
            FleetSubnetRootDrainingPublicationRequest, FleetSubnetRootDrainingReservationRequest,
            FleetSubnetRootDrainingReservationResponse,
            FleetSubnetRootDrainingReservationStatusRequest, FleetSubnetRootJoinRequest,
            FleetSubnetRootJoinResponse, FleetSubnetRootRemovalPublicationRequest,
            FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootSnapshotAcknowledgementRequest,
        },
        role::{OperationReceipt, OperationStatusRequest, RootRemovalRequest},
        state::SetStateResponse,
    },
    log,
    log::Topic,
    protocol,
};
use serde::Deserialize;
use std::time::Duration;

#[derive(CandidType)]
enum RemoteRootCommand {
    AcceptFunding(canic_core::dto::fleet_funding::FleetRootFundingAcceptanceRequest),
    ActivateFleetAdmission(FleetAdmissionActivateRootRequest),
    ActivateFundingPolicyRotation(FleetFundingPolicyRotationRootActivateRequest),
    OpenFleetAdmission(FleetAdmissionOpenRootRequest),
    PrepareFleetAdmission(FleetAdmissionPrepareRootRequest),
    PrepareFundingPolicyRotation(FleetFundingPolicyRotationRootPrepareRequest),
    ProvisionComponents(RootComponentProvisioningAcceptanceRequest),
    RemoveRoot(RootRemovalRequest),
    SynchronizeComponentDirectories(RootComponentDirectorySynchronizationRequest),
}

#[derive(CandidType, Deserialize)]
enum RemoteRootCommandResponse {
    AcceptFunding(Box<FleetRootFundingAcceptanceReceipt>),
    ActivateFleetAdmission(Box<FleetAdmissionRootReceipt>),
    ActivateFundingPolicyRotation(Box<FleetFundingPolicyRotationRootReceipt>),
    OpenFleetAdmission(Box<FleetAdmissionRootReceipt>),
    OperationAccepted(Box<OperationReceipt>),
    PrepareFleetAdmission(Box<FleetAdmissionRootReceipt>),
    PrepareFundingPolicyRotation(Box<FleetFundingPolicyRotationRootReceipt>),
    SynchronizeComponentDirectories(Box<RootComponentDirectorySynchronizationResponse>),
}

#[derive(CandidType)]
enum RemoteRootStatusRequest {
    Operation(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RemoteRootStatusResponse {
    Operation(RemoteRootOperationStatusResponse),
}

#[derive(CandidType, Deserialize)]
enum RemoteRootOperationStatusResponse {
    ProvisionComponents(Box<RootComponentProvisioningStatusResponse>),
    RemoveRoot(Box<RootRemovalOperationStatus>),
}

///
/// FleetCoordinatorWorkflow
///
/// Coordinator lifecycle and query orchestration.
///

pub struct FleetCoordinatorWorkflow;

impl FleetCoordinatorWorkflow {
    pub(crate) fn authorize_registry_caller(
        caller: Principal,
        caller_is_controller: bool,
    ) -> Result<(), InternalError> {
        FleetCoordinatorOps::authorize_registry_caller(caller, caller_is_controller)
    }

    pub(crate) fn authorize_root_snapshot_caller(caller: Principal) -> Result<(), InternalError> {
        FleetCoordinatorOps::authorize_root_snapshot_caller(caller)
    }

    pub(crate) fn authorize_root_funding_caller(caller: Principal) -> Result<(), InternalError> {
        FleetCoordinatorOps::authorize_root_funding_caller(caller)
    }

    #[cfg(test)]
    pub(crate) fn operation_status(
        operation_id: [u8; 32],
    ) -> Result<CoordinatorOperationStatusResponse, InternalError> {
        Self::resolve_operation_status(operation_id)
    }

    pub(crate) fn operation_status_for_caller(
        operation_id: [u8; 32],
        caller: Principal,
        caller_is_controller: bool,
    ) -> Result<CoordinatorOperationStatusResponse, InternalError> {
        FleetCoordinatorOps::authorize_operation_caller(
            operation_id,
            caller,
            caller_is_controller,
        )?;
        Self::resolve_operation_status(operation_id)
    }

    fn resolve_operation_status(
        operation_id: [u8; 32],
    ) -> Result<CoordinatorOperationStatusResponse, InternalError> {
        let registry = FleetCoordinatorOps::registry()?;
        if let Some(status) = FleetAdmissionOps::operation_status(&registry, operation_id)? {
            return Ok(CoordinatorOperationStatusResponse::Admission(status));
        }
        FleetCoordinatorOps::operation_status(operation_id)
    }

    pub(crate) fn initialize(
        args: FleetCoordinatorInitArgs,
        _caller: Principal,
        caller_is_controller: bool,
        coordinator_canister: Principal,
    ) -> Result<(), InternalError> {
        if !caller_is_controller {
            return Err(InternalError::forbidden());
        }
        let admission =
            FleetAdmissionOps::compile_genesis(args.admission.clone(), &args.authority.binding)?;
        let record = FleetCoordinatorOps::compile_genesis(args, coordinator_canister)?;
        FleetCoordinatorOps::commit_genesis(record)?;
        let funding = FleetCoordinatorOps::compile_funding_genesis();
        FleetCoordinatorOps::commit_funding_genesis(funding)?;
        FleetAdmissionOps::commit_genesis(admission)?;
        Ok(())
    }

    /// Plan or exactly replay one protected Fleet-admission mutation.
    pub(crate) fn mutate_admission(
        request: FleetAdmissionMutationRequest,
    ) -> Result<FleetAdmissionMutationResponse, InternalError> {
        let registry = FleetCoordinatorOps::registry()?;
        let admission_replay =
            FleetAdmissionOps::retains_operation_id(&registry, request.operation_id)?;
        if !admission_replay {
            if FleetCoordinatorOps::retains_operation_id(request.operation_id)? {
                return Err(InternalError::conflict());
            }
            FleetCoordinatorOps::require_admission_transition_start_allowed()?;
        }
        let response = FleetAdmissionOps::mutate(&registry, request)?;
        if response.outcome == FleetAdmissionMutationOutcome::Planned {
            schedule_fleet_admission(response.operation_id, Duration::ZERO);
        }
        Ok(response)
    }

    /// Return one bounded protected policy page and current replay state.
    pub(crate) fn admission_status(
        request: FleetAdmissionStatusRequest,
    ) -> Result<FleetAdmissionStatusResponse, InternalError> {
        let registry = FleetCoordinatorOps::registry()?;
        FleetAdmissionOps::status(&registry, request)
    }

    fn require_non_admission_operation_id(operation_id: [u8; 32]) -> Result<(), InternalError> {
        let registry = FleetCoordinatorOps::registry()?;
        if FleetAdmissionOps::retains_operation_id(&registry, operation_id)? {
            Err(InternalError::conflict())
        } else {
            Ok(())
        }
    }

    fn require_admission_transition_idle() -> Result<(), InternalError> {
        let registry = FleetCoordinatorOps::registry()?;
        FleetAdmissionOps::require_transition_idle(&registry)
    }

    /// Decide and execute one exact registered-Root funding operation.
    pub(crate) async fn request_root_funding(
        caller: Principal,
        request: FleetRootFundingRequest,
    ) -> Result<FleetRootFundingResponse, InternalError> {
        Self::authorize_root_funding_caller(caller)?;
        let disposition = FleetCoordinatorOps::prepare_root_funding(
            caller,
            request,
            IcOps::canister_cycle_balance().to_u128(),
            IcOps::now_nanos(),
        )?;
        let call = match disposition {
            FleetRootFundingDisposition::Current(response) => return Ok(response),
            FleetRootFundingDisposition::Invoke(call)
            | FleetRootFundingDisposition::Reconcile(call) => call,
        };
        let fleet_subnet_root = call.fleet_subnet_root;
        let acceptance_request = call.request.clone();
        match call_root_funding_acceptance(call).await? {
            FleetRootFundingCallOutcome::Accepted(receipt) => {
                FleetCoordinatorOps::record_root_funding_acceptance(
                    fleet_subnet_root,
                    &acceptance_request,
                    *receipt,
                    IcOps::now_nanos(),
                )
            }
            FleetRootFundingCallOutcome::Rejected => {
                FleetCoordinatorOps::record_root_funding_rejection(
                    fleet_subnet_root,
                    &acceptance_request,
                    IcOps::now_nanos(),
                )
            }
        }
    }

    pub(crate) fn set_root_funding_enabled(
        enabled: bool,
    ) -> Result<SetStateResponse<bool>, InternalError> {
        FleetCoordinatorOps::set_root_funding_enabled(enabled)
    }

    pub(crate) fn begin_funding_policy_rotation(
        request: FleetFundingPolicyRotationBeginRequest,
    ) -> Result<OperationReceipt, InternalError> {
        Self::require_admission_transition_idle()?;
        Self::require_non_admission_operation_id(request.operation_id)?;
        FleetCoordinatorOps::begin_funding_policy_rotation(request, IcOps::now_nanos())
    }

    pub(crate) fn stage_funding_policy_rotation_root(
        request: FleetFundingPolicyRotationStageRootRequest,
    ) -> Result<OperationReceipt, InternalError> {
        FleetCoordinatorOps::stage_funding_policy_rotation_root(request, IcOps::now_nanos())
    }

    pub(crate) fn apply_funding_policy_rotation(
        request: FleetFundingPolicyRotationApplyRequest,
    ) -> Result<OperationReceipt, InternalError> {
        Self::require_admission_transition_idle()?;
        let receipt =
            FleetCoordinatorOps::apply_funding_policy_rotation(request, IcOps::now_nanos())?;
        schedule_funding_policy_rotation(receipt.operation_id, Duration::ZERO);
        Ok(receipt)
    }

    pub(crate) fn require_root_funding_snapshot_resumable() -> Result<(), InternalError> {
        FleetCoordinatorOps::require_root_funding_snapshot_resumable()
    }

    pub(crate) fn root_funding_status() -> Result<CoordinatorFundingStatusResponse, InternalError> {
        FleetCoordinatorOps::root_funding_status(
            IcOps::canister_cycle_balance().to_u128(),
            IcOps::now_nanos(),
        )
    }

    pub(crate) fn registry() -> Result<FleetRegistry, InternalError> {
        FleetCoordinatorOps::registry()
    }

    pub(crate) fn join_root(
        request: FleetSubnetRootJoinRequest,
    ) -> Result<FleetSubnetRootJoinResponse, InternalError> {
        Self::require_admission_transition_idle()?;
        FleetCoordinatorOps::join_root(request)
    }

    pub(crate) fn manifest() -> Result<FleetRegistryManifest, InternalError> {
        FleetCoordinatorOps::manifest()
    }

    pub(crate) fn registry_for_caller(
        caller: Principal,
        caller_is_controller: bool,
    ) -> Result<FleetRegistry, InternalError> {
        FleetCoordinatorOps::registry_for_caller(caller, caller_is_controller)
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
        Self::require_admission_transition_idle()?;
        FleetCoordinatorOps::prepare_component_provisioning(request, IcOps::now_nanos())
    }

    /// Accept one high-level provisioning intent and privately advance its durable state machine.
    pub(crate) fn accept_component_provisioning(
        request: FleetComponentProvisioningPrepareRequest,
    ) -> Result<OperationReceipt, InternalError> {
        Self::require_non_admission_operation_id(request.operation_id)?;
        let status = Self::prepare_component_provisioning(request)?;
        schedule_component_provisioning(status.operation_id, status.plan_hash, Duration::ZERO);
        Ok(OperationReceipt {
            operation_id: status.operation_id,
        })
    }

    pub(crate) fn component_provisioning_status(
        request: FleetComponentProvisioningStatusRequest,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        FleetCoordinatorOps::component_provisioning_status(request)
    }

    pub(crate) async fn advance_component_provisioning(
        request: &FleetComponentProvisioningAdvanceRequest,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = FleetCoordinatorOps::component_provisioning_status(
            FleetComponentProvisioningStatusRequest {
                operation_id: request.operation_id,
                plan_hash: request.plan_hash,
            },
        )?;
        if request.expected_phase != current.phase {
            return if component_provisioning_phase_rank(request.expected_phase)
                < component_provisioning_phase_rank(current.phase)
            {
                Ok(current)
            } else {
                Err(InternalError::conflict())
            };
        }
        if current.phase == FleetComponentProvisioningPhase::RuntimesActivated {
            return Ok(current);
        }
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
                    | FleetComponentProvisioningPhase::ConfirmingDirectories
                    | FleetComponentProvisioningPhase::DirectoriesConfirmed
                    | FleetComponentProvisioningPhase::ActivatingRuntimes
                    | FleetComponentProvisioningPhase::RuntimesActivated
            )
        {
            return Ok(acceptance_status);
        }
        if matches!(
            acceptance_status.operation,
            FleetComponentProvisioningOperation::ScaleOut { .. }
        ) {
            if matches!(
                acceptance_status.phase,
                FleetComponentProvisioningPhase::ServiceTopologyPublished
                    | FleetComponentProvisioningPhase::ConfirmingDirectories
            ) {
                return advance_component_directory_confirmation(request).await;
            }
            if matches!(
                acceptance_status.phase,
                FleetComponentProvisioningPhase::DirectoriesConfirmed
                    | FleetComponentProvisioningPhase::ActivatingRuntimes
                    | FleetComponentProvisioningPhase::RuntimesActivated
            ) {
                return advance_component_runtime_activation(request).await;
            }
            return advance_component_scale_out_service_publication(request, acceptance_status)
                .await;
        }
        if matches!(
            acceptance_status.phase,
            FleetComponentProvisioningPhase::DirectoriesConfirmed
                | FleetComponentProvisioningPhase::ActivatingRuntimes
                | FleetComponentProvisioningPhase::RuntimesActivated
        ) {
            return advance_component_runtime_activation(request).await;
        }
        if matches!(
            acceptance_status.phase,
            FleetComponentProvisioningPhase::ServiceTopologyPublished
                | FleetComponentProvisioningPhase::ConfirmingDirectories
        ) {
            return advance_component_directory_confirmation(request).await;
        }
        advance_component_root_provisioning(request).await
    }

    #[cfg(test)]
    pub(crate) fn publish_root_draining(
        request: FleetSubnetRootDrainingPublicationRequest,
    ) -> Result<
        canic_core::dto::fleet_registry::FleetSubnetRootDrainingPublicationResponse,
        InternalError,
    > {
        FleetCoordinatorOps::publish_root_draining(request)
    }

    pub(crate) fn prepare_root_draining_reservation(
        request: FleetSubnetRootDrainingReservationRequest,
    ) -> Result<FleetSubnetRootDrainingReservationResponse, InternalError> {
        Self::require_admission_transition_idle()?;
        FleetCoordinatorOps::prepare_root_draining_reservation(request, IcOps::now_nanos())
    }

    /// Accept one Coordinator-owned root removal and submit the exact Root intent once.
    pub(crate) async fn accept_root_removal(
        request: FleetSubnetRootDrainingReservationRequest,
    ) -> Result<OperationReceipt, InternalError> {
        Self::require_non_admission_operation_id(request.operation_id)?;
        let reservation = Self::prepare_root_draining_reservation(request)?;
        let expected = OperationReceipt {
            operation_id: reservation.request.operation_id,
        };
        let call = CallOps::unbounded_wait(
            reservation.request.expected_root.fleet_subnet_root,
            protocol::CANIC_ROOT_COMMAND,
        )
        .with_arg(RemoteRootCommand::RemoveRoot(RootRemovalRequest {
            reservation,
        }))?
        .execute()
        .await?;
        let result: Result<RemoteRootCommandResponse, Error> = call.candid()?;
        match result.map_err(InternalError::observed_public)? {
            RemoteRootCommandResponse::OperationAccepted(receipt) if *receipt == expected => {
                schedule_coordinator_root_removal(receipt.operation_id, Duration::ZERO);
                Ok(*receipt)
            }
            RemoteRootCommandResponse::AcceptFunding(_)
            | RemoteRootCommandResponse::ActivateFleetAdmission(_)
            | RemoteRootCommandResponse::ActivateFundingPolicyRotation(_)
            | RemoteRootCommandResponse::OpenFleetAdmission(_)
            | RemoteRootCommandResponse::OperationAccepted(_)
            | RemoteRootCommandResponse::PrepareFleetAdmission(_)
            | RemoteRootCommandResponse::PrepareFundingPolicyRotation(_)
            | RemoteRootCommandResponse::SynchronizeComponentDirectories(_) => {
                Err(InternalError::conflict())
            }
        }
    }

    pub(crate) fn root_draining_reservation_status(
        caller: Principal,
        caller_is_controller: bool,
        request: FleetSubnetRootDrainingReservationStatusRequest,
    ) -> Result<FleetSubnetRootDrainingReservationResponse, InternalError> {
        if !caller_is_controller && caller != request.fleet_subnet_root {
            return Err(InternalError::forbidden());
        }
        FleetCoordinatorOps::root_draining_reservation_status(request)
    }

    #[cfg(test)]
    pub(crate) fn publish_root_removed(
        caller: Principal,
        request: FleetSubnetRootRemovalPublicationRequest,
    ) -> Result<
        canic_core::dto::fleet_registry::FleetSubnetRootRemovalPublicationResponse,
        InternalError,
    > {
        FleetCoordinatorOps::publish_root_removed(caller, request)
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

fn schedule_fleet_admission(operation_id: [u8; 32], delay: Duration) {
    TimerApi::defer_lifecycle_required(delay, "Fleet admission convergence", async move {
        match advance_fleet_admission_once(operation_id).await {
            Ok(true) => {}
            Ok(false) => schedule_fleet_admission(operation_id, Duration::ZERO),
            Err(_) => schedule_fleet_admission(operation_id, Duration::from_secs(1)),
        }
    });
}

async fn advance_fleet_admission_once(operation_id: [u8; 32]) -> Result<bool, InternalError> {
    let registry = FleetCoordinatorOps::registry()?;
    let step = match FleetAdmissionOps::next_step(&registry) {
        Ok(step) => step,
        Err(error) => {
            let terminal = FleetAdmissionOps::operation_status(&registry, operation_id)?
                .is_some_and(|status| {
                    matches!(status.phase, FleetAdmissionOperationPhase::Completed(_))
                });
            return if terminal { Ok(true) } else { Err(error) };
        }
    };
    match step {
        FleetAdmissionCoordinatorStep::PrepareRoot {
            fleet_subnet_root,
            request,
        } => {
            if request.operation_id != operation_id {
                return Err(InternalError::conflict());
            }
            let receipt = call_root_admission(
                fleet_subnet_root,
                RemoteRootCommand::PrepareFleetAdmission(request),
                RootAdmissionResponsePhase::Prepare,
            )
            .await?;
            FleetAdmissionOps::record_root_receipt(&registry, receipt)?;
            Ok(false)
        }
        FleetAdmissionCoordinatorStep::PublishRegistry { request, successor } => {
            let registry = FleetCoordinatorOps::publish_admission_policy(request, successor)?;
            FleetAdmissionOps::record_registry_published(&registry)?;
            Ok(false)
        }
        FleetAdmissionCoordinatorStep::ActivateRoot {
            fleet_subnet_root,
            request,
        } => {
            if request.operation_id != operation_id {
                return Err(InternalError::conflict());
            }
            let receipt = call_root_admission(
                fleet_subnet_root,
                RemoteRootCommand::ActivateFleetAdmission(request),
                RootAdmissionResponsePhase::Activate,
            )
            .await?;
            FleetAdmissionOps::record_root_receipt(&registry, receipt)?;
            Ok(false)
        }
        FleetAdmissionCoordinatorStep::OpenRoot {
            fleet_subnet_root,
            request,
        } => {
            if request.operation_id != operation_id {
                return Err(InternalError::conflict());
            }
            let receipt = call_root_admission(
                fleet_subnet_root,
                RemoteRootCommand::OpenFleetAdmission(request),
                RootAdmissionResponsePhase::Open,
            )
            .await?;
            FleetAdmissionOps::record_root_receipt(&registry, receipt)?;
            Ok(false)
        }
        FleetAdmissionCoordinatorStep::Complete => {
            let response = FleetAdmissionOps::complete(&registry)?;
            if response.operation_id != operation_id
                || response.outcome != FleetAdmissionMutationOutcome::Converged
            {
                return Err(InternalError::conflict());
            }
            Ok(true)
        }
        FleetAdmissionCoordinatorStep::CompleteCatalogChanged => {
            let response = FleetAdmissionOps::complete_catalog_changed(&registry)?;
            if response.operation_id != operation_id
                || response.outcome != FleetAdmissionMutationOutcome::CatalogChanged
            {
                return Err(InternalError::conflict());
            }
            Ok(true)
        }
    }
}

#[derive(Clone, Copy)]
enum RootAdmissionResponsePhase {
    Activate,
    Open,
    Prepare,
}

async fn call_root_admission(
    fleet_subnet_root: Principal,
    command: RemoteRootCommand,
    expected: RootAdmissionResponsePhase,
) -> Result<FleetAdmissionRootReceipt, InternalError> {
    let result = CallOps::bounded_wait(fleet_subnet_root, protocol::CANIC_ROOT_COMMAND)
        .with_arg(command)?
        .execute()
        .await?;
    let response: Result<RemoteRootCommandResponse, Error> = result.candid()?;
    match (expected, response.map_err(InternalError::observed_public)?) {
        (
            RootAdmissionResponsePhase::Activate,
            RemoteRootCommandResponse::ActivateFleetAdmission(receipt),
        )
        | (
            RootAdmissionResponsePhase::Open,
            RemoteRootCommandResponse::OpenFleetAdmission(receipt),
        )
        | (
            RootAdmissionResponsePhase::Prepare,
            RemoteRootCommandResponse::PrepareFleetAdmission(receipt),
        ) => Ok(*receipt),
        _ => Err(InternalError::conflict()),
    }
}

fn schedule_funding_policy_rotation(operation_id: [u8; 32], delay: Duration) {
    TimerApi::defer_lifecycle_required(delay, "Fleet funding-policy rotation", async move {
        match advance_funding_policy_rotation_once(operation_id).await {
            Ok(true) => {}
            Ok(false) => schedule_funding_policy_rotation(operation_id, Duration::ZERO),
            Err(_) => {
                schedule_funding_policy_rotation(operation_id, Duration::from_secs(1));
            }
        }
    });
}

async fn advance_funding_policy_rotation_once(
    operation_id: [u8; 32],
) -> Result<bool, InternalError> {
    let step = match FleetCoordinatorOps::funding_policy_rotation_step() {
        Ok(step) => step,
        Err(error) => {
            let terminal = FleetCoordinatorOps::funding_policy_rotation_status(operation_id)?
                .is_some_and(|status| {
                    matches!(
                        status.phase,
                        FleetFundingPolicyRotationStatusPhase::Completed(_)
                    )
                });
            if terminal {
                return Ok(true);
            }
            return Err(error);
        }
    };
    match step {
        FleetFundingPolicyRotationStep::PrepareRoot {
            fleet_subnet_root,
            request,
        } => {
            if request.operation_id != operation_id {
                return Err(InternalError::conflict());
            }
            let receipt =
                call_root_funding_policy_rotation_prepare(fleet_subnet_root, request).await?;
            FleetCoordinatorOps::record_funding_policy_rotation_root_prepared(
                receipt,
                IcOps::now_nanos(),
            )?;
            Ok(false)
        }
        FleetFundingPolicyRotationStep::PublishRegistry => {
            FleetCoordinatorOps::publish_funding_policy_rotation_registry(IcOps::now_nanos())?;
            Ok(false)
        }
        FleetFundingPolicyRotationStep::ActivateRoot {
            fleet_subnet_root,
            request,
        } => {
            if request.operation_id != operation_id {
                return Err(InternalError::conflict());
            }
            let receipt =
                call_root_funding_policy_rotation_activate(fleet_subnet_root, request).await?;
            FleetCoordinatorOps::record_funding_policy_rotation_root_activated(
                receipt,
                IcOps::now_nanos(),
            )?;
            Ok(false)
        }
        FleetFundingPolicyRotationStep::Complete => {
            let receipt =
                FleetCoordinatorOps::complete_funding_policy_rotation(IcOps::now_nanos())?;
            if receipt.operation_id != operation_id {
                return Err(InternalError::conflict());
            }
            Ok(true)
        }
    }
}

fn schedule_coordinator_root_removal(operation_id: [u8; 32], delay: Duration) {
    TimerApi::defer_lifecycle_required(delay, "Fleet Coordinator root removal", async move {
        match advance_coordinator_root_removal_once(operation_id).await {
            Ok(true) => {}
            Ok(false) => schedule_coordinator_root_removal(operation_id, Duration::ZERO),
            Err(_) => {
                schedule_coordinator_root_removal(operation_id, Duration::from_secs(1));
            }
        }
    });
}

async fn advance_coordinator_root_removal_once(
    operation_id: [u8; 32],
) -> Result<bool, InternalError> {
    let current = match FleetCoordinatorOps::operation_status(operation_id)? {
        CoordinatorOperationStatusResponse::RootRemoval(current) => current,
        CoordinatorOperationStatusResponse::Admission(_)
        | CoordinatorOperationStatusResponse::ComponentProvisioning(_)
        | CoordinatorOperationStatusResponse::FundingPolicyRotation(_) => {
            return Err(InternalError::conflict());
        }
    };
    if current.readiness.is_some() {
        return Ok(true);
    }

    let root = current.reservation.request.expected_root.fleet_subnet_root;
    let observed = query_root_removal(root, operation_id).await?;
    if observed.operation_id != operation_id || observed.draining.fleet_subnet_root != root {
        return Err(InternalError::conflict());
    }

    if current.draining.is_none() {
        FleetCoordinatorOps::publish_root_draining(FleetSubnetRootDrainingPublicationRequest {
            expected_registry: current.reservation.request.expected_registry,
            root_draining: observed.draining,
        })?;
        return Ok(false);
    }
    if current.removal.is_none() {
        let final_inventory = observed
            .final_inventory
            .ok_or_else(InternalError::unavailable)?;
        FleetCoordinatorOps::publish_root_removed(
            root,
            FleetSubnetRootRemovalPublicationRequest {
                expected_registry: final_inventory.registry.clone(),
                final_inventory,
            },
        )?;
        return Ok(false);
    }
    if current.readiness_intent.is_none() {
        let request = observed
            .deletion_readiness_intent
            .ok_or_else(InternalError::unavailable)?;
        FleetCoordinatorOps::prepare_root_deletion_readiness(
            root,
            IcOps::canister_self(),
            request,
            IcOps::now_nanos(),
        )?;
        return Ok(false);
    }

    let request = observed
        .deletion_readiness
        .ok_or_else(InternalError::unavailable)?;
    FleetCoordinatorOps::record_root_deletion_readiness(
        root,
        IcOps::canister_self(),
        request,
        IcOps::now_nanos(),
    )?;
    Ok(true)
}

async fn query_root_removal(
    root: Principal,
    operation_id: [u8; 32],
) -> Result<RootRemovalOperationStatus, InternalError> {
    let result = CallOps::unbounded_wait(root, protocol::CANIC_ROOT_STATUS)
        .with_arg(RemoteRootStatusRequest::Operation(OperationStatusRequest {
            operation_id,
        }))?
        .execute()
        .await?;
    let response: Result<RemoteRootStatusResponse, Error> = result.candid()?;
    match response.map_err(InternalError::observed_public)? {
        RemoteRootStatusResponse::Operation(RemoteRootOperationStatusResponse::RemoveRoot(
            status,
        )) if status.operation_id == operation_id => Ok(*status),
        RemoteRootStatusResponse::Operation(_) => Err(InternalError::conflict()),
    }
}

fn schedule_component_provisioning(operation_id: [u8; 32], plan_hash: [u8; 32], delay: Duration) {
    TimerApi::defer_lifecycle_required(
        delay,
        "Fleet Coordinator Component provisioning",
        async move {
            advance_scheduled_component_provisioning(operation_id, plan_hash).await;
        },
    );
}

async fn advance_scheduled_component_provisioning(operation_id: [u8; 32], plan_hash: [u8; 32]) {
    let Ok(status) = FleetCoordinatorWorkflow::component_provisioning_status(
        FleetComponentProvisioningStatusRequest {
            operation_id,
            plan_hash,
        },
    ) else {
        return;
    };
    if status.phase == FleetComponentProvisioningPhase::RuntimesActivated {
        return;
    }
    let request = component_provisioning_advance_request(&status);
    match FleetCoordinatorWorkflow::advance_component_provisioning(&request).await {
        Ok(status) if status.phase == FleetComponentProvisioningPhase::RuntimesActivated => {}
        Ok(status) => schedule_component_provisioning(
            operation_id,
            plan_hash,
            component_provisioning_retry_delay(&status, IcOps::now_nanos()),
        ),
        Err(error) => {
            let diagnostic_code = error.public_error().code().raw();
            if let Err(retention_error) =
                FleetCoordinatorOps::record_component_provisioning_root_failure(
                    FleetComponentProvisioningStatusRequest {
                        operation_id,
                        plan_hash,
                    },
                    diagnostic_code,
                    IcOps::now_nanos(),
                )
            {
                log!(
                    Topic::Fleet,
                    Error,
                    "Fleet Component provisioning retry diagnostic retention failed: operation_id={} phase={:?} error={retention_error}",
                    hex_bytes(operation_id),
                    status.phase,
                );
            }
            log!(
                Topic::Fleet,
                Warn,
                "Fleet Component provisioning retry: operation_id={} phase={:?} error={error}",
                hex_bytes(operation_id),
                status.phase,
            );
            schedule_component_provisioning(operation_id, plan_hash, Duration::from_secs(1));
        }
    }
}

fn component_provisioning_retry_delay(
    status: &FleetComponentProvisioningStatusResponse,
    now_ns: u64,
) -> Duration {
    status
        .estate_funding_required
        .as_ref()
        .map_or(Duration::ZERO, |funding| {
            Duration::from_nanos(funding.retry_at_ns.saturating_sub(now_ns))
        })
}

const fn component_provisioning_advance_request(
    status: &FleetComponentProvisioningStatusResponse,
) -> FleetComponentProvisioningAdvanceRequest {
    FleetComponentProvisioningAdvanceRequest {
        operation_id: status.operation_id,
        plan_hash: status.plan_hash,
        expected_phase: status.phase,
        expected_accepted_root_count: status.accepted_root_count,
        expected_provisioned_root_count: status.provisioned_root_count,
        expected_current_root: status.current_root,
        expected_directory_confirmed_root_count: status.directory_confirmed_root_count,
        expected_current_synchronization: status.current_synchronization,
        expected_current_publication: status.current_publication,
        expected_runtime_activated_root_count: status.runtime_activated_root_count,
        expected_current_activation: status.current_activation,
    }
}

async fn advance_component_runtime_activation(
    request: &FleetComponentProvisioningAdvanceRequest,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    let disposition =
        FleetCoordinatorOps::advance_component_runtime_activation(request, IcOps::now_nanos())?;
    let call = match disposition {
        FleetComponentRuntimeActivationDisposition::Current(status) => return Ok(*status),
        FleetComponentRuntimeActivationDisposition::Invoke(call)
        | FleetComponentRuntimeActivationDisposition::Reconcile(call) => call,
    };
    let response = activate_root_component_runtimes(call).await?;
    FleetCoordinatorOps::record_component_runtime_activation(request, &response, IcOps::now_nanos())
}

async fn advance_component_directory_confirmation(
    request: &FleetComponentProvisioningAdvanceRequest,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    let disposition =
        FleetCoordinatorOps::advance_component_directory_confirmation(request, IcOps::now_nanos())?;
    let call = match disposition {
        FleetComponentDirectoryConfirmationDisposition::Current(status) => return Ok(*status),
        FleetComponentDirectoryConfirmationDisposition::Invoke(call)
        | FleetComponentDirectoryConfirmationDisposition::Reconcile(call) => call,
    };
    match advance_root_component_directories(call).await? {
        RootComponentDirectoryAdvanceResponse::FreshPublication(response) => {
            FleetCoordinatorOps::record_component_directory_confirmation(
                request,
                response,
                IcOps::now_nanos(),
            )
        }
        RootComponentDirectoryAdvanceResponse::ScaleOutPublication(response) => {
            FleetCoordinatorOps::record_component_scale_out_directory_publication(
                request,
                response,
                IcOps::now_nanos(),
            )
        }
        RootComponentDirectoryAdvanceResponse::Synchronization(response) => {
            FleetCoordinatorOps::record_component_scale_out_directory_synchronization(
                request,
                response,
                IcOps::now_nanos(),
            )
        }
    }
}

async fn advance_component_scale_out_service_publication(
    request: &FleetComponentProvisioningAdvanceRequest,
    status: FleetComponentProvisioningStatusResponse,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    if scale_out_service_publication_is_complete(&status)? {
        return Ok(status);
    }
    advance_component_root_provisioning(request).await
}

fn scale_out_service_publication_is_complete(
    status: &FleetComponentProvisioningStatusResponse,
) -> Result<bool, InternalError> {
    match status.phase {
        FleetComponentProvisioningPhase::ComponentsProvisioned => {
            let terminal_facts = [
                status.current_root.is_none(),
                status.provisioning_in_flight_root.is_none(),
                status.provisioned_root_count == status.root_batch_count,
                status.components_provisioned_at_ns.is_some(),
                status.published_fleet_registry.is_none(),
                status.service_topology_published_at_ns.is_none(),
            ];
            if terminal_facts.into_iter().all(|fact| fact) {
                return Ok(false);
            }
        }
        FleetComponentProvisioningPhase::ServiceTopologyPublished => {
            let publication_facts = [
                status.current_root.is_none(),
                status.provisioning_in_flight_root.is_none(),
                status.provisioned_root_count == status.root_batch_count,
                status.components_provisioned_at_ns.is_some(),
                status.published_fleet_registry.is_some(),
                status.service_topology_published_at_ns.is_some(),
                status.directory_confirmed_root_count == 0,
                status.current_synchronization.is_none(),
                status.current_publication.is_none(),
                status.runtime_activated_root_count == 0,
                status.current_activation.is_none(),
            ];
            if publication_facts.into_iter().all(|fact| fact) {
                return Ok(true);
            }
        }
        FleetComponentProvisioningPhase::RootsAccepted
        | FleetComponentProvisioningPhase::ProvisioningRoots => {
            validate_scale_out_current_root_progress(status.current_root)?;
            return Ok(false);
        }
        FleetComponentProvisioningPhase::Planned
        | FleetComponentProvisioningPhase::AcceptingRoots
        | FleetComponentProvisioningPhase::ConfirmingDirectories
        | FleetComponentProvisioningPhase::DirectoriesConfirmed
        | FleetComponentProvisioningPhase::ActivatingRuntimes
        | FleetComponentProvisioningPhase::RuntimesActivated => {}
    }
    Err(InternalError::invariant())
}

fn validate_scale_out_current_root_progress(
    progress: Option<FleetComponentProvisioningRootProgress>,
) -> Result<(), InternalError> {
    let Some(progress) = progress else {
        return Err(InternalError::invariant());
    };
    let claim_has_reserved_identity = match progress.claimed_component_count {
        0 => progress.reserved_component_count <= progress.component_count,
        _ => progress.reserved_component_count == progress.component_count,
    };
    let install_has_claimed_canister = match progress.installed_component_count {
        0 => progress.claimed_component_count <= progress.component_count,
        _ => progress.claimed_component_count == progress.component_count,
    };
    let registry_commit_has_install = match progress.registry_committed_component_count {
        0 => progress.installed_component_count <= progress.component_count,
        _ => progress.installed_component_count == progress.component_count,
    };
    let remains_before_terminal_receipt = [
        claim_has_reserved_identity,
        install_has_claimed_canister,
        registry_commit_has_install,
        progress.claimed_component_count <= progress.component_count,
        progress.installed_component_count <= progress.component_count,
        progress.registry_committed_component_count <= progress.component_count,
    ]
    .into_iter()
    .all(|matches| matches);
    if !remains_before_terminal_receipt {
        return Err(InternalError::invariant());
    }
    Ok(())
}

async fn advance_component_root_provisioning(
    request: &FleetComponentProvisioningAdvanceRequest,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    let disposition =
        FleetCoordinatorOps::advance_component_provisioning_root(request, IcOps::now_nanos())?;
    let call = match disposition {
        FleetComponentProvisioningRootProvisionDisposition::Current(status) => return Ok(*status),
        FleetComponentProvisioningRootProvisionDisposition::Invoke(call)
        | FleetComponentProvisioningRootProvisionDisposition::Reconcile(call) => call,
        FleetComponentProvisioningRootProvisionDisposition::Publish => {
            return FleetCoordinatorOps::publish_component_provisioning_services(
                request,
                IcOps::now_nanos(),
            );
        }
    };
    let response = advance_root_component_provisioning(call).await?;
    if response.estate_funding_required.is_some() {
        return FleetCoordinatorOps::record_component_provisioning_estate_funding_pause(
            request,
            response,
            IcOps::now_nanos(),
        );
    }
    FleetCoordinatorOps::record_component_provisioning_root(request, response, IcOps::now_nanos())
}

const fn component_provisioning_phase_rank(phase: FleetComponentProvisioningPhase) -> u8 {
    match phase {
        FleetComponentProvisioningPhase::Planned => 0,
        FleetComponentProvisioningPhase::AcceptingRoots => 1,
        FleetComponentProvisioningPhase::RootsAccepted => 2,
        FleetComponentProvisioningPhase::ProvisioningRoots => 3,
        FleetComponentProvisioningPhase::ComponentsProvisioned => 4,
        FleetComponentProvisioningPhase::ServiceTopologyPublished => 5,
        FleetComponentProvisioningPhase::ConfirmingDirectories => 6,
        FleetComponentProvisioningPhase::DirectoriesConfirmed => 7,
        FleetComponentProvisioningPhase::ActivatingRuntimes => 8,
        FleetComponentProvisioningPhase::RuntimesActivated => 9,
    }
}

async fn accept_root_component_provisioning(
    call: FleetComponentProvisioningRootAcceptanceCallView,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let operation_id = call.request.operation_id;
    let plan_hash = call.request.plan_hash;
    let root = call.fleet_subnet_root;
    let result = CallOps::unbounded_wait(root, protocol::CANIC_ROOT_COMMAND)
        .with_arg(RemoteRootCommand::ProvisionComponents(call.request))?
        .execute()
        .await?;
    let response: Result<RemoteRootCommandResponse, Error> = result.candid()?;
    match response.map_err(InternalError::observed_public)? {
        RemoteRootCommandResponse::OperationAccepted(receipt)
            if receipt.operation_id == operation_id =>
        {
            query_root_component_provisioning(root, operation_id, plan_hash).await
        }
        _ => Err(InternalError::conflict()),
    }
}

enum FleetRootFundingCallOutcome {
    Accepted(Box<FleetRootFundingAcceptanceReceipt>),
    Rejected,
}

async fn call_root_funding_acceptance(
    call: FleetRootFundingCallView,
) -> Result<FleetRootFundingCallOutcome, InternalError> {
    let granted_cycles = call.request.granted_cycles.to_u128();
    let result = CallOps::bounded_wait(call.fleet_subnet_root, protocol::CANIC_ROOT_COMMAND)
        .with_arg(RemoteRootCommand::AcceptFunding(call.request))?
        .with_cycles(granted_cycles)
        .execute()
        .await?;
    let response: Result<RemoteRootCommandResponse, Error> = result.candid()?;
    match response {
        Ok(RemoteRootCommandResponse::AcceptFunding(receipt)) => {
            Ok(FleetRootFundingCallOutcome::Accepted(receipt))
        }
        Err(_) => Ok(FleetRootFundingCallOutcome::Rejected),
        Ok(_) => Err(InternalError::conflict()),
    }
}

async fn call_root_funding_policy_rotation_prepare(
    fleet_subnet_root: Principal,
    request: FleetFundingPolicyRotationRootPrepareRequest,
) -> Result<FleetFundingPolicyRotationRootReceipt, InternalError> {
    let result = CallOps::bounded_wait(fleet_subnet_root, protocol::CANIC_ROOT_COMMAND)
        .with_arg(RemoteRootCommand::PrepareFundingPolicyRotation(request))?
        .execute()
        .await?;
    let response: Result<RemoteRootCommandResponse, Error> = result.candid()?;
    match response.map_err(InternalError::observed_public)? {
        RemoteRootCommandResponse::PrepareFundingPolicyRotation(receipt) => Ok(*receipt),
        _ => Err(InternalError::conflict()),
    }
}

async fn call_root_funding_policy_rotation_activate(
    fleet_subnet_root: Principal,
    request: FleetFundingPolicyRotationRootActivateRequest,
) -> Result<FleetFundingPolicyRotationRootReceipt, InternalError> {
    let result = CallOps::bounded_wait(fleet_subnet_root, protocol::CANIC_ROOT_COMMAND)
        .with_arg(RemoteRootCommand::ActivateFundingPolicyRotation(request))?
        .execute()
        .await?;
    let response: Result<RemoteRootCommandResponse, Error> = result.candid()?;
    match response.map_err(InternalError::observed_public)? {
        RemoteRootCommandResponse::ActivateFundingPolicyRotation(receipt) => Ok(*receipt),
        _ => Err(InternalError::conflict()),
    }
}

async fn advance_root_component_provisioning(
    call: FleetComponentProvisioningRootProvisionCallView,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    query_root_component_provisioning(
        call.fleet_subnet_root,
        call.request.operation_id,
        call.request.plan_hash,
    )
    .await
}

async fn query_root_component_provisioning(
    root: Principal,
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let result = CallOps::unbounded_wait(root, protocol::CANIC_ROOT_STATUS)
        .with_arg(RemoteRootStatusRequest::Operation(OperationStatusRequest {
            operation_id,
        }))?
        .execute()
        .await?;
    let response: Result<RemoteRootStatusResponse, Error> = result.candid()?;
    match response.map_err(InternalError::observed_public)? {
        RemoteRootStatusResponse::Operation(
            RemoteRootOperationStatusResponse::ProvisionComponents(response),
        ) if response.operation_id == operation_id && response.plan_hash == plan_hash => {
            Ok(*response)
        }
        RemoteRootStatusResponse::Operation(_) => Err(InternalError::conflict()),
    }
}

enum RootComponentDirectoryAdvanceResponse {
    FreshPublication(RootComponentProvisioningStatusResponse),
    ScaleOutPublication(RootComponentProvisioningStatusResponse),
    Synchronization(RootComponentDirectorySynchronizationResponse),
}

async fn advance_root_component_directories(
    call: FleetComponentDirectoryConfirmationCallView,
) -> Result<RootComponentDirectoryAdvanceResponse, InternalError> {
    match call {
        FleetComponentDirectoryConfirmationCallView::FreshPublication {
            fleet_subnet_root,
            request,
        } => query_root_component_provisioning(
            fleet_subnet_root,
            request.operation_id,
            request.plan_hash,
        )
        .await
        .map(RootComponentDirectoryAdvanceResponse::FreshPublication),
        FleetComponentDirectoryConfirmationCallView::ScaleOutPublication {
            fleet_subnet_root,
            request,
        } => query_root_component_provisioning(
            fleet_subnet_root,
            request.operation_id,
            request.plan_hash,
        )
        .await
        .map(RootComponentDirectoryAdvanceResponse::ScaleOutPublication),
        FleetComponentDirectoryConfirmationCallView::ScaleOutSynchronization {
            fleet_subnet_root,
            request,
        } => {
            let result = CallOps::unbounded_wait(fleet_subnet_root, protocol::CANIC_ROOT_COMMAND)
                .with_arg(RemoteRootCommand::SynchronizeComponentDirectories(request))?
                .execute()
                .await?;
            let response: Result<RemoteRootCommandResponse, Error> = result.candid()?;
            match response.map_err(InternalError::observed_public)? {
                RemoteRootCommandResponse::SynchronizeComponentDirectories(response) => Ok(
                    RootComponentDirectoryAdvanceResponse::Synchronization(*response),
                ),
                RemoteRootCommandResponse::AcceptFunding(_)
                | RemoteRootCommandResponse::ActivateFleetAdmission(_)
                | RemoteRootCommandResponse::ActivateFundingPolicyRotation(_)
                | RemoteRootCommandResponse::OpenFleetAdmission(_)
                | RemoteRootCommandResponse::OperationAccepted(_)
                | RemoteRootCommandResponse::PrepareFleetAdmission(_)
                | RemoteRootCommandResponse::PrepareFundingPolicyRotation(_) => {
                    Err(InternalError::conflict())
                }
            }
        }
    }
}

async fn activate_root_component_runtimes(
    call: FleetComponentRuntimeActivationCallView,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    query_root_component_provisioning(
        call.fleet_subnet_root,
        call.request.operation_id,
        call.request.plan_hash,
    )
    .await
}
