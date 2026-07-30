//! Module: dto::component_registry
//!
//! Responsibility: carry root-local Component Registry preparation and allocation evidence.
//! Does not own: admission policy, stable mutation, artifact resolution, or lifecycle effects.
//! Boundary: callers name intent and Spec while the root allocates identity under verified authority.

use crate::{
    cdk::types::Cycles,
    config::schema::ComponentChildKind,
    dto::{
        fleet_registry::{FleetDirectorySnapshot, FleetRegistryVersion},
        root_store::RootStoreBootstrapRequest,
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentChildBinding, ComponentInstanceId,
        ComponentSpecId, ComponentTopologyDigest, FleetSubnetRootReleaseSet,
        ManagedCanisterBinding,
    },
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

///
/// RootComponentRegistryPreparationRequest
///
/// Exact authority required before an empty root-local Component Registry may be prepared.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentRegistryPreparationRequest {
    pub store_bootstrap: RootStoreBootstrapRequest,
    pub expected_fleet_registry: FleetRegistryVersion,
}

///
/// RootComponentInitialInventoryStatus
///
/// Durable initial Component inventory sealed for one Fleet Subnet Root activation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentInitialInventoryStatus {
    pub fleet_activation_operation_id: [u8; 32],
    pub component_count: u32,
    pub inventory_hash: [u8; 32],
    pub sealed_at_ns: u64,
    pub directories_converged: bool,
    pub root_runtime_activated: bool,
}

///
/// RootComponentRegistryStatusResponse
///
/// Compact durable Component Registry authority and current allocation counters.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentRegistryStatusResponse {
    pub fleet_subnet_root: Principal,
    pub prepared_against_registry: FleetRegistryVersion,
    pub release_set: FleetSubnetRootReleaseSet,
    pub component_topology_digest: ComponentTopologyDigest,
    pub next_allocation_sequence: u64,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    pub managed_descendants: u32,
    pub known_created_component_canisters: u32,
    pub encoded_bytes: u64,
    pub initial_inventory: Option<RootComponentInitialInventoryStatus>,
}

///
/// RootComponentAllocationRequest
///
/// Controller command naming one idempotent top-level Component reservation intent.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentAllocationRequest {
    pub operation_id: [u8; 32],
    pub component_spec: ComponentSpecId,
}

///
/// RootComponentAllocationStatusRequest
///
/// Read-only lookup key for one durable top-level Component allocation operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentAllocationStatusRequest {
    pub operation_id: [u8; 32],
}

///
/// RootComponentChildAllocationRequest
///
/// Parent command naming one idempotent direct-child reservation intent.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildAllocationRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub expected_registry: ComponentRegistryHead,
    pub child_role: CanisterRole,
}

///
/// RootComponentChildAllocationStatusRequest
///
/// Parent lookup key for one durable direct-child reservation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildAllocationStatusRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
}

///
/// RootComponentSubtreeRemovalRequest
///
/// Controller command durably fencing one registered child subtree.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub target_canister_id: Principal,
    pub expected_registry: ComponentRegistryHead,
}

///
/// RootComponentSubtreeRemovalAdvanceRequest
///
/// Controller command advancing bounded traversal from one observed durable step.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalAdvanceRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub expected_traversal_steps: u32,
}

///
/// RootComponentSubtreeRemovalStopPreparationRequest
///
/// Controller command freezing the exact selected leaf and root stop authority.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalStopPreparationRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub expected_traversal_steps: u32,
    pub expected_leaf_canister_id: Principal,
    pub expected_leaf_parent_canister_id: Principal,
}

///
/// RootComponentSubtreeRemovalStopRequest
///
/// Controller command reconciling and stopping one exactly prepared leaf.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalStopRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub expected_traversal_steps: u32,
    pub expected_leaf_canister_id: Principal,
    pub expected_leaf_parent_canister_id: Principal,
}

///
/// RootComponentSubtreeRemovalDeletePreparationRequest
///
/// Controller command freezing exact deletion authority from one stopped receipt.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalDeletePreparationRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub expected_traversal_steps: u32,
    pub expected_leaf_canister_id: Principal,
    pub expected_leaf_parent_canister_id: Principal,
}

///
/// RootComponentSubtreeRemovalDeleteRequest
///
/// Controller command reconciling and deleting one exactly prepared leaf.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalDeleteRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub expected_traversal_steps: u32,
    pub expected_leaf_canister_id: Principal,
    pub expected_leaf_parent_canister_id: Principal,
}

///
/// RootComponentSubtreeRemovalStatusRequest
///
/// Controller lookup key for one durable child-subtree removal operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalStatusRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
}

///
/// RootComponentChildCreationRequest
///
/// Parent command continuing one already reserved direct-child operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildCreationRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
}

///
/// RootComponentChildInstallRequest
///
/// Parent command installing and verifying one already created direct-child operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildInstallRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
}

///
/// RootComponentChildCommitRequest
///
/// Parent command committing one already verified direct-child operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildCommitRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
}

///
/// RootComponentChildDirectoryPreparationRequest
///
/// Parent command distributing one committed child's Directory and converging its affected members.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildDirectoryPreparationRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
}

///
/// RootComponentChildRuntimeActivationRequest
///
/// Parent command activating one Directory-prepared direct-child runtime.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildRuntimeActivationRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
}

///
/// RootComponentChildMembershipActivationRequest
///
/// Parent command activating one runtime-active direct child's Registry membership.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildMembershipActivationRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
}

///
/// RootComponentCreationRequest
///
/// Controller command continuing one already reserved top-level Component operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentCreationRequest {
    pub operation_id: [u8; 32],
}

///
/// RootComponentInstallRequest
///
/// Controller command continuing one already created top-level Component operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentInstallRequest {
    pub operation_id: [u8; 32],
}

///
/// RootComponentCommitRequest
///
/// Controller command committing one already verified top-level Component operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentCommitRequest {
    pub operation_id: [u8; 32],
}

///
/// RootComponentDirectoryPreparationRequest
///
/// Controller command distributing exact Directories to one committed top-level Component.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentDirectoryPreparationRequest {
    pub operation_id: [u8; 32],
}

///
/// RootComponentRuntimeActivationRequest
///
/// Controller command activating one Directory-prepared top-level Component runtime.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentRuntimeActivationRequest {
    pub operation_id: [u8; 32],
}

///
/// RootComponentMembershipActivationRequest
///
/// Controller command activating one runtime-active Component's Registry membership.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentMembershipActivationRequest {
    pub operation_id: [u8; 32],
}

///
/// ComponentProvisioningOrigin
///
/// Authenticated causal authority retained with one top-level Component allocation.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ComponentProvisioningOrigin {
    FleetAdministrator { caller: Principal },
}

///
/// RootComponentAllocationPhase
///
/// Durable root-local progress of one top-level Component allocation operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentAllocationPhase {
    Reserved,
    CreationIntent,
    Created,
    InstallIntent,
    Installed,
    Verified,
    Committed,
}

///
/// RootComponentSubtreeRemovalPhase
///
/// Durable root-local progress of one child-subtree removal operation.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentSubtreeRemovalPhase {
    Fenced,
    Traversing(RootComponentSubtreeRemovalNode),
    LeafSelected(RootComponentSubtreeRemovalNode),
    StopIntent(RootComponentSubtreeRemovalStopIntent),
    Stopped(RootComponentSubtreeRemovalStoppedReceipt),
    DeleteIntent(RootComponentSubtreeRemovalDeleteIntent),
    Deleted(RootComponentSubtreeRemovalDeletedReceipt),
}

///
/// RootComponentSubtreeRemovalNode
///
/// Exact registered child selected as a traversal cursor or removable leaf.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalNode {
    pub canister_id: Principal,
    pub parent_canister_id: Principal,
    pub role: CanisterRole,
    pub kind: ComponentChildKind,
    pub installed_artifact_hash: [u8; 32],
    pub status: ComponentLifecycleStatus,
}

///
/// RootComponentSubtreeRemovalStopIntent
///
/// Exact registered leaf and sole root controller frozen before a stop call.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalStopIntent {
    pub leaf: RootComponentSubtreeRemovalNode,
    pub controller: Principal,
}

///
/// RootComponentSubtreeRemovalStoppedReceipt
///
/// Frozen stop authority plus the independently observed installed module.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalStoppedReceipt {
    pub stop: RootComponentSubtreeRemovalStopIntent,
    pub observed_module_hash: [u8; 32],
}

///
/// RootComponentSubtreeRemovalDeleteIntent
///
/// Exact stopped receipt frozen before the destructive management call.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalDeleteIntent {
    pub stopped: RootComponentSubtreeRemovalStoppedReceipt,
}

///
/// RootComponentSubtreeRemovalDeletedReceipt
///
/// Frozen deletion authority committed only after independently observed absence.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalDeletedReceipt {
    pub deletion: RootComponentSubtreeRemovalDeleteIntent,
}

///
/// ComponentLifecycleStatus
///
/// Root-owned runtime lifecycle state of one committed Component Registry member.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ComponentLifecycleStatus {
    Prepared,
    Active,
    Draining,
    Removed,
}

///
/// ComponentRegistryHead
///
/// Exact independently versioned authority of one Component Registry partition.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRegistryHead {
    pub component: ComponentInstanceId,
    pub revision: u64,
    pub content_hash: [u8; 32],
}

///
/// ComponentRegistryPartitionRequest
///
/// Read-only lookup key for one committed Component Registry partition.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRegistryPartitionRequest {
    pub component: ComponentInstanceId,
}

///
/// ComponentRegistryPartitionResponse
///
/// Protected top-level row and independent head of one Component Registry partition.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRegistryPartitionResponse {
    pub head: ComponentRegistryHead,
    pub binding: ComponentBinding,
    pub provisioning_origin: ComponentProvisioningOrigin,
    pub release_set: FleetSubnetRootReleaseSet,
    pub status: ComponentLifecycleStatus,
    pub reserved_descendants: u32,
    pub committed_descendants: u32,
    pub encoded_bytes: u64,
}

///
/// ComponentDirectoryProvenance
///
/// Exact Component Registry authority from which one Component Directory is derived.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentDirectoryProvenance {
    pub component: ComponentBinding,
    pub source_fleet_subnet_root: Principal,
    pub component_registry_revision: u64,
    pub component_registry_content_hash: [u8; 32],
    pub synchronized_at_ns: u64,
}

///
/// ComponentDirectoryHead
///
/// Compact independently versioned discovery projection for one Component tree.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentDirectoryHead {
    pub provenance: ComponentDirectoryProvenance,
    pub descendant_count: u32,
}

///
/// ComponentDirectoryHeadRequest
///
/// Read-only lookup key for one committed Component Directory head.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentDirectoryHeadRequest {
    pub component: ComponentInstanceId,
}

///
/// ComponentDirectoryPageCursor
///
/// Opaque revision- and filter-bound continuation for one bounded Directory page.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentDirectoryPageCursor(pub Vec<u8>);

///
/// ComponentDirectoryPageRequest
///
/// Bounded member query against one exact current Component Directory authority.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentDirectoryPageRequest {
    pub directory: ComponentDirectoryHead,
    pub parent_canister_id: Option<Principal>,
    pub role: Option<CanisterRole>,
    pub status: Option<ComponentLifecycleStatus>,
    pub cursor: Option<ComponentDirectoryPageCursor>,
    pub limit: u16,
}

///
/// ComponentDirectoryChildEntry
///
/// One authoritative normalized child projected with its complete protected binding.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentDirectoryChildEntry {
    pub binding: ComponentChildBinding,
    pub kind: ComponentChildKind,
    pub installed_artifact_hash: [u8; 32],
    pub status: ComponentLifecycleStatus,
}

///
/// ComponentDirectoryPageResponse
///
/// One bounded caller-scoped page under the exact requested Directory head.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentDirectoryPageResponse {
    pub directory: ComponentDirectoryHead,
    pub entries: Vec<ComponentDirectoryChildEntry>,
    pub next_cursor: Option<ComponentDirectoryPageCursor>,
}

///
/// ComponentRuntimeDirectoryAuthority
///
/// Exact Fleet and Component discovery authority retained by one managed Component-tree node.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeDirectoryAuthority {
    pub fleet: FleetDirectorySnapshot,
    pub component: ComponentDirectoryHead,
}

///
/// ComponentRuntimeDirectoryPreparationRequest
///
/// Root-issued exact Directory preparation command for one managed Component-tree node.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeDirectoryPreparationRequest {
    pub operation_id: [u8; 32],
    pub authority: ComponentRuntimeDirectoryAuthority,
}

///
/// ComponentRuntimeDirectorySynchronizationRequest
///
/// Root-issued replacement of one active managed Component node's current Directory authority.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeDirectorySynchronizationRequest {
    pub operation_id: [u8; 32],
    pub authority: ComponentRuntimeDirectoryAuthority,
}

///
/// ComponentRuntimePhase
///
/// Target-local progress from installation through Component runtime activation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ComponentRuntimePhase {
    AwaitingDirectory,
    DirectoryPrepared,
    Active,
}

///
/// ComponentRuntimeActivationEvidence
///
/// Exact retained Directory authority under which one Component runtime became Active.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeActivationEvidence {
    pub directory_authority_hash: [u8; 32],
    pub activated_at_ns: u64,
}

///
/// ComponentRuntimeActivationRequest
///
/// Root-issued exact activation command for one Directory-prepared managed Component node.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeActivationRequest {
    pub operation_id: [u8; 32],
    pub directory_authority_hash: [u8; 32],
}

///
/// ComponentRuntimeStatusResponse
///
/// Independently observable target-local binding and exact retained Directory authority.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeStatusResponse {
    pub operation_id: [u8; 32],
    pub binding: ManagedCanisterBinding,
    pub phase: ComponentRuntimePhase,
    pub authority: Option<ComponentRuntimeDirectoryAuthority>,
    pub authority_hash: Option<[u8; 32]>,
    pub activation: Option<ComponentRuntimeActivationEvidence>,
}

///
/// ComponentRuntimeDirectoryConvergenceEvidence
///
/// Stable root evidence that one active member covered at least the required Directory authority.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeDirectoryConvergenceEvidence {
    pub operation_id: [u8; 32],
    pub binding: ManagedCanisterBinding,
    pub covered_authority: ComponentRuntimeDirectoryAuthority,
    pub covered_authority_hash: [u8; 32],
    pub activation: ComponentRuntimeActivationEvidence,
}

///
/// RootComponentCreationEvidence
///
/// Exact Store artifact and root-owned creation settings frozen before the paid effect.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentCreationEvidence {
    pub wasm_store: Principal,
    pub payload_hash: [u8; 32],
    pub payload_size_bytes: u64,
    pub initial_cycles: Cycles,
    pub controller: Principal,
    pub canister: Option<Principal>,
}

///
/// RootComponentInstallEvidence
///
/// Exact raw artifact, chunk source and immutable target binding frozen before installation.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentInstallEvidence {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentBinding,
}

///
/// RootComponentChildInstallEvidence
///
/// Exact child module and immutable retained binding frozen before installation.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildInstallEvidence {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentChildBinding,
}

///
/// RootComponentAllocationResponse
///
/// Durable identity reservation returned identically for exact operation retry.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentAllocationResponse {
    pub operation_id: [u8; 32],
    pub allocation_sequence: u64,
    pub component: ComponentInstanceId,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub role: CanisterRole,
    pub provisioning_origin: ComponentProvisioningOrigin,
    pub release_set: FleetSubnetRootReleaseSet,
    pub phase: RootComponentAllocationPhase,
    pub creation: Option<RootComponentCreationEvidence>,
    pub installation: Option<RootComponentInstallEvidence>,
}

///
/// RootComponentChildAllocationResponse
///
/// Durable direct-child lifecycle progress returned identically for exact parent retry.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildAllocationResponse {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub parent_canister_id: Principal,
    pub parent_role: CanisterRole,
    pub child_role: CanisterRole,
    pub child_kind: ComponentChildKind,
    pub maximum_instances_per_parent: u32,
    pub maximum_descendants: u32,
    pub maximum_registry_bytes: u64,
    pub reserved_against_registry: ComponentRegistryHead,
    pub release_set: FleetSubnetRootReleaseSet,
    pub phase: RootComponentAllocationPhase,
    pub creation: Option<RootComponentCreationEvidence>,
    pub installation: Option<RootComponentChildInstallEvidence>,
}

///
/// RootComponentSubtreeRemovalResponse
///
/// Current durable snapshot of one monotonic subtree-removal operation.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalResponse {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub target_canister_id: Principal,
    pub target_parent_canister_id: Principal,
    pub target_role: CanisterRole,
    pub target_status: ComponentLifecycleStatus,
    pub reserved_against_registry: ComponentRegistryHead,
    pub traversal_steps: u32,
    pub phase: RootComponentSubtreeRemovalPhase,
}

///
/// RootComponentChildCommitResponse
///
/// Exact committed child operation, authoritative Component Registry and next Directory head.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildCommitResponse {
    pub allocation: RootComponentChildAllocationResponse,
    pub registry: ComponentRegistryPartitionResponse,
    pub directory: ComponentDirectoryHead,
}

///
/// RootComponentChildDirectoryPreparationResponse
///
/// Exact child preparation plus stable bounded active-member Directory coverage.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildDirectoryPreparationResponse {
    pub committed: RootComponentChildCommitResponse,
    pub child: ComponentRuntimeStatusResponse,
    pub owning_component: ComponentRuntimeDirectoryConvergenceEvidence,
    pub parent: Option<ComponentRuntimeDirectoryConvergenceEvidence>,
}

///
/// RootComponentChildRuntimeActivationResponse
///
/// Exact child commitment plus independently observed Directory-bound runtime activation.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildRuntimeActivationResponse {
    pub committed: RootComponentChildCommitResponse,
    pub child: ComponentRuntimeStatusResponse,
}

///
/// RootComponentChildMembershipActivationResponse
///
/// Original child commitment plus active Registry, Directory and target convergence evidence.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildMembershipActivationResponse {
    pub committed: RootComponentChildCommitResponse,
    pub registry: ComponentRegistryPartitionResponse,
    pub directory: ComponentDirectoryHead,
    pub child: ComponentRuntimeStatusResponse,
}

///
/// RootComponentCommitResponse
///
/// Exact committed allocation, authoritative Registry row and derived Directory head.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentCommitResponse {
    pub allocation: RootComponentAllocationResponse,
    pub registry: ComponentRegistryPartitionResponse,
    pub directory: ComponentDirectoryHead,
}

///
/// RootComponentDirectoryPreparationResponse
///
/// Exact root authority plus independently observed target-local Directory preparation.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentDirectoryPreparationResponse {
    pub committed: RootComponentCommitResponse,
    pub target: ComponentRuntimeStatusResponse,
}

///
/// RootComponentRuntimeActivationResponse
///
/// Exact root authority plus independently observed target-local runtime activation.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentRuntimeActivationResponse {
    pub committed: RootComponentCommitResponse,
    pub target: ComponentRuntimeStatusResponse,
}

///
/// RootComponentMembershipActivationResponse
///
/// Exact active Registry authority plus independently observed current target Directory.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentMembershipActivationResponse {
    pub allocation: RootComponentAllocationResponse,
    pub registry: ComponentRegistryPartitionResponse,
    pub directory: ComponentDirectoryHead,
    pub target: ComponentRuntimeStatusResponse,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::root_store::RootStoreBootstrapRequest,
        ids::{
            AppId, CanonicalNetworkId, FleetCoordinatorBinding, FleetId, FleetKey,
            FleetRegistryAuthority, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
        },
    };

    #[test]
    fn component_registry_contracts_round_trip_through_candid() {
        let request = RootComponentRegistryPreparationRequest {
            store_bootstrap: RootStoreBootstrapRequest {
                manifest_payload_size_bytes: 128,
            },
            expected_fleet_registry: FleetRegistryVersion {
                authority: fleet_registry_authority(),
                revision: 4,
                content_hash: [5; 32],
            },
        };
        let response = RootComponentRegistryStatusResponse {
            fleet_subnet_root: Principal::from_slice(&[6; 29]),
            prepared_against_registry: request.expected_fleet_registry.clone(),
            release_set: FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [7; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([8; 32]),
            },
            component_topology_digest: ComponentTopologyDigest::from_bytes([9; 32]),
            next_allocation_sequence: 1,
            reserved_component_instances: 0,
            committed_component_instances: 0,
            managed_descendants: 0,
            known_created_component_canisters: 0,
            encoded_bytes: 0,
            initial_inventory: Some(RootComponentInitialInventoryStatus {
                fleet_activation_operation_id: [10; 32],
                component_count: 0,
                inventory_hash: [11; 32],
                sealed_at_ns: 12,
                directories_converged: true,
                root_runtime_activated: true,
            }),
        };
        let allocation = RootComponentAllocationResponse {
            operation_id: [10; 32],
            allocation_sequence: 1,
            component: ComponentInstanceId::from_generated_bytes([11; 32]),
            component_spec: "projects".parse().expect("Component Spec ID"),
            spec_hash: [12; 32],
            role: CanisterRole::new("project_hub"),
            provisioning_origin: ComponentProvisioningOrigin::FleetAdministrator {
                caller: Principal::from_slice(&[13; 29]),
            },
            release_set: response.release_set,
            phase: RootComponentAllocationPhase::Reserved,
            creation: None,
            installation: None,
        };
        let created = RootComponentAllocationResponse {
            phase: RootComponentAllocationPhase::Created,
            creation: Some(RootComponentCreationEvidence {
                wasm_store: Principal::from_slice(&[14; 29]),
                payload_hash: [15; 32],
                payload_size_bytes: 4_096,
                initial_cycles: Cycles::new(5_000_000_000_000),
                controller: Principal::from_slice(&[6; 29]),
                canister: Some(Principal::from_slice(&[16; 29])),
            }),
            installation: None,
            ..allocation.clone()
        };
        let request_bytes = candid::encode_one(&request).expect("encode request");
        let response_bytes = candid::encode_one(&response).expect("encode response");
        let allocation_bytes = candid::encode_one(&allocation).expect("encode allocation");
        let created_bytes = candid::encode_one(&created).expect("encode created allocation");

        assert_eq!(
            candid::decode_one::<RootComponentRegistryPreparationRequest>(&request_bytes)
                .expect("decode request"),
            request
        );
        assert_eq!(
            candid::decode_one::<RootComponentRegistryStatusResponse>(&response_bytes)
                .expect("decode response"),
            response
        );
        assert_eq!(
            candid::decode_one::<RootComponentAllocationResponse>(&allocation_bytes)
                .expect("decode allocation"),
            allocation
        );
        assert_eq!(
            candid::decode_one::<RootComponentAllocationResponse>(&created_bytes)
                .expect("decode created allocation"),
            created
        );
    }

    #[test]
    fn component_commit_response_round_trips_through_candid() {
        let root = Principal::from_slice(&[6; 29]);
        let component = ComponentInstanceId::from_generated_bytes([11; 32]);
        let component_spec: ComponentSpecId = "projects".parse().expect("Component Spec ID");
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [7; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([8; 32]),
        };
        let provisioning_origin = ComponentProvisioningOrigin::FleetAdministrator {
            caller: Principal::from_slice(&[13; 29]),
        };
        let binding = ComponentBinding {
            authority: fleet_registry_authority(),
            component,
            component_spec: component_spec.clone(),
            spec_hash: [12; 32],
            role: CanisterRole::new("project_hub"),
            placement_subnet: SubnetId::from_principal(Principal::from_slice(&[17; 29])),
            fleet_subnet_root: root,
            canister_id: Principal::from_slice(&[16; 29]),
        };
        let head = ComponentRegistryHead {
            component,
            revision: 1,
            content_hash: [18; 32],
        };
        let committed = RootComponentCommitResponse {
            allocation: RootComponentAllocationResponse {
                operation_id: [10; 32],
                allocation_sequence: 1,
                component,
                component_spec,
                spec_hash: binding.spec_hash,
                role: binding.role.clone(),
                provisioning_origin: provisioning_origin.clone(),
                release_set,
                phase: RootComponentAllocationPhase::Committed,
                creation: Some(RootComponentCreationEvidence {
                    wasm_store: Principal::from_slice(&[14; 29]),
                    payload_hash: [15; 32],
                    payload_size_bytes: 4_096,
                    initial_cycles: Cycles::new(5_000_000_000_000),
                    controller: root,
                    canister: Some(binding.canister_id),
                }),
                installation: Some(RootComponentInstallEvidence {
                    raw_module_hash: [20; 32],
                    chunk_hashes: vec![vec![21; 32]],
                    binding: binding.clone(),
                }),
            },
            registry: ComponentRegistryPartitionResponse {
                head: head.clone(),
                binding: binding.clone(),
                provisioning_origin,
                release_set,
                status: ComponentLifecycleStatus::Prepared,
                reserved_descendants: 0,
                committed_descendants: 0,
                encoded_bytes: 2_048,
            },
            directory: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: binding,
                    source_fleet_subnet_root: root,
                    component_registry_revision: head.revision,
                    component_registry_content_hash: head.content_hash,
                    synchronized_at_ns: 19,
                },
                descendant_count: 0,
            },
        };
        let committed_bytes = candid::encode_one(&committed).expect("encode committed allocation");

        assert_eq!(
            candid::decode_one::<RootComponentCommitResponse>(&committed_bytes)
                .expect("decode committed allocation"),
            committed
        );
    }

    #[test]
    fn component_directory_page_contracts_round_trip_through_candid() {
        let root = Principal::from_slice(&[6; 29]);
        let component = ComponentInstanceId::from_generated_bytes([11; 32]);
        let binding = ComponentBinding {
            authority: fleet_registry_authority(),
            component,
            component_spec: "projects".parse().expect("Component Spec ID"),
            spec_hash: [12; 32],
            role: CanisterRole::new("project_hub"),
            placement_subnet: SubnetId::from_principal(Principal::from_slice(&[17; 29])),
            fleet_subnet_root: root,
            canister_id: Principal::from_slice(&[16; 29]),
        };
        let directory = ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: binding.clone(),
                source_fleet_subnet_root: root,
                component_registry_revision: 3,
                component_registry_content_hash: [18; 32],
                synchronized_at_ns: 19,
            },
            descendant_count: 1,
        };
        let request = ComponentDirectoryPageRequest {
            directory: directory.clone(),
            parent_canister_id: Some(binding.canister_id),
            role: Some(CanisterRole::new("project_instance")),
            status: Some(ComponentLifecycleStatus::Active),
            cursor: Some(ComponentDirectoryPageCursor(vec![20; 64])),
            limit: 50,
        };
        let response = ComponentDirectoryPageResponse {
            directory,
            entries: vec![ComponentDirectoryChildEntry {
                binding: ComponentChildBinding {
                    component: binding.clone(),
                    parent_canister_id: binding.canister_id,
                    role: CanisterRole::new("project_instance"),
                    canister_id: Principal::from_slice(&[21; 29]),
                },
                kind: ComponentChildKind::Instance,
                installed_artifact_hash: [22; 32],
                status: ComponentLifecycleStatus::Active,
            }],
            next_cursor: Some(ComponentDirectoryPageCursor(vec![23; 64])),
        };
        let request_bytes = candid::encode_one(&request).expect("encode Directory page request");
        let response_bytes = candid::encode_one(&response).expect("encode Directory page response");

        assert_eq!(
            candid::decode_one::<ComponentDirectoryPageRequest>(&request_bytes)
                .expect("decode Directory page request"),
            request
        );
        assert_eq!(
            candid::decode_one::<ComponentDirectoryPageResponse>(&response_bytes)
                .expect("decode Directory page response"),
            response
        );
    }

    fn fleet_registry_authority() -> FleetRegistryAuthority {
        FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: crate::ids::FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([1; 32]),
                    },
                    app: AppId::from("toko"),
                },
                coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[2; 29])),
                coordinator: Principal::from_slice(&[3; 29]),
            },
            epoch: 1,
        }
    }

    #[test]
    fn component_creation_request_round_trips_through_candid() {
        let request = RootComponentCreationRequest {
            operation_id: [10; 32],
        };
        let bytes = candid::encode_one(request).expect("encode creation request");

        assert_eq!(
            candid::decode_one::<RootComponentCreationRequest>(&bytes)
                .expect("decode creation request"),
            request
        );
    }

    #[test]
    fn component_subtree_removal_contracts_round_trip_through_candid() {
        let component = ComponentInstanceId::from_generated_bytes([41; 32]);
        let registry = ComponentRegistryHead {
            component,
            revision: 7,
            content_hash: [42; 32],
        };
        let request = RootComponentSubtreeRemovalRequest {
            operation_id: [43; 32],
            component,
            target_canister_id: Principal::from_slice(&[44; 29]),
            expected_registry: registry.clone(),
        };
        let status_request = RootComponentSubtreeRemovalStatusRequest {
            operation_id: request.operation_id,
            component,
        };
        let advance_request = RootComponentSubtreeRemovalAdvanceRequest {
            operation_id: request.operation_id,
            component,
            expected_traversal_steps: 1,
        };
        let stop_request = RootComponentSubtreeRemovalStopPreparationRequest {
            operation_id: request.operation_id,
            component,
            expected_traversal_steps: 2,
            expected_leaf_canister_id: Principal::from_slice(&[46; 29]),
            expected_leaf_parent_canister_id: request.target_canister_id,
        };
        let stopped = RootComponentSubtreeRemovalStoppedReceipt {
            observed_module_hash: [49; 32],
            stop: RootComponentSubtreeRemovalStopIntent {
                controller: Principal::from_slice(&[48; 29]),
                leaf: RootComponentSubtreeRemovalNode {
                    canister_id: Principal::from_slice(&[46; 29]),
                    parent_canister_id: request.target_canister_id,
                    role: CanisterRole::new("project_ledger"),
                    kind: ComponentChildKind::Singleton,
                    installed_artifact_hash: [47; 32],
                    status: ComponentLifecycleStatus::Active,
                },
            },
        };
        let response = RootComponentSubtreeRemovalResponse {
            operation_id: request.operation_id,
            component,
            target_canister_id: request.target_canister_id,
            target_parent_canister_id: Principal::from_slice(&[45; 29]),
            target_role: CanisterRole::new("project_instance"),
            target_status: ComponentLifecycleStatus::Active,
            reserved_against_registry: registry,
            traversal_steps: 2,
            phase: RootComponentSubtreeRemovalPhase::Deleted(
                RootComponentSubtreeRemovalDeletedReceipt {
                    deletion: RootComponentSubtreeRemovalDeleteIntent { stopped },
                },
            ),
        };

        let request_bytes = candid::encode_one(&request).expect("encode subtree removal request");
        let advance_bytes =
            candid::encode_one(advance_request).expect("encode subtree removal advance request");
        let stop_bytes =
            candid::encode_one(stop_request).expect("encode subtree removal stop request");
        let status_bytes =
            candid::encode_one(status_request).expect("encode subtree removal status request");
        let response_bytes =
            candid::encode_one(&response).expect("encode subtree removal response");

        assert_eq!(
            candid::decode_one::<RootComponentSubtreeRemovalRequest>(&request_bytes)
                .expect("decode subtree removal request"),
            request
        );
        assert_eq!(
            candid::decode_one::<RootComponentSubtreeRemovalAdvanceRequest>(&advance_bytes)
                .expect("decode subtree removal advance request"),
            advance_request
        );
        assert_eq!(
            candid::decode_one::<RootComponentSubtreeRemovalStopPreparationRequest>(&stop_bytes)
                .expect("decode subtree removal stop request"),
            stop_request
        );
        assert_eq!(
            candid::decode_one::<RootComponentSubtreeRemovalStatusRequest>(&status_bytes)
                .expect("decode subtree removal status request"),
            status_request
        );
        assert_eq!(
            candid::decode_one::<RootComponentSubtreeRemovalResponse>(&response_bytes)
                .expect("decode subtree removal response"),
            response
        );
    }

    #[test]
    fn component_subtree_removal_deletion_requests_round_trip_through_candid() {
        let prepare = RootComponentSubtreeRemovalDeletePreparationRequest {
            operation_id: [50; 32],
            component: ComponentInstanceId::from_generated_bytes([51; 32]),
            expected_traversal_steps: 3,
            expected_leaf_canister_id: Principal::from_slice(&[52; 29]),
            expected_leaf_parent_canister_id: Principal::from_slice(&[53; 29]),
        };
        let request = RootComponentSubtreeRemovalDeleteRequest {
            operation_id: prepare.operation_id,
            component: prepare.component,
            expected_traversal_steps: prepare.expected_traversal_steps,
            expected_leaf_canister_id: prepare.expected_leaf_canister_id,
            expected_leaf_parent_canister_id: prepare.expected_leaf_parent_canister_id,
        };
        let prepare_bytes = candid::encode_one(prepare)
            .expect("encode subtree removal deletion preparation request");
        let request_bytes =
            candid::encode_one(request).expect("encode subtree removal deletion request");

        assert_eq!(
            candid::decode_one::<RootComponentSubtreeRemovalDeletePreparationRequest>(
                &prepare_bytes
            )
            .expect("decode subtree removal deletion preparation request"),
            prepare
        );
        assert_eq!(
            candid::decode_one::<RootComponentSubtreeRemovalDeleteRequest>(&request_bytes)
                .expect("decode subtree removal deletion request"),
            request
        );
    }

    #[test]
    fn component_subtree_removal_stop_request_round_trips_through_candid() {
        let request = RootComponentSubtreeRemovalStopRequest {
            operation_id: [50; 32],
            component: ComponentInstanceId::from_generated_bytes([51; 32]),
            expected_traversal_steps: 3,
            expected_leaf_canister_id: Principal::from_slice(&[52; 29]),
            expected_leaf_parent_canister_id: Principal::from_slice(&[53; 29]),
        };
        let bytes =
            candid::encode_one(request).expect("encode subtree removal stop execution request");

        assert_eq!(
            candid::decode_one::<RootComponentSubtreeRemovalStopRequest>(&bytes)
                .expect("decode subtree removal stop execution request"),
            request
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one round-trip test keeps the complete child lifecycle boundary coherent"
    )]
    fn component_child_lifecycle_contracts_round_trip_through_candid() {
        let component = ComponentInstanceId::from_generated_bytes([11; 32]);
        let registry = ComponentRegistryHead {
            component,
            revision: 2,
            content_hash: [12; 32],
        };
        let request = RootComponentChildAllocationRequest {
            operation_id: [13; 32],
            component,
            expected_registry: registry.clone(),
            child_role: CanisterRole::new("project_instance"),
        };
        let status_request = RootComponentChildAllocationStatusRequest {
            operation_id: request.operation_id,
            component,
        };
        let creation_request = RootComponentChildCreationRequest {
            operation_id: request.operation_id,
            component,
        };
        let install_request = RootComponentChildInstallRequest {
            operation_id: request.operation_id,
            component,
        };
        let commit_request = RootComponentChildCommitRequest {
            operation_id: request.operation_id,
            component,
        };
        let directory_request = RootComponentChildDirectoryPreparationRequest {
            operation_id: request.operation_id,
            component,
        };
        let activation_request = RootComponentChildRuntimeActivationRequest {
            operation_id: request.operation_id,
            component,
        };
        let membership_request = RootComponentChildMembershipActivationRequest {
            operation_id: request.operation_id,
            component,
        };
        let root = Principal::from_slice(&[17; 29]);
        let parent = Principal::from_slice(&[14; 29]);
        let child = Principal::from_slice(&[18; 29]);
        let child_binding = ComponentChildBinding {
            component: ComponentBinding {
                authority: fleet_registry_authority(),
                component,
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [19; 32],
                role: CanisterRole::new("project_hub"),
                placement_subnet: SubnetId::from_principal(Principal::from_slice(&[20; 29])),
                fleet_subnet_root: root,
                canister_id: parent,
            },
            parent_canister_id: parent,
            role: request.child_role.clone(),
            canister_id: child,
        };
        let response = RootComponentChildAllocationResponse {
            operation_id: request.operation_id,
            component,
            parent_canister_id: parent,
            parent_role: CanisterRole::new("project_hub"),
            child_role: request.child_role.clone(),
            child_kind: ComponentChildKind::Instance,
            maximum_instances_per_parent: 10_000,
            maximum_descendants: 20_000,
            maximum_registry_bytes: 16_777_216,
            reserved_against_registry: registry,
            release_set: FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [15; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([16; 32]),
            },
            phase: RootComponentAllocationPhase::Verified,
            creation: Some(RootComponentCreationEvidence {
                wasm_store: Principal::from_slice(&[21; 29]),
                payload_hash: [22; 32],
                payload_size_bytes: 4_096,
                initial_cycles: Cycles::new(5_000_000_000_000),
                controller: root,
                canister: Some(child),
            }),
            installation: Some(RootComponentChildInstallEvidence {
                raw_module_hash: [23; 32],
                chunk_hashes: vec![vec![24; 32]],
                binding: child_binding.clone(),
            }),
        };
        let commit_response = RootComponentChildCommitResponse {
            allocation: response.clone(),
            registry: ComponentRegistryPartitionResponse {
                head: ComponentRegistryHead {
                    component,
                    revision: 3,
                    content_hash: [25; 32],
                },
                binding: child_binding.component.clone(),
                provisioning_origin: ComponentProvisioningOrigin::FleetAdministrator {
                    caller: Principal::from_slice(&[26; 29]),
                },
                release_set: response.release_set,
                status: ComponentLifecycleStatus::Active,
                reserved_descendants: 0,
                committed_descendants: 1,
                encoded_bytes: 8_192,
            },
            directory: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: child_binding.component.clone(),
                    source_fleet_subnet_root: root,
                    component_registry_revision: 3,
                    component_registry_content_hash: [25; 32],
                    synchronized_at_ns: 27,
                },
                descendant_count: 1,
            },
        };
        let runtime_authority = ComponentRuntimeDirectoryAuthority {
            fleet: FleetDirectorySnapshot {
                provenance: crate::dto::fleet_registry::FleetDirectoryProvenance {
                    registry: FleetRegistryVersion {
                        authority: fleet_registry_authority(),
                        revision: 4,
                        content_hash: [28; 32],
                    },
                    source_fleet_subnet_root: root,
                },
                fleet_subnet_roots: vec![
                    crate::dto::fleet_registry::FleetSubnetRootDirectoryEntry {
                        placement_subnet: commit_response.registry.binding.placement_subnet,
                        fleet_subnet_root: root,
                        status: crate::dto::fleet_registry::FleetSubnetRootStatus::Active,
                    },
                ],
            },
            component: commit_response.directory.clone(),
        };
        let activation = ComponentRuntimeActivationEvidence {
            directory_authority_hash: [29; 32],
            activated_at_ns: 30,
        };
        let directory_response = RootComponentChildDirectoryPreparationResponse {
            committed: commit_response.clone(),
            child: ComponentRuntimeStatusResponse {
                operation_id: request.operation_id,
                binding: ManagedCanisterBinding::ComponentChild(child_binding.clone()),
                phase: ComponentRuntimePhase::DirectoryPrepared,
                authority: Some(runtime_authority.clone()),
                authority_hash: Some([31; 32]),
                activation: None,
            },
            owning_component: ComponentRuntimeDirectoryConvergenceEvidence {
                operation_id: [32; 32],
                binding: ManagedCanisterBinding::Component(child_binding.component.clone()),
                covered_authority: runtime_authority.clone(),
                covered_authority_hash: [31; 32],
                activation,
            },
            parent: None,
        };
        let activation_response = RootComponentChildRuntimeActivationResponse {
            committed: commit_response.clone(),
            child: ComponentRuntimeStatusResponse {
                operation_id: request.operation_id,
                binding: ManagedCanisterBinding::ComponentChild(child_binding.clone()),
                phase: ComponentRuntimePhase::Active,
                authority: Some(runtime_authority.clone()),
                authority_hash: Some([31; 32]),
                activation: Some(ComponentRuntimeActivationEvidence {
                    directory_authority_hash: [31; 32],
                    activated_at_ns: 33,
                }),
            },
        };
        let active_directory = ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: child_binding.component.clone(),
                source_fleet_subnet_root: root,
                component_registry_revision: 4,
                component_registry_content_hash: [34; 32],
                synchronized_at_ns: 35,
            },
            descendant_count: 1,
        };
        let active_authority = ComponentRuntimeDirectoryAuthority {
            fleet: runtime_authority.fleet,
            component: active_directory.clone(),
        };
        let membership_response = RootComponentChildMembershipActivationResponse {
            committed: commit_response.clone(),
            registry: ComponentRegistryPartitionResponse {
                head: ComponentRegistryHead {
                    component,
                    revision: 4,
                    content_hash: [34; 32],
                },
                binding: child_binding.component.clone(),
                provisioning_origin: commit_response.registry.provisioning_origin.clone(),
                release_set: commit_response.registry.release_set,
                status: ComponentLifecycleStatus::Active,
                reserved_descendants: 0,
                committed_descendants: 1,
                encoded_bytes: 8_256,
            },
            directory: active_directory,
            child: ComponentRuntimeStatusResponse {
                operation_id: request.operation_id,
                binding: ManagedCanisterBinding::ComponentChild(child_binding),
                phase: ComponentRuntimePhase::Active,
                authority: Some(active_authority),
                authority_hash: Some([36; 32]),
                activation: Some(ComponentRuntimeActivationEvidence {
                    directory_authority_hash: [31; 32],
                    activated_at_ns: 33,
                }),
            },
        };

        let request_bytes = candid::encode_one(&request).expect("encode child reservation");
        let status_bytes =
            candid::encode_one(status_request).expect("encode child reservation status");
        let creation_bytes =
            candid::encode_one(creation_request).expect("encode child creation request");
        let install_bytes =
            candid::encode_one(install_request).expect("encode child install request");
        let response_bytes = candid::encode_one(&response).expect("encode child response");
        let commit_request_bytes =
            candid::encode_one(commit_request).expect("encode child commit request");
        let directory_request_bytes =
            candid::encode_one(directory_request).expect("encode child Directory request");
        let activation_request_bytes =
            candid::encode_one(activation_request).expect("encode child activation request");
        let membership_request_bytes =
            candid::encode_one(membership_request).expect("encode child membership request");
        let commit_response_bytes =
            candid::encode_one(&commit_response).expect("encode child commit response");
        let directory_response_bytes =
            candid::encode_one(&directory_response).expect("encode child Directory response");
        let activation_response_bytes =
            candid::encode_one(&activation_response).expect("encode child activation response");
        let membership_response_bytes =
            candid::encode_one(&membership_response).expect("encode child membership response");

        assert_eq!(
            candid::decode_one::<RootComponentChildAllocationRequest>(&request_bytes)
                .expect("decode child reservation"),
            request
        );
        assert_eq!(
            candid::decode_one::<RootComponentChildAllocationStatusRequest>(&status_bytes)
                .expect("decode child reservation status"),
            status_request
        );
        assert_eq!(
            candid::decode_one::<RootComponentChildCreationRequest>(&creation_bytes)
                .expect("decode child creation request"),
            creation_request
        );
        assert_eq!(
            candid::decode_one::<RootComponentChildInstallRequest>(&install_bytes)
                .expect("decode child install request"),
            install_request
        );
        assert_eq!(
            candid::decode_one::<RootComponentChildAllocationResponse>(&response_bytes)
                .expect("decode child response"),
            response
        );
        assert_eq!(
            candid::decode_one::<RootComponentChildCommitRequest>(&commit_request_bytes)
                .expect("decode child commit request"),
            commit_request
        );
        assert_eq!(
            candid::decode_one::<RootComponentChildDirectoryPreparationRequest>(
                &directory_request_bytes
            )
            .expect("decode child Directory request"),
            directory_request
        );
        assert_eq!(
            candid::decode_one::<RootComponentChildCommitResponse>(&commit_response_bytes)
                .expect("decode child commit response"),
            commit_response
        );
        assert_eq!(
            candid::decode_one::<RootComponentChildDirectoryPreparationResponse>(
                &directory_response_bytes
            )
            .expect("decode child Directory response"),
            directory_response
        );
        assert_eq!(
            candid::decode_one::<RootComponentChildRuntimeActivationRequest>(
                &activation_request_bytes
            )
            .expect("decode child activation request"),
            activation_request
        );
        assert_eq!(
            candid::decode_one::<RootComponentChildRuntimeActivationResponse>(
                &activation_response_bytes
            )
            .expect("decode child activation response"),
            activation_response
        );
        assert_eq!(
            candid::decode_one::<RootComponentChildMembershipActivationRequest>(
                &membership_request_bytes
            )
            .expect("decode child membership request"),
            membership_request
        );
        assert_eq!(
            candid::decode_one::<RootComponentChildMembershipActivationResponse>(
                &membership_response_bytes
            )
            .expect("decode child membership response"),
            membership_response
        );
    }

    #[test]
    fn component_install_request_round_trips_through_candid() {
        let request = RootComponentInstallRequest {
            operation_id: [10; 32],
        };
        let bytes = candid::encode_one(request).expect("encode install request");

        assert_eq!(
            candid::decode_one::<RootComponentInstallRequest>(&bytes)
                .expect("decode install request"),
            request
        );
    }

    #[test]
    fn component_commit_request_round_trips_through_candid() {
        let request = RootComponentCommitRequest {
            operation_id: [10; 32],
        };
        let bytes = candid::encode_one(request).expect("encode commit request");

        assert_eq!(
            candid::decode_one::<RootComponentCommitRequest>(&bytes)
                .expect("decode commit request"),
            request
        );
    }

    #[test]
    fn component_runtime_activation_requests_round_trip_through_candid() {
        let root_request = RootComponentRuntimeActivationRequest {
            operation_id: [22; 32],
        };
        let target_request = ComponentRuntimeActivationRequest {
            operation_id: root_request.operation_id,
            directory_authority_hash: [23; 32],
        };
        let membership_request = RootComponentMembershipActivationRequest {
            operation_id: root_request.operation_id,
        };
        let root_bytes = candid::encode_one(root_request).expect("encode root activation request");
        let target_bytes =
            candid::encode_one(target_request).expect("encode target activation request");
        let membership_bytes =
            candid::encode_one(membership_request).expect("encode membership activation request");

        assert_eq!(
            candid::decode_one::<RootComponentRuntimeActivationRequest>(&root_bytes)
                .expect("decode root activation request"),
            root_request
        );
        assert_eq!(
            candid::decode_one::<ComponentRuntimeActivationRequest>(&target_bytes)
                .expect("decode target activation request"),
            target_request
        );
        assert_eq!(
            candid::decode_one::<RootComponentMembershipActivationRequest>(&membership_bytes)
                .expect("decode membership activation request"),
            membership_request
        );
    }
}
