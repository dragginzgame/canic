//! Complete component-removal observation; persisted deletion authority remains unchanged.
use crate::{
    dto::component_registry::{
        ComponentRegistryHead, ComponentRuntimeActivationEvidence, RootComponentDrainingResponse,
        RootComponentFinalInventory,
    },
    ids::ComponentInstanceId,
};
use ic_platform::Principal;

/// Complete RootComponentRemovalOperationStatus evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Status {
    pub draining: RootComponentDrainingResponse,
    pub deletion: Option<Deletion>,
}

/// Complete RootComponentDeletionResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Deletion {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub phase: Phase,
}

/// Complete RootComponentQuiescenceStopIntent evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct StopIntent {
    pub registry: ComponentRegistryHead,
    pub descendant_count: u32,
    pub descendant_content_hash: [u8; 32],
    pub canister_id: Principal,
    pub controller: Principal,
    pub expected_module_hash: [u8; 32],
    pub covered_fleet_registry_revision: u64,
    pub covered_fleet_registry_content_hash: [u8; 32],
    pub covered_authority_hash: [u8; 32],
    pub runtime_operation_id: [u8; 32],
    pub activation: ComponentRuntimeActivationEvidence,
    pub prepared_at_ns: u64,
}

/// Complete RootComponentQuiescentReceipt evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Quiescent {
    pub stop: StopIntent,
    pub observed_module_hash: [u8; 32],
    pub quiesced_at_ns: u64,
}

/// Complete RootComponentDeletionIntent evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DeleteIntent {
    pub final_inventory: RootComponentFinalInventory,
    pub quiescence: Quiescent,
    pub prepared_at_ns: u64,
}

/// Complete RootComponentDeletedReceipt evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Deleted {
    pub deletion: DeleteIntent,
    pub deleted_at_ns: u64,
}

/// Complete RootComponentMembershipRemovedReceipt evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct MembershipRemoved {
    pub deleted: Deleted,
    pub allocation_operation_id: [u8; 32],
    pub remaining_spec_committed_instances: u32,
    pub root_committed_component_instances: u32,
    pub root_known_created_component_canisters: u32,
    pub root_registry_encoded_bytes: u64,
    pub removed_at_ns: u64,
    pub removal_hash: [u8; 32],
}

/// Exact pre-effect and terminal deletion boundaries.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum Phase {
    DeleteIntent(Box<DeleteIntent>),
    Deleted(Box<Deleted>),
    MembershipRemoved(Box<MembershipRemoved>),
}
