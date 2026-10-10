//! Module: dto::fleet_coordinator
//!
//! Responsibility: carry the protected fresh-install input for one Fleet Coordinator.
//! Does not own: validation, stable state, Registry compilation, or lifecycle effects.
//! Boundary: the Coordinator lifecycle adapter passes this passive payload to workflow.

use crate::{
    cycles::Cycles,
    dto::{
        authority_restore::{AuthorityRestoreFenceStatusResponse, AuthoritySnapshotRequest},
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
            FleetRegistryActivationRequest, FleetRegistryActivationResponse, FleetRegistryManifest,
            FleetRegistryVersion, FleetSubnetRootDeletionCompletionRequest,
            FleetSubnetRootDeletionExecutionRequest, FleetSubnetRootDeletionExecutionResponse,
            FleetSubnetRootDeletionReadinessIntentResponse,
            FleetSubnetRootDeletionReadinessResponse, FleetSubnetRootDeletionResponse,
            FleetSubnetRootDrainingPublicationResponse, FleetSubnetRootDrainingReservationRequest,
            FleetSubnetRootDrainingReservationResponse, FleetSubnetRootJoinRequest,
            FleetSubnetRootJoinResponse, FleetSubnetRootRemovalPublicationResponse,
            FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootSnapshotAcknowledgementRequest,
        },
        role::{OperationReceipt, OperationStatusRequest},
        state::{SetCyclesFundingRequest, SetStateResponse},
    },
    ids::{FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetSubnetRootFundingPolicy},
};
use candid::CandidType;
use serde::Deserialize;

///
/// FleetCoordinatorInitArgs
///
/// Exact authority and compiled provisioning configuration installed into a fresh Coordinator.
///

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
    Retire(crate::dto::fleet_registry::FleetRetirementRequest),
    SetRootFunding(SetCyclesFundingRequest),
    StageFundingPolicyRotationRoot(FleetFundingPolicyRotationStageRootRequest),
}

/// Closed correlated success union for Fleet Coordinator commands.
#[derive(CandidType, Deserialize)]
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
    Retire(crate::dto::fleet_registry::FleetRetirementStatus),
    SetRootFunding(SetStateResponse<bool>),
}

/// Registry reads use the existing Coordinator registry-caller authority.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum CoordinatorRegistryRequest {
    Registry,
}

/// Registry snapshot returned to an authorized registry caller.
#[derive(CandidType, Deserialize)]
pub enum CoordinatorRegistryResponse {
    Registry(crate::dto::fleet_registry::FleetRegistry),
}

/// Operation reads delegate authorization to the durable operation owner.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum CoordinatorOperationReadRequest {
    Operation(OperationStatusRequest),
}

/// One durable operation result under its existing exact caller contract.
#[derive(CandidType, Deserialize)]
pub enum CoordinatorOperationReadResponse {
    Operation(CoordinatorOperationStatusResponse),
}

/// Coordinator diagnostics selected under one observer authorization rule.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum CoordinatorObservabilityRequest {
    Admission(FleetAdmissionStatusRequest),
    AuthorityRestore,
    Funding,
    RegistryManifest,
    RegistryVersion,
    IntentRelease(Option<crate::dto::release_intents::IntentReleaseKey>),
    ReplayRelease(Option<[u8; 32]>),
    RootAcknowledgements,
}

/// Current spent and reserved cycles in one exact epoch-anchored funding window.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct CoordinatorFundingWindowStatusResponse {
    pub window_start_secs: u64,
    pub spent_cycles: Cycles,
    pub reserved_cycles: Cycles,
}

/// Controller-only funding usage and operation state for one registered Root.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CoordinatorRootFundingStatusResponse {
    pub fleet_subnet_root: candid::Principal,
    pub lifecycle_status: crate::dto::fleet_registry::FleetSubnetRootStatus,
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
    Retirement(crate::dto::fleet_registry::FleetRetirementStatus),
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
        successor_registry: Box<FleetRegistryVersion>,
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

/// Operational diagnostics returned only to authorized observers.
#[derive(CandidType, Deserialize)]
pub enum CoordinatorObservabilityResponse {
    Admission(FleetAdmissionStatusResponse),
    AuthorityRestore(AuthorityRestoreFenceStatusResponse),
    Funding(CoordinatorFundingStatusResponse),
    RegistryManifest(FleetRegistryManifest),
    RegistryVersion(FleetRegistryVersion),
    IntentRelease(crate::dto::release_intents::IntentReleaseResponse),
    ReplayRelease(crate::dto::release_receipts::ReplayReleaseResponse),
    RootAcknowledgements(Vec<FleetSubnetRootSnapshotAcknowledgement>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode};

    #[test]
    fn coordinator_status_request_is_one_closed_candid_variant() {
        let requests = [
            CoordinatorObservabilityRequest::Admission(FleetAdmissionStatusRequest {
                selector: crate::ids::FleetAdmissionSelector::Fleet,
                page: crate::dto::page::PageRequest {
                    limit: 128,
                    offset: 0,
                },
            }),
            CoordinatorObservabilityRequest::AuthorityRestore,
            CoordinatorObservabilityRequest::Funding,
            CoordinatorObservabilityRequest::RegistryManifest,
            CoordinatorObservabilityRequest::RegistryVersion,
            CoordinatorObservabilityRequest::IntentRelease(None),
            CoordinatorObservabilityRequest::IntentRelease(Some(
                crate::dto::release_intents::IntentReleaseKey::Local(u64::MAX),
            )),
            CoordinatorObservabilityRequest::IntentRelease(Some(
                crate::dto::release_intents::IntentReleaseKey::ReceiptBacked([0; 32]),
            )),
            CoordinatorObservabilityRequest::ReplayRelease(None),
            CoordinatorObservabilityRequest::ReplayRelease(Some([7; 32])),
            CoordinatorObservabilityRequest::RootAcknowledgements,
        ];

        for request in requests {
            let bytes = Encode!(&request).expect("encode Coordinator status request");
            assert_eq!(
                Decode!(&bytes, CoordinatorObservabilityRequest)
                    .expect("decode Coordinator status request"),
                request
            );
        }
    }
}
