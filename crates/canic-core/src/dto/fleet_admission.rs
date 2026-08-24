//! Passive boundary DTOs for protected Fleet-admission administration and status.

use crate::{
    dto::{page::Page, prelude::*},
    ids::{
        FleetAdmissionSelector, FleetBinding, FleetCoordinatorBinding, FleetSubnetRootBinding,
        ManagedCanisterBinding,
    },
};

/// One closed Fleet-admission membership action.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetAdmissionMutationAction {
    Add,
    Remove,
}

/// One exact controller-authorized Fleet-admission mutation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetAdmissionMutationRequest {
    pub authority: FleetCoordinatorBinding,
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub action: FleetAdmissionMutationAction,
    pub selector: FleetAdmissionSelector,
    pub principal: Principal,
    pub operation_id: [u8; 32],
    pub successor_policy_digest: [u8; 32],
    pub participant_catalog_digest: [u8; 32],
    pub participant_count: u32,
}

/// Public semantic outcome of one accepted request.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetAdmissionMutationOutcome {
    Planned,
    Converged,
    CatalogChanged,
    AlreadyPresent,
    AlreadyAbsent,
}

/// Exact accepted mutation result, including idempotent outcomes.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetAdmissionMutationResponse {
    pub outcome: FleetAdmissionMutationOutcome,
    pub operation_id: [u8; 32],
    pub generation: u64,
    pub policy_digest: [u8; 32],
}

/// Compact current or successor policy identity and bounded counts.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetAdmissionPolicyStatus {
    pub generation: u64,
    pub policy_digest: [u8; 32],
    pub fleet_principal_count: u16,
    pub narrower_rule_count: u16,
    pub narrower_principal_reference_count: u16,
}

/// Durable state of one current or retained Coordinator admission operation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum FleetAdmissionOperationPhase {
    Planned {
        successor: FleetAdmissionPolicyStatus,
    },
    Preparing {
        successor: FleetAdmissionPolicyStatus,
    },
    Releasing {
        successor: FleetAdmissionPolicyStatus,
    },
    PerimeterFenced {
        successor: FleetAdmissionPolicyStatus,
    },
    Activating {
        successor: FleetAdmissionPolicyStatus,
    },
    Opening {
        successor: FleetAdmissionPolicyStatus,
    },
    Completed(FleetAdmissionMutationResponse),
}

/// Protected operation detail selected by exact operation identity.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetAdmissionOperationStatusResponse {
    pub operation_id: [u8; 32],
    pub action: FleetAdmissionMutationAction,
    pub selector: FleetAdmissionSelector,
    pub principal: Principal,
    pub phase: FleetAdmissionOperationPhase,
}

/// Protected selector and page requested from the current active policy.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetAdmissionStatusRequest {
    pub selector: FleetAdmissionSelector,
    pub page: crate::dto::page::PageRequest,
}

/// Controller-only current policy, bounded membership page and replay state.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct FleetAdmissionStatusResponse {
    pub fleet: FleetBinding,
    pub active: FleetAdmissionPolicyStatus,
    pub selector: FleetAdmissionSelector,
    pub principals: Page<Principal>,
    pub maximum_page_size: u16,
    pub current_operation: Option<FleetAdmissionOperationStatusResponse>,
    pub last_result: Option<FleetAdmissionOperationStatusResponse>,
}

/// Target-local ingress phase exposed to its protected Root/controller status.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum FleetAdmissionProjectionPhase {
    Fenced,
    Open,
}

/// Compact optional prepared projection identity.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetAdmissionPreparedProjectionStatus {
    pub generation: u64,
    pub policy_digest: [u8; 32],
    pub projection_digest: [u8; 32],
}

/// Protected bounded status for one managed target's sole local projection.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct FleetAdmissionProjectionStatusResponse {
    pub authority: FleetCoordinatorBinding,
    pub target: ManagedCanisterBinding,
    pub generation: u64,
    pub policy_digest: [u8; 32],
    pub projection_digest: [u8; 32],
    pub phase: FleetAdmissionProjectionPhase,
    pub prepared: Option<FleetAdmissionPreparedProjectionStatus>,
    pub principals: Page<Principal>,
    pub maximum_page_size: u16,
}

/// One target-local phase in the replay-safe admission transition protocol.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetAdmissionTargetTransitionPhase {
    Prepare,
    Activate,
    Open,
}

/// Root-authenticated command that atomically retains a successor and fences ingress.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetAdmissionPrepareTargetRequest {
    pub operation_id: [u8; 32],
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub successor: crate::ids::FleetAdmissionProjection,
}

/// Root-authenticated command that installs the exact prepared successor while fenced.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetAdmissionActivateTargetRequest {
    pub operation_id: [u8; 32],
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub successor_generation: u64,
    pub successor_policy_digest: [u8; 32],
    pub successor_projection_digest: [u8; 32],
}

/// Root-authenticated command that opens the exact active successor.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetAdmissionOpenTargetRequest {
    pub operation_id: [u8; 32],
    pub generation: u64,
    pub policy_digest: [u8; 32],
    pub projection_digest: [u8; 32],
}

/// Exact target receipt retained for response-loss replay and Root reconciliation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetAdmissionTargetReceipt {
    pub operation_id: [u8; 32],
    pub phase: FleetAdmissionTargetTransitionPhase,
    pub target: ManagedCanisterBinding,
    pub generation: u64,
    pub policy_digest: [u8; 32],
    pub projection_digest: [u8; 32],
    pub receipt_hash: [u8; 32],
}

/// Coordinator-authenticated command that snapshots one Root subtree and starts fencing it.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum FleetAdmissionPrepareRootStage {
    Reserve,
    Fence,
    Release,
}

/// Coordinator-authenticated staged Root catalog reservation or fencing command.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetAdmissionPrepareRootRequest {
    pub authority: FleetCoordinatorBinding,
    pub operation_id: [u8; 32],
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub successor: crate::ids::FleetAdmissionPolicy,
    pub stage: FleetAdmissionPrepareRootStage,
}

/// Coordinator-authenticated command that advances one fully fenced Root subtree to activation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetAdmissionActivateRootRequest {
    pub authority: FleetCoordinatorBinding,
    pub operation_id: [u8; 32],
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub successor_generation: u64,
    pub successor_policy_digest: [u8; 32],
}

/// Coordinator-authenticated command that opens one fully activated Root subtree.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetAdmissionOpenRootRequest {
    pub authority: FleetCoordinatorBinding,
    pub operation_id: [u8; 32],
    pub generation: u64,
    pub policy_digest: [u8; 32],
}

/// Root aggregate phase retained for Coordinator response-loss replay.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetAdmissionRootTransitionPhase {
    Preparing,
    PerimeterFenced,
    Activating,
    Opening,
    Converged,
    Released,
}

/// Exact aggregate Root receipt returned after every subtree phase converges.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetAdmissionRootReceipt {
    pub operation_id: [u8; 32],
    pub phase: FleetAdmissionRootTransitionPhase,
    pub root: FleetSubnetRootBinding,
    pub generation: u64,
    pub policy_digest: [u8; 32],
    pub participant_catalog_digest: [u8; 32],
    pub participant_count: u32,
    pub receipt_hash: [u8; 32],
}

/// Monotonic target phase exposed through protected Root admission status.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum FleetAdmissionRootParticipantPhase {
    Pending,
    Prepared,
    Activated,
    Open,
}

/// One bounded target progress row owned only by its Root journal.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetAdmissionRootParticipantStatus {
    pub target: ManagedCanisterBinding,
    pub projection_digest: [u8; 32],
    pub phase: FleetAdmissionRootParticipantPhase,
    pub last_receipt_hash: Option<[u8; 32]>,
}

/// Protected bounded view of the Root's current or retained admission operation.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct FleetAdmissionRootStatusResponse {
    pub operation_id: Option<[u8; 32]>,
    pub phase: Option<FleetAdmissionRootTransitionPhase>,
    pub active_generation: u64,
    pub active_policy_digest: [u8; 32],
    pub successor_generation: Option<u64>,
    pub successor_policy_digest: Option<[u8; 32]>,
    pub participant_catalog_digest: Option<[u8; 32]>,
    pub participants: Page<FleetAdmissionRootParticipantStatus>,
    pub maximum_page_size: u16,
    pub last_result: Option<FleetAdmissionRootReceipt>,
}
