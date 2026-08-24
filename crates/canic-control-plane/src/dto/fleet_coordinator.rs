//! Module: dto::fleet_coordinator
//!
//! Responsibility: carry the protected fresh-install input for one Fleet Coordinator.
//! Does not own: validation, stable state, Registry compilation, or lifecycle effects.
//! Boundary: the Coordinator lifecycle adapter passes this passive payload to workflow.

use candid::CandidType;
use canic_core::{
    cdk::types::Cycles,
    control_plane_support::config::ComponentDeploymentConfiguration,
    dto::{
        authority_restore::AuthorityRestoreFenceStatusResponse,
        authority_restore::AuthoritySnapshotRequest,
        component_provisioning::{
            FleetComponentProvisioningPrepareRequest, FleetComponentProvisioningStatusResponse,
        },
        fleet_admission::{
            FleetAdmissionMutationRequest, FleetAdmissionMutationResponse,
            FleetAdmissionOperationStatusResponse, FleetAdmissionStatusRequest,
            FleetAdmissionStatusResponse,
        },
        fleet_funding::{
            FleetFundingPolicyRotationApplyRequest, FleetFundingPolicyRotationBeginRequest,
            FleetFundingPolicyRotationReceipt, FleetFundingPolicyRotationStageRootRequest,
            FleetRootFundingRequest, FleetRootFundingResponse,
        },
        fleet_registry::{
            FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
            FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootDeletionCompletionRequest,
            FleetSubnetRootDeletionExecutionRequest, FleetSubnetRootDeletionExecutionResponse,
            FleetSubnetRootDeletionReadinessIntentResponse,
            FleetSubnetRootDeletionReadinessResponse, FleetSubnetRootDeletionResponse,
            FleetSubnetRootDrainingPublicationResponse, FleetSubnetRootDrainingReservationRequest,
            FleetSubnetRootDrainingReservationResponse, FleetSubnetRootJoinRequest,
            FleetSubnetRootJoinResponse, FleetSubnetRootRemovalPublicationResponse,
            FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootSnapshotAcknowledgementRequest,
        },
        role::{OperationReceipt, OperationStatusRequest, RoleOverviewResponse},
        state::{SetCyclesFundingRequest, SetStateResponse},
    },
    ids::{
        AppId, FleetAdmissionPolicy, FleetCoordinatorRootFundingPolicy, FleetFundingProfile,
        FleetRegistryAuthority, FleetSubnetRootFundingPolicy,
    },
};
use serde::Deserialize;

///
/// FleetCoordinatorInitArgs
///
/// Exact authority and compiled provisioning configuration installed into a fresh Coordinator.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetCoordinatorInitArgs {
    pub configured_app: AppId,
    pub authority: FleetRegistryAuthority,
    pub admission: FleetAdmissionPolicy,
    pub component_deployment_configuration: ComponentDeploymentConfiguration,
    pub root_funding: Option<FleetCoordinatorRootFundingPolicy>,
}

/// Closed controller command union for the Fleet Coordinator.
#[derive(CandidType, Deserialize)]
pub enum CoordinatorCommand {
    AcknowledgeRootSnapshot(FleetSubnetRootSnapshotAcknowledgementRequest),
    ActivateRegistry(FleetRegistryActivationRequest),
    ApplyFundingPolicyRotation(FleetFundingPolicyRotationApplyRequest),
    BeginFundingPolicyRotation(FleetFundingPolicyRotationBeginRequest),
    CompleteRootDeletion(FleetSubnetRootDeletionCompletionRequest),
    JoinRoot(FleetSubnetRootJoinRequest),
    MutateAdmission(FleetAdmissionMutationRequest),
    PrepareAuthoritySnapshot(AuthoritySnapshotRequest),
    PrepareRootDeletionExecution(FleetSubnetRootDeletionExecutionRequest),
    ProvisionComponents(FleetComponentProvisioningPrepareRequest),
    RemoveRoot(FleetSubnetRootDrainingReservationRequest),
    RequestRootFunding(FleetRootFundingRequest),
    ResumeAuthoritySnapshot(AuthoritySnapshotRequest),
    SetRootFunding(SetCyclesFundingRequest),
    StageFundingPolicyRotationRoot(FleetFundingPolicyRotationStageRootRequest),
}

/// Closed correlated success union for Fleet Coordinator commands.
#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the accepted Candid union keeps each existing command result as its direct payload"
)]
pub enum CoordinatorCommandResponse {
    AcknowledgeRootSnapshot(FleetSubnetRootSnapshotAcknowledgement),
    ActivateRegistry(FleetRegistryActivationResponse),
    CompleteRootDeletion(FleetSubnetRootDeletionResponse),
    JoinRoot(FleetSubnetRootJoinResponse),
    MutateAdmission(FleetAdmissionMutationResponse),
    OperationAccepted(OperationReceipt),
    PrepareAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
    PrepareRootDeletionExecution(FleetSubnetRootDeletionExecutionResponse),
    RequestRootFunding(FleetRootFundingResponse),
    ResumeAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
    SetRootFunding(SetStateResponse<bool>),
}

/// Closed Coordinator observation selector carried by its single status query.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum CoordinatorStatusRequest {
    Admission(FleetAdmissionStatusRequest),
    AuthorityRestore,
    Funding,
    Operation(OperationStatusRequest),
    Overview,
    Registry,
    RegistryManifest,
    RegistryVersion,
    RootAcknowledgements,
}

/// Current spent and reserved cycles in one exact epoch-anchored funding window.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CoordinatorFundingWindowStatusResponse {
    pub window_start_secs: u64,
    pub spent_cycles: Cycles,
    pub reserved_cycles: Cycles,
}

/// Controller-only funding usage and operation state for one registered Root.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CoordinatorRootFundingStatusResponse {
    pub fleet_subnet_root: candid::Principal,
    pub lifecycle_status: canic_core::dto::fleet_registry::FleetSubnetRootStatus,
    pub policy_hash: [u8; 32],
    pub policy: FleetSubnetRootFundingPolicy,
    pub window: CoordinatorFundingWindowStatusResponse,
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: Cycles,
    pub automatic_grants: u32,
    pub automatic_cycles: Cycles,
    pub last_successful_grant_at_ns: Option<u64>,
    pub current_operation: Option<FleetRootFundingRequest>,
    pub last_result: Option<FleetRootFundingResponse>,
}

/// Controller-only Coordinator treasury policy, headroom and per-Root usage.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CoordinatorFundingStatusResponse {
    pub coordinator: candid::Principal,
    pub current_cycles: Cycles,
    pub policy_generation: u64,
    pub funding_enabled: bool,
    pub funding_profile: Option<FleetFundingProfile>,
    pub policy: Option<FleetCoordinatorRootFundingPolicy>,
    pub fleet_window: Option<CoordinatorFundingWindowStatusResponse>,
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: Cycles,
    pub automatic_grants: u32,
    pub automatic_cycles: Cycles,
    pub rotation_checkpoint_count: u32,
    pub rotation_checkpoint_root_count: u32,
    pub rotation_checkpoint_root_capacity_remaining: u32,
    pub rotation: Option<FleetFundingPolicyRotationStatusResponse>,
    pub roots: Vec<CoordinatorRootFundingStatusResponse>,
}

/// Coordinator-owned durable operation detail selected by one operation ID.
#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the accepted Candid union keeps each existing status DTO as its direct payload"
)]
pub enum CoordinatorOperationStatusResponse {
    Admission(FleetAdmissionOperationStatusResponse),
    ComponentProvisioning(FleetComponentProvisioningStatusResponse),
    FundingPolicyRotation(FleetFundingPolicyRotationStatusResponse),
    RootRemoval(CoordinatorRootRemovalOperationStatus),
}

/// Protected durable phase of one Coordinator-owned policy rotation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum FleetFundingPolicyRotationStatusPhase {
    Staging {
        staged_root_count: u32,
        expected_root_count: u32,
    },
    PreparingRoots {
        prepared_root_count: u32,
        expected_root_count: u32,
    },
    ActivatingRoots {
        activated_root_count: u32,
        expected_root_count: u32,
        successor_registry: FleetRegistryVersion,
    },
    Completed(Box<FleetFundingPolicyRotationReceipt>),
}

/// Controller-only status of one exact current or terminal rotation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetFundingPolicyRotationStatusResponse {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub phase: FleetFundingPolicyRotationStatusPhase,
}

/// Coordinator-owned progress across the existing durable root-removal boundaries.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CoordinatorRootRemovalOperationStatus {
    pub operation_id: [u8; 32],
    pub reservation: FleetSubnetRootDrainingReservationResponse,
    pub draining: Option<FleetSubnetRootDrainingPublicationResponse>,
    pub removal: Option<FleetSubnetRootRemovalPublicationResponse>,
    pub readiness_intent: Option<FleetSubnetRootDeletionReadinessIntentResponse>,
    pub readiness: Option<FleetSubnetRootDeletionReadinessResponse>,
    pub execution: Option<FleetSubnetRootDeletionExecutionResponse>,
    pub completion: Option<FleetSubnetRootDeletionResponse>,
}

/// Closed response union for the Coordinator's single status query.
#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the accepted Candid union keeps each existing status DTO as its direct payload"
)]
pub enum CoordinatorStatusResponse {
    Admission(FleetAdmissionStatusResponse),
    AuthorityRestore(AuthorityRestoreFenceStatusResponse),
    Funding(CoordinatorFundingStatusResponse),
    Operation(CoordinatorOperationStatusResponse),
    Overview(RoleOverviewResponse),
    Registry(FleetRegistry),
    RegistryManifest(FleetRegistryManifest),
    RegistryVersion(FleetRegistryVersion),
    RootAcknowledgements(Vec<FleetSubnetRootSnapshotAcknowledgement>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode};

    #[test]
    fn coordinator_status_request_is_one_closed_candid_variant() {
        let requests = [
            CoordinatorStatusRequest::Admission(FleetAdmissionStatusRequest {
                selector: canic_core::ids::FleetAdmissionSelector::Fleet,
                page: canic_core::dto::page::PageRequest {
                    limit: 128,
                    offset: 0,
                },
            }),
            CoordinatorStatusRequest::AuthorityRestore,
            CoordinatorStatusRequest::Funding,
            CoordinatorStatusRequest::Operation(OperationStatusRequest {
                operation_id: [4; 32],
            }),
            CoordinatorStatusRequest::Overview,
            CoordinatorStatusRequest::Registry,
            CoordinatorStatusRequest::RegistryManifest,
            CoordinatorStatusRequest::RegistryVersion,
            CoordinatorStatusRequest::RootAcknowledgements,
        ];

        for request in requests {
            let bytes = Encode!(&request).expect("encode Coordinator status request");
            assert_eq!(
                Decode!(&bytes, CoordinatorStatusRequest)
                    .expect("decode Coordinator status request"),
                request
            );
        }
    }
}
