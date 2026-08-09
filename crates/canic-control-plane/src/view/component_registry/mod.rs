//! Module: view::component_registry
//!
//! Responsibility: model read-only Component Registry authority, lifecycle and root fences.
//! Does not own: persisted records, validation, allocation, or lifecycle mutation.
//! Boundary: Component Registry ops construct these values for workflow consumption.

use crate::ids::WasmStoreBinding;
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::config::schema::ComponentChildKind,
    control_plane_support::model::replay::ReplayCostGuardSettlement,
    dto::{
        component_registry::{
            ComponentLifecycleStatus, ComponentProvisioningOrigin, ComponentRegistryHead,
            ComponentRuntimeActivationEvidence, ComponentRuntimeDirectoryAuthority,
        },
        fleet_registry::FleetRegistryVersion,
        root_store::RootStoreBootstrapRequest,
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentChildBinding, ComponentInstanceId,
        ComponentSpecId, ComponentTopologyDigest, FleetSubnetRootBinding,
        FleetSubnetRootReleaseSet, ManagedCanisterBinding, SubnetId,
    },
};

///
/// ActiveComponentMemberView
///
/// Read-only exact active member plus its current owning Registry head.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveComponentMemberView {
    pub binding: ManagedCanisterBinding,
    pub registry: ComponentRegistryHead,
}

/// Exact partition transition and target request for one Fleet Directory refresh.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentDirectoryRefreshPlanView {
    pub allocation_operation_id: [u8; 32],
    pub previous_registry: ComponentRegistryHead,
    pub registry: ComponentRegistryHead,
    pub directory_synchronized_at_ns: u64,
    pub directory_authority_hash: [u8; 32],
    pub authority: ComponentRuntimeDirectoryAuthority,
}

///
/// RootFleetSubnetDrainingView
///
/// Read-only root-local cutoff for new top-level Component allocation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetDrainingView {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub placement_subnet: SubnetId,
    pub active_registry: FleetRegistryVersion,
    pub component_topology_digest: ComponentTopologyDigest,
    pub active_release_set: FleetSubnetRootReleaseSet,
    pub next_allocation_sequence: u64,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    pub managed_descendants: u32,
    pub known_created_component_canisters: u32,
    pub root_registry_encoded_bytes: u64,
    pub started_at_ns: u64,
    pub final_inventory: Option<RootFleetSubnetFinalInventoryView>,
    pub removal_publication: Option<RootFleetSubnetRemovalPublicationView>,
    pub store_reclamation_intent: Option<RootFleetSubnetStoreReclamationIntentView>,
    pub store_reclamation: Option<RootFleetSubnetStoreReclamationView>,
    pub store_binding_finalization_intent:
        Option<RootFleetSubnetStoreBindingFinalizationIntentView>,
    pub store_binding_finalization: Option<RootFleetSubnetStoreBindingFinalizationView>,
    pub store_deletion_intent: Option<RootFleetSubnetStoreDeletionIntentView>,
    pub store_deletion: Option<RootFleetSubnetStoreDeletionView>,
    pub root_deletion_preparation_intent: Option<RootFleetSubnetDeletionPreparationIntentView>,
    pub root_deletion_preparation: Option<RootFleetSubnetDeletionPreparationView>,
}

///
/// RootFleetSubnetFinalInventoryView
///
/// Read-only terminal Component history and retained write-fenced Store authority.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetFinalInventoryView {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub placement_subnet: SubnetId,
    pub registry: FleetRegistryVersion,
    pub component_topology_digest: ComponentTopologyDigest,
    pub active_release_set: FleetSubnetRootReleaseSet,
    pub next_allocation_sequence: u64,
    pub removed_component_instances: u32,
    pub terminal_component_history_hash: [u8; 32],
    pub root_registry_encoded_bytes: u64,
    pub wasm_store: Principal,
    pub wasm_store_catalog_hash: [u8; 32],
    pub wasm_store_catalog_entries: u32,
    pub wasm_store_occupied_bytes: u64,
    pub wasm_store_template_count: u32,
    pub wasm_store_release_count: u32,
    pub wasm_store_gc_prepared_at_secs: u64,
    pub finalized_at_ns: u64,
    pub inventory_hash: [u8; 32],
}

/// Read-only projection of one exact Coordinator root-removal publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetRemovalPublicationView {
    pub operation_id: [u8; 32],
    pub final_inventory_hash: [u8; 32],
    pub previous_registry: FleetRegistryVersion,
    pub registry: FleetRegistryVersion,
    pub recorded_at_ns: u64,
}

/// Read-only pre-effect authority for reclaiming one logically removed root Store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetStoreReclamationIntentView {
    pub operation_id: [u8; 32],
    pub final_inventory_hash: [u8; 32],
    pub wasm_store: Principal,
    pub prepared_at_ns: u64,
}

/// Read-only terminal receipt for one reclaimed root Store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetStoreReclamationView {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub final_inventory_hash: [u8; 32],
    pub reclaimed_store_bytes: u64,
    pub reclaimed_catalog_entries: u32,
    pub reclaimed_template_count: u32,
    pub reclaimed_release_count: u32,
    pub gc_prepared_at_secs: u64,
    pub gc_started_at_secs: u64,
    pub gc_completed_at_secs: u64,
    pub gc_runs_completed: u32,
    pub completed_at_ns: u64,
    pub reclamation_hash: [u8; 32],
}

/// Exact live terminal Store evidence accepted by Component Registry ops.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetStoreReclamationEvidence {
    pub wasm_store: Principal,
    pub occupied_store_bytes: u64,
    pub catalog_entries: u32,
    pub template_count: u32,
    pub release_count: u32,
    pub gc_prepared_at_secs: u64,
    pub gc_started_at_secs: u64,
    pub gc_completed_at_secs: u64,
    pub gc_runs_completed: u32,
}

/// Read-only pre-effect authority for finalizing one reclaimed Store binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetStoreBindingFinalizationIntentView {
    pub operation_id: [u8; 32],
    pub final_inventory_hash: [u8; 32],
    pub reclamation_hash: [u8; 32],
    pub wasm_store: Principal,
    pub binding: WasmStoreBinding,
    pub source_generation: u64,
    pub prepared_at_ns: u64,
}

/// Exact active publication binding observed before finalization intent is committed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetStoreBindingAuthority {
    pub wasm_store: Principal,
    pub binding: WasmStoreBinding,
    pub source_generation: u64,
}

/// Read-only terminal receipt for one finalized reclaimed Store binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetStoreBindingFinalizationView {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub binding: WasmStoreBinding,
    pub final_inventory_hash: [u8; 32],
    pub reclamation_hash: [u8; 32],
    pub source_generation: u64,
    pub finalized_generation: u64,
    pub finalized_at_secs: u64,
    pub completed_at_ns: u64,
    pub finalization_hash: [u8; 32],
}

/// Exact local publication-state evidence accepted after binding finalization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetStoreBindingFinalizationEvidence {
    pub wasm_store: Principal,
    pub binding: WasmStoreBinding,
    pub source_generation: u64,
    pub finalized_generation: u64,
    pub finalized_at_secs: u64,
}

/// Exact live Store authority observed before physical deletion intent is committed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetStoreDeletionAuthority {
    pub wasm_store: Principal,
    pub binding: WasmStoreBinding,
    pub observed_module_hash: [u8; 32],
    pub observed_controllers: Vec<Principal>,
    pub observed_cycles_before_reclamation: u128,
    pub retained_cycles_target: u128,
}

/// Read-only authority retained before stopping and deleting the reclaimed Store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetStoreDeletionIntentView {
    pub operation_id: [u8; 32],
    pub binding_finalization_hash: [u8; 32],
    pub wasm_store: Principal,
    pub binding: WasmStoreBinding,
    pub observed_module_hash: [u8; 32],
    pub observed_controllers: Vec<Principal>,
    pub observed_cycles_before_reclamation: u128,
    pub retained_cycles_target: u128,
    pub observed_cycles_after_reclamation: Option<u128>,
    pub cycles_reclaimed_at_ns: Option<u64>,
    pub prepared_at_ns: u64,
}

/// Exact live cycle-balance evidence recorded before Store stop/delete may begin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetStoreCycleReclamationEvidence {
    pub observed_cycles_after_reclamation: u128,
    pub cycles_reclaimed_at_ns: u64,
}

/// Exact independently observed Store absence accepted by Component Registry ops.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetStoreDeletionEvidence {
    pub wasm_store: Principal,
    pub binding: WasmStoreBinding,
    pub observed_module_hash: [u8; 32],
    pub observed_controllers: Vec<Principal>,
    pub observed_cycles_before_reclamation: u128,
    pub retained_cycles_target: u128,
    pub observed_cycles_after_reclamation: u128,
    pub cycles_reclaimed_at_ns: u64,
    pub observed_absent_at_ns: u64,
}

/// Read-only terminal receipt for one physically deleted root Store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetStoreDeletionView {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub binding: WasmStoreBinding,
    pub binding_finalization_hash: [u8; 32],
    pub observed_module_hash: [u8; 32],
    pub observed_controllers: Vec<Principal>,
    pub observed_cycles_before_reclamation: u128,
    pub retained_cycles_target: u128,
    pub observed_cycles_after_reclamation: u128,
    pub cycles_reclaimed_at_ns: u64,
    pub prepared_at_ns: u64,
    pub observed_absent_at_ns: u64,
    pub completed_at_ns: u64,
    pub deletion_hash: [u8; 32],
}

/// Root-local preflight evidence frozen before returning cycles to the Coordinator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetDeletionPreparationAuthority {
    pub store_deletion_hash: [u8; 32],
    pub coordinator: Principal,
    pub observed_cycles_before_reclamation: u128,
    pub retained_cycles_target: u128,
    pub observed_reserved_cycles: u128,
    pub observed_idle_cycles_burned_per_day: u128,
    pub observed_freezing_threshold_seconds: u128,
}

/// Read-only root-local authority frozen before returning cycles to the Coordinator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetDeletionPreparationIntentView {
    pub operation_id: [u8; 32],
    pub coordinator: Principal,
    pub final_inventory_hash: [u8; 32],
    pub store_deletion_hash: [u8; 32],
    pub observed_cycles_before_reclamation: u128,
    pub retained_cycles_target: u128,
    pub observed_reserved_cycles: u128,
    pub observed_idle_cycles_burned_per_day: u128,
    pub observed_freezing_threshold_seconds: u128,
    pub coordinator_intent_hash: Option<[u8; 32]>,
    pub observed_cycles_after_reclamation: Option<u128>,
    pub cycles_reclaimed_at_ns: Option<u64>,
    pub prepared_at_ns: u64,
}

/// Read-only local receipt proving the removed root is externally deletable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetDeletionPreparationView {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub coordinator: Principal,
    pub final_inventory_hash: [u8; 32],
    pub store_deletion_hash: [u8; 32],
    pub observed_cycles_before_reclamation: u128,
    pub retained_cycles_target: u128,
    pub observed_reserved_cycles: u128,
    pub observed_idle_cycles_burned_per_day: u128,
    pub observed_freezing_threshold_seconds: u128,
    pub observed_cycles_after_reclamation: u128,
    pub cycles_reclaimed_at_ns: u64,
    pub coordinator_intent_hash: [u8; 32],
    pub coordinator_readiness_hash: [u8; 32],
    pub prepared_at_ns: u64,
    pub completed_at_ns: u64,
}

///
/// RootComponentDrainingView
///
/// Read-only projection of one durable top-level Component draining fence.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentDrainingView {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub previous_registry: ComponentRegistryHead,
    pub registry: ComponentRegistryHead,
    pub descendant_count: u32,
    pub descendant_content_hash: [u8; 32],
    pub directory_authority_hash: [u8; 32],
    pub started_at_ns: u64,
    pub quiescence: Option<RootComponentQuiescenceProgressView>,
    pub final_inventory: Option<RootComponentFinalInventoryView>,
    pub deletion: Option<RootComponentDeletionProgressView>,
}

///
/// RootComponentFinalInventoryView
///
/// Read-only exact empty-inventory authority frozen before top-level deletion.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentFinalInventoryView {
    pub registry: ComponentRegistryHead,
    pub descendant_content_hash: [u8; 32],
    pub registry_encoded_bytes: u64,
    pub directory_synchronized_at_ns: u64,
    pub covered_fleet_registry_revision: u64,
    pub covered_fleet_registry_content_hash: [u8; 32],
    pub directory_authority_hash: [u8; 32],
    pub inventory_hash: [u8; 32],
    pub finalized_at_ns: u64,
}

///
/// RootComponentDeletionIntentView
///
/// Read-only complete authority frozen before top-level Component deletion.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentDeletionIntentView {
    pub final_inventory: RootComponentFinalInventoryView,
    pub quiescence: RootComponentQuiescentReceiptView,
    pub prepared_at_ns: u64,
}

///
/// RootComponentDeletedReceiptView
///
/// Read-only terminal receipt after the top-level workload Canister is recycled.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentDeletedReceiptView {
    pub deletion: RootComponentDeletionIntentView,
    pub deleted_at_ns: u64,
}

///
/// RootComponentMembershipRemovedView
///
/// Read-only terminal local-membership and settled accounting receipt.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentMembershipRemovedView {
    pub deleted: RootComponentDeletedReceiptView,
    pub allocation_operation_id: [u8; 32],
    pub remaining_spec_committed_instances: u32,
    pub root_committed_component_instances: u32,
    pub root_known_created_component_canisters: u32,
    pub root_registry_encoded_bytes: u64,
    pub removed_at_ns: u64,
    pub removal_hash: [u8; 32],
}

///
/// RootComponentDeletionProgressView
///
/// Read-only monotonic top-level deletion progress embedded in one draining fence.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootComponentDeletionProgressView {
    DeleteIntent(RootComponentDeletionIntentView),
    Deleted(RootComponentDeletedReceiptView),
    MembershipRemoved(RootComponentMembershipRemovedView),
}

///
/// RootComponentQuiescenceStopIntentView
///
/// Read-only pre-effect authority for stopping one draining top-level Component.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentQuiescenceStopIntentView {
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
    pub charged_entry_bytes: u64,
}

///
/// RootComponentQuiescentReceiptView
///
/// Read-only terminal evidence for one independently observed stopped Component.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentQuiescentReceiptView {
    pub stop: RootComponentQuiescenceStopIntentView,
    pub observed_module_hash: [u8; 32],
    pub quiesced_at_ns: u64,
}

///
/// RootComponentQuiescenceProgressView
///
/// Read-only monotonic stop progress embedded in one Component draining fence.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootComponentQuiescenceProgressView {
    StopIntent(RootComponentQuiescenceStopIntentView),
    Quiescent(RootComponentQuiescentReceiptView),
}

///
/// RootComponentDrainingAdvanceView
///
/// One bounded driver observation: the current removal operation or exact empty inventory.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootComponentDrainingAdvanceView {
    DescendantSubtreePending {
        operation_id: [u8; 32],
        target_canister_id: Principal,
        reserved_against_registry: ComponentRegistryHead,
    },
    DescendantRemoval(Box<RootComponentSubtreeRemovalView>),
    DescendantsEmpty {
        registry: ComponentRegistryHead,
        descendant_content_hash: [u8; 32],
    },
}

///
/// RootComponentRegistryView
///
/// Read-only durable preparation authority and current allocation counters.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentRegistryView {
    pub root: FleetSubnetRootBinding,
    pub prepared_against_registry: FleetRegistryVersion,
    pub release_set: FleetSubnetRootReleaseSet,
    pub store_bootstrap: RootStoreBootstrapRequest,
    pub next_allocation_sequence: u64,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    pub managed_descendants: u32,
    pub known_created_component_canisters: u32,
    pub encoded_bytes: u64,
    pub initial_inventory: Option<RootComponentInitialInventoryView>,
    pub root_draining: Option<RootFleetSubnetDrainingView>,
}

///
/// RootComponentInitialInventoryView
///
/// Read-only initial Component inventory and root-activation receipt state.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootComponentInitialInventoryView {
    pub fleet_activation_operation_id: [u8; 32],
    pub component_count: u32,
    pub inventory_hash: [u8; 32],
    pub sealed_at_ns: u64,
    pub directories_converged: bool,
    pub root_runtime_activated: bool,
}

///
/// RootComponentAllocationView
///
/// Read-only exact top-level Component identity and capacity reservation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentAllocationView {
    pub operation_id: [u8; 32],
    pub allocation_sequence: u64,
    pub component: ComponentInstanceId,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub role: CanisterRole,
    pub provisioning_origin: ComponentProvisioningOrigin,
    pub release_set: FleetSubnetRootReleaseSet,
    pub progress: RootComponentAllocationProgressView,
}

///
/// RootComponentChildAllocationView
///
/// Read-only exact direct-child lifecycle operation inside one Component Registry partition.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentChildAllocationView {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub parent_canister_id: Principal,
    pub parent_role: CanisterRole,
    pub child_role: CanisterRole,
    pub child_kind: ComponentChildKind,
    pub maximum_instances_per_parent: u32,
    pub maximum_descendants: u32,
    pub maximum_registry_bytes: u64,
    pub application_init_args: Option<Vec<u8>>,
    pub reserved_against_registry: ComponentRegistryHead,
    pub release_set: FleetSubnetRootReleaseSet,
    pub progress: RootComponentChildAllocationProgressView,
}

///
/// RootComponentChildAllocationProgressView
///
/// Read-only paid-effect state for one direct-child allocation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "the read-only child lifecycle mirrors its direct durable authority"
)]
pub enum RootComponentChildAllocationProgressView {
    Reserved,
    CreationIntent(RootComponentCreationEffectView),
    Created {
        effect: RootComponentCreationEffectView,
        canister: Principal,
    },
    InstallIntent {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentChildInstallEffectView,
    },
    Installed {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentChildInstallEffectView,
    },
    Verified {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentChildInstallEffectView,
    },
    Committed {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentChildInstallEffectView,
        commitment: RootComponentChildCommitmentView,
    },
}

///
/// RootComponentSubtreeRemovalView
///
/// Read-only current progress of one child-subtree removal operation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentSubtreeRemovalView {
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
    pub progress: RootComponentSubtreeRemovalProgressView,
}

///
/// RootComponentSubtreeRemovalProgressView
///
/// Read-only durable post-order removal progress.
///

#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "read-only progress preserves the complete durable removal receipt"
)]
pub enum RootComponentSubtreeRemovalProgressView {
    Fenced,
    Traversing {
        cursor: RootComponentSubtreeRemovalNodeView,
    },
    LeafSelected {
        leaf: RootComponentSubtreeRemovalNodeView,
    },
    StopIntent(RootComponentSubtreeStopEffectView),
    Stopped(RootComponentSubtreeStoppedEffectView),
    DeleteIntent(RootComponentSubtreeDeleteEffectView),
    Deleted(RootComponentSubtreeDeletedEffectView),
    MembershipRemoved(RootComponentSubtreeMembershipRemovedView),
    DirectorySynchronized(RootComponentSubtreeDirectorySynchronizedView),
    Completed(RootComponentSubtreeRemovalCompletedView),
}

///
/// RootComponentSubtreeRemovalNodeView
///
/// Read-only exact registered child selected as a traversal cursor or leaf.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentSubtreeRemovalNodeView {
    pub canister_id: Principal,
    pub parent_canister_id: Principal,
    pub role: CanisterRole,
    pub kind: ComponentChildKind,
    pub installed_artifact_hash: [u8; 32],
    pub status: ComponentLifecycleStatus,
}

///
/// RootComponentSubtreeStopEffectView
///
/// Read-only exact leaf and root controller retained before a stop call.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentSubtreeStopEffectView {
    pub leaf: RootComponentSubtreeRemovalNodeView,
    pub controller: Principal,
}

///
/// RootComponentSubtreeStoppedEffectView
///
/// Read-only frozen stop authority and independently observed module hash.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentSubtreeStoppedEffectView {
    pub stop: RootComponentSubtreeStopEffectView,
    pub observed_module_hash: [u8; 32],
}

///
/// RootComponentSubtreeDeleteEffectView
///
/// Read-only stopped receipt retained as exact deletion authority.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentSubtreeDeleteEffectView {
    pub stopped: RootComponentSubtreeStoppedEffectView,
}

///
/// RootComponentSubtreeDeletedEffectView
///
/// Read-only workload-deletion authority retained after Canister recycling.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentSubtreeDeletedEffectView {
    pub deletion: RootComponentSubtreeDeleteEffectView,
}

///
/// RootComponentSubtreeMembershipRemovedView
///
/// Read-only exact Registry transition retained after a deleted leaf is unregistered.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentSubtreeMembershipRemovedView {
    pub deleted: RootComponentSubtreeDeletedEffectView,
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

///
/// RootComponentSubtreeDirectoryConvergenceView
///
/// Read-only compact proof that one surviving member covered the required Directory.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentSubtreeDirectoryConvergenceView {
    pub operation_id: [u8; 32],
    pub canister_id: Principal,
    pub activation: ComponentRuntimeActivationEvidence,
}

///
/// RootComponentSubtreeDirectorySynchronizedView
///
/// Read-only membership removal plus surviving-member convergence receipt.
///
/// The owner is absent only under terminal top-level Component quiescence.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentSubtreeDirectorySynchronizedView {
    pub membership_removed: RootComponentSubtreeMembershipRemovedView,
    pub covered_fleet_registry_revision: u64,
    pub covered_fleet_registry_content_hash: [u8; 32],
    pub covered_component_registry: ComponentRegistryHead,
    pub covered_authority_hash: [u8; 32],
    pub owning_component: Option<RootComponentSubtreeDirectoryConvergenceView>,
    pub parent: Option<RootComponentSubtreeDirectoryConvergenceView>,
}

///
/// RootComponentSubtreeRemovalCompletedView
///
/// Read-only terminal authority after the fenced target is finalized.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentSubtreeRemovalCompletedView {
    pub registry: ComponentRegistryHead,
    pub directory_authority_hash: [u8; 32],
}

///
/// RootComponentChildCommitmentView
///
/// Read-only child-commit Registry head plus later Directory and membership receipts.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentChildCommitmentView {
    pub registry: ComponentRegistryHead,
    pub descendant_content_hash: [u8; 32],
    pub registry_encoded_bytes: u64,
    pub reserved_descendants: u32,
    pub committed_descendants: u32,
    pub directory_synchronized_at_ns: u64,
    pub directory_authority_hash: [u8; 32],
    pub directory_prepared: bool,
    pub runtime_activated: bool,
    pub membership: Option<RootComponentChildMembershipView>,
}

///
/// RootComponentChildMembershipView
///
/// Read-only immutable active child head and current-Directory receipt.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentChildMembershipView {
    pub registry: ComponentRegistryHead,
    pub descendant_content_hash: [u8; 32],
    pub registry_encoded_bytes: u64,
    pub reserved_descendants: u32,
    pub committed_descendants: u32,
    pub directory_synchronized_at_ns: u64,
    pub directory_authority_hash: [u8; 32],
    pub directory_synchronized: bool,
}

///
/// ComponentDirectoryPageSelection
///
/// Registry filters and canonical continuation used for one bounded Directory page.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentDirectoryPageSelection {
    pub parent_canister_id: Option<Principal>,
    pub role: Option<CanisterRole>,
    pub status: Option<ComponentLifecycleStatus>,
    pub start_after: Option<ComponentDirectoryCanonicalCursor>,
}

///
/// ComponentDirectoryCanonicalCursor
///
/// Exact `(parent, role, Canister)` continuation in Directory canonical order.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentDirectoryCanonicalCursor {
    pub parent_canister_id: Principal,
    pub role: CanisterRole,
    pub canister_id: Principal,
}

///
/// ComponentDirectoryChildView
///
/// Read-only normalized child projected with its complete protected binding.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentDirectoryChildView {
    pub binding: ComponentChildBinding,
    pub kind: ComponentChildKind,
    pub installed_artifact_hash: [u8; 32],
    pub status: ComponentLifecycleStatus,
}

///
/// ComponentDirectoryPageView
///
/// Bounded Registry-backed Directory entries plus a filter-matching continuation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentDirectoryPageView {
    pub entries: Vec<ComponentDirectoryChildView>,
    pub next_cursor: Option<ComponentDirectoryCanonicalCursor>,
}

///
/// RootComponentAllocationProgressView
///
/// Read-only paid-effect state for one top-level Component allocation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootComponentAllocationProgressView {
    Reserved,
    CreationIntent(RootComponentCreationEffectView),
    Created {
        effect: RootComponentCreationEffectView,
        canister: Principal,
    },
    InstallIntent {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentInstallEffectView,
    },
    Installed {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentInstallEffectView,
    },
    Verified {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentInstallEffectView,
    },
    Committed {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentInstallEffectView,
        commitment: RootComponentCommitmentView,
    },
    Removed {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentInstallEffectView,
        commitment: RootComponentCommitmentView,
    },
}

///
/// RootComponentCreationEffectView
///
/// Read-only exact Store artifact, creation settings and cost settlement.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentCreationEffectView {
    pub wasm_store: Principal,
    pub payload_hash: [u8; 32],
    pub payload_size_bytes: u64,
    pub initial_cycles: Cycles,
    pub controller: Principal,
    pub cost_guard_settlement: ReplayCostGuardSettlement,
    pub charged_entry_bytes: u64,
}

///
/// RootComponentInstallEffectView
///
/// Read-only exact raw artifact, install source, target identity and cost settlement.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentInstallEffectView {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentBinding,
    pub cost_guard_settlement: ReplayCostGuardSettlement,
    pub charged_entry_bytes: u64,
}

///
/// RootComponentChildInstallEffectView
///
/// Read-only exact child artifact, immutable binding and cost settlement.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentChildInstallEffectView {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentChildBinding,
    pub cost_guard_settlement: ReplayCostGuardSettlement,
    pub charged_entry_bytes: u64,
}

///
/// RootComponentCommitmentView
///
/// Read-only link from one allocation receipt to its Registry and Directory authority.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentCommitmentView {
    pub registry: ComponentRegistryHead,
    pub prepared_registry_encoded_bytes: u64,
    pub directory_synchronized_at_ns: u64,
    pub directory_authority_hash: [u8; 32],
    pub directory_prepared: bool,
    pub runtime_activated: bool,
    pub membership: Option<RootComponentMembershipView>,
}

///
/// RootComponentMembershipView
///
/// Read-only immutable active-membership authority and current-Directory receipt.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentMembershipView {
    pub registry_encoded_bytes: u64,
    pub directory_synchronized_at_ns: u64,
    pub directory_authority_hash: [u8; 32],
    pub directory_synchronized: bool,
}

///
/// ComponentRegistryPartitionView
///
/// Read-only normalized authority for one committed Component tree.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentRegistryPartitionView {
    pub binding: ComponentBinding,
    pub provisioning_origin: ComponentProvisioningOrigin,
    pub release_set: FleetSubnetRootReleaseSet,
    pub status: ComponentLifecycleStatus,
    pub revision: u64,
    pub content_hash: [u8; 32],
    pub descendant_content_hash: [u8; 32],
    pub directory_synchronized_at_ns: u64,
    pub reserved_descendants: u32,
    pub committed_descendants: u32,
    pub encoded_bytes: u64,
}
