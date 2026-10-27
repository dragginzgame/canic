//! Native admission boundaries; persisted plans and authorization semantics retain their owners.
use crate::{
    dto::{
        component_directory::FleetCoordinatorBinding,
        fleet_admission::{
            FleetAdmissionMutationAction, FleetAdmissionOperationPhase, FleetAdmissionPolicyStatus,
        },
        fleet_snapshot::FleetAdmissionSelector,
        page::{Page, PageRequest},
    },
    ids::FleetBinding,
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Exact native MutationRequest contract, independent of other Coordinator capabilities.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct MutationRequest {
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

/// Exact native OperationStatus contract, independent of other Coordinator capabilities.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct OperationStatus {
    pub operation_id: [u8; 32],
    pub action: FleetAdmissionMutationAction,
    pub selector: FleetAdmissionSelector,
    pub principal: Principal,
    pub phase: FleetAdmissionOperationPhase,
}

/// Exact native StatusRequest contract, independent of other Coordinator capabilities.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct StatusRequest {
    pub selector: FleetAdmissionSelector,
    pub page: PageRequest,
}

/// Exact native StatusResponse contract, independent of other Coordinator capabilities.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct StatusResponse {
    pub fleet: FleetBinding,
    pub active: FleetAdmissionPolicyStatus,
    pub selector: FleetAdmissionSelector,
    pub principals: Page<Principal>,
    pub maximum_page_size: u16,
    pub current_operation: Option<OperationStatus>,
    pub last_result: Option<OperationStatus>,
}

/// Native Root admission evidence, preserving original identities and progress.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootReceipt {
    pub operation_id: [u8; 32],
    pub phase: crate::dto::fleet_admission::FleetAdmissionRootTransitionPhase,
    pub root: crate::dto::fleet_snapshot::FleetSubnetRootBinding,
    pub generation: u64,
    pub policy_digest: [u8; 32],
    pub participant_catalog_digest: [u8; 32],
    pub participant_count: u32,
    pub receipt_hash: [u8; 32],
}

/// Native Root admission evidence, preserving original identities and progress.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootParticipantStatus {
    pub target: crate::dto::component_directory::ManagedCanisterBinding,
    pub projection_digest: [u8; 32],
    pub phase: crate::dto::fleet_admission::FleetAdmissionRootParticipantPhase,
    pub last_receipt_hash: Option<[u8; 32]>,
}

/// Native Root admission evidence, preserving original identities and progress.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootStatus {
    pub operation_id: Option<[u8; 32]>,
    pub phase: Option<crate::dto::fleet_admission::FleetAdmissionRootTransitionPhase>,
    pub active_generation: u64,
    pub active_policy_digest: [u8; 32],
    pub successor_generation: Option<u64>,
    pub successor_policy_digest: Option<[u8; 32]>,
    pub participant_catalog_digest: Option<[u8; 32]>,
    pub participants: Page<RootParticipantStatus>,
    pub maximum_page_size: u16,
    pub last_result: Option<RootReceipt>,
}

/// Protected local admission evidence; identities are native only at transport.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ManagedStatus {
    pub authority: FleetCoordinatorBinding,
    pub target: crate::dto::component_directory::ManagedCanisterBinding,
    pub generation: u64,
    pub policy_digest: [u8; 32],
    pub projection_digest: [u8; 32],
    pub phase: crate::dto::fleet_admission::FleetAdmissionProjectionPhase,
    pub prepared: Option<crate::dto::fleet_admission::FleetAdmissionPreparedProjectionStatus>,
    pub principals: Page<Principal>,
    pub maximum_page_size: u16,
}

/// Native target admission contract; canonical records retain their original representation.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TargetProjection {
    pub schema_version: u16,
    pub authority: FleetCoordinatorBinding,
    pub target: crate::dto::component_directory::ManagedCanisterBinding,
    pub generation: u64,
    pub policy_digest: [u8; 32],
    pub projection_digest: [u8; 32],
    pub principals: Vec<Principal>,
}

/// Native target admission contract; canonical records retain their original representation.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PrepareTargetRequest {
    pub operation_id: [u8; 32],
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub successor: TargetProjection,
}

/// Native target admission contract; canonical records retain their original representation.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TargetReceipt {
    pub operation_id: [u8; 32],
    pub phase: crate::dto::fleet_admission::FleetAdmissionTargetTransitionPhase,
    pub target: crate::dto::component_directory::ManagedCanisterBinding,
    pub generation: u64,
    pub policy_digest: [u8; 32],
    pub projection_digest: [u8; 32],
    pub receipt_hash: [u8; 32],
}

/// Native Coordinator-to-Root intent; policy and retained records remain owner-defined.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PrepareRootRequest {
    pub authority: FleetCoordinatorBinding,
    pub operation_id: [u8; 32],
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub successor: crate::dto::fleet_snapshot::FleetAdmissionPolicy,
    pub stage: crate::dto::fleet_admission::FleetAdmissionPrepareRootStage,
}

/// Native Coordinator-to-Root intent; policy and retained records remain owner-defined.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ActivateRootRequest {
    pub authority: FleetCoordinatorBinding,
    pub operation_id: [u8; 32],
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub successor_generation: u64,
    pub successor_policy_digest: [u8; 32],
}

/// Native Coordinator-to-Root intent; policy and retained records remain owner-defined.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct OpenRootRequest {
    pub authority: FleetCoordinatorBinding,
    pub operation_id: [u8; 32],
    pub generation: u64,
    pub policy_digest: [u8; 32],
}
