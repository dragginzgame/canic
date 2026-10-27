//! Complete subtree-removal observations; lifecycle and storage remain owned by the registry.
use crate::{
    deployment::ComponentChildKind,
    dto::component_registry::{
        ComponentLifecycleStatus, ComponentRegistryHead, ComponentRuntimeActivationEvidence,
    },
    ids::{CanisterRole, ComponentInstanceId},
};
use ic_platform::Principal;

/// Complete RootComponentSubtreeRemovalResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Status {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub target_canister_id: Principal,
    pub target_parent_canister_id: Principal,
    pub target_role: CanisterRole,
    pub target_status: ComponentLifecycleStatus,
    pub reserved_against_registry: ComponentRegistryHead,
    pub maximum_completed_leaves: u32,
    pub completed_leaves: u32,
    pub traversal_steps: u32,
    pub phase: Phase,
}

/// Complete RootComponentSubtreeRemovalNode evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Node {
    pub canister_id: Principal,
    pub parent_canister_id: Principal,
    pub role: CanisterRole,
    pub kind: ComponentChildKind,
    pub installed_artifact_hash: [u8; 32],
    pub status: ComponentLifecycleStatus,
}

/// Complete RootComponentSubtreeRemovalStopIntent evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct StopIntent {
    pub leaf: Node,
    pub controller: Principal,
}

/// Complete RootComponentSubtreeRemovalStoppedReceipt evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Stopped {
    pub stop: StopIntent,
    pub observed_module_hash: [u8; 32],
}

/// Complete RootComponentSubtreeRemovalDeleteIntent evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DeleteIntent {
    pub stopped: Stopped,
}

/// Complete RootComponentSubtreeRemovalDeletedReceipt evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Deleted {
    pub deletion: DeleteIntent,
}

/// Complete RootComponentSubtreeRemovalMembershipRemovedReceipt evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct MembershipRemoved {
    pub deleted: Deleted,
    pub removed_from_registry: ComponentRegistryHead,
    pub previous_descendant_content_hash: [u8; 32],
    pub previous_committed_descendants: u32,
    pub registry: ComponentRegistryHead,
    pub descendant_content_hash: [u8; 32],
    pub registry_encoded_bytes: u64,
    pub reserved_descendants: u32,
    pub committed_descendants: u32,
    pub directory_synchronized_at_ns: u64,
    pub directory_authority_hash: [u8; 32],
    pub parent_role_instances: u32,
    pub root_managed_descendants: u32,
    pub root_known_created_component_canisters: u32,
}

/// Complete RootComponentSubtreeRemovalDirectoryConvergenceEvidence evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DirectoryConvergence {
    pub operation_id: [u8; 32],
    pub canister_id: Principal,
    pub activation: ComponentRuntimeActivationEvidence,
}

/// Complete RootComponentSubtreeRemovalDirectorySynchronizedReceipt evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DirectorySynchronized {
    pub membership_removed: MembershipRemoved,
    pub covered_fleet_registry_revision: u64,
    pub covered_fleet_registry_content_hash: [u8; 32],
    pub covered_component_registry: ComponentRegistryHead,
    pub covered_authority_hash: [u8; 32],
    pub owning_component: Option<DirectoryConvergence>,
    pub parent: Option<DirectoryConvergence>,
}

/// Complete RootComponentSubtreeRemovalCompletedReceipt evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Completed {
    pub registry: ComponentRegistryHead,
    pub directory_authority_hash: [u8; 32],
}

/// Exact retained removal boundary; observation never advances it.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum Phase {
    Fenced,
    Traversing(Box<Node>),
    LeafSelected(Box<Node>),
    StopIntent(Box<StopIntent>),
    Stopped(Box<Stopped>),
    DeleteIntent(Box<DeleteIntent>),
    Deleted(Box<Deleted>),
    MembershipRemoved(Box<MembershipRemoved>),
    DirectorySynchronized(Box<DirectorySynchronized>),
    Completed(Box<Completed>),
}
