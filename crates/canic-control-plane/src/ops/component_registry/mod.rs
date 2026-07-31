//! Module: ops::component_registry
//!
//! Responsibility: read and commit Component Registry authority and lifecycle progress.
//! Does not own: Store, Fleet Registry, topology, admission, or lifecycle validation.
//! Boundary: converts stable records into read-only views before workflow use.

use crate::{
    storage::stable::component_registry::{
        ComponentRegistryChildRecord, ComponentRegistryChildTraversalRecord,
        ComponentRegistryParentRoleCountRecord, ComponentRegistryPartitionRecord,
        RootComponentAllocationCommitError, RootComponentAllocationProgressRecord,
        RootComponentAllocationRecord, RootComponentChildAllocationProgressRecord,
        RootComponentChildAllocationRecord, RootComponentChildCommitmentRecord,
        RootComponentChildInstallEffectRecord, RootComponentChildMembershipRecord,
        RootComponentCommitmentRecord, RootComponentCreationEffectRecord,
        RootComponentDeletedReceiptRecord, RootComponentDeletionIntentRecord,
        RootComponentDeletionProgressRecord, RootComponentDrainingRecord,
        RootComponentFinalInventoryRecord, RootComponentInitialInventoryRecord,
        RootComponentInstallEffectRecord, RootComponentMembershipRecord,
        RootComponentQuiescenceProgressRecord, RootComponentQuiescenceStopIntentRecord,
        RootComponentQuiescentReceiptRecord, RootComponentRegistryCommitError,
        RootComponentRegistryMetaRecord, RootComponentRegistryStore,
        RootComponentSubtreeDeleteEffectRecord, RootComponentSubtreeDeletedEffectRecord,
        RootComponentSubtreeDirectoryConvergenceRecord,
        RootComponentSubtreeDirectorySynchronizedRecord,
        RootComponentSubtreeMembershipRemovedRecord, RootComponentSubtreeRemovalBeginCommit,
        RootComponentSubtreeRemovalCompletedLeafRecord, RootComponentSubtreeRemovalCompletedRecord,
        RootComponentSubtreeRemovalProgressRecord, RootComponentSubtreeRemovalRecord,
        RootComponentSubtreeStopEffectRecord, RootComponentSubtreeStoppedEffectRecord,
    },
    view::component_registry::{
        ComponentDirectoryCanonicalCursor, ComponentDirectoryChildView,
        ComponentDirectoryPageSelection, ComponentDirectoryPageView,
        ComponentRegistryPartitionView, RootComponentAllocationProgressView,
        RootComponentAllocationView, RootComponentChildAllocationProgressView,
        RootComponentChildAllocationView, RootComponentChildCommitmentView,
        RootComponentChildInstallEffectView, RootComponentChildMembershipView,
        RootComponentCommitmentView, RootComponentCreationEffectView,
        RootComponentDeletedReceiptView, RootComponentDeletionIntentView,
        RootComponentDeletionProgressView, RootComponentDrainingAdvanceView,
        RootComponentDrainingView, RootComponentFinalInventoryView,
        RootComponentInitialInventoryView, RootComponentInstallEffectView,
        RootComponentMembershipView, RootComponentQuiescenceProgressView,
        RootComponentQuiescenceStopIntentView, RootComponentQuiescentReceiptView,
        RootComponentRegistryView, RootComponentSubtreeDeleteEffectView,
        RootComponentSubtreeDeletedEffectView, RootComponentSubtreeDirectoryConvergenceView,
        RootComponentSubtreeDirectorySynchronizedView, RootComponentSubtreeMembershipRemovedView,
        RootComponentSubtreeRemovalCompletedView, RootComponentSubtreeRemovalNodeView,
        RootComponentSubtreeRemovalProgressView, RootComponentSubtreeRemovalView,
        RootComponentSubtreeStopEffectView, RootComponentSubtreeStoppedEffectView,
    },
};
use candid::CandidType;
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::{
        config::schema::ComponentChildKind,
        error::InternalError,
        model::replay::ReplayCostGuardSettlement,
        ops::component_runtime::ComponentRuntimeOps,
        policy::{
            component_allocation::TopLevelComponentAllocationDecision,
            component_child_allocation::ComponentChildAllocationDecision,
        },
    },
    dto::{
        component_registry::{
            ComponentDirectoryHead, ComponentDirectoryProvenance, ComponentLifecycleStatus,
            ComponentProvisioningOrigin, ComponentRegistryHead, ComponentRuntimeDirectoryAuthority,
            ComponentRuntimeDirectoryConvergenceEvidence,
        },
        fleet_registry::{FleetDirectorySnapshot, FleetRegistryVersion},
        root_store::RootStoreBootstrapRequest,
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentChildBinding, ComponentInstanceId,
        ComponentSpecId, FleetSubnetRootBinding, FleetSubnetRootReleaseSet, IntentId,
        ManagedCanisterBinding,
    },
};
use sha2::{Digest, Sha256};

const SUBTREE_REMOVAL_TRAVERSAL_BATCH_SIZE: u32 = 64;
const COMPONENT_DRAINING_SUBTREE_OPERATION_DOMAIN: &[u8] =
    b"canic.component-draining.subtree-operation.v1";
const COMPONENT_FINAL_INVENTORY_HASH_DOMAIN: &[u8] = b"canic.component.final-inventory.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SubtreeRemovalOrigin {
    Ordinary,
    DrainingDriver,
}

///
/// ComponentRegistryOps
///
/// Single-step root-local Component Registry meta storage operations.
///

pub struct ComponentRegistryOps;

///
/// ComponentSpecInstanceCounts
///
/// Root-local reserved and committed top-level instance counts for one Component Spec.
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComponentSpecInstanceCounts {
    pub reserved: u32,
    pub committed: u32,
}

///
/// RootComponentInitialInventoryPlan
///
/// Exact sealed initial Component operations consumed by root activation workflow.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentInitialInventoryPlan {
    pub receipt: RootComponentInitialInventoryView,
    pub operation_ids: Vec<[u8; 32]>,
}

#[derive(CandidType)]
struct RootComponentInitialInventoryHashEntry {
    operation_id: [u8; 32],
    allocation_sequence: u64,
    component: ComponentInstanceId,
    component_spec: ComponentSpecId,
    spec_hash: [u8; 32],
    role: CanisterRole,
    provisioning_origin: ComponentProvisioningOrigin,
    release_set: FleetSubnetRootReleaseSet,
    prepared_registry: ComponentRegistryHead,
    prepared_registry_encoded_bytes: u64,
    prepared_directory_synchronized_at_ns: u64,
    prepared_directory_authority_hash: [u8; 32],
    active_binding: ComponentBinding,
    active_registry: ComponentRegistryHead,
    active_registry_encoded_bytes: u64,
    active_directory_synchronized_at_ns: u64,
    active_directory_authority_hash: [u8; 32],
}

#[derive(CandidType)]
struct RootComponentFinalInventoryHashAuthority {
    binding: ComponentBinding,
    provisioning_origin: ComponentProvisioningOrigin,
    release_set: FleetSubnetRootReleaseSet,
    status: ComponentLifecycleStatus,
    registry: ComponentRegistryHead,
    descendant_content_hash: [u8; 32],
    directory_synchronized_at_ns: u64,
    reserved_descendants: u32,
    committed_descendants: u32,
    registry_encoded_bytes: u64,
    covered_fleet_registry_revision: u64,
    covered_fleet_registry_content_hash: [u8; 32],
    directory_authority_hash: [u8; 32],
    finalized_at_ns: u64,
}

#[derive(Debug, Eq, PartialEq)]
struct RootComponentFinalInventorySnapshotAuthority {
    registry: ComponentRegistryHead,
    descendant_content_hash: [u8; 32],
    registry_encoded_bytes: u64,
    directory_synchronized_at_ns: u64,
}

impl RootComponentFinalInventorySnapshotAuthority {
    const fn from_partition(partition: &ComponentRegistryPartitionRecord) -> Self {
        Self {
            registry: component_partition_head(partition),
            descendant_content_hash: partition.descendant_content_hash,
            registry_encoded_bytes: partition.encoded_bytes,
            directory_synchronized_at_ns: partition.directory_synchronized_at_ns,
        }
    }

    fn from_inventory(inventory: &RootComponentFinalInventoryRecord) -> Self {
        Self {
            registry: inventory.registry.clone(),
            descendant_content_hash: inventory.descendant_content_hash,
            registry_encoded_bytes: inventory.registry_encoded_bytes,
            directory_synchronized_at_ns: inventory.directory_synchronized_at_ns,
        }
    }
}

struct RootComponentFinalInventoryAuthority<'a> {
    partition: &'a ComponentRegistryPartitionRecord,
    draining: &'a RootComponentDrainingRecord,
    inventory: &'a RootComponentFinalInventoryRecord,
}

impl RootComponentFinalInventoryAuthority<'_> {
    fn validate(&self) -> Result<(), InternalError> {
        let quiesced_at_ns =
            terminal_component_quiesced_at_ns(self.draining).ok_or_else(Self::invalid)?;
        let current = RootComponentFinalInventorySnapshotAuthority::from_partition(self.partition);
        let frozen = RootComponentFinalInventorySnapshotAuthority::from_inventory(self.inventory);
        if current != frozen {
            return Err(Self::invalid());
        }
        if !component_final_inventory_fleet_coverage_is_versioned(self.inventory) {
            return Err(Self::invalid());
        }
        if self.inventory.directory_authority_hash == [0; 32] {
            return Err(Self::invalid());
        }
        if !component_final_inventory_time_is_monotonic(
            self.partition,
            quiesced_at_ns,
            self.inventory.finalized_at_ns,
        ) {
            return Err(Self::invalid());
        }
        let expected_hash = component_final_inventory_hash(self.partition, self.inventory)?;
        if self.inventory.inventory_hash != expected_hash {
            return Err(Self::invalid());
        }
        if !component_partition_is_empty_and_draining(self.partition) {
            return Err(Self::invalid());
        }
        if !component_final_inventory_indexes_are_empty(self.partition) {
            return Err(Self::invalid());
        }
        if !component_draining_cursor_is_terminal(self.draining) {
            return Err(Self::invalid());
        }
        Ok(())
    }

    fn invalid() -> InternalError {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component final inventory differs from exact empty Registry authority",
        )
    }
}

struct RootComponentDeletionAuthority<'a> {
    draining: &'a RootComponentDrainingRecord,
    progress: &'a RootComponentDeletionProgressRecord,
}

impl RootComponentDeletionAuthority<'_> {
    fn validate(&self) -> Result<(), InternalError> {
        let final_inventory = self
            .draining
            .final_inventory
            .as_ref()
            .ok_or_else(Self::invalid)?;
        let quiescence = terminal_component_quiescence(self.draining).ok_or_else(Self::invalid)?;
        let (intent, deleted_at_ns) = match self.progress {
            RootComponentDeletionProgressRecord::DeleteIntent(intent) => (intent, None),
            RootComponentDeletionProgressRecord::Deleted(receipt) => {
                (&receipt.deletion, Some(receipt.deleted_at_ns))
            }
        };
        if intent.final_inventory != *final_inventory {
            return Err(Self::invalid());
        }
        if intent.quiescence != *quiescence {
            return Err(Self::invalid());
        }
        if intent.prepared_at_ns < final_inventory.finalized_at_ns {
            return Err(Self::invalid());
        }
        if deleted_at_ns.is_some_and(|deleted_at_ns| deleted_at_ns < intent.prepared_at_ns) {
            return Err(Self::invalid());
        }
        if RootComponentRegistryStore::component_draining_entry_bytes(self.draining)
            > quiescence.stop.charged_entry_bytes
        {
            return Err(Self::invalid());
        }
        Ok(())
    }

    fn invalid() -> InternalError {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component deletion progress differs from frozen final authority",
        )
    }
}

struct CompleteInitialInventory {
    component_count: u32,
    inventory_hash: [u8; 32],
    operation_ids: Vec<[u8; 32]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ComponentTreeNodeIdentity<'a> {
    component: ComponentInstanceId,
    parent_canister_id: Principal,
    role: &'a CanisterRole,
    canister_id: Principal,
}

impl<'a> ComponentTreeNodeIdentity<'a> {
    const fn new(
        component: ComponentInstanceId,
        parent_canister_id: Principal,
        role: &'a CanisterRole,
        canister_id: Principal,
    ) -> Self {
        Self {
            component,
            parent_canister_id,
            role,
            canister_id,
        }
    }

    const fn from_child(child: &'a ComponentRegistryChildRecord) -> Self {
        Self::new(
            child.component,
            child.parent_canister_id,
            &child.role,
            child.canister_id,
        )
    }

    const fn from_traversal(traversal: &'a ComponentRegistryChildTraversalRecord) -> Self {
        Self::new(
            traversal.component,
            traversal.parent_canister_id,
            &traversal.role,
            traversal.canister_id,
        )
    }

    fn is_valid_for(self, component: ComponentInstanceId) -> bool {
        self.component == component
            && self.parent_canister_id != Principal::anonymous()
            && self.canister_id != Principal::anonymous()
            && self.parent_canister_id != self.canister_id
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ComponentTreeNodeState<'a> {
    identity: ComponentTreeNodeIdentity<'a>,
    kind: ComponentChildKind,
    installed_artifact_hash: [u8; 32],
    status: ComponentLifecycleStatus,
}

impl<'a> ComponentTreeNodeState<'a> {
    const fn new(
        identity: ComponentTreeNodeIdentity<'a>,
        kind: ComponentChildKind,
        installed_artifact_hash: [u8; 32],
        status: ComponentLifecycleStatus,
    ) -> Self {
        Self {
            identity,
            kind,
            installed_artifact_hash,
            status,
        }
    }

    const fn from_child(child: &'a ComponentRegistryChildRecord) -> Self {
        Self::new(
            ComponentTreeNodeIdentity::from_child(child),
            child.kind,
            child.installed_artifact_hash,
            child.status,
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct ComponentTreeBoundary {
    component: ComponentInstanceId,
    root_canister: Principal,
    fleet_subnet_root: Principal,
    coordinator: Principal,
}

impl ComponentTreeBoundary {
    const fn from_partition(partition: &ComponentRegistryPartitionRecord) -> Self {
        Self {
            component: partition.binding.component,
            root_canister: partition.binding.canister_id,
            fleet_subnet_root: partition.binding.fleet_subnet_root,
            coordinator: partition.binding.authority.binding.coordinator,
        }
    }

    fn admits(self, child: &ComponentRegistryChildRecord) -> bool {
        let protected_child_principals = [
            Principal::anonymous(),
            self.root_canister,
            self.fleet_subnet_root,
            self.coordinator,
        ];
        let protected_parent_principals = [
            Principal::anonymous(),
            self.fleet_subnet_root,
            self.coordinator,
        ];
        child.component == self.component
            && child.canister_id != child.parent_canister_id
            && !protected_child_principals.contains(&child.canister_id)
            && !protected_parent_principals.contains(&child.parent_canister_id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ComponentParentRoleIdentity<'a> {
    component: ComponentInstanceId,
    parent_canister_id: Principal,
    child_role: &'a CanisterRole,
}

impl<'a> ComponentParentRoleIdentity<'a> {
    const fn new(
        component: ComponentInstanceId,
        parent_canister_id: Principal,
        child_role: &'a CanisterRole,
    ) -> Self {
        Self {
            component,
            parent_canister_id,
            child_role,
        }
    }

    const fn from_count(record: &'a ComponentRegistryParentRoleCountRecord) -> Self {
        Self::new(
            record.component,
            record.parent_canister_id,
            &record.child_role,
        )
    }

    const fn from_allocation(record: &'a RootComponentChildAllocationRecord) -> Self {
        Self::new(
            record.component,
            record.parent_canister_id,
            &record.child_role,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ComponentPartitionLifecycleAuthority {
    component: ComponentInstanceId,
    release_set: FleetSubnetRootReleaseSet,
    status: ComponentLifecycleStatus,
}

impl ComponentPartitionLifecycleAuthority {
    const fn active_reservation(record: &RootComponentChildAllocationRecord) -> Self {
        Self {
            component: record.component,
            release_set: record.release_set,
            status: ComponentLifecycleStatus::Active,
        }
    }

    const fn from_partition(partition: &ComponentRegistryPartitionRecord) -> Self {
        Self {
            component: partition.binding.component,
            release_set: partition.release_set,
            status: partition.status,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ComponentPartitionCoverage<'a> {
    current: &'a ComponentRegistryPartitionRecord,
    historical: &'a ComponentRegistryPartitionRecord,
}

impl<'a> ComponentPartitionCoverage<'a> {
    const fn new(
        current: &'a ComponentRegistryPartitionRecord,
        historical: &'a ComponentRegistryPartitionRecord,
    ) -> Self {
        Self {
            current,
            historical,
        }
    }

    fn is_monotonic(self) -> bool {
        match self.current.revision.cmp(&self.historical.revision) {
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => {
                self.current.content_hash == self.historical.content_hash
                    && self.current.descendant_content_hash
                        == self.historical.descendant_content_hash
                    && self.current.directory_synchronized_at_ns
                        == self.historical.directory_synchronized_at_ns
                    && self.current.reserved_descendants >= self.historical.reserved_descendants
                    && self.current.committed_descendants == self.historical.committed_descendants
                    && self.current.encoded_bytes >= self.historical.encoded_bytes
            }
            std::cmp::Ordering::Greater => {
                self.current.directory_synchronized_at_ns
                    >= self.historical.directory_synchronized_at_ns
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ComponentChildActivationEvidence<'a> {
    component: ComponentInstanceId,
    commitment: &'a RootComponentChildCommitmentRecord,
    membership: &'a RootComponentChildMembershipRecord,
}

impl<'a> ComponentChildActivationEvidence<'a> {
    const fn new(
        component: ComponentInstanceId,
        commitment: &'a RootComponentChildCommitmentRecord,
        membership: &'a RootComponentChildMembershipRecord,
    ) -> Self {
        Self {
            component,
            commitment,
            membership,
        }
    }

    fn is_valid(self) -> bool {
        let activation_receipt_is_complete =
            self.commitment.directory_prepared && self.commitment.runtime_activated;
        let registry_authority_advances = self.membership.registry.component == self.component
            && self.membership.registry.revision > self.commitment.registry.revision
            && self.membership.descendant_content_hash != self.commitment.descendant_content_hash;
        let directory_authority_advances = self.membership.directory_synchronized_at_ns
            > self.commitment.directory_synchronized_at_ns
            && self.membership.directory_authority_hash != [0; 32];
        activation_receipt_is_complete
            && registry_authority_advances
            && directory_authority_advances
    }
}

#[derive(Clone, Copy, Debug)]
struct ComponentActivationEvidence<'a> {
    commitment: &'a RootComponentCommitmentRecord,
    membership: &'a RootComponentMembershipRecord,
    current: &'a ComponentRegistryPartitionRecord,
}

impl<'a> ComponentActivationEvidence<'a> {
    const fn new(
        commitment: &'a RootComponentCommitmentRecord,
        membership: &'a RootComponentMembershipRecord,
        current: &'a ComponentRegistryPartitionRecord,
    ) -> Self {
        Self {
            commitment,
            membership,
            current,
        }
    }

    fn is_valid(self) -> bool {
        let activation_receipt_is_complete =
            self.commitment.directory_prepared && self.commitment.runtime_activated;
        let registry_bytes_cover_receipt =
            self.membership.registry_encoded_bytes <= self.current.encoded_bytes;
        let directory_authority_advances = self.membership.directory_synchronized_at_ns
            > self.commitment.directory_synchronized_at_ns
            && self.membership.directory_authority_hash != [0; 32];
        activation_receipt_is_complete
            && registry_bytes_cover_receipt
            && directory_authority_advances
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SubtreeLeafSelection {
    traversal_steps: u32,
    canister_id: Principal,
    parent_canister_id: Principal,
}

impl SubtreeLeafSelection {
    const fn new(
        traversal_steps: u32,
        canister_id: Principal,
        parent_canister_id: Principal,
    ) -> Self {
        Self {
            traversal_steps,
            canister_id,
            parent_canister_id,
        }
    }

    const fn from_record(traversal_steps: u32, leaf: &ComponentRegistryChildRecord) -> Self {
        Self::new(traversal_steps, leaf.canister_id, leaf.parent_canister_id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SubtreeLeafStopAuthority {
    selection: SubtreeLeafSelection,
    controller: Principal,
}

impl SubtreeLeafStopAuthority {
    const fn new(selection: SubtreeLeafSelection, controller: Principal) -> Self {
        Self {
            selection,
            controller,
        }
    }

    const fn from_record(
        traversal_steps: u32,
        stop: &RootComponentSubtreeStopEffectRecord,
    ) -> Self {
        Self::new(
            SubtreeLeafSelection::from_record(traversal_steps, &stop.leaf),
            stop.controller,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SubtreeLeafStoppedAuthority {
    stop: SubtreeLeafStopAuthority,
    observed_module_hash: [u8; 32],
}

impl SubtreeLeafStoppedAuthority {
    const fn from_record(
        traversal_steps: u32,
        stopped: &RootComponentSubtreeStoppedEffectRecord,
    ) -> Self {
        Self {
            stop: SubtreeLeafStopAuthority::from_record(traversal_steps, &stopped.stop),
            observed_module_hash: stopped.observed_module_hash,
        }
    }
}

const fn retained_subtree_stop_effect(
    progress: &RootComponentSubtreeRemovalProgressRecord,
) -> Option<&RootComponentSubtreeStopEffectRecord> {
    match progress {
        RootComponentSubtreeRemovalProgressRecord::StopIntent(effect) => Some(effect),
        RootComponentSubtreeRemovalProgressRecord::Stopped(receipt) => Some(&receipt.stop),
        RootComponentSubtreeRemovalProgressRecord::DeleteIntent(deletion) => {
            Some(&deletion.stopped.stop)
        }
        RootComponentSubtreeRemovalProgressRecord::Deleted(receipt) => {
            Some(&receipt.deletion.stopped.stop)
        }
        RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) => {
            Some(&receipt.deleted.deletion.stopped.stop)
        }
        RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) => {
            Some(&receipt.membership_removed.deleted.deletion.stopped.stop)
        }
        RootComponentSubtreeRemovalProgressRecord::Fenced
        | RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
        | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
        | RootComponentSubtreeRemovalProgressRecord::Completed(_) => None,
    }
}

const fn retained_subtree_stopped_effect(
    progress: &RootComponentSubtreeRemovalProgressRecord,
) -> Option<&RootComponentSubtreeStoppedEffectRecord> {
    match progress {
        RootComponentSubtreeRemovalProgressRecord::Stopped(receipt) => Some(receipt),
        RootComponentSubtreeRemovalProgressRecord::DeleteIntent(deletion) => {
            Some(&deletion.stopped)
        }
        RootComponentSubtreeRemovalProgressRecord::Deleted(receipt) => {
            Some(&receipt.deletion.stopped)
        }
        RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) => {
            Some(&receipt.deleted.deletion.stopped)
        }
        RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) => {
            Some(&receipt.membership_removed.deleted.deletion.stopped)
        }
        RootComponentSubtreeRemovalProgressRecord::Fenced
        | RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
        | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
        | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
        | RootComponentSubtreeRemovalProgressRecord::Completed(_) => None,
    }
}

const fn retained_subtree_deleted_effect(
    progress: &RootComponentSubtreeRemovalProgressRecord,
) -> Option<&RootComponentSubtreeDeletedEffectRecord> {
    match progress {
        RootComponentSubtreeRemovalProgressRecord::Deleted(receipt) => Some(receipt),
        RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) => {
            Some(&receipt.deleted)
        }
        RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) => {
            Some(&receipt.membership_removed.deleted)
        }
        RootComponentSubtreeRemovalProgressRecord::Fenced
        | RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
        | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
        | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
        | RootComponentSubtreeRemovalProgressRecord::Stopped(_)
        | RootComponentSubtreeRemovalProgressRecord::DeleteIntent(_)
        | RootComponentSubtreeRemovalProgressRecord::Completed(_) => None,
    }
}

///
/// RootComponentCreationPlan
///
/// Exact artifact and root-owned settings selected before a creation effect is admitted.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentCreationPlan {
    pub wasm_store: Principal,
    pub payload_hash: [u8; 32],
    pub payload_size_bytes: u64,
    pub initial_cycles: Cycles,
    pub controller: Principal,
}

#[derive(Debug, Eq, PartialEq)]
struct RootComponentCreationAuthority<'a> {
    wasm_store: Principal,
    payload_hash: [u8; 32],
    payload_size_bytes: u64,
    initial_cycles: &'a Cycles,
    controller: Principal,
}

impl<'a> From<&'a RootComponentCreationPlan> for RootComponentCreationAuthority<'a> {
    fn from(plan: &'a RootComponentCreationPlan) -> Self {
        Self {
            wasm_store: plan.wasm_store,
            payload_hash: plan.payload_hash,
            payload_size_bytes: plan.payload_size_bytes,
            initial_cycles: &plan.initial_cycles,
            controller: plan.controller,
        }
    }
}

impl<'a> From<&'a RootComponentCreationEffectView> for RootComponentCreationAuthority<'a> {
    fn from(effect: &'a RootComponentCreationEffectView) -> Self {
        Self {
            wasm_store: effect.wasm_store,
            payload_hash: effect.payload_hash,
            payload_size_bytes: effect.payload_size_bytes,
            initial_cycles: &effect.initial_cycles,
            controller: effect.controller,
        }
    }
}

impl RootComponentCreationPlan {
    pub(crate) fn matches_effect(&self, effect: &RootComponentCreationEffectView) -> bool {
        RootComponentCreationAuthority::from(self) == RootComponentCreationAuthority::from(effect)
    }

    fn has_valid_store_artifact(&self) -> bool {
        self.wasm_store != Principal::anonymous()
            && self.payload_hash != [0; 32]
            && self.payload_size_bytes > 0
    }
}

///
/// RootComponentInstallPlan
///
/// Exact module source and immutable target binding selected before installation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentInstallPlan {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentBinding,
    pub maximum_registry_bytes: u64,
}

#[derive(Debug, Eq, PartialEq)]
struct RootComponentInstallAuthority<'a> {
    raw_module_hash: [u8; 32],
    chunk_hashes: &'a [Vec<u8>],
    binding: &'a ComponentBinding,
}

impl<'a> From<&'a RootComponentInstallPlan> for RootComponentInstallAuthority<'a> {
    fn from(plan: &'a RootComponentInstallPlan) -> Self {
        Self {
            raw_module_hash: plan.raw_module_hash,
            chunk_hashes: &plan.chunk_hashes,
            binding: &plan.binding,
        }
    }
}

impl<'a> From<&'a RootComponentInstallEffectView> for RootComponentInstallAuthority<'a> {
    fn from(effect: &'a RootComponentInstallEffectView) -> Self {
        Self {
            raw_module_hash: effect.raw_module_hash,
            chunk_hashes: &effect.chunk_hashes,
            binding: &effect.binding,
        }
    }
}

impl RootComponentInstallPlan {
    pub(crate) fn matches_effect(&self, effect: &RootComponentInstallEffectView) -> bool {
        RootComponentInstallAuthority::from(self) == RootComponentInstallAuthority::from(effect)
    }
}

///
/// RootComponentChildInstallPlan
///
/// Exact child module source and immutable target binding selected before installation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentChildInstallPlan {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentChildBinding,
    pub maximum_registry_bytes: u64,
}

#[derive(Debug, Eq, PartialEq)]
struct RootComponentChildInstallAuthority<'a> {
    raw_module_hash: [u8; 32],
    chunk_hashes: &'a [Vec<u8>],
    binding: &'a ComponentChildBinding,
}

#[derive(Debug, Eq, PartialEq)]
struct ComponentChildInstallReservationAuthority<'a> {
    binding: &'a ComponentChildBinding,
    partition: ComponentPartitionLifecycleAuthority,
    root_release_set: FleetSubnetRootReleaseSet,
    maximum_registry_bytes: u64,
}

impl<'a> From<&'a RootComponentChildInstallPlan> for RootComponentChildInstallAuthority<'a> {
    fn from(plan: &'a RootComponentChildInstallPlan) -> Self {
        Self {
            raw_module_hash: plan.raw_module_hash,
            chunk_hashes: &plan.chunk_hashes,
            binding: &plan.binding,
        }
    }
}

impl<'a> From<&'a RootComponentChildInstallEffectView> for RootComponentChildInstallAuthority<'a> {
    fn from(effect: &'a RootComponentChildInstallEffectView) -> Self {
        Self {
            raw_module_hash: effect.raw_module_hash,
            chunk_hashes: &effect.chunk_hashes,
            binding: &effect.binding,
        }
    }
}

impl RootComponentChildInstallPlan {
    pub(crate) fn matches_effect(&self, effect: &RootComponentChildInstallEffectView) -> bool {
        RootComponentChildInstallAuthority::from(self)
            == RootComponentChildInstallAuthority::from(effect)
    }
}

impl ComponentRegistryOps {
    pub(crate) fn current() -> Option<RootComponentRegistryView> {
        RootComponentRegistryStore::current().map(record_to_view)
    }

    pub(crate) fn seal_initial_inventory(
        fleet_activation_operation_id: [u8; 32],
        sealed_at_ns: u64,
    ) -> Result<RootComponentInitialInventoryPlan, InternalError> {
        if sealed_at_ns == 0 {
            return Err(InternalError::invalid_input(
                "initial Component inventory seal time must be positive",
            ));
        }
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let inventory = complete_initial_inventory(&current)?;
        if let Some(existing) = current.initial_inventory {
            validate_initial_inventory_receipt(
                &existing,
                fleet_activation_operation_id,
                inventory.component_count,
                inventory.inventory_hash,
            )?;
            return Ok(RootComponentInitialInventoryPlan {
                receipt: initial_inventory_record_to_view(existing),
                operation_ids: inventory.operation_ids,
            });
        }

        let receipt = RootComponentInitialInventoryRecord {
            fleet_activation_operation_id,
            component_count: inventory.component_count,
            inventory_hash: inventory.inventory_hash,
            sealed_at_ns,
            directories_converged: false,
            root_runtime_activated: false,
        };
        let mut next = current.clone();
        next.initial_inventory = Some(receipt);
        RootComponentRegistryStore::replace_meta(&current, next)
            .map_err(map_allocation_commit_error)?;
        Ok(RootComponentInitialInventoryPlan {
            receipt: initial_inventory_record_to_view(receipt),
            operation_ids: inventory.operation_ids,
        })
    }

    pub(crate) fn validate_sealed_initial_inventory(
        fleet_activation_operation_id: [u8; 32],
    ) -> Result<RootComponentInitialInventoryPlan, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let receipt = current.initial_inventory.ok_or_else(|| {
            InternalError::unavailable("initial Component inventory has not been sealed")
        })?;
        let inventory = complete_initial_inventory(&current)?;
        validate_initial_inventory_receipt(
            &receipt,
            fleet_activation_operation_id,
            inventory.component_count,
            inventory.inventory_hash,
        )?;
        Ok(RootComponentInitialInventoryPlan {
            receipt: initial_inventory_record_to_view(receipt),
            operation_ids: inventory.operation_ids,
        })
    }

    pub(crate) fn initial_inventory(
        fleet_activation_operation_id: [u8; 32],
    ) -> Result<RootComponentInitialInventoryView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let receipt = current.initial_inventory.ok_or_else(|| {
            InternalError::unavailable("initial Component inventory has not been sealed")
        })?;
        if receipt.fleet_activation_operation_id != fleet_activation_operation_id {
            return Err(InternalError::conflict(
                "initial Component inventory is bound to a different Fleet activation",
            ));
        }
        Ok(initial_inventory_record_to_view(receipt))
    }

    pub(crate) fn mark_initial_inventory_directories_converged(
        fleet_activation_operation_id: [u8; 32],
        expected_inventory_hash: [u8; 32],
    ) -> Result<RootComponentInitialInventoryView, InternalError> {
        update_initial_inventory_receipt(
            fleet_activation_operation_id,
            expected_inventory_hash,
            true,
            false,
        )
    }

    pub(crate) fn mark_initial_inventory_root_runtime_activated(
        fleet_activation_operation_id: [u8; 32],
        expected_inventory_hash: [u8; 32],
    ) -> Result<RootComponentInitialInventoryView, InternalError> {
        update_initial_inventory_receipt(
            fleet_activation_operation_id,
            expected_inventory_hash,
            true,
            true,
        )
    }

    pub(crate) fn prepare(
        root: FleetSubnetRootBinding,
        prepared_against_registry: FleetRegistryVersion,
        release_set: FleetSubnetRootReleaseSet,
        store_bootstrap: RootStoreBootstrapRequest,
    ) -> Result<RootComponentRegistryView, InternalError> {
        let record = RootComponentRegistryMetaRecord {
            root,
            prepared_against_registry,
            release_set,
            store_bootstrap,
            next_allocation_sequence: 1,
            reserved_component_instances: 0,
            committed_component_instances: 0,
            managed_descendants: 0,
            known_created_component_canisters: 0,
            encoded_bytes: 0,
            initial_inventory: None,
        };
        RootComponentRegistryStore::prepare(record.clone()).map_err(|error| match error {
            RootComponentRegistryCommitError::ConflictingState => InternalError::conflict(
                "root Component Registry is already prepared under different authority",
            ),
        })?;
        Ok(record_to_view(record))
    }

    pub(crate) fn allocation(operation_id: [u8; 32]) -> Option<RootComponentAllocationView> {
        RootComponentRegistryStore::allocation(operation_id).map(allocation_record_to_view)
    }

    pub(crate) fn component_spec_counts(
        component_spec: &ComponentSpecId,
    ) -> Result<ComponentSpecInstanceCounts, InternalError> {
        let (reserved, committed) = RootComponentRegistryStore::allocation_counts(component_spec);
        Ok(ComponentSpecInstanceCounts {
            reserved: u32::try_from(reserved).map_err(|_| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root Component reservation count exceeds u32",
                )
            })?,
            committed: u32::try_from(committed).map_err(|_| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root committed Component count exceeds u32",
                )
            })?,
        })
    }

    pub(crate) fn reserve_allocation(
        decision: TopLevelComponentAllocationDecision,
        operation_id: [u8; 32],
        provisioning_origin: ComponentProvisioningOrigin,
        root_runtime_active: bool,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentAllocationRecord {
            operation_id,
            allocation_sequence: decision.allocation_sequence,
            component: decision.component,
            component_spec: decision.component_spec,
            spec_hash: decision.spec_hash,
            role: decision.role,
            provisioning_origin,
            release_set: current.release_set,
            progress: RootComponentAllocationProgressRecord::Reserved,
        };
        if let Some(existing) = RootComponentRegistryStore::allocation(operation_id) {
            return if existing == record {
                Ok(allocation_record_to_view(existing))
            } else {
                Err(InternalError::conflict(
                    "Component allocation operation is already bound to different intent",
                ))
            };
        }
        match (current.initial_inventory.as_ref(), root_runtime_active) {
            (Some(_), false) => {
                return Err(InternalError::conflict(
                    "initial Component inventory is sealed while the root runtime is Prepared",
                ));
            }
            (Some(receipt), true) if !receipt.root_runtime_activated => {
                return Err(InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "active root runtime has no terminal initial-inventory receipt",
                ));
            }
            (None, true) => {
                return Err(InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "active root runtime has no sealed initial Component inventory",
                ));
            }
            (None, false) | (Some(_), true) => {}
        }

        if current.next_allocation_sequence != record.allocation_sequence {
            return Err(InternalError::conflict(
                "Component allocation sequence changed before reservation commit",
            ));
        }
        let entry_bytes = RootComponentRegistryStore::allocation_entry_bytes(&record);
        let encoded_bytes = current
            .encoded_bytes
            .checked_add(entry_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if encoded_bytes > current.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "Component Registry reservation requires {encoded_bytes} bytes, exceeding protected limit {}",
                current.root.limits.maximum_registry_bytes
            )));
        }
        let mut next = current.clone();
        next.next_allocation_sequence =
            next.next_allocation_sequence
                .checked_add(1)
                .ok_or_else(|| {
                    InternalError::resource_exhausted("Component allocation sequence is exhausted")
                })?;
        next.reserved_component_instances = next
            .reserved_component_instances
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::resource_exhausted("reserved Component instance count overflow")
            })?;
        next.encoded_bytes = encoded_bytes;

        RootComponentRegistryStore::reserve_allocation(&current, next, record.clone())
            .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(record))
    }

    pub(crate) fn validate_creation_capacity(
        operation_id: [u8; 32],
        plan: &RootComponentCreationPlan,
    ) -> Result<(), InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        if !matches!(
            record.progress,
            RootComponentAllocationProgressRecord::Reserved
        ) {
            return Err(InternalError::conflict(
                "Component allocation has already crossed its creation-intent boundary",
            ));
        }

        let charged_entry_bytes = creation_charged_entry_bytes(&record, plan);
        validate_creation_capacity(&current, &record, charged_entry_bytes).map(|_| ())
    }

    pub(crate) fn begin_creation(
        operation_id: [u8; 32],
        plan: RootComponentCreationPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        if !matches!(
            record.progress,
            RootComponentAllocationProgressRecord::Reserved
        ) {
            return Err(InternalError::conflict(
                "Component allocation has already crossed its creation-intent boundary",
            ));
        }

        let charged_entry_bytes = creation_charged_entry_bytes(&record, &plan);
        let next_encoded_bytes =
            validate_creation_capacity(&current, &record, charged_entry_bytes)?;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::CreationIntent(
            RootComponentCreationEffectRecord {
                wasm_store: plan.wasm_store,
                payload_hash: plan.payload_hash,
                payload_size_bytes: plan.payload_size_bytes,
                initial_cycles: plan.initial_cycles,
                controller: plan.controller,
                cost_guard_settlement,
                charged_entry_bytes,
            },
        );
        validate_charged_record_size(&next_record, charged_entry_bytes)?;

        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_encoded_bytes;
        RootComponentRegistryStore::replace_allocation(
            &current,
            next_meta,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(next_record))
    }

    pub(crate) fn mark_created(
        operation_id: [u8; 32],
        canister: Principal,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let effect = match &record.progress {
            RootComponentAllocationProgressRecord::CreationIntent(effect) => effect.clone(),
            RootComponentAllocationProgressRecord::Created {
                canister: existing, ..
            } if existing == &canister => return Ok(allocation_record_to_view(record)),
            RootComponentAllocationProgressRecord::InstallIntent {
                canister: existing, ..
            }
            | RootComponentAllocationProgressRecord::Installed {
                canister: existing, ..
            }
            | RootComponentAllocationProgressRecord::Verified {
                canister: existing, ..
            }
            | RootComponentAllocationProgressRecord::Committed {
                canister: existing, ..
            } if existing == &canister => return Ok(allocation_record_to_view(record)),
            RootComponentAllocationProgressRecord::Created { .. }
            | RootComponentAllocationProgressRecord::InstallIntent { .. }
            | RootComponentAllocationProgressRecord::Installed { .. }
            | RootComponentAllocationProgressRecord::Verified { .. }
            | RootComponentAllocationProgressRecord::Committed { .. } => {
                return Err(InternalError::conflict(
                    "Component allocation is already bound to a different created Canister",
                ));
            }
            RootComponentAllocationProgressRecord::Reserved => {
                return Err(InternalError::conflict(
                    "Component allocation has no durable creation intent",
                ));
            }
        };
        let charged_entry_bytes = effect.charged_entry_bytes;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::Created { effect, canister };
        validate_charged_record_size(&next_record, charged_entry_bytes)?;
        let mut next_meta = current.clone();
        next_meta.known_created_component_canisters = next_meta
            .known_created_component_canisters
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "known-created Component Canister count overflowed",
                )
            })?;
        let allocated_component_canisters = current
            .reserved_component_instances
            .checked_add(current.committed_component_instances)
            .and_then(|count| count.checked_add(current.managed_descendants))
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "allocated Component-tree Canister count overflowed",
                )
            })?;
        if next_meta.known_created_component_canisters > allocated_component_canisters {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "known-created Component Canisters exceed allocated Component-tree capacity",
            ));
        }
        RootComponentRegistryStore::replace_allocation(
            &current,
            next_meta,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(next_record))
    }

    pub(crate) fn validate_install_capacity(
        operation_id: [u8; 32],
        plan: &RootComponentInstallPlan,
    ) -> Result<(), InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        if !matches!(
            record.progress,
            RootComponentAllocationProgressRecord::Created { .. }
        ) {
            return Err(InternalError::conflict(
                "Component allocation is not ready to cross its install-intent boundary",
            ));
        }

        let charged_entry_bytes = install_charged_entry_bytes(&record, plan)?;
        validate_install_capacity(&current, &record, charged_entry_bytes).map(|_| ())
    }

    pub(crate) fn begin_install(
        operation_id: [u8; 32],
        plan: RootComponentInstallPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let (creation, canister) = match &record.progress {
            RootComponentAllocationProgressRecord::Created { effect, canister } => {
                (effect.clone(), *canister)
            }
            _ => {
                return Err(InternalError::conflict(
                    "Component allocation is not ready for installation",
                ));
            }
        };
        let charged_entry_bytes = install_charged_entry_bytes(&record, &plan)?;
        let next_encoded_bytes = validate_install_capacity(&current, &record, charged_entry_bytes)?;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::InstallIntent {
            creation,
            canister,
            installation: RootComponentInstallEffectRecord {
                raw_module_hash: plan.raw_module_hash,
                chunk_hashes: plan.chunk_hashes,
                binding: plan.binding,
                cost_guard_settlement,
                charged_entry_bytes,
            },
        };
        validate_charged_record_size(&next_record, charged_entry_bytes)?;

        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_encoded_bytes;
        RootComponentRegistryStore::replace_allocation(
            &current,
            next_meta,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(next_record))
    }

    pub(crate) fn renew_install_intent(
        operation_id: [u8; 32],
        plan: &RootComponentInstallPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let (creation, canister, existing) = match &record.progress {
            RootComponentAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            } => (creation.clone(), *canister, installation),
            _ => {
                return Err(InternalError::conflict(
                    "Component allocation has no renewable install intent",
                ));
            }
        };
        validate_install_effect_record(existing, plan)?;
        let charged_entry_bytes = existing.charged_entry_bytes;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::InstallIntent {
            creation,
            canister,
            installation: RootComponentInstallEffectRecord {
                raw_module_hash: plan.raw_module_hash,
                chunk_hashes: plan.chunk_hashes.clone(),
                binding: plan.binding.clone(),
                cost_guard_settlement,
                charged_entry_bytes,
            },
        };
        validate_charged_record_size(&next_record, charged_entry_bytes)?;
        RootComponentRegistryStore::replace_allocation(
            &current,
            current.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(next_record))
    }

    pub(crate) fn mark_installed(
        operation_id: [u8; 32],
    ) -> Result<RootComponentAllocationView, InternalError> {
        advance_install_phase(operation_id, false)
    }

    pub(crate) fn mark_verified(
        operation_id: [u8; 32],
    ) -> Result<RootComponentAllocationView, InternalError> {
        advance_install_phase(operation_id, true)
    }

    pub(crate) fn partition(
        component: canic_core::ids::ComponentInstanceId,
    ) -> Result<Option<ComponentRegistryPartitionView>, InternalError> {
        let Some(record) = RootComponentRegistryStore::partition(component) else {
            return Ok(None);
        };
        validate_partition_record(&record)?;
        Ok(Some(partition_record_to_view(record)))
    }

    pub(crate) fn directory_page(
        component: ComponentInstanceId,
        selection: &ComponentDirectoryPageSelection,
        scan_limit: usize,
    ) -> Result<ComponentDirectoryPageView, InternalError> {
        if scan_limit == 0 {
            return Err(InternalError::invalid_input(
                "Component Directory page scan limit must be positive",
            ));
        }
        if selection.start_after.as_ref().is_some_and(|cursor| {
            selection
                .parent_canister_id
                .is_some_and(|parent| cursor.parent_canister_id != parent)
        }) {
            return Err(InternalError::invalid_input(
                "Component Directory cursor is outside the selected parent",
            ));
        }
        if selection.start_after.as_ref().is_some_and(|cursor| {
            selection.parent_canister_id.is_some()
                && selection
                    .role
                    .as_ref()
                    .is_some_and(|role| role != &cursor.role)
        }) {
            return Err(InternalError::invalid_input(
                "Component Directory cursor is outside the selected parent-role index",
            ));
        }

        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let start_after = selection.start_after.as_ref().map(|cursor| {
            (
                &cursor.parent_canister_id,
                &cursor.role,
                &cursor.canister_id,
            )
        });
        let mut traversals = RootComponentRegistryStore::child_traversals_page(
            component,
            selection.parent_canister_id,
            selection.role.as_ref(),
            start_after,
            scan_limit.saturating_add(1),
        );
        let has_more = traversals.len() > scan_limit;
        traversals.truncate(scan_limit);
        let next_cursor = has_more
            .then(|| traversals.last().map(traversal_record_to_cursor))
            .flatten();

        let mut entries = Vec::with_capacity(traversals.len());
        for traversal in traversals {
            validate_child_traversal_record(component, &traversal)?;
            let child = RootComponentRegistryStore::child(component, traversal.canister_id)
                .ok_or_else(|| {
                    InternalError::invariant(
                        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                        "Component Directory traversal has no normalized child row",
                    )
                })?;
            validate_child_record(&partition, &child)?;
            if ComponentTreeNodeIdentity::from_traversal(&traversal)
                != ComponentTreeNodeIdentity::from_child(&child)
                || RootComponentRegistryStore::component_for_principal(traversal.parent_canister_id)
                    != Some(component)
            {
                return Err(InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component Directory traversal differs from normalized child authority",
                ));
            }
            if selection
                .role
                .as_ref()
                .is_some_and(|role| role != &child.role)
                || selection
                    .status
                    .is_some_and(|status| status != child.status)
            {
                continue;
            }
            entries.push(child_record_to_directory_view(&partition, child));
        }

        Ok(ComponentDirectoryPageView {
            entries,
            next_cursor,
        })
    }

    pub(crate) fn prepared_partition(
        operation_id: [u8; 32],
    ) -> Result<ComponentRegistryPartitionView, InternalError> {
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let RootComponentAllocationProgressRecord::Committed { commitment, .. } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component allocation has no committed Registry authority",
            ));
        };
        exact_committed_partition(&record, commitment).map(partition_record_to_view)
    }

    pub(crate) fn committed_child_authority(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        fleet_directory: &FleetDirectorySnapshot,
    ) -> Result<
        (
            RootComponentChildAllocationView,
            ComponentRegistryPartitionView,
        ),
        InternalError,
    > {
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed { commitment, .. } =
            &record.progress
        else {
            return Err(InternalError::conflict(
                "Component Child allocation has no committed Registry authority",
            ));
        };
        let committed = exact_committed_child_partition(&record, commitment)?;
        validate_child_directory_authority_hash(&committed, fleet_directory, commitment)?;
        Ok((
            child_allocation_record_to_view(record),
            partition_record_to_view(committed),
        ))
    }

    pub(crate) fn component_for_principal(
        canister: Principal,
    ) -> Option<canic_core::ids::ComponentInstanceId> {
        RootComponentRegistryStore::component_for_principal(canister)
    }

    pub(crate) fn registered_parent(
        component: ComponentInstanceId,
        canister: Principal,
    ) -> Result<Option<(ManagedCanisterBinding, ComponentLifecycleStatus)>, InternalError> {
        if RootComponentRegistryStore::component_for_principal(canister) != Some(component) {
            return Ok(None);
        }
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "indexed Component Registry member has no partition",
            )
        })?;
        validate_partition_record(&partition)?;
        if partition.binding.canister_id == canister {
            return Ok(Some((
                ManagedCanisterBinding::Component(partition.binding),
                partition.status,
            )));
        }
        let child = RootComponentRegistryStore::child(component, canister).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "indexed Component Registry member has no normalized child row",
            )
        })?;
        validate_child_record(&partition, &child)?;
        let traversal = ComponentRegistryChildTraversalRecord {
            component,
            parent_canister_id: child.parent_canister_id,
            role: child.role.clone(),
            canister_id: child.canister_id,
        };
        if RootComponentRegistryStore::component_for_principal(child.parent_canister_id)
            != Some(component)
            || RootComponentRegistryStore::child_traversal(
                component,
                traversal.parent_canister_id,
                &traversal.role,
                traversal.canister_id,
            )
            .as_ref()
                != Some(&traversal)
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "indexed Component Registry child differs from its parent or traversal index",
            ));
        }
        Ok(Some((
            ManagedCanisterBinding::ComponentChild(ComponentChildBinding {
                component: partition.binding,
                parent_canister_id: child.parent_canister_id,
                role: child.role,
                canister_id: child.canister_id,
            }),
            child.status,
        )))
    }

    pub(crate) fn child_allocation(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<Option<RootComponentChildAllocationView>, InternalError> {
        let Some(record) = RootComponentRegistryStore::child_allocation(component, operation_id)
        else {
            return Ok(None);
        };
        validate_child_allocation_record(&record)?;
        Ok(Some(child_allocation_record_to_view(record)))
    }

    pub(crate) fn subtree_removal(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<Option<RootComponentSubtreeRemovalView>, InternalError> {
        let Some(record) = RootComponentRegistryStore::subtree_removal(component, operation_id)
        else {
            return Ok(None);
        };
        validate_subtree_removal_record(&record)?;
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component subtree-removal operation has no root Registry authority",
            )
        })?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component subtree-removal operation has no owning partition",
            )
        })?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;
        Ok(Some(subtree_removal_record_to_view(record)))
    }

    pub(crate) fn component_draining(
        component: ComponentInstanceId,
    ) -> Result<Option<RootComponentDrainingView>, InternalError> {
        let Some(record) = RootComponentRegistryStore::component_draining(component) else {
            return Ok(None);
        };
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component draining authority has no Registry partition",
            )
        })?;
        validate_partition_record(&partition)?;
        validate_component_draining_record(&partition, &record)?;
        Ok(Some(component_draining_record_to_view(record)))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one synchronous transition validates, derives and atomically charges the complete draining fence"
    )]
    pub(crate) fn begin_component_draining(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_registry: ComponentRegistryHead,
        started_at_ns: u64,
        maximum_component_registry_bytes: u64,
        fleet_directory: FleetDirectorySnapshot,
    ) -> Result<RootComponentDrainingView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        if let Some(existing) = RootComponentRegistryStore::component_draining(component) {
            validate_component_draining_record(&partition, &existing)?;
            return if existing.operation_id == operation_id
                && existing.previous_registry == expected_registry
            {
                Ok(component_draining_record_to_view(existing))
            } else {
                Err(InternalError::conflict(
                    "Component draining is already bound to different intent",
                ))
            };
        }
        let current_registry = component_partition_head(&partition);
        if operation_id == [0; 32]
            || partition.status != ComponentLifecycleStatus::Active
            || expected_registry != current_registry
        {
            return Err(InternalError::conflict(
                "Component draining authority changed before durable mutation",
            ));
        }
        if started_at_ns <= partition.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input(
                "Component draining authority must advance its Directory time",
            ));
        }
        if partition.reserved_descendants != 0 {
            return Err(InternalError::unavailable(
                "Component has an incomplete child lifecycle operation",
            ));
        }
        for allocation in RootComponentRegistryStore::child_allocations(component) {
            validate_child_allocation_record(&allocation)?;
            if !child_allocation_is_terminal(&allocation) {
                return Err(InternalError::unavailable(
                    "Component has an incomplete child lifecycle operation",
                ));
            }
        }
        for removal in RootComponentRegistryStore::subtree_removals(component) {
            validate_subtree_removal_record(&removal)?;
            validate_subtree_removal_root(&removal, &current.root)?;
            validate_subtree_removal_progress(&partition, &removal)?;
            if !matches!(
                removal.progress,
                RootComponentSubtreeRemovalProgressRecord::Completed(_)
            ) {
                return Err(InternalError::unavailable(
                    "Component has an in-progress subtree-removal operation",
                ));
            }
        }

        let revision = partition.revision.checked_add(1).ok_or_else(|| {
            InternalError::resource_exhausted("Component Registry revision overflow")
        })?;
        let content_hash = component_partition_content_hash(
            &partition.binding,
            &partition.provisioning_origin,
            partition.release_set,
            ComponentLifecycleStatus::Draining,
            revision,
            partition.descendant_content_hash,
            partition.committed_descendants,
        )?;
        let mut next_partition = partition.clone();
        next_partition.status = ComponentLifecycleStatus::Draining;
        next_partition.revision = revision;
        next_partition.content_hash = content_hash;
        next_partition.directory_synchronized_at_ns = started_at_ns;
        let registry = component_partition_head(&next_partition);
        let record = RootComponentDrainingRecord {
            operation_id,
            component,
            previous_registry: current_registry,
            registry,
            descendant_count: next_partition.committed_descendants,
            descendant_content_hash: next_partition.descendant_content_hash,
            directory_authority_hash: component_directory_authority_hash(
                &next_partition.binding,
                next_partition.revision,
                next_partition.content_hash,
                started_at_ns,
                next_partition.committed_descendants,
                &fleet_directory,
            )?,
            started_at_ns,
            quiescence: None,
            subtree_operation_id: None,
            final_inventory: None,
            deletion: None,
        };
        let (next_partition, next_meta) = component_draining_state(
            &current,
            &partition,
            next_partition,
            &record,
            maximum_component_registry_bytes,
        )?;
        validate_partition_record(&next_partition)?;
        validate_component_draining_record(&next_partition, &record)?;
        RootComponentRegistryStore::begin_component_draining(
            &current,
            next_meta,
            &partition,
            next_partition,
            record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(component_draining_record_to_view(record))
    }

    pub(crate) fn prepare_component_quiescence(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_registry: ComponentRegistryHead,
        evidence: ComponentRuntimeDirectoryConvergenceEvidence,
        expected_module_hash: [u8; 32],
        prepared_at_ns: u64,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentDrainingView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record =
            RootComponentRegistryStore::component_draining(component).ok_or_else(|| {
                InternalError::unavailable("Component has not been durably fenced for draining")
            })?;
        validate_partition_record(&partition)?;
        validate_component_draining_record(&partition, &record)?;
        if operation_id != record.operation_id || expected_registry != record.registry {
            return Err(InternalError::conflict(
                "Component quiescence request differs from its durable draining fence",
            ));
        }
        if record.quiescence.is_some() {
            return Ok(component_draining_record_to_view(record));
        }
        if component_partition_head(&partition) != record.registry
            || partition.committed_descendants != record.descendant_count
            || partition.descendant_content_hash != record.descendant_content_hash
        {
            return Err(InternalError::conflict(
                "Component quiescence must be prepared before descendant removal begins",
            ));
        }
        if expected_module_hash == [0; 32] || prepared_at_ns < record.started_at_ns {
            return Err(InternalError::invalid_input(
                "Component quiescence requires qualified module and time evidence",
            ));
        }
        let expected_binding = ManagedCanisterBinding::Component(partition.binding.clone());
        let (coverage, convergence) =
            subtree_directory_convergence_record(&partition, &expected_binding, evidence)?;
        if coverage.component_registry_revision != record.registry.revision
            || coverage.component_registry_content_hash != record.registry.content_hash
        {
            return Err(InternalError::conflict(
                "Component quiescence Directory evidence differs from its draining fence",
            ));
        }

        let mut intent = RootComponentQuiescenceStopIntentRecord {
            registry: record.registry.clone(),
            descendant_count: record.descendant_count,
            descendant_content_hash: record.descendant_content_hash,
            canister_id: partition.binding.canister_id,
            controller: partition.binding.fleet_subnet_root,
            expected_module_hash,
            covered_fleet_registry_revision: coverage.fleet_registry_revision,
            covered_fleet_registry_content_hash: coverage.fleet_registry_content_hash,
            covered_authority_hash: coverage.authority_hash,
            runtime_operation_id: convergence.operation_id,
            activation: convergence.activation,
            prepared_at_ns,
            charged_entry_bytes: 0,
        };
        intent.charged_entry_bytes = component_quiescence_terminal_entry_bytes(&record, &intent)?;
        let mut next_record = record.clone();
        next_record.quiescence = Some(RootComponentQuiescenceProgressRecord::StopIntent(intent));
        let (next_partition, next_meta) = component_quiescence_intent_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        validate_partition_record(&next_partition)?;
        validate_component_draining_record(&next_partition, &next_record)?;
        RootComponentRegistryStore::prepare_component_quiescence(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(component_draining_record_to_view(next_record))
    }

    pub(crate) fn mark_component_quiescent(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        observed_module_hash: [u8; 32],
        quiesced_at_ns: u64,
    ) -> Result<RootComponentDrainingView, InternalError> {
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record =
            RootComponentRegistryStore::component_draining(component).ok_or_else(|| {
                InternalError::unavailable("Component has not been durably fenced for draining")
            })?;
        validate_partition_record(&partition)?;
        validate_component_draining_record(&partition, &record)?;
        if operation_id != record.operation_id {
            return Err(InternalError::conflict(
                "Component quiescence operation differs from its draining fence",
            ));
        }
        let intent = match &record.quiescence {
            Some(RootComponentQuiescenceProgressRecord::StopIntent(intent)) => intent,
            Some(RootComponentQuiescenceProgressRecord::Quiescent(receipt)) => {
                return if receipt.observed_module_hash == observed_module_hash {
                    Ok(component_draining_record_to_view(record))
                } else {
                    Err(InternalError::conflict(
                        "Component quiescence receipt differs from observed module authority",
                    ))
                };
            }
            None => {
                return Err(InternalError::unavailable(
                    "Component quiescence stop intent has not been durably prepared",
                ));
            }
        };
        if observed_module_hash != intent.expected_module_hash
            || quiesced_at_ns < intent.prepared_at_ns
        {
            return Err(InternalError::conflict(
                "Component quiescence observation differs from its durable stop intent",
            ));
        }
        let receipt = RootComponentQuiescentReceiptRecord {
            stop: intent.clone(),
            observed_module_hash,
            quiesced_at_ns,
        };
        let mut next_record = record.clone();
        next_record.quiescence = Some(RootComponentQuiescenceProgressRecord::Quiescent(receipt));
        validate_component_draining_record(&partition, &next_record)?;
        RootComponentRegistryStore::mark_component_quiescent(&record, next_record.clone())
            .map_err(map_allocation_commit_error)?;
        Ok(component_draining_record_to_view(next_record))
    }

    pub(crate) fn subtree_removal_completed_leaf_matches(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        traversal_steps: u32,
        leaf_canister_id: Principal,
        leaf_parent_canister_id: Principal,
    ) -> Result<bool, InternalError> {
        let Some(removal) = RootComponentRegistryStore::subtree_removal(component, operation_id)
        else {
            return Ok(false);
        };
        let Some(partition) = RootComponentRegistryStore::partition(component) else {
            return Ok(false);
        };
        let selection =
            SubtreeLeafSelection::new(traversal_steps, leaf_canister_id, leaf_parent_canister_id);
        completed_subtree_leaf_for_selection(&removal, &partition, selection)
            .map(|leaf| leaf.is_some())
    }

    pub(crate) fn advance_component_draining(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<RootComponentDrainingAdvanceView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let draining =
            RootComponentRegistryStore::component_draining(component).ok_or_else(|| {
                InternalError::unavailable(
                    "Component draining operation has not been durably fenced",
                )
            })?;
        validate_component_draining_record(&partition, &draining)?;
        if operation_id != draining.operation_id
            || partition.status != ComponentLifecycleStatus::Draining
            || !component_has_terminal_quiescence(&partition)?
        {
            return Err(InternalError::conflict(
                "Component draining advance differs from terminal quiescent authority",
            ));
        }

        if let Some(subtree_operation_id) = draining.subtree_operation_id {
            let existing =
                RootComponentRegistryStore::subtree_removal(component, subtree_operation_id)
                    .ok_or_else(|| {
                        InternalError::invariant(
                            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                            "Component draining cursor has no durable subtree removal",
                        )
                    })?;
            validate_subtree_removal_record(&existing)?;
            validate_subtree_removal_root(&existing, &current.root)?;
            validate_subtree_removal_progress(&partition, &existing)?;
            if !matches!(
                existing.progress,
                RootComponentSubtreeRemovalProgressRecord::Completed(_)
            ) {
                return Ok(RootComponentDrainingAdvanceView::DescendantRemoval(
                    Box::new(subtree_removal_record_to_view(existing)),
                ));
            }
        }

        let Some(target) = first_registered_child(&partition, partition.binding.canister_id)?
        else {
            if partition.committed_descendants != 0
                || partition.descendant_content_hash
                    != empty_component_descendant_content_hash(component)
            {
                return Err(InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component draining inventory has descendants without a direct root child",
                ));
            }
            return Ok(RootComponentDrainingAdvanceView::DescendantsEmpty {
                registry: component_partition_head(&partition),
                descendant_content_hash: partition.descendant_content_hash,
            });
        };
        let subtree_operation_id =
            component_draining_subtree_operation_id(&draining, target.canister_id);
        if let Some(existing) =
            RootComponentRegistryStore::subtree_removal(component, subtree_operation_id)
        {
            validate_subtree_removal_record(&existing)?;
            validate_subtree_removal_root(&existing, &current.root)?;
            validate_subtree_removal_progress(&partition, &existing)?;
            if existing.target != target {
                return Err(InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component draining cursor differs from its derived subtree operation",
                ));
            }
            return Ok(RootComponentDrainingAdvanceView::DescendantRemoval(
                Box::new(subtree_removal_record_to_view(existing)),
            ));
        }

        Ok(RootComponentDrainingAdvanceView::DescendantSubtreePending {
            operation_id: subtree_operation_id,
            target_canister_id: target.canister_id,
            reserved_against_registry: component_partition_head(&partition),
        })
    }

    pub(crate) fn finalize_component_inventory(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_registry: ComponentRegistryHead,
        fleet_directory: FleetDirectorySnapshot,
        finalized_at_ns: u64,
    ) -> Result<RootComponentFinalInventoryView, InternalError> {
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let draining =
            RootComponentRegistryStore::component_draining(component).ok_or_else(|| {
                InternalError::unavailable(
                    "Component draining operation has not been durably fenced",
                )
            })?;
        validate_component_draining_record(&partition, &draining)?;
        if operation_id != draining.operation_id {
            return Err(InternalError::conflict(
                "Component final inventory is bound to different draining intent",
            ));
        }
        if let Some(existing) = draining.final_inventory {
            return if expected_registry == existing.registry {
                Ok(component_final_inventory_record_to_view(existing))
            } else {
                Err(InternalError::conflict(
                    "Component final inventory is already bound to a different Registry head",
                ))
            };
        }

        let current_registry = component_partition_head(&partition);
        ensure_component_final_inventory_candidate(&partition, &expected_registry)?;
        let quiesced_at_ns = terminal_component_quiesced_at_ns(&draining).ok_or_else(|| {
            InternalError::conflict("Component final inventory requires terminal quiescence")
        })?;
        ensure_component_final_inventory_time(&partition, quiesced_at_ns, finalized_at_ns)?;
        ensure_component_final_inventory_indexes_are_empty(&partition)?;
        ensure_component_lifecycle_history_is_terminal(&partition)?;
        ensure_component_final_inventory_fleet_authority(&partition, &fleet_directory)?;

        let mut inventory = RootComponentFinalInventoryRecord {
            registry: current_registry,
            descendant_content_hash: partition.descendant_content_hash,
            registry_encoded_bytes: partition.encoded_bytes,
            directory_synchronized_at_ns: partition.directory_synchronized_at_ns,
            covered_fleet_registry_revision: fleet_directory.provenance.registry.revision,
            covered_fleet_registry_content_hash: fleet_directory.provenance.registry.content_hash,
            directory_authority_hash: component_directory_authority_hash(
                &partition.binding,
                partition.revision,
                partition.content_hash,
                partition.directory_synchronized_at_ns,
                0,
                &fleet_directory,
            )?,
            inventory_hash: [0; 32],
            finalized_at_ns,
        };
        inventory.inventory_hash = component_final_inventory_hash(&partition, &inventory)?;
        let mut next_draining = draining.clone();
        next_draining.final_inventory = Some(inventory.clone());
        validate_component_draining_record(&partition, &next_draining)?;
        RootComponentRegistryStore::mark_component_final_inventory(&draining, next_draining)
            .map_err(map_allocation_commit_error)?;
        Ok(component_final_inventory_record_to_view(inventory))
    }

    pub(crate) fn prepare_component_deletion(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_inventory_hash: [u8; 32],
        prepared_at_ns: u64,
    ) -> Result<RootComponentDrainingView, InternalError> {
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let draining =
            RootComponentRegistryStore::component_draining(component).ok_or_else(|| {
                InternalError::unavailable(
                    "Component draining operation has not been durably fenced",
                )
            })?;
        validate_partition_record(&partition)?;
        validate_component_draining_record(&partition, &draining)?;
        ensure_component_deletion_operation(&draining, operation_id)?;
        if let Some(progress) = &draining.deletion {
            ensure_component_deletion_inventory(progress, expected_inventory_hash)?;
            return Ok(component_draining_record_to_view(draining));
        }

        let final_inventory = draining.final_inventory.clone().ok_or_else(|| {
            InternalError::unavailable("Component final inventory has not been durably frozen")
        })?;
        if final_inventory.inventory_hash != expected_inventory_hash {
            return Err(InternalError::conflict(
                "Component deletion request differs from frozen final inventory",
            ));
        }
        let quiescence = terminal_component_quiescence(&draining)
            .cloned()
            .ok_or_else(|| {
                InternalError::unavailable("Component deletion requires terminal quiescence")
            })?;
        if prepared_at_ns < final_inventory.finalized_at_ns {
            return Err(InternalError::invalid_input(
                "Component deletion preparation time precedes final inventory",
            ));
        }

        let mut next_draining = draining.clone();
        next_draining.deletion = Some(RootComponentDeletionProgressRecord::DeleteIntent(
            RootComponentDeletionIntentRecord {
                final_inventory,
                quiescence,
                prepared_at_ns,
            },
        ));
        validate_component_draining_record(&partition, &next_draining)?;
        RootComponentRegistryStore::prepare_component_deletion(&draining, next_draining.clone())
            .map_err(map_allocation_commit_error)?;
        Ok(component_draining_record_to_view(next_draining))
    }

    pub(crate) fn mark_component_deleted(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_inventory_hash: [u8; 32],
        deleted_at_ns: u64,
    ) -> Result<RootComponentDrainingView, InternalError> {
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let draining =
            RootComponentRegistryStore::component_draining(component).ok_or_else(|| {
                InternalError::unavailable(
                    "Component draining operation has not been durably fenced",
                )
            })?;
        validate_partition_record(&partition)?;
        validate_component_draining_record(&partition, &draining)?;
        ensure_component_deletion_operation(&draining, operation_id)?;
        let Some(progress) = &draining.deletion else {
            return Err(InternalError::unavailable(
                "Component deletion intent has not been durably prepared",
            ));
        };
        ensure_component_deletion_inventory(progress, expected_inventory_hash)?;
        let intent = match progress {
            RootComponentDeletionProgressRecord::DeleteIntent(intent) => intent,
            RootComponentDeletionProgressRecord::Deleted(_) => {
                return Ok(component_draining_record_to_view(draining));
            }
        };
        if deleted_at_ns < intent.prepared_at_ns {
            return Err(InternalError::invalid_input(
                "Component deletion observation time precedes its durable intent",
            ));
        }

        let mut next_draining = draining.clone();
        next_draining.deletion = Some(RootComponentDeletionProgressRecord::Deleted(
            RootComponentDeletedReceiptRecord {
                deletion: intent.clone(),
                deleted_at_ns,
            },
        ));
        validate_component_draining_record(&partition, &next_draining)?;
        RootComponentRegistryStore::mark_component_deleted(&draining, next_draining.clone())
            .map_err(map_allocation_commit_error)?;
        Ok(component_draining_record_to_view(next_draining))
    }

    pub(crate) fn begin_draining_subtree_removal(
        component: ComponentInstanceId,
        draining_operation_id: [u8; 32],
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let RootComponentDrainingAdvanceView::DescendantSubtreePending {
            operation_id,
            target_canister_id,
            reserved_against_registry,
        } = Self::advance_component_draining(component, draining_operation_id)?
        else {
            return Err(InternalError::conflict(
                "Component draining has no new direct subtree to fence",
            ));
        };
        Self::begin_subtree_removal_with_origin(
            component,
            operation_id,
            target_canister_id,
            reserved_against_registry,
            maximum_component_registry_bytes,
            SubtreeRemovalOrigin::DrainingDriver,
        )
    }

    pub(crate) fn begin_subtree_removal(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        target_canister_id: Principal,
        reserved_against_registry: ComponentRegistryHead,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        Self::begin_subtree_removal_with_origin(
            component,
            operation_id,
            target_canister_id,
            reserved_against_registry,
            maximum_component_registry_bytes,
            SubtreeRemovalOrigin::Ordinary,
        )
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one synchronous transaction validates and durably charges the exact subtree fence"
    )]
    fn begin_subtree_removal_with_origin(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        target_canister_id: Principal,
        reserved_against_registry: ComponentRegistryHead,
        maximum_component_registry_bytes: u64,
        origin: SubtreeRemovalOrigin,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let lifecycle_matches_origin = match origin {
            SubtreeRemovalOrigin::Ordinary => partition.status == ComponentLifecycleStatus::Active,
            SubtreeRemovalOrigin::DrainingDriver => {
                partition.status == ComponentLifecycleStatus::Draining
                    && component_has_terminal_quiescence(&partition)?
            }
        };
        if !lifecycle_matches_origin {
            return Err(InternalError::conflict(
                "Component subtree-removal origin differs from lifecycle authority",
            ));
        }
        if let Some(existing) = RootComponentRegistryStore::subtree_removal(component, operation_id)
        {
            validate_subtree_removal_record(&existing)?;
            validate_subtree_removal_root(&existing, &current.root)?;
            validate_subtree_removal_progress(&partition, &existing)?;
            return if existing.target.canister_id == target_canister_id
                && existing.reserved_against_registry == reserved_against_registry
            {
                Ok(subtree_removal_record_to_view(existing))
            } else {
                Err(InternalError::conflict(
                    "Component subtree-removal operation is already bound to a different fence",
                ))
            };
        }

        if reserved_against_registry
            != (ComponentRegistryHead {
                component,
                revision: partition.revision,
                content_hash: partition.content_hash,
            })
        {
            return Err(InternalError::conflict(
                "Component subtree-removal fence authority changed before durable mutation",
            ));
        }
        if origin == SubtreeRemovalOrigin::Ordinary
            && RootComponentRegistryStore::subtree_removals(component)
                .iter()
                .any(|removal| {
                    !matches!(
                        &removal.progress,
                        RootComponentSubtreeRemovalProgressRecord::Completed(_)
                    )
                })
        {
            return Err(InternalError::conflict(
                "Component already has an in-progress subtree-removal operation",
            ));
        }

        let target =
            RootComponentRegistryStore::child(component, target_canister_id).ok_or_else(|| {
                InternalError::unavailable(
                    "Component subtree-removal target is not a registered child",
                )
            })?;
        validate_registered_child_record(&partition, &target)?;
        if target.status != ComponentLifecycleStatus::Active {
            return Err(InternalError::conflict(
                "Component subtree removal requires an Active target",
            ));
        }
        let traversal_limit = partition
            .committed_descendants
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component descendant count overflow")
            })?;
        for allocation in RootComponentRegistryStore::child_allocations(component) {
            validate_child_allocation_record(&allocation)?;
            if !child_allocation_is_terminal(&allocation)
                && canister_is_in_subtree(
                    &partition,
                    allocation.parent_canister_id,
                    target_canister_id,
                    traversal_limit,
                )?
            {
                return Err(InternalError::unavailable(
                    "Component subtree has an incomplete child lifecycle operation",
                ));
            }
        }

        let record = RootComponentSubtreeRemovalRecord {
            operation_id,
            component,
            target,
            reserved_against_registry,
            maximum_completed_leaves: partition.committed_descendants,
            completed_leaves: 0,
            traversal_steps: 0,
            progress: RootComponentSubtreeRemovalProgressRecord::Fenced,
        };
        validate_subtree_removal_record(&record)?;
        let (next_partition, registry_delta) = subtree_fence_partition(&partition, &record)?;
        if next_partition.encoded_bytes > maximum_component_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "Component subtree-removal fence requires {} bytes, exceeding protected Component limit {maximum_component_registry_bytes}",
                next_partition.encoded_bytes
            )));
        }
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if next_meta.encoded_bytes > next_meta.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "Component subtree-removal fence requires {} root Registry bytes, exceeding protected limit {}",
                next_meta.encoded_bytes, next_meta.root.limits.maximum_registry_bytes
            )));
        }

        let draining_transition = match origin {
            SubtreeRemovalOrigin::Ordinary => None,
            SubtreeRemovalOrigin::DrainingDriver => {
                let current_draining = RootComponentRegistryStore::component_draining(component)
                    .ok_or_else(|| {
                        InternalError::invariant(
                            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                            "draining subtree fence has no Component draining authority",
                        )
                    })?;
                validate_component_draining_record(&partition, &current_draining)?;
                let mut next_draining = current_draining.clone();
                next_draining.subtree_operation_id = Some(operation_id);
                validate_component_draining_record(&partition, &next_draining)?;
                Some((current_draining, next_draining))
            }
        };
        RootComponentRegistryStore::begin_subtree_removal(RootComponentSubtreeRemovalBeginCommit {
            expected_meta: &current,
            next_meta,
            expected_partition: &partition,
            next_partition,
            expected_target: &record.target,
            record: record.clone(),
            expected_draining: draining_transition.as_ref().map(|(expected, _)| expected),
            next_draining: draining_transition.as_ref().map(|(_, next)| next.clone()),
        })
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(record))
    }

    pub(crate) fn advance_subtree_removal(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component subtree-removal operation has not been durably fenced",
                )
            })?;
        validate_subtree_removal_record(&record)?;
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;
        if expected_traversal_steps < record.traversal_steps {
            return Ok(subtree_removal_record_to_view(record));
        }
        if expected_traversal_steps > record.traversal_steps {
            return Err(InternalError::conflict(
                "Component subtree-removal traversal expectation is ahead of durable progress",
            ));
        }
        if matches!(
            &record.progress,
            RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
                | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
                | RootComponentSubtreeRemovalProgressRecord::Stopped(_)
                | RootComponentSubtreeRemovalProgressRecord::DeleteIntent(_)
                | RootComponentSubtreeRemovalProgressRecord::Deleted(_)
                | RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(_)
                | RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(_)
                | RootComponentSubtreeRemovalProgressRecord::Completed(_)
        ) {
            return Ok(subtree_removal_record_to_view(record));
        }

        let mut next_record = record.clone();
        for _ in 0..SUBTREE_REMOVAL_TRAVERSAL_BATCH_SIZE {
            let cursor = match &next_record.progress {
                RootComponentSubtreeRemovalProgressRecord::Fenced => next_record.target.clone(),
                RootComponentSubtreeRemovalProgressRecord::Traversing { cursor } => cursor.clone(),
                RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
                | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
                | RootComponentSubtreeRemovalProgressRecord::Stopped(_)
                | RootComponentSubtreeRemovalProgressRecord::DeleteIntent(_)
                | RootComponentSubtreeRemovalProgressRecord::Deleted(_)
                | RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(_)
                | RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(_)
                | RootComponentSubtreeRemovalProgressRecord::Completed(_) => break,
            };
            next_record.progress = match first_registered_child(&partition, cursor.canister_id)? {
                Some(child) => {
                    RootComponentSubtreeRemovalProgressRecord::Traversing { cursor: child }
                }
                None => RootComponentSubtreeRemovalProgressRecord::LeafSelected { leaf: cursor },
            };
            next_record.traversal_steps =
                next_record.traversal_steps.checked_add(1).ok_or_else(|| {
                    InternalError::resource_exhausted(
                        "Component subtree-removal traversal step overflow",
                    )
                })?;
        }
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        validate_subtree_removal_progress(&partition, &next_record)?;
        let (next_partition, next_meta) = subtree_removal_progress_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    pub(crate) fn prepare_subtree_leaf_stop(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component subtree-removal operation has not been durably fenced",
                )
            })?;
        validate_subtree_removal_record(&record)?;
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?.is_some()
        {
            return Ok(subtree_removal_record_to_view(record));
        }
        let expected_stop =
            SubtreeLeafStopAuthority::new(expected_selection, current.root.fleet_subnet_root);
        let leaf = match &record.progress {
            RootComponentSubtreeRemovalProgressRecord::LeafSelected { leaf } => leaf,
            RootComponentSubtreeRemovalProgressRecord::Fenced
            | RootComponentSubtreeRemovalProgressRecord::Traversing { .. } => {
                return Err(InternalError::unavailable(
                    "Component subtree removal has not selected a leaf to stop",
                ));
            }
            progress => {
                let durable_stop = retained_subtree_stop_effect(progress).ok_or_else(|| {
                    InternalError::invariant(
                        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                        "Component subtree stop progress has no retained stop authority",
                    )
                })?;
                if SubtreeLeafStopAuthority::from_record(record.traversal_steps, durable_stop)
                    == expected_stop
                {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict(
                    "Component subtree stop preparation differs from durable authority",
                ));
            }
        };
        if SubtreeLeafSelection::from_record(record.traversal_steps, leaf) != expected_selection {
            return Err(InternalError::conflict(
                "Component subtree stop preparation differs from the selected leaf",
            ));
        }
        if current.root.fleet_subnet_root == Principal::anonymous() {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component subtree stop preparation has anonymous root authority",
            ));
        }

        let mut next_record = record.clone();
        next_record.progress = RootComponentSubtreeRemovalProgressRecord::StopIntent(
            RootComponentSubtreeStopEffectRecord {
                leaf: leaf.clone(),
                controller: current.root.fleet_subnet_root,
            },
        );
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        validate_subtree_removal_progress(&partition, &next_record)?;
        let (next_partition, next_meta) = subtree_removal_progress_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    pub(crate) fn mark_subtree_leaf_stopped(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        observed_module_hash: [u8; 32],
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component subtree-removal operation has not been durably fenced",
                )
            })?;
        validate_subtree_removal_record(&record)?;
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if let Some(history) =
            completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?
        {
            if history.observed_module_hash == observed_module_hash {
                return Ok(subtree_removal_record_to_view(record));
            }
            return Err(InternalError::conflict(
                "Component subtree stopped observation differs from completed history",
            ));
        }
        let expected_stop =
            SubtreeLeafStopAuthority::new(expected_selection, current.root.fleet_subnet_root);
        let expected_stopped = SubtreeLeafStoppedAuthority {
            stop: expected_stop,
            observed_module_hash,
        };
        let stop = match &record.progress {
            RootComponentSubtreeRemovalProgressRecord::StopIntent(effect) => effect,
            RootComponentSubtreeRemovalProgressRecord::Fenced
            | RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
            | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. } => {
                return Err(InternalError::unavailable(
                    "Component subtree leaf has no durable stop intent",
                ));
            }
            progress => {
                let durable_stopped =
                    retained_subtree_stopped_effect(progress).ok_or_else(|| {
                        InternalError::invariant(
                            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                            "Component subtree stop progress has no retained stopped receipt",
                        )
                    })?;
                if SubtreeLeafStoppedAuthority::from_record(record.traversal_steps, durable_stopped)
                    == expected_stopped
                {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict(
                    "Component subtree stopped observation differs from durable authority",
                ));
            }
        };
        if SubtreeLeafStopAuthority::from_record(record.traversal_steps, stop) != expected_stop {
            return Err(InternalError::conflict(
                "Component subtree stopped observation differs from prepared authority",
            ));
        }

        let mut next_record = record.clone();
        next_record.progress = RootComponentSubtreeRemovalProgressRecord::Stopped(
            RootComponentSubtreeStoppedEffectRecord {
                stop: stop.clone(),
                observed_module_hash,
            },
        );
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        validate_subtree_removal_progress(&partition, &next_record)?;
        let (next_partition, next_meta) = subtree_removal_progress_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "deletion preparation reconciles every later durable removal phase"
    )]
    pub(crate) fn prepare_subtree_leaf_delete(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component subtree-removal operation has not been durably fenced",
                )
            })?;
        validate_subtree_removal_record(&record)?;
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?.is_some()
        {
            return Ok(subtree_removal_record_to_view(record));
        }
        let expected_stop =
            SubtreeLeafStopAuthority::new(expected_selection, current.root.fleet_subnet_root);
        let stopped = match &record.progress {
            RootComponentSubtreeRemovalProgressRecord::Stopped(receipt) => receipt,
            RootComponentSubtreeRemovalProgressRecord::DeleteIntent(deletion) => {
                if SubtreeLeafStopAuthority::from_record(
                    record.traversal_steps,
                    &deletion.stopped.stop,
                ) == expected_stop
                {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict(
                    "Component subtree deletion preparation differs from durable intent",
                ));
            }
            RootComponentSubtreeRemovalProgressRecord::Deleted(receipt) => {
                if SubtreeLeafStopAuthority::from_record(
                    record.traversal_steps,
                    &receipt.deletion.stopped.stop,
                ) == expected_stop
                {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict(
                    "Component subtree deletion preparation differs from durable receipt",
                ));
            }
            RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) => {
                if SubtreeLeafStopAuthority::from_record(
                    record.traversal_steps,
                    &receipt.deleted.deletion.stopped.stop,
                ) == expected_stop
                {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict(
                    "Component subtree deletion preparation differs from durable membership-removal receipt",
                ));
            }
            RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) => {
                if SubtreeLeafStopAuthority::from_record(
                    record.traversal_steps,
                    &receipt.membership_removed.deleted.deletion.stopped.stop,
                ) == expected_stop
                {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict(
                    "Component subtree deletion preparation differs from durable Directory receipt",
                ));
            }
            RootComponentSubtreeRemovalProgressRecord::Fenced
            | RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
            | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
            | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
            | RootComponentSubtreeRemovalProgressRecord::Completed(_) => {
                return Err(InternalError::unavailable(
                    "Component subtree leaf has no durable stopped receipt",
                ));
            }
        };
        if SubtreeLeafStopAuthority::from_record(record.traversal_steps, &stopped.stop)
            != expected_stop
        {
            return Err(InternalError::conflict(
                "Component subtree deletion preparation differs from stopped authority",
            ));
        }

        let mut next_record = record.clone();
        next_record.progress = RootComponentSubtreeRemovalProgressRecord::DeleteIntent(
            RootComponentSubtreeDeleteEffectRecord {
                stopped: stopped.clone(),
            },
        );
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        validate_subtree_removal_progress(&partition, &next_record)?;
        let (next_partition, next_meta) = subtree_removal_progress_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    pub(crate) fn mark_subtree_leaf_deleted(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component subtree-removal operation has not been durably fenced",
                )
            })?;
        validate_subtree_removal_record(&record)?;
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?.is_some()
        {
            return Ok(subtree_removal_record_to_view(record));
        }
        let expected_stop =
            SubtreeLeafStopAuthority::new(expected_selection, current.root.fleet_subnet_root);
        let RootComponentSubtreeRemovalProgressRecord::DeleteIntent(deletion) = &record.progress
        else {
            let Some(receipt) = retained_subtree_deleted_effect(&record.progress) else {
                return Err(InternalError::unavailable(
                    "Component subtree leaf has no durable deletion intent",
                ));
            };
            let durable_stop = SubtreeLeafStopAuthority::from_record(
                record.traversal_steps,
                &receipt.deletion.stopped.stop,
            );
            if durable_stop == expected_stop {
                return Ok(subtree_removal_record_to_view(record));
            }
            return Err(InternalError::conflict(
                "Component subtree deleted observation differs from durable receipt",
            ));
        };
        if SubtreeLeafStopAuthority::from_record(record.traversal_steps, &deletion.stopped.stop)
            != expected_stop
        {
            return Err(InternalError::conflict(
                "Component subtree deleted observation differs from prepared authority",
            ));
        }

        let mut next_record = record.clone();
        next_record.progress = RootComponentSubtreeRemovalProgressRecord::Deleted(
            RootComponentSubtreeDeletedEffectRecord {
                deletion: deletion.clone(),
            },
        );
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        validate_subtree_removal_progress(&partition, &next_record)?;
        let (next_partition, next_meta) = subtree_removal_progress_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "one synchronous operation validates and atomically removes every leaf index"
    )]
    pub(crate) fn remove_subtree_leaf_membership(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        directory_synchronized_at_ns: u64,
        maximum_component_registry_bytes: u64,
        fleet_directory: FleetDirectorySnapshot,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component subtree-removal operation has not been durably fenced",
                )
            })?;
        validate_subtree_removal_record(&record)?;
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?.is_some()
        {
            return Ok(subtree_removal_record_to_view(record));
        }
        let deleted = match &record.progress {
            RootComponentSubtreeRemovalProgressRecord::Deleted(receipt) => receipt,
            RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) => {
                let durable_selection = SubtreeLeafSelection::from_record(
                    record.traversal_steps,
                    &receipt.deleted.deletion.stopped.stop.leaf,
                );
                if durable_selection == expected_selection {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict(
                    "Component subtree membership removal differs from durable authority",
                ));
            }
            RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) => {
                let durable_selection = SubtreeLeafSelection::from_record(
                    record.traversal_steps,
                    &receipt
                        .membership_removed
                        .deleted
                        .deletion
                        .stopped
                        .stop
                        .leaf,
                );
                if durable_selection == expected_selection {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict(
                    "Component subtree membership removal differs from durable Directory authority",
                ));
            }
            RootComponentSubtreeRemovalProgressRecord::Fenced
            | RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
            | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
            | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
            | RootComponentSubtreeRemovalProgressRecord::Stopped(_)
            | RootComponentSubtreeRemovalProgressRecord::DeleteIntent(_)
            | RootComponentSubtreeRemovalProgressRecord::Completed(_) => {
                return Err(InternalError::unavailable(
                    "Component subtree leaf has no durable deletion receipt",
                ));
            }
        };
        let leaf = &deleted.deletion.stopped.stop.leaf;
        if SubtreeLeafSelection::from_record(record.traversal_steps, leaf) != expected_selection {
            return Err(InternalError::conflict(
                "Component subtree membership removal differs from the deleted leaf",
            ));
        }
        if directory_synchronized_at_ns <= partition.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input(
                "Component subtree membership removal must advance the Component Directory authority time",
            ));
        }
        if first_registered_child(&partition, leaf.canister_id)?.is_some() {
            return Err(InternalError::conflict(
                "Component subtree membership removal requires a childless deleted leaf",
            ));
        }
        let traversal = ComponentRegistryChildTraversalRecord {
            component,
            parent_canister_id: leaf.parent_canister_id,
            role: leaf.role.clone(),
            canister_id: leaf.canister_id,
        };
        let parent_role_count = RootComponentRegistryStore::parent_role_count(
            component,
            leaf.parent_canister_id,
            &leaf.role,
        )
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "deleted Component subtree leaf has no parent-role count",
            )
        })?;
        if parent_role_count.instances == 0 {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "deleted Component subtree leaf has an empty parent-role count",
            ));
        }
        let next_parent_role_count =
            parent_role_count
                .instances
                .checked_sub(1)
                .and_then(|instances| {
                    (instances > 0).then(|| ComponentRegistryParentRoleCountRecord {
                        component,
                        parent_canister_id: leaf.parent_canister_id,
                        child_role: leaf.role.clone(),
                        instances,
                    })
                });

        let revision = partition.revision.checked_add(1).ok_or_else(|| {
            InternalError::resource_exhausted("Component Registry revision overflow")
        })?;
        let committed_descendants =
            partition
                .committed_descendants
                .checked_sub(1)
                .ok_or_else(|| {
                    InternalError::invariant(
                        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                        "Component Registry has no committed descendant to remove",
                    )
                })?;
        let descendant_content_hash = removed_component_descendant_content_hash(
            component,
            partition.descendant_content_hash,
            partition.revision,
            partition.committed_descendants,
            revision,
            leaf,
        )?;
        let content_hash = component_partition_content_hash(
            &partition.binding,
            &partition.provisioning_origin,
            partition.release_set,
            partition.status,
            revision,
            descendant_content_hash,
            committed_descendants,
        )?;
        let directory_authority_hash = component_directory_authority_hash(
            &partition.binding,
            revision,
            content_hash,
            directory_synchronized_at_ns,
            committed_descendants,
            &fleet_directory,
        )?;
        let mut next_meta = current.clone();
        next_meta.managed_descendants =
            next_meta
                .managed_descendants
                .checked_sub(1)
                .ok_or_else(|| {
                    InternalError::invariant(
                        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                        "root has no managed descendant to remove",
                    )
                })?;
        next_meta.known_created_component_canisters = next_meta
            .known_created_component_canisters
            .checked_sub(1)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root has no known-created Component Canister to remove",
                )
            })?;
        let registry = ComponentRegistryHead {
            component,
            revision,
            content_hash,
        };
        let mut next_partition = partition.clone();
        next_partition.revision = revision;
        next_partition.content_hash = content_hash;
        next_partition.descendant_content_hash = descendant_content_hash;
        next_partition.directory_synchronized_at_ns = directory_synchronized_at_ns;
        next_partition.committed_descendants = committed_descendants;
        let mut next_record = record.clone();
        next_record.progress = RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(
            RootComponentSubtreeMembershipRemovedRecord {
                deleted: deleted.clone(),
                removed_from_registry: ComponentRegistryHead {
                    component,
                    revision: partition.revision,
                    content_hash: partition.content_hash,
                },
                previous_descendant_content_hash: partition.descendant_content_hash,
                previous_committed_descendants: partition.committed_descendants,
                registry,
                descendant_content_hash,
                registry_encoded_bytes: 0,
                reserved_descendants: partition.reserved_descendants,
                committed_descendants,
                directory_synchronized_at_ns,
                directory_authority_hash,
                parent_role_instances: next_parent_role_count
                    .as_ref()
                    .map_or(0, |count| count.instances),
                root_managed_descendants: next_meta.managed_descendants,
                root_known_created_component_canisters: next_meta.known_created_component_canisters,
            },
        );
        converge_subtree_membership_removal_bytes(
            &current,
            &partition,
            &record,
            leaf,
            &traversal,
            &parent_role_count,
            next_parent_role_count.as_ref(),
            &mut next_meta,
            &mut next_partition,
            &mut next_record,
            maximum_component_registry_bytes,
        )?;
        validate_subtree_removal_record(&next_record)?;
        RootComponentRegistryStore::remove_subtree_leaf_membership(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
            leaf,
            &traversal,
            &parent_role_count,
            next_parent_role_count,
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "the exact leaf selection and both independently observed recipients are one durable transition"
    )]
    pub(crate) fn mark_subtree_leaf_directory_synchronized(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        authority: ComponentRuntimeDirectoryAuthority,
        authority_hash: [u8; 32],
        owning_component: Option<ComponentRuntimeDirectoryConvergenceEvidence>,
        parent: Option<ComponentRuntimeDirectoryConvergenceEvidence>,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component subtree-removal operation has not been durably fenced",
                )
            })?;
        validate_subtree_removal_record(&record)?;
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?.is_some()
        {
            return Ok(subtree_removal_record_to_view(record));
        }
        let membership_removed = match &record.progress {
            RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) => receipt,
            RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) => {
                let durable_selection = SubtreeLeafSelection::from_record(
                    record.traversal_steps,
                    &receipt
                        .membership_removed
                        .deleted
                        .deletion
                        .stopped
                        .stop
                        .leaf,
                );
                if durable_selection == expected_selection {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict(
                    "Component subtree Directory synchronization differs from durable authority",
                ));
            }
            RootComponentSubtreeRemovalProgressRecord::Fenced
            | RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
            | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
            | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
            | RootComponentSubtreeRemovalProgressRecord::Stopped(_)
            | RootComponentSubtreeRemovalProgressRecord::DeleteIntent(_)
            | RootComponentSubtreeRemovalProgressRecord::Deleted(_)
            | RootComponentSubtreeRemovalProgressRecord::Completed(_) => {
                return Err(InternalError::unavailable(
                    "Component subtree leaf membership has not been removed",
                ));
            }
        };
        let leaf = &membership_removed.deleted.deletion.stopped.stop.leaf;
        if SubtreeLeafSelection::from_record(record.traversal_steps, leaf) != expected_selection {
            return Err(InternalError::conflict(
                "Component subtree Directory synchronization differs from the removed leaf",
            ));
        }

        let owning_binding = ManagedCanisterBinding::Component(partition.binding.clone());
        let coverage = subtree_directory_coverage(&partition, &authority, authority_hash)?;
        let owning_component = match (partition.status, owning_component) {
            (ComponentLifecycleStatus::Active, Some(evidence)) => {
                let (observed_coverage, evidence) =
                    subtree_directory_convergence_record(&partition, &owning_binding, evidence)?;
                if observed_coverage != coverage {
                    return Err(InternalError::conflict(
                        "Component owner Directory evidence differs from covered authority",
                    ));
                }
                Some(evidence)
            }
            (ComponentLifecycleStatus::Draining, None)
                if component_has_terminal_quiescence(&partition)? =>
            {
                None
            }
            _ => {
                return Err(InternalError::conflict(
                    "Component owner Directory convergence differs from lifecycle authority",
                ));
            }
        };
        let parent = subtree_directory_parent_convergence_record(
            &partition,
            component,
            leaf.parent_canister_id,
            parent,
            &coverage,
        )?;

        let mut next_record = record.clone();
        next_record.progress = RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(
            RootComponentSubtreeDirectorySynchronizedRecord {
                membership_removed: membership_removed.clone(),
                covered_fleet_registry_revision: coverage.fleet_registry_revision,
                covered_fleet_registry_content_hash: coverage.fleet_registry_content_hash,
                covered_component_registry_revision: coverage.component_registry_revision,
                covered_component_registry_content_hash: coverage.component_registry_content_hash,
                covered_authority_hash: coverage.authority_hash,
                owning_component,
                parent,
            },
        );
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        validate_subtree_removal_progress(&partition, &next_record)?;
        let (next_partition, next_meta) = subtree_removal_progress_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    pub(crate) fn finalize_subtree_leaf(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component subtree-removal operation has not been durably fenced",
                )
            })?;
        validate_subtree_removal_record(&record)?;
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?.is_some()
        {
            return Ok(subtree_removal_record_to_view(record));
        }

        let RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) =
            &record.progress
        else {
            return Err(InternalError::unavailable(
                "Component subtree leaf Directory has not been synchronized",
            ));
        };
        let leaf = &receipt
            .membership_removed
            .deleted
            .deletion
            .stopped
            .stop
            .leaf;
        if SubtreeLeafSelection::from_record(record.traversal_steps, leaf) != expected_selection {
            return Err(InternalError::conflict(
                "Component subtree leaf finalization differs from synchronized authority",
            ));
        }

        let completed_leaves = record.completed_leaves.checked_add(1).ok_or_else(|| {
            InternalError::resource_exhausted("Component subtree completed-leaf count overflow")
        })?;
        if completed_leaves > record.maximum_completed_leaves {
            return Err(InternalError::resource_exhausted(
                "Component subtree completed-leaf history exceeded its frozen bound",
            ));
        }
        let completed_leaf = completed_subtree_leaf_record(&record, receipt)?;
        validate_subtree_removal_completed_leaf(&record, &partition, &completed_leaf)?;
        let next_progress =
            finalized_subtree_removal_progress(component, &partition, &record, receipt)?;

        let mut next_record = record.clone();
        next_record.completed_leaves = completed_leaves;
        next_record.progress = next_progress;
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        if matches!(
            &next_record.progress,
            RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
        ) {
            validate_subtree_removal_progress(&partition, &next_record)?;
        }
        let (next_partition, next_meta) = subtree_removal_leaf_finalization_state(
            &current,
            &partition,
            &record,
            &next_record,
            &completed_leaf,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::finalize_subtree_removal_leaf(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record,
            completed_leaf,
        )
        .map_err(map_allocation_commit_error)?;
        Self::subtree_removal(component, operation_id)?.ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "finalized Component subtree-removal operation disappeared",
            )
        })
    }

    pub(crate) fn parent_role_instances(
        component: ComponentInstanceId,
        parent_canister_id: Principal,
        child_role: &CanisterRole,
    ) -> Result<u32, InternalError> {
        let Some(record) = RootComponentRegistryStore::parent_role_count(
            component,
            parent_canister_id,
            child_role,
        ) else {
            return Ok(0);
        };
        let expected_identity =
            ComponentParentRoleIdentity::new(component, parent_canister_id, child_role);
        if ComponentParentRoleIdentity::from_count(&record) != expected_identity
            || record.instances == 0
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry parent-role count index is invalid",
            ));
        }
        Ok(record.instances)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one synchronous transaction keeps every child-reservation capacity mutation together"
    )]
    pub(crate) fn reserve_child_allocation(
        decision: ComponentChildAllocationDecision,
        operation_id: [u8; 32],
        reserved_against_registry: ComponentRegistryHead,
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition =
            RootComponentRegistryStore::partition(decision.component).ok_or_else(|| {
                InternalError::unavailable("Component Registry partition has not been committed")
            })?;
        validate_partition_record(&partition)?;
        let record = RootComponentChildAllocationRecord {
            operation_id,
            component: decision.component,
            parent_canister_id: decision.parent_canister_id,
            parent_role: decision.parent_role,
            child_role: decision.child_role,
            child_kind: decision.child_kind,
            maximum_instances_per_parent: decision.maximum_instances_per_parent,
            maximum_descendants: decision.maximum_descendants,
            maximum_registry_bytes: decision.maximum_registry_bytes,
            reserved_against_registry,
            release_set: current.release_set,
            progress: RootComponentChildAllocationProgressRecord::Reserved,
        };
        if let Some(existing) =
            RootComponentRegistryStore::child_allocation(record.component, operation_id)
        {
            return if existing.has_same_reservation(&record) {
                Ok(child_allocation_record_to_view(existing))
            } else {
                Err(InternalError::conflict(
                    "Component Child allocation operation is already bound to different intent",
                ))
            };
        }
        let spec_authority_matches = partition.binding.component_spec == decision.component_spec
            && partition.binding.spec_hash == decision.spec_hash;
        let partition_is_active = partition.release_set == current.release_set
            && partition.status == ComponentLifecycleStatus::Active;
        let expected_registry = ComponentRegistryHead {
            component: decision.component,
            revision: partition.revision,
            content_hash: partition.content_hash,
        };
        if !spec_authority_matches
            || !partition_is_active
            || record.reserved_against_registry != expected_registry
        {
            return Err(InternalError::conflict(
                "Component Child reservation authority changed before durable mutation",
            ));
        }
        let traversal_limit = partition
            .committed_descendants
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component descendant count overflow")
            })?;
        for removal in RootComponentRegistryStore::subtree_removals(record.component) {
            validate_subtree_removal_record(&removal)?;
            validate_subtree_removal_root(&removal, &current.root)?;
            if matches!(
                removal.progress,
                RootComponentSubtreeRemovalProgressRecord::Completed(_)
            ) {
                continue;
            }
            if canister_is_in_subtree(
                &partition,
                record.parent_canister_id,
                removal.target.canister_id,
                traversal_limit,
            )? {
                return Err(InternalError::conflict(
                    "Component Child parent is fenced by an in-progress subtree removal",
                ));
            }
        }

        let current_count = RootComponentRegistryStore::parent_role_count(
            record.component,
            record.parent_canister_id,
            &record.child_role,
        );
        let next_count = ComponentRegistryParentRoleCountRecord {
            component: record.component,
            parent_canister_id: record.parent_canister_id,
            child_role: record.child_role.clone(),
            instances: current_count
                .as_ref()
                .map_or(0, |count| count.instances)
                .checked_add(1)
                .ok_or_else(|| {
                    InternalError::resource_exhausted("per-parent child count overflow")
                })?,
        };
        if next_count.instances > decision.maximum_instances_per_parent {
            return Err(InternalError::resource_exhausted(
                "registered parent exhausted its direct-child role capacity",
            ));
        }
        let (next_partition, registry_delta) =
            child_reservation_partition(&partition, &record, current_count.as_ref(), &next_count)?;
        let component_descendants = next_partition
            .reserved_descendants
            .checked_add(next_partition.committed_descendants)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component descendant count overflow")
            })?;
        if component_descendants > decision.maximum_descendants {
            return Err(InternalError::resource_exhausted(
                "Component descendant capacity is exhausted",
            ));
        }
        if next_partition.encoded_bytes > decision.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "Component Child reservation requires {} bytes, exceeding protected Component limit {}",
                next_partition.encoded_bytes, decision.maximum_registry_bytes
            )));
        }
        let mut next_meta = current.clone();
        next_meta.managed_descendants =
            next_meta
                .managed_descendants
                .checked_add(1)
                .ok_or_else(|| {
                    InternalError::resource_exhausted("root managed descendant count overflow")
                })?;
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let managed_canisters = 1_u32
            .checked_add(next_meta.reserved_component_instances)
            .and_then(|count| count.checked_add(next_meta.committed_component_instances))
            .and_then(|count| count.checked_add(next_meta.managed_descendants))
            .ok_or_else(|| {
                InternalError::resource_exhausted("root managed-Canister count overflow")
            })?;
        if managed_canisters > next_meta.root.limits.maximum_managed_canisters {
            return Err(InternalError::resource_exhausted(
                "root managed-Canister capacity is exhausted",
            ));
        }
        if next_meta.encoded_bytes > next_meta.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "Component Child reservation requires {} root Registry bytes, exceeding protected limit {}",
                next_meta.encoded_bytes, next_meta.root.limits.maximum_registry_bytes
            )));
        }

        RootComponentRegistryStore::reserve_child_allocation(
            &current,
            next_meta,
            &partition,
            next_partition,
            record.clone(),
            current_count.as_ref(),
            next_count,
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(record))
    }

    pub(crate) fn validate_child_creation_capacity(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        plan: &RootComponentCreationPlan,
    ) -> Result<(), InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        validate_child_creation_authority(&current, &partition, &record, plan)?;
        if !matches!(
            record.progress,
            RootComponentChildAllocationProgressRecord::Reserved
        ) {
            return Err(InternalError::conflict(
                "Component Child allocation has already crossed its creation-intent boundary",
            ));
        }
        let charged_entry_bytes = child_creation_charged_entry_bytes(&record, plan);
        child_creation_capacity(&current, &partition, &record, charged_entry_bytes).map(|_| ())
    }

    pub(crate) fn begin_child_creation(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        plan: RootComponentCreationPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        validate_child_creation_authority(&current, &partition, &record, &plan)?;
        if !matches!(
            record.progress,
            RootComponentChildAllocationProgressRecord::Reserved
        ) {
            return Err(InternalError::conflict(
                "Component Child allocation has already crossed its creation-intent boundary",
            ));
        }

        let charged_entry_bytes = child_creation_charged_entry_bytes(&record, &plan);
        let (next_partition, registry_delta) =
            child_creation_capacity(&current, &partition, &record, charged_entry_bytes)?;
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::CreationIntent(
            RootComponentCreationEffectRecord {
                wasm_store: plan.wasm_store,
                payload_hash: plan.payload_hash,
                payload_size_bytes: plan.payload_size_bytes,
                initial_cycles: plan.initial_cycles,
                controller: plan.controller,
                cost_guard_settlement,
                charged_entry_bytes,
            },
        );
        validate_charged_child_record_size(&next_record, charged_entry_bytes)?;
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;

        RootComponentRegistryStore::replace_child_allocation(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(next_record))
    }

    pub(crate) fn mark_child_created(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        canister: Principal,
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        let effect = match &record.progress {
            RootComponentChildAllocationProgressRecord::CreationIntent(effect) => effect.clone(),
            RootComponentChildAllocationProgressRecord::Created {
                canister: existing, ..
            } if existing == &canister => return Ok(child_allocation_record_to_view(record)),
            RootComponentChildAllocationProgressRecord::InstallIntent {
                canister: existing,
                ..
            }
            | RootComponentChildAllocationProgressRecord::Installed {
                canister: existing, ..
            }
            | RootComponentChildAllocationProgressRecord::Verified {
                canister: existing, ..
            }
            | RootComponentChildAllocationProgressRecord::Committed {
                canister: existing, ..
            } if existing == &canister => return Ok(child_allocation_record_to_view(record)),
            RootComponentChildAllocationProgressRecord::Created { .. }
            | RootComponentChildAllocationProgressRecord::InstallIntent { .. }
            | RootComponentChildAllocationProgressRecord::Installed { .. }
            | RootComponentChildAllocationProgressRecord::Verified { .. }
            | RootComponentChildAllocationProgressRecord::Committed { .. } => {
                return Err(InternalError::conflict(
                    "Component Child allocation is already bound to a different created Canister",
                ));
            }
            RootComponentChildAllocationProgressRecord::Reserved => {
                return Err(InternalError::conflict(
                    "Component Child allocation has no durable creation intent",
                ));
            }
        };
        let protected_principals = [
            Principal::anonymous(),
            current.root.fleet_subnet_root,
            current.root.authority.binding.coordinator,
            partition.binding.canister_id,
            record.parent_canister_id,
        ];
        if protected_principals.contains(&canister)
            || RootComponentRegistryStore::component_for_principal(canister).is_some()
        {
            return Err(InternalError::conflict(
                "created Component Child principal conflicts with protected Registry authority",
            ));
        }

        let charged_entry_bytes = effect.charged_entry_bytes;
        let mut next_record = record.clone();
        next_record.progress =
            RootComponentChildAllocationProgressRecord::Created { effect, canister };
        validate_charged_child_record_size(&next_record, charged_entry_bytes)?;
        let mut next_meta = current.clone();
        next_meta.known_created_component_canisters = next_meta
            .known_created_component_canisters
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "known-created Component Canister count overflowed",
                )
            })?;
        let allocated_component_canisters = current
            .reserved_component_instances
            .checked_add(current.committed_component_instances)
            .and_then(|count| count.checked_add(current.managed_descendants))
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "allocated Component-tree Canister count overflowed",
                )
            })?;
        if next_meta.known_created_component_canisters > allocated_component_canisters {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "known-created Component Canisters exceed allocated Component-tree capacity",
            ));
        }

        RootComponentRegistryStore::replace_child_allocation(
            &current,
            next_meta,
            &partition,
            partition.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(next_record))
    }

    pub(crate) fn validate_child_install_capacity(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        plan: &RootComponentChildInstallPlan,
    ) -> Result<(), InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        validate_child_install_authority(&current, &partition, &record, plan)?;
        if !matches!(
            record.progress,
            RootComponentChildAllocationProgressRecord::Created { .. }
        ) {
            return Err(InternalError::conflict(
                "Component Child allocation is not ready to cross its install-intent boundary",
            ));
        }
        let charged_entry_bytes = child_install_charged_entry_bytes(&record, plan)?;
        child_install_capacity(&current, &partition, &record, charged_entry_bytes).map(|_| ())
    }

    pub(crate) fn begin_child_install(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        plan: RootComponentChildInstallPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        validate_child_install_authority(&current, &partition, &record, &plan)?;
        let (creation, canister) = match &record.progress {
            RootComponentChildAllocationProgressRecord::Created { effect, canister } => {
                (effect.clone(), *canister)
            }
            _ => {
                return Err(InternalError::conflict(
                    "Component Child allocation is not ready for installation",
                ));
            }
        };
        let charged_entry_bytes = child_install_charged_entry_bytes(&record, &plan)?;
        let (next_partition, registry_delta) =
            child_install_capacity(&current, &partition, &record, charged_entry_bytes)?;
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::InstallIntent {
            creation,
            canister,
            installation: RootComponentChildInstallEffectRecord {
                raw_module_hash: plan.raw_module_hash,
                chunk_hashes: plan.chunk_hashes,
                binding: plan.binding,
                cost_guard_settlement,
                charged_entry_bytes,
            },
        };
        validate_charged_child_record_size(&next_record, charged_entry_bytes)?;
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;

        RootComponentRegistryStore::replace_child_allocation(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(next_record))
    }

    pub(crate) fn renew_child_install_intent(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        plan: &RootComponentChildInstallPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        validate_child_install_authority(&current, &partition, &record, plan)?;
        let (creation, canister, existing) = match &record.progress {
            RootComponentChildAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            } => (creation.clone(), *canister, installation),
            _ => {
                return Err(InternalError::conflict(
                    "Component Child allocation has no renewable install intent",
                ));
            }
        };
        validate_child_install_effect_record(existing, plan)?;
        let charged_entry_bytes = existing.charged_entry_bytes;
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::InstallIntent {
            creation,
            canister,
            installation: RootComponentChildInstallEffectRecord {
                raw_module_hash: plan.raw_module_hash,
                chunk_hashes: plan.chunk_hashes.clone(),
                binding: plan.binding.clone(),
                cost_guard_settlement,
                charged_entry_bytes,
            },
        };
        validate_charged_child_record_size(&next_record, charged_entry_bytes)?;
        RootComponentRegistryStore::replace_child_allocation(
            &current,
            current.clone(),
            &partition,
            partition.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(next_record))
    }

    pub(crate) fn mark_child_installed(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        advance_child_install_phase(component, operation_id, false)
    }

    pub(crate) fn mark_child_verified(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        advance_child_install_phase(component, operation_id, true)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one synchronous operation validates and atomically commits every child index"
    )]
    pub(crate) fn commit_verified_child(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        directory_synchronized_at_ns: u64,
        fleet_directory: FleetDirectorySnapshot,
    ) -> Result<
        (
            RootComponentChildAllocationView,
            ComponentRegistryPartitionView,
        ),
        InternalError,
    > {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        if let RootComponentChildAllocationProgressRecord::Committed { commitment, .. } =
            &record.progress
        {
            let committed = exact_committed_child_partition(&record, commitment)?;
            validate_child_directory_authority_hash(&committed, &fleet_directory, commitment)?;
            return Ok((
                child_allocation_record_to_view(record),
                partition_record_to_view(committed),
            ));
        }
        if directory_synchronized_at_ns <= partition.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input(
                "Component Child Directory synchronization must advance the current Component authority",
            ));
        }
        let RootComponentChildAllocationProgressRecord::Verified {
            creation,
            canister,
            installation,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component Child allocation is not ready for Registry commitment",
            ));
        };

        let (next_record, next_partition, child, traversal) = committed_child_records(
            &record,
            creation,
            *canister,
            installation,
            &partition,
            directory_synchronized_at_ns,
            &fleet_directory,
        )?;
        let actual_terminal_bytes =
            RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
                .checked_add(RootComponentRegistryStore::child_entry_bytes(&child))
                .and_then(|value| {
                    value.checked_add(RootComponentRegistryStore::child_traversal_entry_bytes(
                        &traversal,
                    ))
                })
                .and_then(|value| {
                    value.checked_add(RootComponentRegistryStore::principal_index_entry_bytes(
                        child.canister_id,
                        component,
                    ))
                })
                .ok_or_else(|| {
                    InternalError::resource_exhausted("Component Registry bytes overflow")
                })?;
        if actual_terminal_bytes > installation.charged_entry_bytes {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Child commitment exceeds its pre-install Registry byte reservation",
            ));
        }
        if next_partition.encoded_bytes > record.maximum_registry_bytes {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "pre-install child reservation exceeds the protected Component limit at commitment",
            ));
        }
        let registry_reduction = partition
            .encoded_bytes
            .checked_sub(next_partition.encoded_bytes)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "exact Component Child commitment exceeded its maximum terminal precharge",
                )
            })?;
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_sub(registry_reduction)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root Component Registry cannot release excess child precharge",
                )
            })?;

        RootComponentRegistryStore::commit_child(
            &current,
            next_meta,
            &partition,
            next_partition.clone(),
            &record,
            next_record.clone(),
            child,
            traversal,
        )
        .map_err(map_allocation_commit_error)?;
        Ok((
            child_allocation_record_to_view(next_record),
            partition_record_to_view(next_partition),
        ))
    }

    pub(crate) fn mark_child_directory_prepared(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component Child allocation is not committed for Directory preparation",
            ));
        };
        let _committed = exact_committed_child_partition(&record, commitment)?;
        if commitment.directory_authority_hash != expected_authority_hash {
            return Err(InternalError::conflict(
                "Component Child Directory authority differs from its committed root receipt",
            ));
        }
        if commitment.directory_prepared {
            return Ok(child_allocation_record_to_view(record));
        }

        let mut next_commitment = commitment.clone();
        next_commitment.directory_prepared = true;
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: next_commitment,
        };
        validate_charged_child_record_size(&next_record, installation.charged_entry_bytes)?;
        if RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
            != RootComponentRegistryStore::child_allocation_entry_bytes(&record)
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Child Directory receipt changed its precharged stable footprint",
            ));
        }
        RootComponentRegistryStore::replace_child_allocation(
            &current,
            current.clone(),
            &partition,
            partition.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(next_record))
    }

    pub(crate) fn mark_child_runtime_activated(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component Child allocation is not committed for runtime activation",
            ));
        };
        let _committed = exact_committed_child_partition(&record, commitment)?;
        if commitment.directory_authority_hash != expected_authority_hash
            || !commitment.directory_prepared
        {
            return Err(InternalError::conflict(
                "Component Child runtime activation requires its exact prepared Directory authority",
            ));
        }
        if commitment.runtime_activated {
            return Ok(child_allocation_record_to_view(record));
        }

        let mut next_commitment = commitment.clone();
        next_commitment.runtime_activated = true;
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: next_commitment,
        };
        validate_charged_child_record_size(&next_record, installation.charged_entry_bytes)?;
        if RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
            != RootComponentRegistryStore::child_allocation_entry_bytes(&record)
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Child runtime receipt changed its precharged stable footprint",
            ));
        }
        RootComponentRegistryStore::replace_child_allocation(
            &current,
            current.clone(),
            &partition,
            partition.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(next_record))
    }

    pub(crate) fn activate_child_membership(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        directory_synchronized_at_ns: u64,
        fleet_directory: FleetDirectorySnapshot,
    ) -> Result<
        (
            RootComponentChildAllocationView,
            ComponentRegistryPartitionView,
        ),
        InternalError,
    > {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed {
            canister,
            commitment,
            ..
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component Child allocation is not committed for membership activation",
            ));
        };
        let _committed = exact_committed_child_partition(&record, commitment)?;
        if let Some(membership) = &commitment.membership {
            let active = exact_active_child_partition(&record, commitment, membership)?;
            validate_child_membership_directory_authority_hash(
                &active,
                &fleet_directory,
                membership,
            )?;
            return Ok((
                child_allocation_record_to_view(record),
                partition_record_to_view(active),
            ));
        }
        if !commitment.directory_prepared || !commitment.runtime_activated {
            return Err(InternalError::conflict(
                "Component Child membership activation requires terminal Directory and runtime receipts",
            ));
        }
        if directory_synchronized_at_ns <= partition.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input(
                "active Component Child Directory synchronization must follow current authority",
            ));
        }
        let child = RootComponentRegistryStore::child(component, *canister).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "committed Component Child allocation has no normalized row",
            )
        })?;
        validate_child_record(&partition, &child)?;
        if child.status != ComponentLifecycleStatus::Prepared {
            return Err(InternalError::conflict(
                "Component Child membership activation requires a Prepared Registry row",
            ));
        }

        persist_child_membership_activation(
            &current,
            &partition,
            &record,
            &child,
            directory_synchronized_at_ns,
            &fleet_directory,
        )
    }

    pub(crate) fn mark_child_membership_synchronized(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component Child allocation is not committed for membership synchronization",
            ));
        };
        let membership = commitment.membership.as_ref().ok_or_else(|| {
            InternalError::conflict("Component Child Registry membership has not been activated")
        })?;
        let _active = exact_active_child_partition(&record, commitment, membership)?;
        if membership.directory_authority_hash != expected_authority_hash {
            return Err(InternalError::conflict(
                "current Component Child Directory differs from its active membership authority",
            ));
        }
        if membership.directory_synchronized {
            return Ok(child_allocation_record_to_view(record));
        }

        let mut next_membership = membership.clone();
        next_membership.directory_synchronized = true;
        let mut next_commitment = commitment.clone();
        next_commitment.membership = Some(next_membership);
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: next_commitment,
        };
        validate_charged_child_record_size(&next_record, installation.charged_entry_bytes)?;
        if RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
            != RootComponentRegistryStore::child_allocation_entry_bytes(&record)
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Child membership receipt changed its precharged stable footprint",
            ));
        }
        RootComponentRegistryStore::replace_child_allocation(
            &current,
            current.clone(),
            &partition,
            partition.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(next_record))
    }

    pub(crate) fn commit_verified(
        operation_id: [u8; 32],
        directory_synchronized_at_ns: u64,
        maximum_component_registry_bytes: u64,
        fleet_directory: FleetDirectorySnapshot,
    ) -> Result<(RootComponentAllocationView, ComponentRegistryPartitionView), InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        if let RootComponentAllocationProgressRecord::Committed { commitment, .. } =
            &record.progress
        {
            let partition = exact_committed_partition(&record, commitment)?;
            validate_directory_authority_hash(&partition, &fleet_directory, commitment)?;
            return Ok((
                allocation_record_to_view(record),
                partition_record_to_view(partition),
            ));
        }
        if directory_synchronized_at_ns == 0 {
            return Err(InternalError::invalid_input(
                "Component Directory synchronization timestamp must be positive",
            ));
        }
        let RootComponentAllocationProgressRecord::Verified {
            creation,
            canister,
            installation,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component allocation is not ready for Registry commitment",
            ));
        };

        let (next_record, partition) = committed_records(
            &record,
            creation,
            *canister,
            installation,
            directory_synchronized_at_ns,
            &fleet_directory,
        )?;
        if partition.encoded_bytes > installation.charged_entry_bytes {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component commitment exceeds its pre-install Registry byte reservation",
            ));
        }
        if partition.encoded_bytes > maximum_component_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "Component Registry commitment requires {} bytes, exceeding protected Component limit {maximum_component_registry_bytes}",
                partition.encoded_bytes
            )));
        }
        let encoded_bytes = current
            .encoded_bytes
            .checked_sub(installation.charged_entry_bytes)
            .and_then(|value| value.checked_add(partition.encoded_bytes))
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root Component Registry byte accounting cannot commit its reserved partition",
                )
            })?;
        if encoded_bytes > current.root.limits.maximum_registry_bytes {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "pre-install Registry reservation exceeds the protected root limit at commitment",
            ));
        }

        let mut next_meta = current.clone();
        next_meta.reserved_component_instances = next_meta
            .reserved_component_instances
            .checked_sub(1)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root reserved Component count is zero at commitment",
                )
            })?;
        next_meta.committed_component_instances = next_meta
            .committed_component_instances
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::resource_exhausted("committed Component instance count overflow")
            })?;
        next_meta.encoded_bytes = encoded_bytes;

        RootComponentRegistryStore::commit_component(
            &current,
            next_meta,
            &record,
            next_record.clone(),
            partition.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok((
            allocation_record_to_view(next_record),
            partition_record_to_view(partition),
        ))
    }

    pub(crate) fn mark_directory_prepared(
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let RootComponentAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component allocation is not committed for Directory preparation",
            ));
        };
        if commitment.directory_authority_hash != expected_authority_hash {
            return Err(InternalError::conflict(
                "Component Directory authority differs from its committed root receipt",
            ));
        }
        if commitment.directory_prepared {
            return Ok(allocation_record_to_view(record));
        }
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: RootComponentCommitmentRecord {
                registry: commitment.registry.clone(),
                prepared_registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
                directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
                directory_authority_hash: commitment.directory_authority_hash,
                directory_prepared: true,
                runtime_activated: commitment.runtime_activated,
                membership: commitment.membership.clone(),
            },
        };
        validate_charged_record_size(&next_record, installation.charged_entry_bytes)?;
        if RootComponentRegistryStore::allocation_entry_bytes(&next_record)
            != RootComponentRegistryStore::allocation_entry_bytes(&record)
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Directory receipt changed its precharged stable footprint",
            ));
        }
        RootComponentRegistryStore::replace_allocation(
            &current,
            current.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(next_record))
    }

    pub(crate) fn mark_runtime_activated(
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let RootComponentAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component allocation is not committed for runtime activation",
            ));
        };
        if commitment.directory_authority_hash != expected_authority_hash
            || !commitment.directory_prepared
        {
            return Err(InternalError::conflict(
                "Component runtime activation requires its exact prepared Directory authority",
            ));
        }
        if commitment.runtime_activated {
            return Ok(allocation_record_to_view(record));
        }
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: RootComponentCommitmentRecord {
                registry: commitment.registry.clone(),
                prepared_registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
                directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
                directory_authority_hash: commitment.directory_authority_hash,
                directory_prepared: true,
                runtime_activated: true,
                membership: commitment.membership.clone(),
            },
        };
        validate_charged_record_size(&next_record, installation.charged_entry_bytes)?;
        if RootComponentRegistryStore::allocation_entry_bytes(&next_record)
            != RootComponentRegistryStore::allocation_entry_bytes(&record)
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component runtime activation receipt changed its precharged stable footprint",
            ));
        }
        RootComponentRegistryStore::replace_allocation(
            &current,
            current.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(next_record))
    }

    pub(crate) fn activate_membership(
        operation_id: [u8; 32],
        directory_synchronized_at_ns: u64,
        maximum_component_registry_bytes: u64,
        fleet_directory: FleetDirectorySnapshot,
    ) -> Result<(RootComponentAllocationView, ComponentRegistryPartitionView), InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let RootComponentAllocationProgressRecord::Committed {
            installation,
            commitment,
            ..
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component allocation is not committed for membership activation",
            ));
        };
        let prepared = exact_committed_partition(&record, commitment)?;
        if let Some(membership) = &commitment.membership {
            let active = exact_active_partition(&record, commitment, membership)?;
            validate_membership_directory_authority_hash(&active, &fleet_directory, membership)?;
            return Ok((
                allocation_record_to_view(record),
                partition_record_to_view(active),
            ));
        }
        if !commitment.directory_prepared || !commitment.runtime_activated {
            return Err(InternalError::conflict(
                "Component membership activation requires terminal Directory and runtime receipts",
            ));
        }
        if directory_synchronized_at_ns <= commitment.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input(
                "active Component Directory synchronization must follow its prepared authority",
            ));
        }

        let (next_record, active) = active_membership_records(
            &record,
            commitment,
            directory_synchronized_at_ns,
            &fleet_directory,
        )?;
        if active.encoded_bytes > installation.charged_entry_bytes {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component membership exceeds its pre-install Registry byte reservation",
            ));
        }
        if active.encoded_bytes > maximum_component_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "active Component Registry requires {} bytes, exceeding protected Component limit {maximum_component_registry_bytes}",
                active.encoded_bytes
            )));
        }
        let encoded_bytes = current
            .encoded_bytes
            .checked_sub(prepared.encoded_bytes)
            .and_then(|value| value.checked_add(active.encoded_bytes))
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root Component Registry byte accounting cannot activate membership",
                )
            })?;
        if encoded_bytes > current.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted(
                "active Component Registry exceeds the protected root byte limit",
            ));
        }
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = encoded_bytes;
        RootComponentRegistryStore::replace_component_partition(
            &current,
            next_meta,
            &record,
            next_record.clone(),
            &prepared,
            active.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok((
            allocation_record_to_view(next_record),
            partition_record_to_view(active),
        ))
    }

    pub(crate) fn mark_membership_synchronized(
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let RootComponentAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component allocation is not committed for membership synchronization",
            ));
        };
        let membership = commitment.membership.as_ref().ok_or_else(|| {
            InternalError::conflict("Component Registry membership has not been activated")
        })?;
        let _active = exact_active_partition(&record, commitment, membership)?;
        if membership.directory_authority_hash != expected_authority_hash {
            return Err(InternalError::conflict(
                "current Component Directory differs from its active membership authority",
            ));
        }
        if membership.directory_synchronized {
            return Ok(allocation_record_to_view(record));
        }

        let mut next_membership = membership.clone();
        next_membership.directory_synchronized = true;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: RootComponentCommitmentRecord {
                registry: commitment.registry.clone(),
                prepared_registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
                directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
                directory_authority_hash: commitment.directory_authority_hash,
                directory_prepared: commitment.directory_prepared,
                runtime_activated: commitment.runtime_activated,
                membership: Some(next_membership),
            },
        };
        validate_charged_record_size(&next_record, installation.charged_entry_bytes)?;
        if RootComponentRegistryStore::allocation_entry_bytes(&next_record)
            != RootComponentRegistryStore::allocation_entry_bytes(&record)
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component membership receipt changed its precharged stable footprint",
            ));
        }
        RootComponentRegistryStore::replace_allocation(
            &current,
            current.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(next_record))
    }
}

fn complete_initial_inventory(
    current: &RootComponentRegistryMetaRecord,
) -> Result<CompleteInitialInventory, InternalError> {
    if current.reserved_component_instances != 0 {
        return Err(InternalError::unavailable(
            "initial Component inventory still contains nonterminal allocations",
        ));
    }

    let mut allocations = RootComponentRegistryStore::allocations();
    allocations.sort_by_key(|record| record.allocation_sequence);
    let component_count = u32::try_from(allocations.len()).map_err(|_| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "initial Component inventory exceeds u32",
        )
    })?;
    if component_count != current.committed_component_instances
        || current.next_allocation_sequence != u64::from(component_count) + 1
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Registry counters differ from the initial allocation inventory",
        ));
    }
    let maximum_known_created = component_count
        .checked_add(current.managed_descendants)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "initial Component-tree Canister count overflowed",
            )
        })?;
    if current.known_created_component_canisters < component_count
        || current.known_created_component_canisters > maximum_known_created
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "known-created Canister counter differs from the complete initial inventory",
        ));
    }

    let partitions = RootComponentRegistryStore::partitions();
    if partitions.len() != allocations.len() {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "initial Component allocations and Registry partitions differ in cardinality",
        ));
    }

    let mut entries = Vec::with_capacity(allocations.len());
    let mut operation_ids = Vec::with_capacity(allocations.len());
    let mut encoded_bytes = 0_u64;
    for (index, record) in allocations.iter().enumerate() {
        let (entry, partition_bytes) = initial_inventory_hash_entry(record, index)?;
        encoded_bytes = encoded_bytes.checked_add(partition_bytes).ok_or_else(|| {
            InternalError::resource_exhausted("Component Registry bytes overflow")
        })?;
        operation_ids.push(record.operation_id);
        entries.push(entry);
    }
    if encoded_bytes != current.encoded_bytes {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "initial Component inventory differs from root Registry byte accounting",
        ));
    }

    let inventory_hash = initial_inventory_hash(&entries)?;
    Ok(CompleteInitialInventory {
        component_count,
        inventory_hash,
        operation_ids,
    })
}

fn initial_inventory_hash_entry(
    record: &RootComponentAllocationRecord,
    index: usize,
) -> Result<(RootComponentInitialInventoryHashEntry, u64), InternalError> {
    if record.allocation_sequence != index as u64 + 1 {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "initial Component allocation sequences are not consecutive",
        ));
    }
    let RootComponentAllocationProgressRecord::Committed { commitment, .. } = &record.progress
    else {
        return Err(InternalError::unavailable(
            "initial Component inventory contains an allocation without Registry commitment",
        ));
    };
    let membership = commitment.membership.as_ref().ok_or_else(|| {
        InternalError::unavailable(
            "initial Component inventory contains an allocation without active membership",
        )
    })?;
    if !commitment.directory_prepared
        || !commitment.runtime_activated
        || !membership.directory_synchronized
    {
        return Err(InternalError::unavailable(
            "initial Component inventory lacks terminal Directory, runtime or membership evidence",
        ));
    }
    let active = exact_active_partition(record, commitment, membership)?;
    validate_partition_record(&active)?;
    let partition_bytes = active.encoded_bytes;
    Ok((
        RootComponentInitialInventoryHashEntry {
            operation_id: record.operation_id,
            allocation_sequence: record.allocation_sequence,
            component: record.component,
            component_spec: record.component_spec.clone(),
            spec_hash: record.spec_hash,
            role: record.role.clone(),
            provisioning_origin: record.provisioning_origin.clone(),
            release_set: record.release_set,
            prepared_registry: commitment.registry.clone(),
            prepared_registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
            prepared_directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
            prepared_directory_authority_hash: commitment.directory_authority_hash,
            active_binding: active.binding.clone(),
            active_registry: ComponentRegistryHead {
                component: active.binding.component,
                revision: active.revision,
                content_hash: active.content_hash,
            },
            active_registry_encoded_bytes: active.encoded_bytes,
            active_directory_synchronized_at_ns: membership.directory_synchronized_at_ns,
            active_directory_authority_hash: membership.directory_authority_hash,
        },
        partition_bytes,
    ))
}

fn initial_inventory_hash(
    entries: &[RootComponentInitialInventoryHashEntry],
) -> Result<[u8; 32], InternalError> {
    const DOMAIN: &[u8] = b"canic.root-component-initial-inventory.v1";
    let payload = candid::encode_one(entries).map_err(|error| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            format!("initial Component inventory cannot be encoded: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn validate_initial_inventory_receipt(
    receipt: &RootComponentInitialInventoryRecord,
    fleet_activation_operation_id: [u8; 32],
    component_count: u32,
    inventory_hash: [u8; 32],
) -> Result<(), InternalError> {
    if receipt.fleet_activation_operation_id != fleet_activation_operation_id {
        return Err(InternalError::conflict(
            "initial Component inventory is bound to a different Fleet activation",
        ));
    }
    if receipt.component_count != component_count
        || receipt.inventory_hash != inventory_hash
        || receipt.sealed_at_ns == 0
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "sealed initial Component inventory differs from current protected authority",
        ));
    }
    Ok(())
}

fn update_initial_inventory_receipt(
    fleet_activation_operation_id: [u8; 32],
    expected_inventory_hash: [u8; 32],
    directories_converged: bool,
    root_runtime_activated: bool,
) -> Result<RootComponentInitialInventoryView, InternalError> {
    let current = RootComponentRegistryStore::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    let mut receipt = current.initial_inventory.ok_or_else(|| {
        InternalError::unavailable("initial Component inventory has not been sealed")
    })?;
    if receipt.fleet_activation_operation_id != fleet_activation_operation_id
        || receipt.inventory_hash != expected_inventory_hash
    {
        return Err(InternalError::conflict(
            "root activation receipt differs from its sealed initial Component inventory",
        ));
    }
    if root_runtime_activated && !directories_converged {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "root runtime activation cannot precede initial Directory convergence",
        ));
    }
    receipt.directories_converged |= directories_converged;
    receipt.root_runtime_activated |= root_runtime_activated;
    if receipt.root_runtime_activated && !receipt.directories_converged {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "root runtime receipt has no initial Directory convergence evidence",
        ));
    }
    if current.initial_inventory == Some(receipt) {
        return Ok(initial_inventory_record_to_view(receipt));
    }
    let mut next = current.clone();
    next.initial_inventory = Some(receipt);
    RootComponentRegistryStore::replace_meta(&current, next)
        .map_err(map_allocation_commit_error)?;
    Ok(initial_inventory_record_to_view(receipt))
}

fn record_to_view(record: RootComponentRegistryMetaRecord) -> RootComponentRegistryView {
    RootComponentRegistryView {
        root: record.root,
        prepared_against_registry: record.prepared_against_registry,
        release_set: record.release_set,
        store_bootstrap: record.store_bootstrap,
        next_allocation_sequence: record.next_allocation_sequence,
        reserved_component_instances: record.reserved_component_instances,
        committed_component_instances: record.committed_component_instances,
        managed_descendants: record.managed_descendants,
        known_created_component_canisters: record.known_created_component_canisters,
        encoded_bytes: record.encoded_bytes,
        initial_inventory: record
            .initial_inventory
            .map(initial_inventory_record_to_view),
    }
}

const fn initial_inventory_record_to_view(
    record: RootComponentInitialInventoryRecord,
) -> RootComponentInitialInventoryView {
    RootComponentInitialInventoryView {
        fleet_activation_operation_id: record.fleet_activation_operation_id,
        component_count: record.component_count,
        inventory_hash: record.inventory_hash,
        sealed_at_ns: record.sealed_at_ns,
        directories_converged: record.directories_converged,
        root_runtime_activated: record.root_runtime_activated,
    }
}

fn allocation_record_to_view(record: RootComponentAllocationRecord) -> RootComponentAllocationView {
    RootComponentAllocationView {
        operation_id: record.operation_id,
        allocation_sequence: record.allocation_sequence,
        component: record.component,
        component_spec: record.component_spec,
        spec_hash: record.spec_hash,
        role: record.role,
        provisioning_origin: record.provisioning_origin,
        release_set: record.release_set,
        progress: match record.progress {
            RootComponentAllocationProgressRecord::Reserved => {
                RootComponentAllocationProgressView::Reserved
            }
            RootComponentAllocationProgressRecord::CreationIntent(effect) => {
                RootComponentAllocationProgressView::CreationIntent(creation_effect_record_to_view(
                    effect,
                ))
            }
            RootComponentAllocationProgressRecord::Created { effect, canister } => {
                RootComponentAllocationProgressView::Created {
                    effect: creation_effect_record_to_view(effect),
                    canister,
                }
            }
            RootComponentAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            } => RootComponentAllocationProgressView::InstallIntent {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: install_effect_record_to_view(installation),
            },
            RootComponentAllocationProgressRecord::Installed {
                creation,
                canister,
                installation,
            } => RootComponentAllocationProgressView::Installed {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: install_effect_record_to_view(installation),
            },
            RootComponentAllocationProgressRecord::Verified {
                creation,
                canister,
                installation,
            } => RootComponentAllocationProgressView::Verified {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: install_effect_record_to_view(installation),
            },
            RootComponentAllocationProgressRecord::Committed {
                creation,
                canister,
                installation,
                commitment,
            } => RootComponentAllocationProgressView::Committed {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: install_effect_record_to_view(installation),
                commitment: commitment_record_to_view(commitment),
            },
        },
    }
}

const fn creation_effect_record_to_view(
    effect: RootComponentCreationEffectRecord,
) -> RootComponentCreationEffectView {
    RootComponentCreationEffectView {
        wasm_store: effect.wasm_store,
        payload_hash: effect.payload_hash,
        payload_size_bytes: effect.payload_size_bytes,
        initial_cycles: effect.initial_cycles,
        controller: effect.controller,
        cost_guard_settlement: effect.cost_guard_settlement,
        charged_entry_bytes: effect.charged_entry_bytes,
    }
}

fn install_effect_record_to_view(
    effect: RootComponentInstallEffectRecord,
) -> RootComponentInstallEffectView {
    RootComponentInstallEffectView {
        raw_module_hash: effect.raw_module_hash,
        chunk_hashes: effect.chunk_hashes,
        binding: effect.binding,
        cost_guard_settlement: effect.cost_guard_settlement,
        charged_entry_bytes: effect.charged_entry_bytes,
    }
}

fn commitment_record_to_view(
    commitment: RootComponentCommitmentRecord,
) -> RootComponentCommitmentView {
    RootComponentCommitmentView {
        registry: commitment.registry,
        prepared_registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
        directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
        directory_authority_hash: commitment.directory_authority_hash,
        directory_prepared: commitment.directory_prepared,
        runtime_activated: commitment.runtime_activated,
        membership: commitment.membership.map(membership_record_to_view),
    }
}

const fn membership_record_to_view(
    membership: RootComponentMembershipRecord,
) -> RootComponentMembershipView {
    RootComponentMembershipView {
        registry_encoded_bytes: membership.registry_encoded_bytes,
        directory_synchronized_at_ns: membership.directory_synchronized_at_ns,
        directory_authority_hash: membership.directory_authority_hash,
        directory_synchronized: membership.directory_synchronized,
    }
}

fn partition_record_to_view(
    record: ComponentRegistryPartitionRecord,
) -> ComponentRegistryPartitionView {
    ComponentRegistryPartitionView {
        binding: record.binding,
        provisioning_origin: record.provisioning_origin,
        release_set: record.release_set,
        status: record.status,
        revision: record.revision,
        content_hash: record.content_hash,
        descendant_content_hash: record.descendant_content_hash,
        directory_synchronized_at_ns: record.directory_synchronized_at_ns,
        reserved_descendants: record.reserved_descendants,
        committed_descendants: record.committed_descendants,
        encoded_bytes: record.encoded_bytes,
    }
}

fn child_allocation_record_to_view(
    record: RootComponentChildAllocationRecord,
) -> RootComponentChildAllocationView {
    RootComponentChildAllocationView {
        operation_id: record.operation_id,
        component: record.component,
        parent_canister_id: record.parent_canister_id,
        parent_role: record.parent_role,
        child_role: record.child_role,
        child_kind: record.child_kind,
        maximum_instances_per_parent: record.maximum_instances_per_parent,
        maximum_descendants: record.maximum_descendants,
        maximum_registry_bytes: record.maximum_registry_bytes,
        reserved_against_registry: record.reserved_against_registry,
        release_set: record.release_set,
        progress: match record.progress {
            RootComponentChildAllocationProgressRecord::Reserved => {
                RootComponentChildAllocationProgressView::Reserved
            }
            RootComponentChildAllocationProgressRecord::CreationIntent(effect) => {
                RootComponentChildAllocationProgressView::CreationIntent(
                    creation_effect_record_to_view(effect),
                )
            }
            RootComponentChildAllocationProgressRecord::Created { effect, canister } => {
                RootComponentChildAllocationProgressView::Created {
                    effect: creation_effect_record_to_view(effect),
                    canister,
                }
            }
            RootComponentChildAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            } => RootComponentChildAllocationProgressView::InstallIntent {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: child_install_effect_record_to_view(installation),
            },
            RootComponentChildAllocationProgressRecord::Installed {
                creation,
                canister,
                installation,
            } => RootComponentChildAllocationProgressView::Installed {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: child_install_effect_record_to_view(installation),
            },
            RootComponentChildAllocationProgressRecord::Verified {
                creation,
                canister,
                installation,
            } => RootComponentChildAllocationProgressView::Verified {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: child_install_effect_record_to_view(installation),
            },
            RootComponentChildAllocationProgressRecord::Committed {
                creation,
                canister,
                installation,
                commitment,
            } => RootComponentChildAllocationProgressView::Committed {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: child_install_effect_record_to_view(installation),
                commitment: child_commitment_record_to_view(commitment),
            },
        },
    }
}

fn subtree_removal_record_to_view(
    record: RootComponentSubtreeRemovalRecord,
) -> RootComponentSubtreeRemovalView {
    RootComponentSubtreeRemovalView {
        operation_id: record.operation_id,
        component: record.component,
        target_canister_id: record.target.canister_id,
        target_parent_canister_id: record.target.parent_canister_id,
        target_role: record.target.role,
        target_status: record.target.status,
        reserved_against_registry: record.reserved_against_registry,
        maximum_completed_leaves: record.maximum_completed_leaves,
        completed_leaves: record.completed_leaves,
        traversal_steps: record.traversal_steps,
        progress: match record.progress {
            RootComponentSubtreeRemovalProgressRecord::Fenced => {
                RootComponentSubtreeRemovalProgressView::Fenced
            }
            RootComponentSubtreeRemovalProgressRecord::Traversing { cursor } => {
                RootComponentSubtreeRemovalProgressView::Traversing {
                    cursor: subtree_removal_node_view(cursor),
                }
            }
            RootComponentSubtreeRemovalProgressRecord::LeafSelected { leaf } => {
                RootComponentSubtreeRemovalProgressView::LeafSelected {
                    leaf: subtree_removal_node_view(leaf),
                }
            }
            RootComponentSubtreeRemovalProgressRecord::StopIntent(effect) => {
                RootComponentSubtreeRemovalProgressView::StopIntent(
                    RootComponentSubtreeStopEffectView {
                        leaf: subtree_removal_node_view(effect.leaf),
                        controller: effect.controller,
                    },
                )
            }
            RootComponentSubtreeRemovalProgressRecord::Stopped(receipt) => {
                RootComponentSubtreeRemovalProgressView::Stopped(
                    subtree_stopped_effect_record_to_view(receipt),
                )
            }
            RootComponentSubtreeRemovalProgressRecord::DeleteIntent(deletion) => {
                RootComponentSubtreeRemovalProgressView::DeleteIntent(
                    subtree_delete_effect_record_to_view(deletion),
                )
            }
            RootComponentSubtreeRemovalProgressRecord::Deleted(receipt) => {
                RootComponentSubtreeRemovalProgressView::Deleted(
                    RootComponentSubtreeDeletedEffectView {
                        deletion: subtree_delete_effect_record_to_view(receipt.deletion),
                    },
                )
            }
            RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) => {
                RootComponentSubtreeRemovalProgressView::MembershipRemoved(
                    subtree_membership_removed_record_to_view(receipt),
                )
            }
            RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) => {
                RootComponentSubtreeRemovalProgressView::DirectorySynchronized(
                    RootComponentSubtreeDirectorySynchronizedView {
                        membership_removed: subtree_membership_removed_record_to_view(
                            receipt.membership_removed,
                        ),
                        covered_fleet_registry_revision: receipt.covered_fleet_registry_revision,
                        covered_fleet_registry_content_hash: receipt
                            .covered_fleet_registry_content_hash,
                        covered_component_registry: ComponentRegistryHead {
                            component: record.component,
                            revision: receipt.covered_component_registry_revision,
                            content_hash: receipt.covered_component_registry_content_hash,
                        },
                        covered_authority_hash: receipt.covered_authority_hash,
                        owning_component: receipt
                            .owning_component
                            .map(subtree_directory_convergence_record_to_view),
                        parent: receipt
                            .parent
                            .map(subtree_directory_convergence_record_to_view),
                    },
                )
            }
            RootComponentSubtreeRemovalProgressRecord::Completed(completed) => {
                RootComponentSubtreeRemovalProgressView::Completed(
                    RootComponentSubtreeRemovalCompletedView {
                        registry: completed.registry,
                        directory_authority_hash: completed.directory_authority_hash,
                    },
                )
            }
        },
    }
}

fn component_draining_record_to_view(
    record: RootComponentDrainingRecord,
) -> RootComponentDrainingView {
    RootComponentDrainingView {
        operation_id: record.operation_id,
        component: record.component,
        previous_registry: record.previous_registry,
        registry: record.registry,
        descendant_count: record.descendant_count,
        descendant_content_hash: record.descendant_content_hash,
        directory_authority_hash: record.directory_authority_hash,
        started_at_ns: record.started_at_ns,
        quiescence: record.quiescence.map(|progress| match progress {
            RootComponentQuiescenceProgressRecord::StopIntent(intent) => {
                RootComponentQuiescenceProgressView::StopIntent(
                    component_quiescence_stop_intent_record_to_view(intent),
                )
            }
            RootComponentQuiescenceProgressRecord::Quiescent(receipt) => {
                RootComponentQuiescenceProgressView::Quiescent(RootComponentQuiescentReceiptView {
                    stop: component_quiescence_stop_intent_record_to_view(receipt.stop),
                    observed_module_hash: receipt.observed_module_hash,
                    quiesced_at_ns: receipt.quiesced_at_ns,
                })
            }
        }),
        final_inventory: record
            .final_inventory
            .map(component_final_inventory_record_to_view),
        deletion: record.deletion.map(|progress| match progress {
            RootComponentDeletionProgressRecord::DeleteIntent(intent) => {
                RootComponentDeletionProgressView::DeleteIntent(
                    component_deletion_intent_record_to_view(intent),
                )
            }
            RootComponentDeletionProgressRecord::Deleted(receipt) => {
                RootComponentDeletionProgressView::Deleted(RootComponentDeletedReceiptView {
                    deletion: component_deletion_intent_record_to_view(receipt.deletion),
                    deleted_at_ns: receipt.deleted_at_ns,
                })
            }
        }),
    }
}

const fn component_deletion_intent_record_to_view(
    record: RootComponentDeletionIntentRecord,
) -> RootComponentDeletionIntentView {
    RootComponentDeletionIntentView {
        final_inventory: component_final_inventory_record_to_view(record.final_inventory),
        quiescence: RootComponentQuiescentReceiptView {
            stop: component_quiescence_stop_intent_record_to_view(record.quiescence.stop),
            observed_module_hash: record.quiescence.observed_module_hash,
            quiesced_at_ns: record.quiescence.quiesced_at_ns,
        },
        prepared_at_ns: record.prepared_at_ns,
    }
}

const fn component_final_inventory_record_to_view(
    record: RootComponentFinalInventoryRecord,
) -> RootComponentFinalInventoryView {
    RootComponentFinalInventoryView {
        registry: record.registry,
        descendant_content_hash: record.descendant_content_hash,
        registry_encoded_bytes: record.registry_encoded_bytes,
        directory_synchronized_at_ns: record.directory_synchronized_at_ns,
        covered_fleet_registry_revision: record.covered_fleet_registry_revision,
        covered_fleet_registry_content_hash: record.covered_fleet_registry_content_hash,
        directory_authority_hash: record.directory_authority_hash,
        inventory_hash: record.inventory_hash,
        finalized_at_ns: record.finalized_at_ns,
    }
}

const fn component_quiescence_stop_intent_record_to_view(
    record: RootComponentQuiescenceStopIntentRecord,
) -> RootComponentQuiescenceStopIntentView {
    RootComponentQuiescenceStopIntentView {
        registry: record.registry,
        descendant_count: record.descendant_count,
        descendant_content_hash: record.descendant_content_hash,
        canister_id: record.canister_id,
        controller: record.controller,
        expected_module_hash: record.expected_module_hash,
        covered_fleet_registry_revision: record.covered_fleet_registry_revision,
        covered_fleet_registry_content_hash: record.covered_fleet_registry_content_hash,
        covered_authority_hash: record.covered_authority_hash,
        runtime_operation_id: record.runtime_operation_id,
        activation: record.activation,
        prepared_at_ns: record.prepared_at_ns,
        charged_entry_bytes: record.charged_entry_bytes,
    }
}

fn subtree_removal_node_view(
    record: ComponentRegistryChildRecord,
) -> RootComponentSubtreeRemovalNodeView {
    RootComponentSubtreeRemovalNodeView {
        canister_id: record.canister_id,
        parent_canister_id: record.parent_canister_id,
        role: record.role,
        kind: record.kind,
        installed_artifact_hash: record.installed_artifact_hash,
        status: record.status,
    }
}

fn subtree_stopped_effect_record_to_view(
    record: RootComponentSubtreeStoppedEffectRecord,
) -> RootComponentSubtreeStoppedEffectView {
    RootComponentSubtreeStoppedEffectView {
        stop: RootComponentSubtreeStopEffectView {
            leaf: subtree_removal_node_view(record.stop.leaf),
            controller: record.stop.controller,
        },
        observed_module_hash: record.observed_module_hash,
    }
}

fn subtree_delete_effect_record_to_view(
    record: RootComponentSubtreeDeleteEffectRecord,
) -> RootComponentSubtreeDeleteEffectView {
    RootComponentSubtreeDeleteEffectView {
        stopped: subtree_stopped_effect_record_to_view(record.stopped),
    }
}

fn subtree_membership_removed_record_to_view(
    receipt: RootComponentSubtreeMembershipRemovedRecord,
) -> RootComponentSubtreeMembershipRemovedView {
    RootComponentSubtreeMembershipRemovedView {
        deleted: RootComponentSubtreeDeletedEffectView {
            deletion: subtree_delete_effect_record_to_view(receipt.deleted.deletion),
        },
        removed_from_registry: receipt.removed_from_registry,
        previous_descendant_content_hash: receipt.previous_descendant_content_hash,
        previous_committed_descendants: receipt.previous_committed_descendants,
        registry: receipt.registry,
        descendant_content_hash: receipt.descendant_content_hash,
        registry_encoded_bytes: receipt.registry_encoded_bytes,
        reserved_descendants: receipt.reserved_descendants,
        committed_descendants: receipt.committed_descendants,
        directory_synchronized_at_ns: receipt.directory_synchronized_at_ns,
        directory_authority_hash: receipt.directory_authority_hash,
        parent_role_instances: receipt.parent_role_instances,
        root_managed_descendants: receipt.root_managed_descendants,
        root_known_created_component_canisters: receipt.root_known_created_component_canisters,
    }
}

const fn subtree_directory_convergence_record_to_view(
    evidence: RootComponentSubtreeDirectoryConvergenceRecord,
) -> RootComponentSubtreeDirectoryConvergenceView {
    RootComponentSubtreeDirectoryConvergenceView {
        operation_id: evidence.operation_id,
        canister_id: evidence.canister_id,
        activation: evidence.activation,
    }
}

fn child_commitment_record_to_view(
    commitment: RootComponentChildCommitmentRecord,
) -> RootComponentChildCommitmentView {
    RootComponentChildCommitmentView {
        registry: commitment.registry,
        descendant_content_hash: commitment.descendant_content_hash,
        registry_encoded_bytes: commitment.registry_encoded_bytes,
        reserved_descendants: commitment.reserved_descendants,
        committed_descendants: commitment.committed_descendants,
        directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
        directory_authority_hash: commitment.directory_authority_hash,
        directory_prepared: commitment.directory_prepared,
        runtime_activated: commitment.runtime_activated,
        membership: commitment.membership.map(child_membership_record_to_view),
    }
}

const fn child_membership_record_to_view(
    membership: RootComponentChildMembershipRecord,
) -> RootComponentChildMembershipView {
    RootComponentChildMembershipView {
        registry: membership.registry,
        descendant_content_hash: membership.descendant_content_hash,
        registry_encoded_bytes: membership.registry_encoded_bytes,
        reserved_descendants: membership.reserved_descendants,
        committed_descendants: membership.committed_descendants,
        directory_synchronized_at_ns: membership.directory_synchronized_at_ns,
        directory_authority_hash: membership.directory_authority_hash,
        directory_synchronized: membership.directory_synchronized,
    }
}

fn child_install_effect_record_to_view(
    effect: RootComponentChildInstallEffectRecord,
) -> RootComponentChildInstallEffectView {
    RootComponentChildInstallEffectView {
        raw_module_hash: effect.raw_module_hash,
        chunk_hashes: effect.chunk_hashes,
        binding: effect.binding,
        cost_guard_settlement: effect.cost_guard_settlement,
        charged_entry_bytes: effect.charged_entry_bytes,
    }
}

fn child_reservation_partition(
    current: &ComponentRegistryPartitionRecord,
    allocation: &RootComponentChildAllocationRecord,
    current_count: Option<&ComponentRegistryParentRoleCountRecord>,
    next_count: &ComponentRegistryParentRoleCountRecord,
) -> Result<(ComponentRegistryPartitionRecord, u64), InternalError> {
    let allocation_identity = ComponentParentRoleIdentity::from_allocation(allocation);
    let current_count_is_valid = current_count.is_none_or(|count| {
        ComponentParentRoleIdentity::from_count(count) == allocation_identity && count.instances > 0
    });
    if !current_count_is_valid {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Registry parent-role count index is invalid",
        ));
    }
    let current_partition_bytes = RootComponentRegistryStore::partition_entry_bytes(current);
    let current_count_bytes = current_count
        .map(RootComponentRegistryStore::parent_role_count_entry_bytes)
        .unwrap_or_default();
    let allocation_bytes = RootComponentRegistryStore::child_allocation_entry_bytes(allocation);
    let next_count_bytes = RootComponentRegistryStore::parent_role_count_entry_bytes(next_count);
    let mut next = current.clone();
    next.reserved_descendants = next.reserved_descendants.checked_add(1).ok_or_else(|| {
        InternalError::resource_exhausted("reserved Component descendant count overflow")
    })?;

    for _ in 0..8 {
        let next_partition_bytes = RootComponentRegistryStore::partition_entry_bytes(&next);
        let next_total = next_partition_bytes
            .checked_add(allocation_bytes)
            .and_then(|value| value.checked_add(next_count_bytes))
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let current_total = current_partition_bytes
            .checked_add(current_count_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let delta = next_total.checked_sub(current_total).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Child reservation unexpectedly reduced Registry bytes",
            )
        })?;
        let encoded_bytes = current.encoded_bytes.checked_add(delta).ok_or_else(|| {
            InternalError::resource_exhausted("Component Registry bytes overflow")
        })?;
        if next.encoded_bytes == encoded_bytes {
            return Ok((next, delta));
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component Child reservation byte accounting did not converge",
    ))
}

fn subtree_fence_partition(
    current: &ComponentRegistryPartitionRecord,
    removal: &RootComponentSubtreeRemovalRecord,
) -> Result<(ComponentRegistryPartitionRecord, u64), InternalError> {
    let current_partition_bytes = RootComponentRegistryStore::partition_entry_bytes(current);
    let removal_bytes = RootComponentRegistryStore::subtree_removal_entry_bytes(removal);
    let mut next = current.clone();

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next)
            .checked_add(removal_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let registry_delta = next_total
            .checked_sub(current_partition_bytes)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component subtree-removal fence unexpectedly reduced Registry bytes",
                )
            })?;
        let encoded_bytes = current
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if next.encoded_bytes == encoded_bytes {
            return Ok((next, registry_delta));
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component subtree-removal fence byte accounting did not converge",
    ))
}

fn subtree_removal_progress_state(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    current_record: &RootComponentSubtreeRemovalRecord,
    next_record: &RootComponentSubtreeRemovalRecord,
    maximum_component_registry_bytes: u64,
) -> Result<
    (
        ComponentRegistryPartitionRecord,
        RootComponentRegistryMetaRecord,
    ),
    InternalError,
> {
    let current_total = RootComponentRegistryStore::partition_entry_bytes(partition)
        .checked_add(RootComponentRegistryStore::subtree_removal_entry_bytes(
            current_record,
        ))
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    let component_without_current = partition
        .encoded_bytes
        .checked_sub(current_total)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry bytes are below subtree traversal authority",
            )
        })?;
    let root_without_current = current
        .encoded_bytes
        .checked_sub(current_total)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "root Registry bytes are below subtree traversal authority",
            )
        })?;
    let next_record_bytes = RootComponentRegistryStore::subtree_removal_entry_bytes(next_record);
    let mut next_partition = partition.clone();

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next_partition)
            .checked_add(next_record_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let next_component_bytes = component_without_current
            .checked_add(next_total)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if next_partition.encoded_bytes == next_component_bytes {
            if next_component_bytes > maximum_component_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component subtree-removal progress requires {next_component_bytes} bytes, exceeding protected Component limit {maximum_component_registry_bytes}"
                )));
            }
            let next_root_bytes =
                root_without_current
                    .checked_add(next_total)
                    .ok_or_else(|| {
                        InternalError::resource_exhausted("Component Registry bytes overflow")
                    })?;
            if next_root_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component subtree-removal progress requires {next_root_bytes} root Registry bytes, exceeding protected limit {}",
                    current.root.limits.maximum_registry_bytes
                )));
            }
            let mut next_meta = current.clone();
            next_meta.encoded_bytes = next_root_bytes;
            return Ok((next_partition, next_meta));
        }
        next_partition.encoded_bytes = next_component_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component subtree-removal progress byte accounting did not converge",
    ))
}

fn component_draining_state(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    mut next_partition: ComponentRegistryPartitionRecord,
    record: &RootComponentDrainingRecord,
    maximum_component_registry_bytes: u64,
) -> Result<
    (
        ComponentRegistryPartitionRecord,
        RootComponentRegistryMetaRecord,
    ),
    InternalError,
> {
    let current_partition_bytes = RootComponentRegistryStore::partition_entry_bytes(partition);
    let component_without_partition = partition
        .encoded_bytes
        .checked_sub(current_partition_bytes)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry bytes are below the partition being drained",
            )
        })?;
    let root_without_partition = current
        .encoded_bytes
        .checked_sub(current_partition_bytes)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "root Registry bytes are below the Component partition being drained",
            )
        })?;
    let draining_bytes = RootComponentRegistryStore::component_draining_entry_bytes(record);

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next_partition)
            .checked_add(draining_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let next_component_bytes = component_without_partition
            .checked_add(next_total)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if next_partition.encoded_bytes == next_component_bytes {
            if next_component_bytes > maximum_component_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component draining requires {next_component_bytes} bytes, exceeding protected Component limit {maximum_component_registry_bytes}"
                )));
            }
            let next_root_bytes =
                root_without_partition
                    .checked_add(next_total)
                    .ok_or_else(|| {
                        InternalError::resource_exhausted("Component Registry bytes overflow")
                    })?;
            if next_root_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component draining requires {next_root_bytes} root Registry bytes, exceeding protected limit {}",
                    current.root.limits.maximum_registry_bytes
                )));
            }
            let mut next_meta = current.clone();
            next_meta.encoded_bytes = next_root_bytes;
            return Ok((next_partition, next_meta));
        }
        next_partition.encoded_bytes = next_component_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component draining byte accounting did not converge",
    ))
}

fn component_quiescence_terminal_entry_bytes(
    draining: &RootComponentDrainingRecord,
    intent: &RootComponentQuiescenceStopIntentRecord,
) -> Result<u64, InternalError> {
    let mut charged_entry_bytes = 0;
    for _ in 0..8 {
        let mut terminal_intent = intent.clone();
        terminal_intent.charged_entry_bytes = charged_entry_bytes;
        let mut terminal = draining.clone();
        terminal.subtree_operation_id = Some([u8::MAX; 32]);
        let final_inventory = RootComponentFinalInventoryRecord {
            registry: ComponentRegistryHead {
                component: draining.component,
                revision: u64::MAX,
                content_hash: [u8::MAX; 32],
            },
            descendant_content_hash: [u8::MAX; 32],
            registry_encoded_bytes: u64::MAX,
            directory_synchronized_at_ns: u64::MAX,
            covered_fleet_registry_revision: u64::MAX,
            covered_fleet_registry_content_hash: [u8::MAX; 32],
            directory_authority_hash: [u8::MAX; 32],
            inventory_hash: [u8::MAX; 32],
            finalized_at_ns: u64::MAX,
        };
        let quiescence = RootComponentQuiescentReceiptRecord {
            stop: terminal_intent,
            observed_module_hash: intent.expected_module_hash,
            quiesced_at_ns: u64::MAX,
        };
        terminal.final_inventory = Some(final_inventory.clone());
        terminal.quiescence = Some(RootComponentQuiescenceProgressRecord::Quiescent(
            quiescence.clone(),
        ));
        terminal.deletion = Some(RootComponentDeletionProgressRecord::Deleted(
            RootComponentDeletedReceiptRecord {
                deletion: RootComponentDeletionIntentRecord {
                    final_inventory,
                    quiescence,
                    prepared_at_ns: u64::MAX,
                },
                deleted_at_ns: u64::MAX,
            },
        ));
        let next = RootComponentRegistryStore::component_draining_entry_bytes(&terminal);
        if next == charged_entry_bytes {
            return Ok(next);
        }
        charged_entry_bytes = next;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component quiescence terminal byte reservation did not converge",
    ))
}

fn charged_component_draining_entry_bytes(record: &RootComponentDrainingRecord) -> u64 {
    match &record.quiescence {
        Some(RootComponentQuiescenceProgressRecord::StopIntent(intent)) => {
            intent.charged_entry_bytes
        }
        Some(RootComponentQuiescenceProgressRecord::Quiescent(receipt)) => {
            receipt.stop.charged_entry_bytes
        }
        None => RootComponentRegistryStore::component_draining_entry_bytes(record),
    }
}

fn component_quiescence_intent_state(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    current_draining: &RootComponentDrainingRecord,
    next_draining: &RootComponentDrainingRecord,
    maximum_component_registry_bytes: u64,
) -> Result<
    (
        ComponentRegistryPartitionRecord,
        RootComponentRegistryMetaRecord,
    ),
    InternalError,
> {
    let current_partition_bytes = RootComponentRegistryStore::partition_entry_bytes(partition);
    let current_draining_bytes = charged_component_draining_entry_bytes(current_draining);
    let component_without_mutated_entries = partition
        .encoded_bytes
        .checked_sub(current_partition_bytes)
        .and_then(|bytes| bytes.checked_sub(current_draining_bytes))
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry bytes are below its partition and draining authority",
            )
        })?;
    let root_without_mutated_entries = current
        .encoded_bytes
        .checked_sub(current_partition_bytes)
        .and_then(|bytes| bytes.checked_sub(current_draining_bytes))
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "root Registry bytes are below Component quiescence authority",
            )
        })?;
    let next_draining_bytes = charged_component_draining_entry_bytes(next_draining);
    let mut next_partition = partition.clone();
    for _ in 0..8 {
        let next_mutated_bytes = RootComponentRegistryStore::partition_entry_bytes(&next_partition)
            .checked_add(next_draining_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let next_component_bytes = component_without_mutated_entries
            .checked_add(next_mutated_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if next_partition.encoded_bytes == next_component_bytes {
            if next_component_bytes > maximum_component_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component quiescence requires {next_component_bytes} bytes, exceeding protected Component limit {maximum_component_registry_bytes}"
                )));
            }
            let next_root_bytes = root_without_mutated_entries
                .checked_add(next_mutated_bytes)
                .ok_or_else(|| {
                    InternalError::resource_exhausted("Component Registry bytes overflow")
                })?;
            if next_root_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component quiescence requires {next_root_bytes} root Registry bytes, exceeding protected limit {}",
                    current.root.limits.maximum_registry_bytes
                )));
            }
            let mut next_meta = current.clone();
            next_meta.encoded_bytes = next_root_bytes;
            return Ok((next_partition, next_meta));
        }
        next_partition.encoded_bytes = next_component_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component quiescence byte accounting did not converge",
    ))
}

fn subtree_removal_leaf_finalization_state(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    current_record: &RootComponentSubtreeRemovalRecord,
    next_record: &RootComponentSubtreeRemovalRecord,
    completed_leaf: &RootComponentSubtreeRemovalCompletedLeafRecord,
    maximum_component_registry_bytes: u64,
) -> Result<
    (
        ComponentRegistryPartitionRecord,
        RootComponentRegistryMetaRecord,
    ),
    InternalError,
> {
    let current_total = RootComponentRegistryStore::partition_entry_bytes(partition)
        .checked_add(RootComponentRegistryStore::subtree_removal_entry_bytes(
            current_record,
        ))
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    let component_without_current = partition
        .encoded_bytes
        .checked_sub(current_total)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry bytes are below completed-leaf authority",
            )
        })?;
    let root_without_current = current
        .encoded_bytes
        .checked_sub(current_total)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "root Registry bytes are below completed-leaf authority",
            )
        })?;
    let next_record_bytes = RootComponentRegistryStore::subtree_removal_entry_bytes(next_record);
    let history_bytes =
        RootComponentRegistryStore::subtree_removal_completed_leaf_entry_bytes(completed_leaf);
    let mut next_partition = partition.clone();

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next_partition)
            .checked_add(next_record_bytes)
            .and_then(|bytes| bytes.checked_add(history_bytes))
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let next_component_bytes = component_without_current
            .checked_add(next_total)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if next_partition.encoded_bytes == next_component_bytes {
            if next_component_bytes > maximum_component_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component subtree leaf finalization requires {next_component_bytes} bytes, exceeding protected Component limit {maximum_component_registry_bytes}"
                )));
            }
            let next_root_bytes =
                root_without_current
                    .checked_add(next_total)
                    .ok_or_else(|| {
                        InternalError::resource_exhausted("Component Registry bytes overflow")
                    })?;
            if next_root_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component subtree leaf finalization requires {next_root_bytes} root Registry bytes, exceeding protected limit {}",
                    current.root.limits.maximum_registry_bytes
                )));
            }
            let mut next_meta = current.clone();
            next_meta.encoded_bytes = next_root_bytes;
            return Ok((next_partition, next_meta));
        }
        next_partition.encoded_bytes = next_component_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component subtree leaf-finalization byte accounting did not converge",
    ))
}

#[expect(
    clippy::too_many_arguments,
    reason = "byte convergence covers every authority removed or rewritten atomically"
)]
fn converge_subtree_membership_removal_bytes(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    current_record: &RootComponentSubtreeRemovalRecord,
    child: &ComponentRegistryChildRecord,
    traversal: &ComponentRegistryChildTraversalRecord,
    parent_role_count: &ComponentRegistryParentRoleCountRecord,
    next_parent_role_count: Option<&ComponentRegistryParentRoleCountRecord>,
    next_meta: &mut RootComponentRegistryMetaRecord,
    next_partition: &mut ComponentRegistryPartitionRecord,
    next_record: &mut RootComponentSubtreeRemovalRecord,
    maximum_component_registry_bytes: u64,
) -> Result<(), InternalError> {
    let current_total = RootComponentRegistryStore::partition_entry_bytes(partition)
        .checked_add(RootComponentRegistryStore::subtree_removal_entry_bytes(
            current_record,
        ))
        .and_then(|bytes| bytes.checked_add(RootComponentRegistryStore::child_entry_bytes(child)))
        .and_then(|bytes| {
            bytes.checked_add(RootComponentRegistryStore::child_traversal_entry_bytes(
                traversal,
            ))
        })
        .and_then(|bytes| {
            bytes.checked_add(RootComponentRegistryStore::principal_index_entry_bytes(
                child.canister_id,
                child.component,
            ))
        })
        .and_then(|bytes| {
            bytes.checked_add(RootComponentRegistryStore::parent_role_count_entry_bytes(
                parent_role_count,
            ))
        })
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    let component_without_current = partition
        .encoded_bytes
        .checked_sub(current_total)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry bytes are below removed leaf authority",
            )
        })?;
    let root_without_current = current
        .encoded_bytes
        .checked_sub(current_total)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "root Registry bytes are below removed leaf authority",
            )
        })?;
    let next_count_bytes =
        next_parent_role_count.map_or(0, RootComponentRegistryStore::parent_role_count_entry_bytes);

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(next_partition)
            .checked_add(RootComponentRegistryStore::subtree_removal_entry_bytes(
                next_record,
            ))
            .and_then(|bytes| bytes.checked_add(next_count_bytes))
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let next_component_bytes = component_without_current
            .checked_add(next_total)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let next_root_bytes = root_without_current
            .checked_add(next_total)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "Component membership-removal byte convergence has no removal receipt",
            ));
        };
        if next_partition.encoded_bytes == next_component_bytes
            && receipt.registry_encoded_bytes == next_component_bytes
        {
            if next_component_bytes > maximum_component_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component subtree membership removal requires {next_component_bytes} bytes, exceeding protected Component limit {maximum_component_registry_bytes}"
                )));
            }
            if next_root_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component subtree membership removal requires {next_root_bytes} root Registry bytes, exceeding protected limit {}",
                    current.root.limits.maximum_registry_bytes
                )));
            }
            next_meta.encoded_bytes = next_root_bytes;
            return Ok(());
        }
        next_partition.encoded_bytes = next_component_bytes;
        receipt.registry_encoded_bytes = next_component_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component subtree membership-removal byte accounting did not converge",
    ))
}

fn validate_child_creation_authority(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentChildAllocationRecord,
    plan: &RootComponentCreationPlan,
) -> Result<(), InternalError> {
    validate_partition_record(partition)?;
    validate_child_allocation_record(record)?;
    let partition_authority = ComponentPartitionLifecycleAuthority::from_partition(partition);
    let reservation_authority = ComponentPartitionLifecycleAuthority::active_reservation(record);
    let root_controls_creation = plan.controller == current.root.fleet_subnet_root;
    if partition_authority != reservation_authority
        || !root_controls_creation
        || !plan.has_valid_store_artifact()
    {
        return Err(InternalError::conflict(
            "Component Child creation authority differs from its active reservation",
        ));
    }
    Ok(())
}

fn child_creation_capacity(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentChildAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<(ComponentRegistryPartitionRecord, u64), InternalError> {
    let current_partition_bytes = RootComponentRegistryStore::partition_entry_bytes(partition);
    let current_record_bytes = RootComponentRegistryStore::child_allocation_entry_bytes(record);
    if charged_entry_bytes < current_record_bytes {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "Component Child creation charge is smaller than its reservation record",
        ));
    }
    let current_total = current_partition_bytes
        .checked_add(current_record_bytes)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    let mut next = partition.clone();

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next)
            .checked_add(charged_entry_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let registry_delta = next_total.checked_sub(current_total).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "Component Child creation precharge unexpectedly reduced Registry bytes",
            )
        })?;
        let encoded_bytes = partition
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if next.encoded_bytes == encoded_bytes {
            if encoded_bytes > record.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component Child creation requires {encoded_bytes} bytes, exceeding protected Component limit {}",
                    record.maximum_registry_bytes
                )));
            }
            let root_encoded_bytes = current
                .encoded_bytes
                .checked_add(registry_delta)
                .ok_or_else(|| {
                    InternalError::resource_exhausted("Component Registry bytes overflow")
                })?;
            if root_encoded_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component Child creation requires {root_encoded_bytes} root Registry bytes, exceeding protected limit {}",
                    current.root.limits.maximum_registry_bytes
                )));
            }
            return Ok((next, registry_delta));
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component Child creation byte accounting did not converge",
    ))
}

fn child_creation_charged_entry_bytes(
    record: &RootComponentChildAllocationRecord,
    plan: &RootComponentCreationPlan,
) -> u64 {
    let mut maximum = record.clone();
    maximum.progress = RootComponentChildAllocationProgressRecord::Created {
        effect: RootComponentCreationEffectRecord {
            wasm_store: plan.wasm_store,
            payload_hash: plan.payload_hash,
            payload_size_bytes: u64::MAX,
            initial_cycles: Cycles::new(u128::MAX),
            controller: plan.controller,
            cost_guard_settlement: ReplayCostGuardSettlement {
                quota_intent_id: IntentId(u64::MAX),
                reservation_intent_id: IntentId(u64::MAX),
            },
            charged_entry_bytes: u64::MAX,
        },
        canister: Principal::from_slice(&[u8::MAX; 29]),
    };
    RootComponentRegistryStore::child_allocation_entry_bytes(&maximum)
}

fn validate_charged_child_record_size(
    record: &RootComponentChildAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<(), InternalError> {
    if RootComponentRegistryStore::child_allocation_entry_bytes(record) > charged_entry_bytes {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Child allocation exceeded its precharged stable footprint",
        ));
    }
    Ok(())
}

fn validate_child_install_authority(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentChildAllocationRecord,
    plan: &RootComponentChildInstallPlan,
) -> Result<(), InternalError> {
    validate_partition_record(partition)?;
    validate_child_allocation_record(record)?;
    let canister = match &record.progress {
        RootComponentChildAllocationProgressRecord::Created { canister, .. }
        | RootComponentChildAllocationProgressRecord::InstallIntent { canister, .. }
        | RootComponentChildAllocationProgressRecord::Installed { canister, .. }
        | RootComponentChildAllocationProgressRecord::Verified { canister, .. }
        | RootComponentChildAllocationProgressRecord::Committed { canister, .. } => *canister,
        RootComponentChildAllocationProgressRecord::Reserved
        | RootComponentChildAllocationProgressRecord::CreationIntent(_) => {
            return Err(InternalError::conflict(
                "Component Child allocation has no created Canister",
            ));
        }
    };
    let expected_binding = ComponentChildBinding {
        component: partition.binding.clone(),
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        canister_id: canister,
    };
    let partition_authority = ComponentPartitionLifecycleAuthority::from_partition(partition);
    let reservation_authority = ComponentPartitionLifecycleAuthority::active_reservation(record);
    let artifact_source_is_valid = plan.raw_module_hash != [0; 32] && !plan.chunk_hashes.is_empty();
    let observed_authority = ComponentChildInstallReservationAuthority {
        binding: &plan.binding,
        partition: partition_authority,
        root_release_set: current.release_set,
        maximum_registry_bytes: plan.maximum_registry_bytes,
    };
    let expected_authority = ComponentChildInstallReservationAuthority {
        binding: &expected_binding,
        partition: reservation_authority,
        root_release_set: record.release_set,
        maximum_registry_bytes: record.maximum_registry_bytes,
    };
    if observed_authority != expected_authority || !artifact_source_is_valid {
        return Err(InternalError::conflict(
            "Component Child install authority differs from its active reservation",
        ));
    }
    Ok(())
}

fn child_install_charged_entry_bytes(
    record: &RootComponentChildAllocationRecord,
    plan: &RootComponentChildInstallPlan,
) -> Result<u64, InternalError> {
    let (creation, canister) = match &record.progress {
        RootComponentChildAllocationProgressRecord::Created { effect, canister } => {
            (effect.clone(), *canister)
        }
        _ => {
            return Err(InternalError::conflict(
                "Component Child allocation is not ready for installation",
            ));
        }
    };
    let installation = RootComponentChildInstallEffectRecord {
        raw_module_hash: plan.raw_module_hash,
        chunk_hashes: plan.chunk_hashes.clone(),
        binding: plan.binding.clone(),
        cost_guard_settlement: ReplayCostGuardSettlement {
            quota_intent_id: IntentId(u64::MAX),
            reservation_intent_id: IntentId(u64::MAX),
        },
        charged_entry_bytes: u64::MAX,
    };
    let mut maximum = record.clone();
    maximum.progress = RootComponentChildAllocationProgressRecord::Committed {
        creation,
        canister,
        installation,
        commitment: RootComponentChildCommitmentRecord {
            registry: ComponentRegistryHead {
                component: record.component,
                revision: u64::MAX,
                content_hash: [u8::MAX; 32],
            },
            descendant_content_hash: [u8::MAX; 32],
            registry_encoded_bytes: u64::MAX,
            reserved_descendants: u32::MAX,
            committed_descendants: u32::MAX,
            directory_synchronized_at_ns: u64::MAX,
            directory_authority_hash: [u8::MAX; 32],
            directory_prepared: true,
            runtime_activated: true,
            membership: Some(RootComponentChildMembershipRecord {
                registry: ComponentRegistryHead {
                    component: record.component,
                    revision: u64::MAX,
                    content_hash: [u8::MAX; 32],
                },
                descendant_content_hash: [u8::MAX; 32],
                registry_encoded_bytes: u64::MAX,
                reserved_descendants: u32::MAX,
                committed_descendants: u32::MAX,
                directory_synchronized_at_ns: u64::MAX,
                directory_authority_hash: [u8::MAX; 32],
                directory_synchronized: true,
            }),
        },
    };
    let child = ComponentRegistryChildRecord {
        component: record.component,
        canister_id: canister,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        kind: record.child_kind,
        installed_artifact_hash: plan.raw_module_hash,
        status: ComponentLifecycleStatus::Active,
    };
    let traversal = ComponentRegistryChildTraversalRecord {
        component: record.component,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        canister_id: canister,
    };
    RootComponentRegistryStore::child_allocation_entry_bytes(&maximum)
        .checked_add(RootComponentRegistryStore::child_entry_bytes(&child))
        .and_then(|value| {
            value.checked_add(RootComponentRegistryStore::child_traversal_entry_bytes(
                &traversal,
            ))
        })
        .and_then(|value| {
            value.checked_add(RootComponentRegistryStore::principal_index_entry_bytes(
                canister,
                record.component,
            ))
        })
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))
}

fn child_install_capacity(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentChildAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<(ComponentRegistryPartitionRecord, u64), InternalError> {
    let current_reserved_bytes = match &record.progress {
        RootComponentChildAllocationProgressRecord::Created { effect, .. } => {
            effect.charged_entry_bytes
        }
        _ => {
            return Err(InternalError::conflict(
                "Component Child allocation is not ready to reserve install capacity",
            ));
        }
    };
    let current_total = RootComponentRegistryStore::partition_entry_bytes(partition)
        .checked_add(current_reserved_bytes)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    let mut next = partition.clone();

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next)
            .checked_add(charged_entry_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let registry_delta = next_total.checked_sub(current_total).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "Component Child install precharge unexpectedly reduced Registry bytes",
            )
        })?;
        let encoded_bytes = partition
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if next.encoded_bytes == encoded_bytes {
            if encoded_bytes > record.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component Child installation requires {encoded_bytes} bytes, exceeding protected Component limit {}",
                    record.maximum_registry_bytes
                )));
            }
            let root_encoded_bytes = current
                .encoded_bytes
                .checked_add(registry_delta)
                .ok_or_else(|| {
                    InternalError::resource_exhausted("Component Registry bytes overflow")
                })?;
            if root_encoded_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component Child installation requires {root_encoded_bytes} root Registry bytes, exceeding protected limit {}",
                    current.root.limits.maximum_registry_bytes
                )));
            }
            return Ok((next, registry_delta));
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component Child installation byte accounting did not converge",
    ))
}

fn validate_child_install_effect_record(
    effect: &RootComponentChildInstallEffectRecord,
    plan: &RootComponentChildInstallPlan,
) -> Result<(), InternalError> {
    if effect.raw_module_hash != plan.raw_module_hash
        || effect.chunk_hashes != plan.chunk_hashes
        || effect.binding != plan.binding
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "durable Component Child install intent differs from verified module or binding authority",
        ));
    }
    Ok(())
}

fn advance_child_install_phase(
    component: ComponentInstanceId,
    operation_id: [u8; 32],
    verified: bool,
) -> Result<RootComponentChildAllocationView, InternalError> {
    let current = RootComponentRegistryStore::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    let record =
        RootComponentRegistryStore::child_allocation(component, operation_id).ok_or_else(|| {
            InternalError::unavailable("Component Child allocation operation has not been reserved")
        })?;
    let next_progress = match (&record.progress, verified) {
        (
            RootComponentChildAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            },
            false,
        ) => RootComponentChildAllocationProgressRecord::Installed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
        },
        (RootComponentChildAllocationProgressRecord::Installed { .. }, false)
        | (
            RootComponentChildAllocationProgressRecord::Verified { .. }
            | RootComponentChildAllocationProgressRecord::Committed { .. },
            _,
        ) => return Ok(child_allocation_record_to_view(record)),
        (
            RootComponentChildAllocationProgressRecord::Installed {
                creation,
                canister,
                installation,
            },
            true,
        ) => RootComponentChildAllocationProgressRecord::Verified {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
        },
        _ => {
            return Err(InternalError::conflict(if verified {
                "Component Child allocation has not recorded successful installation"
            } else {
                "Component Child allocation has no durable install intent"
            }));
        }
    };
    let charged_entry_bytes = match &next_progress {
        RootComponentChildAllocationProgressRecord::Installed { installation, .. }
        | RootComponentChildAllocationProgressRecord::Verified { installation, .. } => {
            installation.charged_entry_bytes
        }
        _ => unreachable!(),
    };
    let mut next_record = record.clone();
    next_record.progress = next_progress;
    validate_charged_child_record_size(&next_record, charged_entry_bytes)?;
    RootComponentRegistryStore::replace_child_allocation(
        &current,
        current.clone(),
        &partition,
        partition.clone(),
        &record,
        next_record.clone(),
    )
    .map_err(map_allocation_commit_error)?;
    Ok(child_allocation_record_to_view(next_record))
}

fn creation_charged_entry_bytes(
    record: &RootComponentAllocationRecord,
    plan: &RootComponentCreationPlan,
) -> u64 {
    let mut maximum = record.clone();
    maximum.progress = RootComponentAllocationProgressRecord::Created {
        effect: RootComponentCreationEffectRecord {
            wasm_store: plan.wasm_store,
            payload_hash: plan.payload_hash,
            payload_size_bytes: u64::MAX,
            initial_cycles: Cycles::new(u128::MAX),
            controller: plan.controller,
            cost_guard_settlement: ReplayCostGuardSettlement {
                quota_intent_id: IntentId(u64::MAX),
                reservation_intent_id: IntentId(u64::MAX),
            },
            charged_entry_bytes: u64::MAX,
        },
        canister: Principal::from_slice(&[u8::MAX; 29]),
    };
    RootComponentRegistryStore::allocation_entry_bytes(&maximum)
}

fn install_charged_entry_bytes(
    record: &RootComponentAllocationRecord,
    plan: &RootComponentInstallPlan,
) -> Result<u64, InternalError> {
    let (creation, canister) = match &record.progress {
        RootComponentAllocationProgressRecord::Created { effect, canister } => {
            (effect.clone(), *canister)
        }
        _ => {
            return Err(InternalError::conflict(
                "Component allocation is not ready for installation",
            ));
        }
    };
    let mut maximum = record.clone();
    let installation = RootComponentInstallEffectRecord {
        raw_module_hash: plan.raw_module_hash,
        chunk_hashes: plan.chunk_hashes.clone(),
        binding: plan.binding.clone(),
        cost_guard_settlement: ReplayCostGuardSettlement {
            quota_intent_id: IntentId(u64::MAX),
            reservation_intent_id: IntentId(u64::MAX),
        },
        charged_entry_bytes: u64::MAX,
    };
    let registry = ComponentRegistryHead {
        component: record.component,
        revision: 1,
        content_hash: component_partition_content_hash(
            &plan.binding,
            &record.provisioning_origin,
            record.release_set,
            ComponentLifecycleStatus::Prepared,
            1,
            empty_component_descendant_content_hash(record.component),
            0,
        )?,
    };
    maximum.progress = RootComponentAllocationProgressRecord::Committed {
        creation,
        canister,
        installation,
        commitment: RootComponentCommitmentRecord {
            registry,
            prepared_registry_encoded_bytes: u64::MAX,
            directory_synchronized_at_ns: u64::MAX,
            directory_authority_hash: [u8::MAX; 32],
            directory_prepared: true,
            runtime_activated: true,
            membership: Some(RootComponentMembershipRecord {
                registry_encoded_bytes: u64::MAX,
                directory_synchronized_at_ns: u64::MAX,
                directory_authority_hash: [u8::MAX; 32],
                directory_synchronized: true,
            }),
        },
    };
    let partition = ComponentRegistryPartitionRecord {
        binding: plan.binding.clone(),
        provisioning_origin: record.provisioning_origin.clone(),
        release_set: record.release_set,
        status: ComponentLifecycleStatus::Active,
        revision: u64::MAX,
        content_hash: [u8::MAX; 32],
        descendant_content_hash: [u8::MAX; 32],
        directory_synchronized_at_ns: u64::MAX,
        reserved_descendants: u32::MAX,
        committed_descendants: u32::MAX,
        encoded_bytes: u64::MAX,
    };
    let charged = RootComponentRegistryStore::allocation_entry_bytes(&maximum)
        .checked_add(RootComponentRegistryStore::partition_entry_bytes(
            &partition,
        ))
        .and_then(|value| {
            value.checked_add(RootComponentRegistryStore::principal_index_entry_bytes(
                plan.binding.canister_id,
                record.component,
            ))
        })
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    if charged > plan.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted(format!(
            "Component Registry commitment requires {charged} bytes, exceeding protected Component limit {}",
            plan.maximum_registry_bytes
        )));
    }
    Ok(charged)
}

#[expect(
    clippy::too_many_lines,
    reason = "one constructor converges the complete child receipt and Registry byte ledger"
)]
fn committed_child_records(
    record: &RootComponentChildAllocationRecord,
    creation: &RootComponentCreationEffectRecord,
    canister: Principal,
    installation: &RootComponentChildInstallEffectRecord,
    partition: &ComponentRegistryPartitionRecord,
    directory_synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<
    (
        RootComponentChildAllocationRecord,
        ComponentRegistryPartitionRecord,
        ComponentRegistryChildRecord,
        ComponentRegistryChildTraversalRecord,
    ),
    InternalError,
> {
    if RootComponentRegistryStore::child(record.component, canister).is_some() {
        return Err(InternalError::conflict(
            "Component Child principal is already committed",
        ));
    }
    let child = ComponentRegistryChildRecord {
        component: record.component,
        canister_id: canister,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        kind: record.child_kind,
        installed_artifact_hash: installation.raw_module_hash,
        status: ComponentLifecycleStatus::Prepared,
    };
    validate_child_record(partition, &child)?;

    let revision = partition
        .revision
        .checked_add(1)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry revision overflow"))?;
    let reserved_descendants = partition
        .reserved_descendants
        .checked_sub(1)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry has no reserved descendant to commit",
            )
        })?;
    let committed_descendants =
        partition
            .committed_descendants
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::resource_exhausted("committed Component descendant count overflow")
            })?;
    let descendant_content_hash = committed_component_descendant_content_hash(
        partition.descendant_content_hash,
        partition.committed_descendants,
        revision,
        &child,
    )?;
    let content_hash = component_partition_content_hash(
        &partition.binding,
        &partition.provisioning_origin,
        partition.release_set,
        partition.status,
        revision,
        descendant_content_hash,
        committed_descendants,
    )?;
    let registry = ComponentRegistryHead {
        component: record.component,
        revision,
        content_hash,
    };
    let directory_authority_hash = component_directory_authority_hash(
        &partition.binding,
        revision,
        content_hash,
        directory_synchronized_at_ns,
        committed_descendants,
        fleet_directory,
    )?;
    let traversal = ComponentRegistryChildTraversalRecord {
        component: record.component,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        canister_id: canister,
    };
    let mut next_record = record.clone();
    next_record.progress = RootComponentChildAllocationProgressRecord::Committed {
        creation: creation.clone(),
        canister,
        installation: installation.clone(),
        commitment: RootComponentChildCommitmentRecord {
            registry,
            descendant_content_hash,
            registry_encoded_bytes: 0,
            reserved_descendants,
            committed_descendants,
            directory_synchronized_at_ns,
            directory_authority_hash,
            directory_prepared: false,
            runtime_activated: false,
            membership: None,
        },
    };
    let mut next_partition = ComponentRegistryPartitionRecord {
        binding: partition.binding.clone(),
        provisioning_origin: partition.provisioning_origin.clone(),
        release_set: partition.release_set,
        status: partition.status,
        revision,
        content_hash,
        descendant_content_hash,
        directory_synchronized_at_ns,
        reserved_descendants,
        committed_descendants,
        encoded_bytes: partition.encoded_bytes,
    };
    let current_total = RootComponentRegistryStore::partition_entry_bytes(partition)
        .checked_add(installation.charged_entry_bytes)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    let child_bytes = RootComponentRegistryStore::child_entry_bytes(&child);
    let traversal_bytes = RootComponentRegistryStore::child_traversal_entry_bytes(&traversal);
    let index_bytes =
        RootComponentRegistryStore::principal_index_entry_bytes(canister, record.component);

    for _ in 0..8 {
        let terminal_bytes = RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
            .checked_add(child_bytes)
            .and_then(|value| value.checked_add(traversal_bytes))
            .and_then(|value| value.checked_add(index_bytes))
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next_partition)
            .checked_add(terminal_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let released_precharge = current_total.checked_sub(next_total).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "exact Component Child commitment exceeds its maximum terminal precharge",
            )
        })?;
        let encoded_bytes = partition
            .encoded_bytes
            .checked_sub(released_precharge)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component Registry cannot release excess child precharge",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed { commitment, .. } =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "new Component Child commitment changed phase during byte accounting",
            ));
        };
        if next_partition.encoded_bytes == encoded_bytes
            && commitment.registry_encoded_bytes == encoded_bytes
        {
            return Ok((next_record, next_partition, child, traversal));
        }
        next_partition.encoded_bytes = encoded_bytes;
        commitment.registry_encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component Child commitment byte accounting did not converge",
    ))
}

fn persist_child_membership_activation(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentChildAllocationRecord,
    child: &ComponentRegistryChildRecord,
    directory_synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<
    (
        RootComponentChildAllocationView,
        ComponentRegistryPartitionView,
    ),
    InternalError,
> {
    let RootComponentChildAllocationProgressRecord::Committed {
        installation,
        commitment,
        ..
    } = &record.progress
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "membership activation persistence requires a committed Component Child allocation",
        ));
    };
    let (next_record, active_partition, active_child) = active_child_membership_records(
        record,
        commitment,
        partition,
        child,
        directory_synchronized_at_ns,
        fleet_directory,
    )?;
    let traversal = ComponentRegistryChildTraversalRecord {
        component: record.component,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        canister_id: child.canister_id,
    };
    let terminal_bytes = RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
        .checked_add(RootComponentRegistryStore::child_entry_bytes(&active_child))
        .and_then(|value| {
            value.checked_add(RootComponentRegistryStore::child_traversal_entry_bytes(
                &traversal,
            ))
        })
        .and_then(|value| {
            value.checked_add(RootComponentRegistryStore::principal_index_entry_bytes(
                child.canister_id,
                record.component,
            ))
        })
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    if terminal_bytes > installation.charged_entry_bytes {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Child membership exceeds its pre-install Registry byte reservation",
        ));
    }
    if active_partition.encoded_bytes > record.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted(format!(
            "active Component Registry requires {} bytes, exceeding protected Component limit {}",
            active_partition.encoded_bytes, record.maximum_registry_bytes
        )));
    }
    let encoded_bytes = current
        .encoded_bytes
        .checked_sub(partition.encoded_bytes)
        .and_then(|value| value.checked_add(active_partition.encoded_bytes))
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "root Component Registry byte accounting cannot activate child membership",
            )
        })?;
    if encoded_bytes > current.root.limits.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted(
            "active Component Child Registry exceeds the protected root byte limit",
        ));
    }
    let mut next_meta = current.clone();
    next_meta.encoded_bytes = encoded_bytes;
    RootComponentRegistryStore::activate_child_membership(
        current,
        next_meta,
        partition,
        active_partition.clone(),
        record,
        next_record.clone(),
        child,
        active_child,
    )
    .map_err(map_allocation_commit_error)?;
    Ok((
        child_allocation_record_to_view(next_record),
        partition_record_to_view(active_partition),
    ))
}

#[expect(
    clippy::too_many_lines,
    reason = "one constructor freezes the exact active child head and converges its byte ledger"
)]
fn active_child_membership_records(
    record: &RootComponentChildAllocationRecord,
    commitment: &RootComponentChildCommitmentRecord,
    partition: &ComponentRegistryPartitionRecord,
    child: &ComponentRegistryChildRecord,
    directory_synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<
    (
        RootComponentChildAllocationRecord,
        ComponentRegistryPartitionRecord,
        ComponentRegistryChildRecord,
    ),
    InternalError,
> {
    let RootComponentChildAllocationProgressRecord::Committed {
        creation,
        canister,
        installation,
        ..
    } = &record.progress
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "membership activation requires a committed Component Child allocation",
        ));
    };
    let revision = partition
        .revision
        .checked_add(1)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry revision overflow"))?;
    let mut active_child = child.clone();
    active_child.status = ComponentLifecycleStatus::Active;
    let descendant_content_hash = activated_component_descendant_content_hash(
        partition.descendant_content_hash,
        partition.revision,
        revision,
        &active_child,
    )?;
    let content_hash = component_partition_content_hash(
        &partition.binding,
        &partition.provisioning_origin,
        partition.release_set,
        partition.status,
        revision,
        descendant_content_hash,
        partition.committed_descendants,
    )?;
    let registry = ComponentRegistryHead {
        component: record.component,
        revision,
        content_hash,
    };
    let directory_authority_hash = component_directory_authority_hash(
        &partition.binding,
        revision,
        content_hash,
        directory_synchronized_at_ns,
        partition.committed_descendants,
        fleet_directory,
    )?;
    let mut next_record = record.clone();
    next_record.progress = RootComponentChildAllocationProgressRecord::Committed {
        creation: creation.clone(),
        canister: *canister,
        installation: installation.clone(),
        commitment: RootComponentChildCommitmentRecord {
            registry: commitment.registry.clone(),
            descendant_content_hash: commitment.descendant_content_hash,
            registry_encoded_bytes: commitment.registry_encoded_bytes,
            reserved_descendants: commitment.reserved_descendants,
            committed_descendants: commitment.committed_descendants,
            directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
            directory_authority_hash: commitment.directory_authority_hash,
            directory_prepared: commitment.directory_prepared,
            runtime_activated: commitment.runtime_activated,
            membership: Some(RootComponentChildMembershipRecord {
                registry,
                descendant_content_hash,
                registry_encoded_bytes: 0,
                reserved_descendants: partition.reserved_descendants,
                committed_descendants: partition.committed_descendants,
                directory_synchronized_at_ns,
                directory_authority_hash,
                directory_synchronized: false,
            }),
        },
    };
    let mut active_partition = ComponentRegistryPartitionRecord {
        binding: partition.binding.clone(),
        provisioning_origin: partition.provisioning_origin.clone(),
        release_set: partition.release_set,
        status: partition.status,
        revision,
        content_hash,
        descendant_content_hash,
        directory_synchronized_at_ns,
        reserved_descendants: partition.reserved_descendants,
        committed_descendants: partition.committed_descendants,
        encoded_bytes: partition.encoded_bytes,
    };
    let previous_variable_bytes = RootComponentRegistryStore::partition_entry_bytes(partition)
        .checked_add(RootComponentRegistryStore::child_allocation_entry_bytes(
            record,
        ))
        .and_then(|value| value.checked_add(RootComponentRegistryStore::child_entry_bytes(child)))
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;

    for _ in 0..8 {
        let next_variable_bytes =
            RootComponentRegistryStore::partition_entry_bytes(&active_partition)
                .checked_add(RootComponentRegistryStore::child_allocation_entry_bytes(
                    &next_record,
                ))
                .and_then(|value| {
                    value.checked_add(RootComponentRegistryStore::child_entry_bytes(&active_child))
                })
                .ok_or_else(|| {
                    InternalError::resource_exhausted("Component Registry bytes overflow")
                })?;
        let encoded_bytes = partition
            .encoded_bytes
            .checked_sub(previous_variable_bytes)
            .and_then(|value| value.checked_add(next_variable_bytes))
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component Registry bytes cannot activate child membership",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed { commitment, .. } =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "active Component Child commitment changed phase during byte accounting",
            ));
        };
        let membership = commitment.membership.as_mut().ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "active Component Child commitment lost membership during byte accounting",
            )
        })?;
        if active_partition.encoded_bytes == encoded_bytes
            && membership.registry_encoded_bytes == encoded_bytes
        {
            return Ok((next_record, active_partition, active_child));
        }
        active_partition.encoded_bytes = encoded_bytes;
        membership.registry_encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "active Component Child Registry byte accounting did not converge",
    ))
}

fn committed_records(
    record: &RootComponentAllocationRecord,
    creation: &RootComponentCreationEffectRecord,
    canister: Principal,
    installation: &RootComponentInstallEffectRecord,
    directory_synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<
    (
        RootComponentAllocationRecord,
        ComponentRegistryPartitionRecord,
    ),
    InternalError,
> {
    let revision = 1;
    let content_hash = component_partition_content_hash(
        &installation.binding,
        &record.provisioning_origin,
        record.release_set,
        ComponentLifecycleStatus::Prepared,
        revision,
        empty_component_descendant_content_hash(record.component),
        0,
    )?;
    let registry = ComponentRegistryHead {
        component: record.component,
        revision,
        content_hash,
    };
    let directory = ComponentDirectoryHead {
        provenance: ComponentDirectoryProvenance {
            component: installation.binding.clone(),
            source_fleet_subnet_root: installation.binding.fleet_subnet_root,
            component_registry_revision: registry.revision,
            component_registry_content_hash: registry.content_hash,
            synchronized_at_ns: directory_synchronized_at_ns,
        },
        descendant_count: 0,
    };
    let directory_authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&ComponentRuntimeDirectoryAuthority {
            fleet: fleet_directory.clone(),
            component: directory,
        })?;
    let mut next_record = record.clone();
    next_record.progress = RootComponentAllocationProgressRecord::Committed {
        creation: creation.clone(),
        canister,
        installation: installation.clone(),
        commitment: RootComponentCommitmentRecord {
            registry,
            prepared_registry_encoded_bytes: 0,
            directory_synchronized_at_ns,
            directory_authority_hash,
            directory_prepared: false,
            runtime_activated: false,
            membership: None,
        },
    };
    let mut partition = ComponentRegistryPartitionRecord {
        binding: installation.binding.clone(),
        provisioning_origin: record.provisioning_origin.clone(),
        release_set: record.release_set,
        status: ComponentLifecycleStatus::Prepared,
        revision,
        content_hash,
        descendant_content_hash: empty_component_descendant_content_hash(record.component),
        directory_synchronized_at_ns,
        reserved_descendants: 0,
        committed_descendants: 0,
        encoded_bytes: 0,
    };
    let index_bytes = RootComponentRegistryStore::principal_index_entry_bytes(
        installation.binding.canister_id,
        record.component,
    );
    for _ in 0..8 {
        let operation_bytes = RootComponentRegistryStore::allocation_entry_bytes(&next_record);
        let encoded_bytes = operation_bytes
            .checked_add(RootComponentRegistryStore::partition_entry_bytes(
                &partition,
            ))
            .and_then(|value| value.checked_add(index_bytes))
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let RootComponentAllocationProgressRecord::Committed { commitment, .. } =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "new Component commitment changed phase during byte accounting",
            ));
        };
        if partition.encoded_bytes == encoded_bytes
            && commitment.prepared_registry_encoded_bytes == encoded_bytes
        {
            return Ok((next_record, partition));
        }
        partition.encoded_bytes = encoded_bytes;
        commitment.prepared_registry_encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component Registry partition byte accounting did not converge",
    ))
}

#[expect(
    clippy::too_many_lines,
    reason = "one constructor freezes the complete top-level membership authority"
)]
fn active_membership_records(
    record: &RootComponentAllocationRecord,
    commitment: &RootComponentCommitmentRecord,
    directory_synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<
    (
        RootComponentAllocationRecord,
        ComponentRegistryPartitionRecord,
    ),
    InternalError,
> {
    let RootComponentAllocationProgressRecord::Committed {
        creation,
        canister,
        installation,
        ..
    } = &record.progress
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "membership activation requires a committed Component allocation",
        ));
    };
    let revision =
        commitment.registry.revision.checked_add(1).ok_or_else(|| {
            InternalError::resource_exhausted("Component Registry revision overflow")
        })?;
    let content_hash = component_partition_content_hash(
        &installation.binding,
        &record.provisioning_origin,
        record.release_set,
        ComponentLifecycleStatus::Active,
        revision,
        empty_component_descendant_content_hash(record.component),
        0,
    )?;
    let directory_authority_hash = component_directory_authority_hash(
        &installation.binding,
        revision,
        content_hash,
        directory_synchronized_at_ns,
        0,
        fleet_directory,
    )?;
    let mut next_record = record.clone();
    let mut active = ComponentRegistryPartitionRecord {
        binding: installation.binding.clone(),
        provisioning_origin: record.provisioning_origin.clone(),
        release_set: record.release_set,
        status: ComponentLifecycleStatus::Active,
        revision,
        content_hash,
        descendant_content_hash: empty_component_descendant_content_hash(record.component),
        directory_synchronized_at_ns,
        reserved_descendants: 0,
        committed_descendants: 0,
        encoded_bytes: 0,
    };
    next_record.progress = RootComponentAllocationProgressRecord::Committed {
        creation: creation.clone(),
        canister: *canister,
        installation: installation.clone(),
        commitment: RootComponentCommitmentRecord {
            registry: commitment.registry.clone(),
            prepared_registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
            directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
            directory_authority_hash: commitment.directory_authority_hash,
            directory_prepared: commitment.directory_prepared,
            runtime_activated: commitment.runtime_activated,
            membership: Some(RootComponentMembershipRecord {
                registry_encoded_bytes: 0,
                directory_synchronized_at_ns,
                directory_authority_hash,
                directory_synchronized: false,
            }),
        },
    };
    let index_bytes = RootComponentRegistryStore::principal_index_entry_bytes(
        installation.binding.canister_id,
        record.component,
    );
    for _ in 0..8 {
        let encoded_bytes = RootComponentRegistryStore::allocation_entry_bytes(&next_record)
            .checked_add(RootComponentRegistryStore::partition_entry_bytes(&active))
            .and_then(|value| value.checked_add(index_bytes))
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let RootComponentAllocationProgressRecord::Committed { commitment, .. } =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "active Component commitment changed phase during byte accounting",
            ));
        };
        let membership = commitment.membership.as_mut().ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "active Component commitment lost membership during byte accounting",
            )
        })?;
        if active.encoded_bytes == encoded_bytes
            && membership.registry_encoded_bytes == encoded_bytes
        {
            return Ok((next_record, active));
        }
        active.encoded_bytes = encoded_bytes;
        membership.registry_encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "active Component Registry byte accounting did not converge",
    ))
}

fn component_directory_authority_hash(
    binding: &ComponentBinding,
    revision: u64,
    content_hash: [u8; 32],
    synchronized_at_ns: u64,
    descendant_count: u32,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<[u8; 32], InternalError> {
    ComponentRuntimeOps::directory_authority_hash(&ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory.clone(),
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: binding.clone(),
                source_fleet_subnet_root: binding.fleet_subnet_root,
                component_registry_revision: revision,
                component_registry_content_hash: content_hash,
                synchronized_at_ns,
            },
            descendant_count,
        },
    })
}

fn exact_committed_child_partition(
    record: &RootComponentChildAllocationRecord,
    commitment: &RootComponentChildCommitmentRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let RootComponentChildAllocationProgressRecord::Committed {
        canister,
        installation,
        ..
    } = &record.progress
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "Component Child partition validation requires a committed allocation",
        ));
    };
    let current = RootComponentRegistryStore::partition(record.component).ok_or_else(|| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "committed Component Child allocation has no Registry partition",
        )
    })?;
    validate_partition_record(&current)?;
    let child =
        RootComponentRegistryStore::child(record.component, *canister).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "committed Component Child allocation has no normalized row",
            )
        })?;
    validate_child_record(&current, &child)?;
    let traversal = ComponentRegistryChildTraversalRecord {
        component: record.component,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        canister_id: *canister,
    };
    let committed = ComponentRegistryPartitionRecord {
        binding: installation.binding.component.clone(),
        provisioning_origin: current.provisioning_origin.clone(),
        release_set: record.release_set,
        status: ComponentLifecycleStatus::Active,
        revision: commitment.registry.revision,
        content_hash: commitment.registry.content_hash,
        descendant_content_hash: commitment.descendant_content_hash,
        directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
        reserved_descendants: commitment.reserved_descendants,
        committed_descendants: commitment.committed_descendants,
        encoded_bytes: commitment.registry_encoded_bytes,
    };
    validate_partition_snapshot(&committed)?;
    if commitment.registry.component != record.component
        || child.component != record.component
        || child.canister_id != *canister
        || child.parent_canister_id != record.parent_canister_id
        || child.role != record.child_role
        || child.kind != record.child_kind
        || child.installed_artifact_hash != installation.raw_module_hash
        || !matches!(
            child.status,
            ComponentLifecycleStatus::Prepared | ComponentLifecycleStatus::Active
        )
        || RootComponentRegistryStore::child_traversal(
            traversal.component,
            traversal.parent_canister_id,
            &traversal.role,
            traversal.canister_id,
        )
        .as_ref()
            != Some(&traversal)
        || current.binding != committed.binding
        || current.release_set != committed.release_set
        || !component_partition_retains_active_membership(current.status)
        || current.revision < committed.revision
        || current.directory_synchronized_at_ns < committed.directory_synchronized_at_ns
        || current.committed_descendants < committed.committed_descendants
        || current.encoded_bytes > record.maximum_registry_bytes
        || (current.revision == committed.revision
            && (current.content_hash != committed.content_hash
                || current.directory_synchronized_at_ns != committed.directory_synchronized_at_ns
                || current.committed_descendants != committed.committed_descendants))
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "committed Component Child differs from its immutable Registry receipt",
        ));
    }
    Ok(committed)
}

fn exact_active_child_partition(
    record: &RootComponentChildAllocationRecord,
    commitment: &RootComponentChildCommitmentRecord,
    membership: &RootComponentChildMembershipRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let current = RootComponentRegistryStore::partition(record.component).ok_or_else(|| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component Child allocation has no Registry partition",
        )
    })?;
    validate_active_child_partition(record, commitment, membership, &current)
}

fn validate_active_child_partition(
    record: &RootComponentChildAllocationRecord,
    commitment: &RootComponentChildCommitmentRecord,
    membership: &RootComponentChildMembershipRecord,
    current: &ComponentRegistryPartitionRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let RootComponentChildAllocationProgressRecord::Committed {
        canister,
        installation,
        ..
    } = &record.progress
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "active child validation requires a committed Component Child allocation",
        ));
    };
    let child =
        RootComponentRegistryStore::child(record.component, *canister).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "active Component Child allocation has no normalized row",
            )
        })?;
    validate_child_record(current, &child)?;
    let historical = ComponentRegistryPartitionRecord {
        binding: current.binding.clone(),
        provisioning_origin: current.provisioning_origin.clone(),
        release_set: current.release_set,
        status: current.status,
        revision: membership.registry.revision,
        content_hash: membership.registry.content_hash,
        descendant_content_hash: membership.descendant_content_hash,
        directory_synchronized_at_ns: membership.directory_synchronized_at_ns,
        reserved_descendants: membership.reserved_descendants,
        committed_descendants: membership.committed_descendants,
        encoded_bytes: membership.registry_encoded_bytes,
    };
    validate_partition_snapshot(&historical)?;
    let activation_evidence =
        ComponentChildActivationEvidence::new(record.component, commitment, membership);
    let child_state = ComponentTreeNodeState::from_child(&child);
    let expected_child_state = ComponentTreeNodeState::new(
        ComponentTreeNodeIdentity::new(
            record.component,
            record.parent_canister_id,
            &record.child_role,
            *canister,
        ),
        record.child_kind,
        installation.raw_module_hash,
        ComponentLifecycleStatus::Active,
    );
    let current_identity_is_valid = current.binding == historical.binding
        && current.release_set == historical.release_set
        && component_partition_retains_active_membership(current.status);
    let current_progress_is_valid =
        ComponentPartitionCoverage::new(current, &historical).is_monotonic();
    if !activation_evidence.is_valid()
        || child_state != expected_child_state
        || !current_identity_is_valid
        || !current_progress_is_valid
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component Child partition differs from its immutable membership receipt",
        ));
    }
    validate_partition_record(current)?;
    Ok(historical)
}

fn exact_committed_partition(
    record: &RootComponentAllocationRecord,
    commitment: &RootComponentCommitmentRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let RootComponentAllocationProgressRecord::Committed { installation, .. } = &record.progress
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "Component partition validation requires a committed allocation",
        ));
    };
    let current = RootComponentRegistryStore::partition(record.component).ok_or_else(|| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "committed Component allocation has no Registry partition",
        )
    })?;
    let prepared = ComponentRegistryPartitionRecord {
        binding: installation.binding.clone(),
        provisioning_origin: record.provisioning_origin.clone(),
        release_set: record.release_set,
        status: ComponentLifecycleStatus::Prepared,
        revision: commitment.registry.revision,
        content_hash: commitment.registry.content_hash,
        descendant_content_hash: empty_component_descendant_content_hash(record.component),
        directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
        reserved_descendants: 0,
        committed_descendants: 0,
        encoded_bytes: commitment.prepared_registry_encoded_bytes,
    };
    if prepared.binding.component != record.component
        || commitment.registry.component != record.component
        || RootComponentRegistryStore::component_for_principal(prepared.binding.canister_id)
            != Some(record.component)
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "committed Component allocation differs from its prepared Registry receipt",
        ));
    }
    validate_partition_snapshot(&prepared)?;
    match &commitment.membership {
        None if current == prepared => {}
        Some(membership) => {
            let _active = validate_active_partition(record, commitment, membership, &current)?;
        }
        None => {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "current Component partition differs from its prepared Registry receipt",
            ));
        }
    }
    Ok(prepared)
}

fn exact_active_partition(
    record: &RootComponentAllocationRecord,
    commitment: &RootComponentCommitmentRecord,
    membership: &RootComponentMembershipRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let current = RootComponentRegistryStore::partition(record.component).ok_or_else(|| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component allocation has no Registry partition",
        )
    })?;
    validate_active_partition(record, commitment, membership, &current)
}

fn validate_active_partition(
    record: &RootComponentAllocationRecord,
    commitment: &RootComponentCommitmentRecord,
    membership: &RootComponentMembershipRecord,
    current: &ComponentRegistryPartitionRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let expected_revision =
        commitment.registry.revision.checked_add(1).ok_or_else(|| {
            InternalError::resource_exhausted("Component Registry revision overflow")
        })?;
    let historical = ComponentRegistryPartitionRecord {
        binding: current.binding.clone(),
        provisioning_origin: current.provisioning_origin.clone(),
        release_set: current.release_set,
        status: ComponentLifecycleStatus::Active,
        revision: expected_revision,
        content_hash: component_partition_content_hash(
            &current.binding,
            &current.provisioning_origin,
            current.release_set,
            ComponentLifecycleStatus::Active,
            expected_revision,
            empty_component_descendant_content_hash(record.component),
            0,
        )?,
        descendant_content_hash: empty_component_descendant_content_hash(record.component),
        directory_synchronized_at_ns: membership.directory_synchronized_at_ns,
        reserved_descendants: 0,
        committed_descendants: 0,
        encoded_bytes: membership.registry_encoded_bytes,
    };
    validate_partition_snapshot(&historical)?;
    // Later child reservations and commitments may advance charged bytes and
    // the current head without changing this immutable top-level receipt.
    let activation_evidence = ComponentActivationEvidence::new(commitment, membership, current);
    let current_identity_is_valid = current.binding.component == record.component
        && component_partition_retains_active_membership(current.status);
    let current_progress_is_valid =
        ComponentPartitionCoverage::new(current, &historical).is_monotonic();
    if !activation_evidence.is_valid() || !current_identity_is_valid || !current_progress_is_valid {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component partition differs from its immutable membership receipt",
        ));
    }
    validate_partition_record(current)?;
    Ok(historical)
}

fn validate_membership_directory_authority_hash(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
    membership: &RootComponentMembershipRecord,
) -> Result<(), InternalError> {
    let authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory.clone(),
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: partition.binding.clone(),
                source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                component_registry_revision: partition.revision,
                component_registry_content_hash: partition.content_hash,
                synchronized_at_ns: partition.directory_synchronized_at_ns,
            },
            descendant_count: 0,
        },
    };
    if ComponentRuntimeOps::directory_authority_hash(&authority)?
        != membership.directory_authority_hash
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component Directory authority differs from its membership receipt",
        ));
    }
    Ok(())
}

fn validate_child_directory_authority_hash(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
    commitment: &RootComponentChildCommitmentRecord,
) -> Result<(), InternalError> {
    let authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory.clone(),
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: partition.binding.clone(),
                source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                component_registry_revision: commitment.registry.revision,
                component_registry_content_hash: commitment.registry.content_hash,
                synchronized_at_ns: commitment.directory_synchronized_at_ns,
            },
            descendant_count: commitment.committed_descendants,
        },
    };
    if ComponentRuntimeOps::directory_authority_hash(&authority)?
        != commitment.directory_authority_hash
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "committed Component Child Directory authority differs from its Registry receipt",
        ));
    }
    Ok(())
}

fn validate_child_membership_directory_authority_hash(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
    membership: &RootComponentChildMembershipRecord,
) -> Result<(), InternalError> {
    let authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory.clone(),
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: partition.binding.clone(),
                source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                component_registry_revision: membership.registry.revision,
                component_registry_content_hash: membership.registry.content_hash,
                synchronized_at_ns: membership.directory_synchronized_at_ns,
            },
            descendant_count: membership.committed_descendants,
        },
    };
    if ComponentRuntimeOps::directory_authority_hash(&authority)?
        != membership.directory_authority_hash
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component Child Directory differs from its membership receipt",
        ));
    }
    Ok(())
}

fn validate_directory_authority_hash(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
    commitment: &RootComponentCommitmentRecord,
) -> Result<(), InternalError> {
    let authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory.clone(),
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: partition.binding.clone(),
                source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                component_registry_revision: partition.revision,
                component_registry_content_hash: partition.content_hash,
                synchronized_at_ns: partition.directory_synchronized_at_ns,
            },
            descendant_count: 0,
        },
    };
    if ComponentRuntimeOps::directory_authority_hash(&authority)?
        != commitment.directory_authority_hash
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "committed Component Directory authority hash differs from current Registry authority",
        ));
    }
    Ok(())
}

fn validate_partition_record(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<(), InternalError> {
    validate_partition_snapshot(partition)
}

const fn component_partition_head(
    partition: &ComponentRegistryPartitionRecord,
) -> ComponentRegistryHead {
    ComponentRegistryHead {
        component: partition.binding.component,
        revision: partition.revision,
        content_hash: partition.content_hash,
    }
}

const fn component_partition_retains_active_membership(status: ComponentLifecycleStatus) -> bool {
    matches!(
        status,
        ComponentLifecycleStatus::Active | ComponentLifecycleStatus::Draining
    )
}

fn validate_component_draining_record(
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentDrainingRecord,
) -> Result<(), InternalError> {
    let previous_content_hash = component_partition_content_hash(
        &partition.binding,
        &partition.provisioning_origin,
        partition.release_set,
        ComponentLifecycleStatus::Active,
        record.previous_registry.revision,
        record.descendant_content_hash,
        record.descendant_count,
    )?;
    let draining_content_hash = component_partition_content_hash(
        &partition.binding,
        &partition.provisioning_origin,
        partition.release_set,
        ComponentLifecycleStatus::Draining,
        record.registry.revision,
        record.descendant_content_hash,
        record.descendant_count,
    )?;
    let quiescence_is_valid = record.quiescence.as_ref().is_none_or(|progress| {
        let (intent, terminal) = match progress {
            RootComponentQuiescenceProgressRecord::StopIntent(intent) => (intent, None),
            RootComponentQuiescenceProgressRecord::Quiescent(receipt) => {
                (&receipt.stop, Some(receipt))
            }
        };
        let intent_is_valid = intent.registry == record.registry
            && intent.descendant_count == record.descendant_count
            && intent.descendant_content_hash == record.descendant_content_hash
            && intent.canister_id == partition.binding.canister_id
            && intent.canister_id != Principal::anonymous()
            && intent.controller == partition.binding.fleet_subnet_root
            && intent.controller != Principal::anonymous()
            && intent.expected_module_hash != [0; 32]
            && intent.covered_fleet_registry_revision > 0
            && intent.covered_fleet_registry_content_hash != [0; 32]
            && intent.covered_authority_hash != [0; 32]
            && intent.runtime_operation_id != [0; 32]
            && intent.activation.directory_authority_hash != [0; 32]
            && intent.activation.activated_at_ns > 0
            && intent.prepared_at_ns >= record.started_at_ns
            && intent.charged_entry_bytes
                >= RootComponentRegistryStore::component_draining_entry_bytes(record)
            && component_quiescence_terminal_entry_bytes(record, intent)
                .is_ok_and(|bytes| bytes == intent.charged_entry_bytes);
        let terminal_is_valid = terminal.is_none_or(|receipt| {
            receipt.observed_module_hash == intent.expected_module_hash
                && receipt.quiesced_at_ns >= intent.prepared_at_ns
        });
        intent_is_valid && terminal_is_valid
    });
    let subtree_cursor_is_valid = record.subtree_operation_id.is_none()
        || (record
            .subtree_operation_id
            .is_some_and(|operation_id| operation_id != [0; 32])
            && matches!(
                record.quiescence,
                Some(RootComponentQuiescenceProgressRecord::Quiescent(_))
            ));
    if let Some(inventory) = &record.final_inventory {
        validate_component_final_inventory_record(partition, record, inventory)?;
    }
    if let Some(deletion) = &record.deletion {
        RootComponentDeletionAuthority {
            draining: record,
            progress: deletion,
        }
        .validate()?;
    }
    let valid = record.operation_id != [0; 32]
        && record.component == partition.binding.component
        && record.previous_registry.component == record.component
        && record.previous_registry.revision > 0
        && record.previous_registry.content_hash == previous_content_hash
        && record.registry.component == record.component
        && record.previous_registry.revision.checked_add(1) == Some(record.registry.revision)
        && record.registry.content_hash == draining_content_hash
        && record.descendant_count >= partition.committed_descendants
        && record.descendant_content_hash != [0; 32]
        && record.directory_authority_hash != [0; 32]
        && record.started_at_ns > 0
        && partition.status == ComponentLifecycleStatus::Draining
        && partition.revision >= record.registry.revision
        && partition.directory_synchronized_at_ns >= record.started_at_ns
        && quiescence_is_valid
        && subtree_cursor_is_valid
        && (partition.revision != record.registry.revision
            || (partition.content_hash == record.registry.content_hash
                && partition.descendant_content_hash == record.descendant_content_hash
                && partition.committed_descendants == record.descendant_count
                && partition.directory_synchronized_at_ns == record.started_at_ns));
    if !valid {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component draining receipt differs from protected Registry authority",
        ));
    }
    Ok(())
}

fn validate_component_final_inventory_record(
    partition: &ComponentRegistryPartitionRecord,
    draining: &RootComponentDrainingRecord,
    inventory: &RootComponentFinalInventoryRecord,
) -> Result<(), InternalError> {
    RootComponentFinalInventoryAuthority {
        partition,
        draining,
        inventory,
    }
    .validate()?;
    ensure_component_lifecycle_history_is_terminal(partition)
}

const fn terminal_component_quiesced_at_ns(draining: &RootComponentDrainingRecord) -> Option<u64> {
    let receipt = terminal_component_quiescence(draining);
    match receipt {
        Some(receipt) => Some(receipt.quiesced_at_ns),
        None => None,
    }
}

const fn terminal_component_quiescence(
    draining: &RootComponentDrainingRecord,
) -> Option<&RootComponentQuiescentReceiptRecord> {
    match &draining.quiescence {
        Some(RootComponentQuiescenceProgressRecord::Quiescent(receipt)) => Some(receipt),
        None | Some(RootComponentQuiescenceProgressRecord::StopIntent(_)) => None,
    }
}

fn component_final_inventory_fleet_coverage_is_versioned(
    inventory: &RootComponentFinalInventoryRecord,
) -> bool {
    inventory.covered_fleet_registry_revision > 0
        && inventory.covered_fleet_registry_content_hash != [0; 32]
}

const fn component_final_inventory_time_is_monotonic(
    partition: &ComponentRegistryPartitionRecord,
    quiesced_at_ns: u64,
    finalized_at_ns: u64,
) -> bool {
    finalized_at_ns >= quiesced_at_ns && finalized_at_ns >= partition.directory_synchronized_at_ns
}

fn component_partition_is_empty_and_draining(partition: &ComponentRegistryPartitionRecord) -> bool {
    if partition.status != ComponentLifecycleStatus::Draining {
        return false;
    }
    if partition.reserved_descendants != 0 {
        return false;
    }
    if partition.committed_descendants != 0 {
        return false;
    }
    partition.descendant_content_hash
        == empty_component_descendant_content_hash(partition.binding.component)
}

fn component_final_inventory_indexes_are_empty(
    partition: &ComponentRegistryPartitionRecord,
) -> bool {
    if !RootComponentRegistryStore::component_live_inventory_is_empty(partition.binding.component) {
        return false;
    }
    RootComponentRegistryStore::component_principal_inventory_is_exact(
        partition.binding.component,
        partition.binding.canister_id,
    )
}

fn component_draining_cursor_is_terminal(draining: &RootComponentDrainingRecord) -> bool {
    let Some(operation_id) = draining.subtree_operation_id else {
        return true;
    };
    RootComponentRegistryStore::subtree_removal(draining.component, operation_id).is_some_and(
        |removal| {
            matches!(
                removal.progress,
                RootComponentSubtreeRemovalProgressRecord::Completed(_)
            )
        },
    )
}

fn ensure_component_final_inventory_candidate(
    partition: &ComponentRegistryPartitionRecord,
    expected_registry: &ComponentRegistryHead,
) -> Result<(), InternalError> {
    if expected_registry != &component_partition_head(partition) {
        return Err(InternalError::conflict(
            "Component final inventory differs from current Registry head",
        ));
    }
    if !component_partition_is_empty_and_draining(partition) {
        return Err(InternalError::conflict(
            "Component final inventory differs from current empty draining authority",
        ));
    }
    Ok(())
}

fn ensure_component_final_inventory_time(
    partition: &ComponentRegistryPartitionRecord,
    quiesced_at_ns: u64,
    finalized_at_ns: u64,
) -> Result<(), InternalError> {
    if component_final_inventory_time_is_monotonic(partition, quiesced_at_ns, finalized_at_ns) {
        return Ok(());
    }
    Err(InternalError::invalid_input(
        "Component final inventory time precedes its terminal authority",
    ))
}

fn ensure_component_final_inventory_indexes_are_empty(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<(), InternalError> {
    if component_final_inventory_indexes_are_empty(partition) {
        return Ok(());
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component final inventory still contains live descendant membership",
    ))
}

fn ensure_component_final_inventory_fleet_authority(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<(), InternalError> {
    if fleet_directory.provenance.source_fleet_subnet_root != partition.binding.fleet_subnet_root {
        return Err(InternalError::conflict(
            "Component final inventory has a foreign Fleet Directory root",
        ));
    }
    if fleet_directory.provenance.registry.revision == 0 {
        return Err(InternalError::conflict(
            "Component final inventory has an unversioned Fleet Directory",
        ));
    }
    if fleet_directory.provenance.registry.content_hash == [0; 32] {
        return Err(InternalError::conflict(
            "Component final inventory has an empty Fleet Directory hash",
        ));
    }
    Ok(())
}

fn ensure_component_deletion_operation(
    draining: &RootComponentDrainingRecord,
    operation_id: [u8; 32],
) -> Result<(), InternalError> {
    if draining.operation_id == operation_id {
        return Ok(());
    }
    Err(InternalError::conflict(
        "Component deletion is bound to different draining intent",
    ))
}

fn ensure_component_deletion_inventory(
    progress: &RootComponentDeletionProgressRecord,
    expected_inventory_hash: [u8; 32],
) -> Result<(), InternalError> {
    let intent = match progress {
        RootComponentDeletionProgressRecord::DeleteIntent(intent) => intent,
        RootComponentDeletionProgressRecord::Deleted(receipt) => &receipt.deletion,
    };
    if intent.final_inventory.inventory_hash == expected_inventory_hash {
        return Ok(());
    }
    Err(InternalError::conflict(
        "Component deletion differs from durable final inventory authority",
    ))
}

fn component_has_terminal_quiescence(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<bool, InternalError> {
    let draining = RootComponentRegistryStore::component_draining(partition.binding.component)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Draining Component partition has no durable draining authority",
            )
        })?;
    validate_component_draining_record(partition, &draining)?;
    Ok(matches!(
        draining.quiescence,
        Some(RootComponentQuiescenceProgressRecord::Quiescent(_))
    ))
}

fn validate_partition_snapshot(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<(), InternalError> {
    let empty_descendant_hash =
        empty_component_descendant_content_hash(partition.binding.component);
    let descendant_hash_matches_count = match partition.committed_descendants {
        0 => partition.descendant_content_hash == empty_descendant_hash,
        _ => partition.descendant_content_hash != empty_descendant_hash,
    };
    let expected_content_hash = component_partition_content_hash(
        &partition.binding,
        &partition.provisioning_origin,
        partition.release_set,
        partition.status,
        partition.revision,
        partition.descendant_content_hash,
        partition.committed_descendants,
    )?;
    let principal_index_matches =
        RootComponentRegistryStore::component_for_principal(partition.binding.canister_id)
            == Some(partition.binding.component);
    let head_is_versioned = partition.revision > 0;
    let directory_is_synchronized = partition.directory_synchronized_at_ns > 0;
    let content_is_canonical = partition.descendant_content_hash != [0; 32]
        && descendant_hash_matches_count
        && partition.content_hash == expected_content_hash;
    if !head_is_versioned
        || !directory_is_synchronized
        || !content_is_canonical
        || !principal_index_matches
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Registry partition has invalid head, Directory time or principal index",
        ));
    }
    Ok(())
}

fn validate_child_record(
    partition: &ComponentRegistryPartitionRecord,
    child: &ComponentRegistryChildRecord,
) -> Result<(), InternalError> {
    if !ComponentTreeBoundary::from_partition(partition).admits(child) {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Registry child row has invalid tree identity",
        ));
    }
    Ok(())
}

fn validate_registered_child_record(
    partition: &ComponentRegistryPartitionRecord,
    child: &ComponentRegistryChildRecord,
) -> Result<(), InternalError> {
    validate_child_record(partition, child)?;
    let traversal = ComponentRegistryChildTraversalRecord {
        component: child.component,
        parent_canister_id: child.parent_canister_id,
        role: child.role.clone(),
        canister_id: child.canister_id,
    };
    if RootComponentRegistryStore::component_for_principal(child.canister_id)
        != Some(child.component)
        || RootComponentRegistryStore::component_for_principal(child.parent_canister_id)
            != Some(child.component)
        || RootComponentRegistryStore::child_traversal(
            child.component,
            child.parent_canister_id,
            &child.role,
            child.canister_id,
        )
        .as_ref()
            != Some(&traversal)
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Registry child differs from its principal or traversal index",
        ));
    }
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
struct SubtreeDirectoryCoverage {
    fleet_registry_revision: u64,
    fleet_registry_content_hash: [u8; 32],
    component_registry_revision: u64,
    component_registry_content_hash: [u8; 32],
    authority_hash: [u8; 32],
}

const fn managed_canister_principal(binding: &ManagedCanisterBinding) -> Principal {
    match binding {
        ManagedCanisterBinding::Component(component) => component.canister_id,
        ManagedCanisterBinding::ComponentChild(child) => child.canister_id,
    }
}

fn subtree_directory_coverage(
    partition: &ComponentRegistryPartitionRecord,
    authority: &ComponentRuntimeDirectoryAuthority,
    authority_hash: [u8; 32],
) -> Result<SubtreeDirectoryCoverage, InternalError> {
    let directory = &authority.component;
    let provenance = &directory.provenance;
    let expected_hash = ComponentRuntimeOps::directory_authority_hash(authority)?;
    let coverage_is_exact = authority.fleet.provenance.source_fleet_subnet_root
        == partition.binding.fleet_subnet_root
        && provenance.component == partition.binding
        && provenance.source_fleet_subnet_root == partition.binding.fleet_subnet_root
        && provenance.component_registry_revision == partition.revision
        && provenance.component_registry_content_hash == partition.content_hash
        && provenance.synchronized_at_ns == partition.directory_synchronized_at_ns
        && directory.descendant_count == partition.committed_descendants
        && authority_hash != [0; 32]
        && authority_hash == expected_hash;
    if !coverage_is_exact {
        return Err(InternalError::conflict(
            "Component subtree Directory coverage differs from current protected authority",
        ));
    }
    Ok(SubtreeDirectoryCoverage {
        fleet_registry_revision: authority.fleet.provenance.registry.revision,
        fleet_registry_content_hash: authority.fleet.provenance.registry.content_hash,
        component_registry_revision: partition.revision,
        component_registry_content_hash: partition.content_hash,
        authority_hash,
    })
}

fn subtree_directory_convergence_record(
    partition: &ComponentRegistryPartitionRecord,
    expected_binding: &ManagedCanisterBinding,
    evidence: ComponentRuntimeDirectoryConvergenceEvidence,
) -> Result<
    (
        SubtreeDirectoryCoverage,
        RootComponentSubtreeDirectoryConvergenceRecord,
    ),
    InternalError,
> {
    let authority = &evidence.covered_authority;
    let coverage =
        subtree_directory_coverage(partition, authority, evidence.covered_authority_hash)?;
    let evidence_is_exact = evidence.operation_id != [0; 32]
        && evidence.binding == *expected_binding
        && evidence.activation.directory_authority_hash != [0; 32]
        && evidence.activation.activated_at_ns > 0;
    if !evidence_is_exact {
        return Err(InternalError::conflict(
            "Component subtree Directory evidence differs from current protected authority",
        ));
    }
    Ok((
        coverage,
        RootComponentSubtreeDirectoryConvergenceRecord {
            operation_id: evidence.operation_id,
            canister_id: managed_canister_principal(&evidence.binding),
            activation: evidence.activation,
        },
    ))
}

fn subtree_directory_parent_convergence_record(
    partition: &ComponentRegistryPartitionRecord,
    component: ComponentInstanceId,
    parent_canister_id: Principal,
    evidence: Option<ComponentRuntimeDirectoryConvergenceEvidence>,
    expected_coverage: &SubtreeDirectoryCoverage,
) -> Result<Option<RootComponentSubtreeDirectoryConvergenceRecord>, InternalError> {
    if parent_canister_id == partition.binding.canister_id {
        return if evidence.is_none() {
            Ok(None)
        } else {
            Err(InternalError::conflict(
                "top-level Component parent must not have duplicate Directory evidence",
            ))
        };
    }
    let (binding, status) = ComponentRegistryOps::registered_parent(component, parent_canister_id)?
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "removed subtree leaf has no retained registered parent",
            )
        })?;
    if status != ComponentLifecycleStatus::Active {
        return Err(InternalError::conflict(
            "removed subtree leaf parent is not Active",
        ));
    }
    let evidence = evidence.ok_or_else(|| {
        InternalError::unavailable(
            "removed subtree leaf parent has no Directory convergence evidence",
        )
    })?;
    let (coverage, record) = subtree_directory_convergence_record(partition, &binding, evidence)?;
    if &coverage != expected_coverage {
        return Err(InternalError::conflict(
            "surviving Component subtree members covered different Directory authority",
        ));
    }
    Ok(Some(record))
}

fn validate_subtree_removal_record(
    record: &RootComponentSubtreeRemovalRecord,
) -> Result<(), InternalError> {
    let progress_is_valid = match &record.progress {
        RootComponentSubtreeRemovalProgressRecord::Fenced => record.traversal_steps == 0,
        RootComponentSubtreeRemovalProgressRecord::Traversing { cursor } => {
            record.traversal_steps > 0 && valid_subtree_removal_node(record.component, cursor)
        }
        RootComponentSubtreeRemovalProgressRecord::LeafSelected { leaf } => {
            record.traversal_steps > 0 && valid_subtree_removal_node(record.component, leaf)
        }
        RootComponentSubtreeRemovalProgressRecord::StopIntent(effect) => {
            record.traversal_steps > 0
                && effect.controller != Principal::anonymous()
                && valid_subtree_removal_node(record.component, &effect.leaf)
        }
        RootComponentSubtreeRemovalProgressRecord::Stopped(receipt) => {
            record.traversal_steps > 0 && valid_subtree_stopped_effect(record.component, receipt)
        }
        RootComponentSubtreeRemovalProgressRecord::DeleteIntent(deletion) => {
            record.traversal_steps > 0
                && valid_subtree_stopped_effect(record.component, &deletion.stopped)
        }
        RootComponentSubtreeRemovalProgressRecord::Deleted(receipt) => {
            record.traversal_steps > 0
                && valid_subtree_stopped_effect(record.component, &receipt.deletion.stopped)
        }
        RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) => {
            record.traversal_steps > 0
                && valid_subtree_membership_removed_record(record.component, receipt)
        }
        RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) => {
            record.traversal_steps > 0
                && valid_subtree_membership_removed_record(
                    record.component,
                    &receipt.membership_removed,
                )
                && receipt.covered_fleet_registry_revision > 0
                && receipt.covered_fleet_registry_content_hash != [0; 32]
                && receipt.covered_component_registry_revision
                    >= receipt.membership_removed.registry.revision
                && receipt.covered_component_registry_content_hash != [0; 32]
                && receipt.covered_authority_hash != [0; 32]
                && receipt
                    .owning_component
                    .as_ref()
                    .is_none_or(valid_subtree_directory_convergence_record)
                && receipt
                    .parent
                    .as_ref()
                    .is_none_or(valid_subtree_directory_convergence_record)
        }
        RootComponentSubtreeRemovalProgressRecord::Completed(completed) => {
            record.traversal_steps > 0
                && completed.registry.component == record.component
                && completed.registry.revision > 0
                && completed.registry.content_hash != [0; 32]
                && completed.directory_authority_hash != [0; 32]
        }
    };
    let target_is_active = ComponentTreeNodeIdentity::from_child(&record.target)
        .is_valid_for(record.component)
        && record.target.status == ComponentLifecycleStatus::Active;
    let registry_fence_is_versioned = record.component
        == record.reserved_against_registry.component
        && record.reserved_against_registry.revision > 0;
    let completion_count_is_valid = record.maximum_completed_leaves > 0
        && record.completed_leaves <= record.maximum_completed_leaves
        && (!matches!(
            &record.progress,
            RootComponentSubtreeRemovalProgressRecord::Fenced
        ) || record.completed_leaves == 0)
        && (!matches!(
            &record.progress,
            RootComponentSubtreeRemovalProgressRecord::Completed(_)
        ) || record.completed_leaves > 0);
    if record.operation_id == [0; 32]
        || !target_is_active
        || !registry_fence_is_versioned
        || !progress_is_valid
        || !completion_count_is_valid
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component subtree-removal fence has invalid protected identity",
        ));
    }
    Ok(())
}

fn valid_subtree_removal_node(
    component: ComponentInstanceId,
    node: &ComponentRegistryChildRecord,
) -> bool {
    ComponentTreeNodeIdentity::from_child(node).is_valid_for(component)
        && node.status == ComponentLifecycleStatus::Active
}

fn valid_subtree_stopped_effect(
    component: ComponentInstanceId,
    stopped: &RootComponentSubtreeStoppedEffectRecord,
) -> bool {
    stopped.stop.controller != Principal::anonymous()
        && valid_subtree_removal_node(component, &stopped.stop.leaf)
}

fn valid_subtree_membership_removed_record(
    component: ComponentInstanceId,
    receipt: &RootComponentSubtreeMembershipRemovedRecord,
) -> bool {
    valid_subtree_stopped_effect(component, &receipt.deleted.deletion.stopped)
        && receipt.removed_from_registry.component == component
        && receipt.removed_from_registry.revision > 0
        && receipt.previous_descendant_content_hash != [0; 32]
        && receipt.previous_committed_descendants > 0
        && receipt.registry.component == component
        && receipt.registry.revision > receipt.removed_from_registry.revision
        && receipt.descendant_content_hash != [0; 32]
        && receipt.registry_encoded_bytes > 0
        && receipt.committed_descendants == receipt.previous_committed_descendants.saturating_sub(1)
        && receipt.directory_synchronized_at_ns > 0
        && receipt.directory_authority_hash != [0; 32]
}

fn valid_subtree_directory_convergence_record(
    evidence: &RootComponentSubtreeDirectoryConvergenceRecord,
) -> bool {
    evidence.operation_id != [0; 32]
        && evidence.canister_id != Principal::anonymous()
        && evidence.activation.directory_authority_hash != [0; 32]
        && evidence.activation.activated_at_ns > 0
}

fn validate_subtree_removal_root(
    record: &RootComponentSubtreeRemovalRecord,
    root: &FleetSubnetRootBinding,
) -> Result<(), InternalError> {
    let stop_controller = match &record.progress {
        RootComponentSubtreeRemovalProgressRecord::StopIntent(effect) => Some(effect.controller),
        RootComponentSubtreeRemovalProgressRecord::Stopped(receipt) => {
            Some(receipt.stop.controller)
        }
        RootComponentSubtreeRemovalProgressRecord::DeleteIntent(deletion) => {
            Some(deletion.stopped.stop.controller)
        }
        RootComponentSubtreeRemovalProgressRecord::Deleted(receipt) => {
            Some(receipt.deletion.stopped.stop.controller)
        }
        RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) => {
            Some(receipt.deleted.deletion.stopped.stop.controller)
        }
        RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) => Some(
            receipt
                .membership_removed
                .deleted
                .deletion
                .stopped
                .stop
                .controller,
        ),
        RootComponentSubtreeRemovalProgressRecord::Completed(_)
        | RootComponentSubtreeRemovalProgressRecord::Fenced
        | RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
        | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. } => None,
    };
    if stop_controller.is_some_and(|controller| controller != root.fleet_subnet_root) {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component subtree stop intent differs from protected root authority",
        ));
    }
    Ok(())
}

fn validate_subtree_removal_progress(
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentSubtreeRemovalRecord,
) -> Result<(), InternalError> {
    if let RootComponentSubtreeRemovalProgressRecord::Completed(completed) = &record.progress {
        return validate_completed_subtree_removal(partition, record, completed);
    }
    if let RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) = &record.progress
    {
        return validate_subtree_membership_removed(partition, record, receipt);
    }
    if let RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) =
        &record.progress
    {
        validate_subtree_membership_removed(partition, record, &receipt.membership_removed)?;
        return validate_subtree_directory_synchronized(partition, record, receipt);
    }
    let current_target =
        RootComponentRegistryStore::child(record.component, record.target.canister_id).ok_or_else(
            || {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component subtree-removal target is no longer registered",
                )
            },
        )?;
    validate_registered_child_record(partition, &current_target)?;
    if current_target != record.target {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component subtree-removal target differs from its frozen fence",
        ));
    }

    let node = match &record.progress {
        RootComponentSubtreeRemovalProgressRecord::Fenced => None,
        RootComponentSubtreeRemovalProgressRecord::Traversing { cursor } => Some((cursor, false)),
        RootComponentSubtreeRemovalProgressRecord::LeafSelected { leaf } => Some((leaf, true)),
        RootComponentSubtreeRemovalProgressRecord::StopIntent(effect) => Some((&effect.leaf, true)),
        RootComponentSubtreeRemovalProgressRecord::Stopped(receipt) => {
            Some((&receipt.stop.leaf, true))
        }
        RootComponentSubtreeRemovalProgressRecord::DeleteIntent(deletion) => {
            Some((&deletion.stopped.stop.leaf, true))
        }
        RootComponentSubtreeRemovalProgressRecord::Deleted(receipt) => {
            Some((&receipt.deletion.stopped.stop.leaf, true))
        }
        RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(_) => {
            unreachable!("membership-removed progress is validated before registered-cursor checks")
        }
        RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(_) => {
            unreachable!("Directory-synchronized progress is validated before cursor checks")
        }
        RootComponentSubtreeRemovalProgressRecord::Completed(_) => {
            unreachable!("completed progress is validated before cursor checks")
        }
    };
    let Some((node, must_be_leaf)) = node else {
        return Ok(());
    };
    let current_node = RootComponentRegistryStore::child(record.component, node.canister_id)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component subtree-removal cursor is no longer registered",
            )
        })?;
    validate_registered_child_record(partition, &current_node)?;
    if &current_node != node {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component subtree-removal cursor differs from current Registry authority",
        ));
    }
    let traversal_limit = partition
        .committed_descendants
        .checked_add(1)
        .ok_or_else(|| InternalError::resource_exhausted("Component descendant count overflow"))?;
    if !canister_is_in_subtree(
        partition,
        node.canister_id,
        record.target.canister_id,
        traversal_limit,
    )? {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component subtree-removal cursor escaped its fenced subtree",
        ));
    }
    if must_be_leaf && first_registered_child(partition, node.canister_id)?.is_some() {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component subtree-removal selected a node that still has a registered child",
        ));
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "one validator reconstructs the complete historical and current membership-removal authority"
)]
fn validate_subtree_membership_removed(
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentSubtreeRemovalRecord,
    receipt: &RootComponentSubtreeMembershipRemovedRecord,
) -> Result<(), InternalError> {
    let leaf = &receipt.deleted.deletion.stopped.stop.leaf;
    let expected_descendant_hash = removed_component_descendant_content_hash(
        record.component,
        receipt.previous_descendant_content_hash,
        receipt.removed_from_registry.revision,
        receipt.previous_committed_descendants,
        receipt.registry.revision,
        leaf,
    )?;
    let expected_content_hash = component_partition_content_hash(
        &partition.binding,
        &partition.provisioning_origin,
        partition.release_set,
        partition.status,
        receipt.registry.revision,
        receipt.descendant_content_hash,
        receipt.committed_descendants,
    )?;
    let expected_previous_content_hash = component_partition_content_hash(
        &partition.binding,
        &partition.provisioning_origin,
        partition.release_set,
        partition.status,
        receipt.removed_from_registry.revision,
        receipt.previous_descendant_content_hash,
        receipt.previous_committed_descendants,
    )?;
    let later_progress_can_grow_registry_bytes = matches!(
        &record.progress,
        RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(_)
    ) && partition.encoded_bytes
        >= receipt.registry_encoded_bytes;
    let exact_head_is_current = partition.revision == receipt.registry.revision
        && partition.content_hash == receipt.registry.content_hash
        && partition.descendant_content_hash == receipt.descendant_content_hash
        && partition.committed_descendants == receipt.committed_descendants
        && partition.reserved_descendants == receipt.reserved_descendants
        && (partition.encoded_bytes == receipt.registry_encoded_bytes
            || later_progress_can_grow_registry_bytes)
        && partition.directory_synchronized_at_ns == receipt.directory_synchronized_at_ns;
    let head_was_advanced = partition.revision > receipt.registry.revision
        && partition.directory_synchronized_at_ns >= receipt.directory_synchronized_at_ns;
    let removed_indexes_are_absent =
        RootComponentRegistryStore::child(record.component, leaf.canister_id).is_none()
            && RootComponentRegistryStore::child_traversal(
                record.component,
                leaf.parent_canister_id,
                &leaf.role,
                leaf.canister_id,
            )
            .is_none()
            && RootComponentRegistryStore::component_for_principal(leaf.canister_id).is_none();
    let current_parent_role_instances = RootComponentRegistryStore::parent_role_count(
        record.component,
        leaf.parent_canister_id,
        &leaf.role,
    )
    .map_or(0, |count| count.instances);
    let receipt_is_canonical = receipt.removed_from_registry.content_hash
        == expected_previous_content_hash
        && receipt.registry.content_hash == expected_content_hash
        && receipt.descendant_content_hash == expected_descendant_hash
        && receipt.committed_descendants
            == receipt
                .previous_committed_descendants
                .checked_sub(1)
                .ok_or_else(|| {
                    InternalError::invariant(
                        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                        "Component membership-removal receipt has no previous descendant",
                    )
                })?
        && current_parent_role_instances >= receipt.parent_role_instances;
    if !receipt_is_canonical
        || (!exact_head_is_current && !head_was_advanced)
        || !removed_indexes_are_absent
        || first_registered_child(partition, leaf.canister_id)?.is_some()
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component subtree membership-removal receipt differs from Registry authority",
        ));
    }
    if record.target.canister_id != leaf.canister_id {
        let current_target = RootComponentRegistryStore::child(
            record.component,
            record.target.canister_id,
        )
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component subtree-removal target disappeared before its selected descendant",
            )
        })?;
        validate_registered_child_record(partition, &current_target)?;
        if current_target != record.target {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component subtree-removal target differs from its frozen fence",
            ));
        }
    }
    Ok(())
}

fn validate_subtree_directory_synchronized(
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentSubtreeRemovalRecord,
    receipt: &RootComponentSubtreeDirectorySynchronizedRecord,
) -> Result<(), InternalError> {
    let membership = &receipt.membership_removed;
    let leaf = &membership.deleted.deletion.stopped.stop.leaf;
    let covered_component_registry = ComponentRegistryHead {
        component: record.component,
        revision: receipt.covered_component_registry_revision,
        content_hash: receipt.covered_component_registry_content_hash,
    };
    let coverage_covers_current_or_prior = covered_component_registry.revision
        <= partition.revision
        && (covered_component_registry.revision != partition.revision
            || covered_component_registry.content_hash == partition.content_hash);
    let coverage_covers_membership = covered_component_registry.revision
        > membership.registry.revision
        || covered_component_registry == membership.registry;
    let owner_is_exact = match (partition.status, receipt.owning_component.as_ref()) {
        (ComponentLifecycleStatus::Active, Some(owner)) => {
            owner.canister_id == partition.binding.canister_id
        }
        (ComponentLifecycleStatus::Draining, None) => component_has_terminal_quiescence(partition)?,
        _ => false,
    };
    let expected_parent = if leaf.parent_canister_id == partition.binding.canister_id {
        None
    } else {
        ComponentRegistryOps::registered_parent(record.component, leaf.parent_canister_id)?
    };
    let parent_is_exact = match (expected_parent, receipt.parent.as_ref()) {
        (None, None) => true,
        (Some((binding, ComponentLifecycleStatus::Active)), Some(parent)) => {
            parent.canister_id == managed_canister_principal(&binding)
        }
        _ => false,
    };
    if !owner_is_exact
        || !coverage_covers_current_or_prior
        || !coverage_covers_membership
        || !parent_is_exact
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component subtree Directory receipt differs from surviving Registry authority",
        ));
    }
    Ok(())
}

fn validate_completed_subtree_removal(
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentSubtreeRemovalRecord,
    completed: &RootComponentSubtreeRemovalCompletedRecord,
) -> Result<(), InternalError> {
    let history = RootComponentRegistryStore::subtree_removal_completed_leaf(
        record.component,
        record.operation_id,
        record.traversal_steps,
    )
    .ok_or_else(|| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "completed Component subtree removal has no terminal leaf history",
        )
    })?;
    validate_subtree_removal_completed_leaf(record, partition, &history)?;
    let terminal_authority_matches = history.leaf_canister_id == record.target.canister_id
        && completed.registry == history.registry
        && completed.directory_authority_hash == history.directory_authority_hash
        && partition.revision >= completed.registry.revision
        && (partition.revision != completed.registry.revision
            || partition.content_hash == completed.registry.content_hash);
    if !terminal_authority_matches
        || RootComponentRegistryStore::child(record.component, record.target.canister_id).is_some()
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "completed Component subtree removal differs from terminal Registry authority",
        ));
    }
    Ok(())
}

fn completed_subtree_leaf_record(
    removal: &RootComponentSubtreeRemovalRecord,
    receipt: &RootComponentSubtreeDirectorySynchronizedRecord,
) -> Result<RootComponentSubtreeRemovalCompletedLeafRecord, InternalError> {
    let stopped = &receipt.membership_removed.deleted.deletion.stopped;
    Ok(RootComponentSubtreeRemovalCompletedLeafRecord {
        operation_id: removal.operation_id,
        component: removal.component,
        traversal_steps: removal.traversal_steps,
        leaf_canister_id: stopped.stop.leaf.canister_id,
        leaf_parent_canister_id: stopped.stop.leaf.parent_canister_id,
        observed_module_hash: stopped.observed_module_hash,
        registry: ComponentRegistryHead {
            component: removal.component,
            revision: receipt.covered_component_registry_revision,
            content_hash: receipt.covered_component_registry_content_hash,
        },
        directory_authority_hash: receipt.covered_authority_hash,
        receipt_hash: subtree_directory_synchronized_receipt_hash(receipt)?,
    })
}

fn subtree_directory_synchronized_receipt_hash(
    receipt: &RootComponentSubtreeDirectorySynchronizedRecord,
) -> Result<[u8; 32], InternalError> {
    const DOMAIN: &[u8] = b"canic.component-subtree-removal.completed-leaf.v1";
    let payload = canic_core::cdk::serialize::serialize(receipt).map_err(|error| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            format!("completed subtree leaf receipt cannot be encoded: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn finalized_subtree_removal_progress(
    component: ComponentInstanceId,
    partition: &ComponentRegistryPartitionRecord,
    removal: &RootComponentSubtreeRemovalRecord,
    receipt: &RootComponentSubtreeDirectorySynchronizedRecord,
) -> Result<RootComponentSubtreeRemovalProgressRecord, InternalError> {
    let leaf = &receipt
        .membership_removed
        .deleted
        .deletion
        .stopped
        .stop
        .leaf;
    if leaf.canister_id == removal.target.canister_id {
        return Ok(RootComponentSubtreeRemovalProgressRecord::Completed(
            RootComponentSubtreeRemovalCompletedRecord {
                registry: ComponentRegistryHead {
                    component,
                    revision: receipt.covered_component_registry_revision,
                    content_hash: receipt.covered_component_registry_content_hash,
                },
                directory_authority_hash: receipt.covered_authority_hash,
            },
        ));
    }

    let parent =
        RootComponentRegistryStore::child(component, leaf.parent_canister_id).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "finalized Component subtree leaf has no retained parent row",
            )
        })?;
    validate_registered_child_record(partition, &parent)?;
    if parent.status != ComponentLifecycleStatus::Active {
        return Err(InternalError::conflict(
            "finalized Component subtree leaf parent is not Active",
        ));
    }
    Ok(RootComponentSubtreeRemovalProgressRecord::Traversing { cursor: parent })
}

fn validate_subtree_removal_completed_leaf(
    removal: &RootComponentSubtreeRemovalRecord,
    partition: &ComponentRegistryPartitionRecord,
    history: &RootComponentSubtreeRemovalCompletedLeafRecord,
) -> Result<(), InternalError> {
    let valid = history.operation_id == removal.operation_id
        && history.component == removal.component
        && history.traversal_steps > 0
        && history.traversal_steps <= removal.traversal_steps
        && history.leaf_canister_id != Principal::anonymous()
        && history.leaf_parent_canister_id != Principal::anonymous()
        && history.observed_module_hash != [0; 32]
        && history.registry.component == removal.component
        && history.registry.revision > 0
        && history.registry.content_hash != [0; 32]
        && history.registry.revision <= partition.revision
        && (history.registry.revision != partition.revision
            || history.registry.content_hash == partition.content_hash)
        && history.directory_authority_hash != [0; 32]
        && history.receipt_hash != [0; 32];
    if !valid {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "completed Component subtree leaf history has invalid protected authority",
        ));
    }
    Ok(())
}

fn completed_subtree_leaf_for_selection(
    removal: &RootComponentSubtreeRemovalRecord,
    partition: &ComponentRegistryPartitionRecord,
    selection: SubtreeLeafSelection,
) -> Result<Option<RootComponentSubtreeRemovalCompletedLeafRecord>, InternalError> {
    let Some(history) = RootComponentRegistryStore::subtree_removal_completed_leaf(
        removal.component,
        removal.operation_id,
        selection.traversal_steps,
    ) else {
        return Ok(None);
    };
    validate_subtree_removal_completed_leaf(removal, partition, &history)?;
    if history.leaf_canister_id != selection.canister_id
        || history.leaf_parent_canister_id != selection.parent_canister_id
    {
        return Err(InternalError::conflict(
            "Component subtree leaf request differs from completed history",
        ));
    }
    Ok(Some(history))
}

fn first_registered_child(
    partition: &ComponentRegistryPartitionRecord,
    parent_canister_id: Principal,
) -> Result<Option<ComponentRegistryChildRecord>, InternalError> {
    let Some(traversal) = RootComponentRegistryStore::child_traversals_page(
        partition.binding.component,
        Some(parent_canister_id),
        None,
        None,
        1,
    )
    .into_iter()
    .next() else {
        return Ok(None);
    };
    validate_child_traversal_record(partition.binding.component, &traversal)?;
    let child =
        RootComponentRegistryStore::child(partition.binding.component, traversal.canister_id)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component subtree traversal references an absent child row",
                )
            })?;
    validate_registered_child_record(partition, &child)?;
    let expected_identity = ComponentTreeNodeIdentity::new(
        partition.binding.component,
        parent_canister_id,
        &child.role,
        child.canister_id,
    );
    if ComponentTreeNodeIdentity::from_traversal(&traversal) != expected_identity {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component subtree traversal index differs from its child row",
        ));
    }
    Ok(Some(child))
}

const fn child_allocation_is_terminal(record: &RootComponentChildAllocationRecord) -> bool {
    matches!(
        &record.progress,
        RootComponentChildAllocationProgressRecord::Committed {
            commitment:
                RootComponentChildCommitmentRecord {
                    membership: Some(membership),
                    ..
                },
            ..
        } if membership.directory_synchronized
    )
}

fn ensure_component_lifecycle_history_is_terminal(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<(), InternalError> {
    for allocation in RootComponentRegistryStore::child_allocations(partition.binding.component) {
        validate_child_allocation_record(&allocation)?;
        if !child_allocation_is_terminal(&allocation) {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component final inventory has incomplete child lifecycle history",
            ));
        }
    }
    for removal in RootComponentRegistryStore::subtree_removals(partition.binding.component) {
        validate_subtree_removal_record(&removal)?;
        validate_subtree_removal_progress(partition, &removal)?;
        if !matches!(
            removal.progress,
            RootComponentSubtreeRemovalProgressRecord::Completed(_)
        ) {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component final inventory has incomplete subtree-removal history",
            ));
        }
    }
    Ok(())
}

fn canister_is_in_subtree(
    partition: &ComponentRegistryPartitionRecord,
    candidate: Principal,
    target: Principal,
    traversal_limit: u32,
) -> Result<bool, InternalError> {
    let mut current = candidate;
    for _ in 0..traversal_limit {
        if current == target {
            return Ok(true);
        }
        if current == partition.binding.canister_id {
            return Ok(false);
        }
        let child = RootComponentRegistryStore::child(partition.binding.component, current)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component subtree ancestry references an unregistered child",
                )
            })?;
        validate_registered_child_record(partition, &child)?;
        current = child.parent_canister_id;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component subtree ancestry exceeded its committed descendant bound",
    ))
}

fn validate_child_traversal_record(
    component: ComponentInstanceId,
    traversal: &ComponentRegistryChildTraversalRecord,
) -> Result<(), InternalError> {
    if !ComponentTreeNodeIdentity::from_traversal(traversal).is_valid_for(component) {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Directory traversal has invalid tree identity",
        ));
    }
    Ok(())
}

fn traversal_record_to_cursor(
    traversal: &ComponentRegistryChildTraversalRecord,
) -> ComponentDirectoryCanonicalCursor {
    ComponentDirectoryCanonicalCursor {
        parent_canister_id: traversal.parent_canister_id,
        role: traversal.role.clone(),
        canister_id: traversal.canister_id,
    }
}

fn child_record_to_directory_view(
    partition: &ComponentRegistryPartitionRecord,
    child: ComponentRegistryChildRecord,
) -> ComponentDirectoryChildView {
    ComponentDirectoryChildView {
        binding: ComponentChildBinding {
            component: partition.binding.clone(),
            parent_canister_id: child.parent_canister_id,
            role: child.role,
            canister_id: child.canister_id,
        },
        kind: child.kind,
        installed_artifact_hash: child.installed_artifact_hash,
        status: child.status,
    }
}

fn validate_child_allocation_record(
    record: &RootComponentChildAllocationRecord,
) -> Result<(), InternalError> {
    if record.operation_id == [0; 32]
        || record.component != record.reserved_against_registry.component
        || record.reserved_against_registry.revision == 0
        || record.parent_canister_id == Principal::anonymous()
        || record.maximum_instances_per_parent == 0
        || record.maximum_descendants == 0
        || record.maximum_registry_bytes == 0
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Child allocation record has invalid protected identity",
        ));
    }
    Ok(())
}

fn component_partition_content_hash(
    binding: &ComponentBinding,
    provisioning_origin: &ComponentProvisioningOrigin,
    release_set: FleetSubnetRootReleaseSet,
    status: ComponentLifecycleStatus,
    revision: u64,
    descendant_content_hash: [u8; 32],
    committed_descendants: u32,
) -> Result<[u8; 32], InternalError> {
    const DOMAIN: &[u8] = b"canic.component-registry.partition.v1";
    let payload = candid::encode_one((
        binding.clone(),
        provisioning_origin.clone(),
        release_set,
        status,
        revision,
        descendant_content_hash,
        committed_descendants,
    ))
    .map_err(|error| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            format!("Component Registry hash input cannot be encoded: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn empty_component_descendant_content_hash(component: ComponentInstanceId) -> [u8; 32] {
    const DOMAIN: &[u8] = b"canic.component-registry.descendants.v1";
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update(component.as_bytes());
    hasher.finalize().into()
}

fn component_final_inventory_hash(
    partition: &ComponentRegistryPartitionRecord,
    inventory: &RootComponentFinalInventoryRecord,
) -> Result<[u8; 32], InternalError> {
    let payload = candid::encode_one(RootComponentFinalInventoryHashAuthority {
        binding: partition.binding.clone(),
        provisioning_origin: partition.provisioning_origin.clone(),
        release_set: partition.release_set,
        status: partition.status,
        registry: inventory.registry.clone(),
        descendant_content_hash: inventory.descendant_content_hash,
        directory_synchronized_at_ns: inventory.directory_synchronized_at_ns,
        reserved_descendants: partition.reserved_descendants,
        committed_descendants: partition.committed_descendants,
        registry_encoded_bytes: inventory.registry_encoded_bytes,
        covered_fleet_registry_revision: inventory.covered_fleet_registry_revision,
        covered_fleet_registry_content_hash: inventory.covered_fleet_registry_content_hash,
        directory_authority_hash: inventory.directory_authority_hash,
        finalized_at_ns: inventory.finalized_at_ns,
    })
    .map_err(|error| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            format!("Component final inventory hash input cannot be encoded: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(COMPONENT_FINAL_INVENTORY_HASH_DOMAIN);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn component_draining_subtree_operation_id(
    draining: &RootComponentDrainingRecord,
    target_canister_id: Principal,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(COMPONENT_DRAINING_SUBTREE_OPERATION_DOMAIN);
    hasher.update(draining.operation_id);
    hasher.update(draining.component.as_bytes());
    hasher.update(draining.registry.revision.to_be_bytes());
    hasher.update(draining.registry.content_hash);
    hasher.update((target_canister_id.as_slice().len() as u64).to_be_bytes());
    hasher.update(target_canister_id.as_slice());
    hasher.finalize().into()
}

fn committed_component_descendant_content_hash(
    previous: [u8; 32],
    previous_committed_descendants: u32,
    revision: u64,
    child: &ComponentRegistryChildRecord,
) -> Result<[u8; 32], InternalError> {
    const DOMAIN: &[u8] = b"canic.component-registry.descendant-commit.v1";
    if previous == [0; 32] || child.status != ComponentLifecycleStatus::Prepared {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "Component descendant digest input is invalid",
        ));
    }
    let payload = candid::encode_one((
        previous,
        previous_committed_descendants,
        revision,
        child.canister_id,
        child.parent_canister_id,
        child.role.clone(),
        child.kind,
        child.installed_artifact_hash,
        child.status,
    ))
    .map_err(|error| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            format!("Component descendant digest input cannot be encoded: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn activated_component_descendant_content_hash(
    previous: [u8; 32],
    previous_revision: u64,
    revision: u64,
    child: &ComponentRegistryChildRecord,
) -> Result<[u8; 32], InternalError> {
    const DOMAIN: &[u8] = b"canic.component-registry.descendant-activate.v1";
    if previous == [0; 32]
        || previous_revision == 0
        || revision <= previous_revision
        || child.status != ComponentLifecycleStatus::Active
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "Component descendant activation digest input is invalid",
        ));
    }
    let payload = candid::encode_one((
        previous,
        previous_revision,
        revision,
        child.canister_id,
        child.parent_canister_id,
        child.role.clone(),
        child.kind,
        child.installed_artifact_hash,
        ComponentLifecycleStatus::Prepared,
        child.status,
    ))
    .map_err(|error| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            format!("Component descendant activation digest input cannot be encoded: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn removed_component_descendant_content_hash(
    component: ComponentInstanceId,
    previous: [u8; 32],
    previous_revision: u64,
    previous_committed_descendants: u32,
    revision: u64,
    child: &ComponentRegistryChildRecord,
) -> Result<[u8; 32], InternalError> {
    const DOMAIN: &[u8] = b"canic.component-registry.descendant-remove.v1";
    if previous == [0; 32]
        || previous_revision == 0
        || previous_committed_descendants == 0
        || revision <= previous_revision
        || child.component != component
        || child.status != ComponentLifecycleStatus::Active
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "Component descendant removal digest input is invalid",
        ));
    }
    if previous_committed_descendants == 1 {
        return Ok(empty_component_descendant_content_hash(component));
    }
    let payload = candid::encode_one((
        previous,
        previous_revision,
        previous_committed_descendants,
        revision,
        child.canister_id,
        child.parent_canister_id,
        child.role.clone(),
        child.kind,
        child.installed_artifact_hash,
        child.status,
    ))
    .map_err(|error| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            format!("Component descendant removal digest input cannot be encoded: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn validate_install_capacity(
    current: &RootComponentRegistryMetaRecord,
    record: &RootComponentAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<u64, InternalError> {
    let current_reserved_bytes = match &record.progress {
        RootComponentAllocationProgressRecord::Created { effect, .. } => effect.charged_entry_bytes,
        _ => {
            return Err(InternalError::conflict(
                "Component allocation is not ready to reserve install capacity",
            ));
        }
    };
    let without_current = current
        .encoded_bytes
        .checked_sub(current_reserved_bytes)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry encoded-byte accounting is below its creation reservation",
            )
        })?;
    let next_encoded_bytes = without_current
        .checked_add(charged_entry_bytes)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    if next_encoded_bytes > current.root.limits.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted(format!(
            "Component installation evidence requires {next_encoded_bytes} bytes, exceeding protected limit {}",
            current.root.limits.maximum_registry_bytes
        )));
    }
    Ok(next_encoded_bytes)
}

fn validate_install_effect_record(
    effect: &RootComponentInstallEffectRecord,
    plan: &RootComponentInstallPlan,
) -> Result<(), InternalError> {
    if effect.raw_module_hash != plan.raw_module_hash
        || effect.chunk_hashes != plan.chunk_hashes
        || effect.binding != plan.binding
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "durable Component install intent differs from verified module or binding authority",
        ));
    }
    Ok(())
}

fn advance_install_phase(
    operation_id: [u8; 32],
    verified: bool,
) -> Result<RootComponentAllocationView, InternalError> {
    let current = RootComponentRegistryStore::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
        InternalError::unavailable("Component allocation operation has not been reserved")
    })?;
    let next_progress = match (&record.progress, verified) {
        (
            RootComponentAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            },
            false,
        ) => RootComponentAllocationProgressRecord::Installed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
        },
        (RootComponentAllocationProgressRecord::Installed { .. }, false)
        | (
            RootComponentAllocationProgressRecord::Verified { .. }
            | RootComponentAllocationProgressRecord::Committed { .. },
            _,
        ) => {
            return Ok(allocation_record_to_view(record));
        }
        (
            RootComponentAllocationProgressRecord::Installed {
                creation,
                canister,
                installation,
            },
            true,
        ) => RootComponentAllocationProgressRecord::Verified {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
        },
        _ => {
            return Err(InternalError::conflict(if verified {
                "Component allocation has not recorded successful installation"
            } else {
                "Component allocation has no durable install intent"
            }));
        }
    };
    let charged_entry_bytes = match &next_progress {
        RootComponentAllocationProgressRecord::Installed { installation, .. }
        | RootComponentAllocationProgressRecord::Verified { installation, .. } => {
            installation.charged_entry_bytes
        }
        _ => unreachable!(),
    };
    let mut next_record = record.clone();
    next_record.progress = next_progress;
    validate_charged_record_size(&next_record, charged_entry_bytes)?;
    RootComponentRegistryStore::replace_allocation(
        &current,
        current.clone(),
        &record,
        next_record.clone(),
    )
    .map_err(map_allocation_commit_error)?;
    Ok(allocation_record_to_view(next_record))
}

fn validate_creation_capacity(
    current: &RootComponentRegistryMetaRecord,
    record: &RootComponentAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<u64, InternalError> {
    if charged_entry_bytes > RootComponentRegistryStore::allocation_record_max_bytes() + 128 {
        return Err(InternalError::resource_exhausted(
            "Component creation evidence exceeds its stable record bound",
        ));
    }
    let current_entry_bytes = RootComponentRegistryStore::allocation_entry_bytes(record);
    let without_current = current
        .encoded_bytes
        .checked_sub(current_entry_bytes)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry encoded-byte accounting is below its reserved record",
            )
        })?;
    let next_encoded_bytes = without_current
        .checked_add(charged_entry_bytes)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    if next_encoded_bytes > current.root.limits.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted(format!(
            "Component creation evidence requires {next_encoded_bytes} bytes, exceeding protected limit {}",
            current.root.limits.maximum_registry_bytes
        )));
    }
    Ok(next_encoded_bytes)
}

fn validate_charged_record_size(
    record: &RootComponentAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<(), InternalError> {
    let entry_bytes = RootComponentRegistryStore::allocation_entry_bytes(record);
    if entry_bytes > charged_entry_bytes {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component allocation record exceeds its pre-effect Registry byte charge",
        ));
    }
    Ok(())
}

fn map_allocation_commit_error(error: RootComponentAllocationCommitError) -> InternalError {
    match error {
        RootComponentAllocationCommitError::ComponentIdentityConflict => InternalError::conflict(
            "derived Component identity is already reserved by another operation",
        ),
        RootComponentAllocationCommitError::ComponentPrincipalConflict => InternalError::conflict(
            "Component Canister principal is already indexed by another Registry partition",
        ),
        RootComponentAllocationCommitError::ConflictingChildEntry => InternalError::conflict(
            "Component Child reservation differs from its Registry partition or count index",
        ),
        RootComponentAllocationCommitError::ConflictingPartition => InternalError::conflict(
            "Component Registry partition is already committed under different authority",
        ),
        RootComponentAllocationCommitError::ConflictingOperation => InternalError::conflict(
            "Component allocation operation is already bound to different intent",
        ),
        RootComponentAllocationCommitError::ConflictingState => InternalError::conflict(
            "Component Registry authority changed before allocation mutation",
        ),
        RootComponentAllocationCommitError::MissingOperation => {
            InternalError::unavailable("Component allocation operation has not been reserved")
        }
        RootComponentAllocationCommitError::ParentPrincipalConflict => InternalError::forbidden(
            "Component Child reservation parent is not indexed by its Component Registry",
        ),
        RootComponentAllocationCommitError::Uninitialized => {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::stable::component_registry::RootComponentRegistryData;
    use canic_core::{
        cdk::types::Cycles,
        control_plane_support::{
            config::schema::ComponentChildKind,
            policy::{
                component_allocation::TopLevelComponentAllocationDecision,
                component_child_allocation::ComponentChildAllocationDecision,
            },
        },
        dto::{
            component_registry::{ComponentProvisioningOrigin, ComponentRuntimeActivationEvidence},
            fleet_registry::{
                FleetDirectoryProvenance, FleetDirectorySnapshot, FleetRegistryVersion,
                FleetSubnetRootDirectoryEntry, FleetSubnetRootStatus,
            },
            root_store::RootStoreBootstrapRequest,
        },
        ids::{
            AppId, CanisterRole, CanonicalNetworkId, ComponentInstanceId, ComponentSpecAdmission,
            ComponentTopologyDigest, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
            FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootLimits, ReleaseBuildId,
            ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
        },
    };

    fn restart_component_registry() -> RootComponentRegistryData {
        let snapshot = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(snapshot.clone());
        assert_eq!(RootComponentRegistryStore::export(), snapshot);
        snapshot
    }

    fn exact_registry_entry_bytes(data: &RootComponentRegistryData) -> u64 {
        data.allocations
            .iter()
            .map(RootComponentRegistryStore::allocation_entry_bytes)
            .chain(
                data.partitions
                    .iter()
                    .map(RootComponentRegistryStore::partition_entry_bytes),
            )
            .chain(data.partitions.iter().map(|partition| {
                RootComponentRegistryStore::principal_index_entry_bytes(
                    partition.binding.canister_id,
                    partition.binding.component,
                )
            }))
            .chain(
                data.children
                    .iter()
                    .map(RootComponentRegistryStore::child_entry_bytes),
            )
            .chain(data.children.iter().map(|child| {
                RootComponentRegistryStore::principal_index_entry_bytes(
                    child.canister_id,
                    child.component,
                )
            }))
            .chain(
                data.child_traversals
                    .iter()
                    .map(RootComponentRegistryStore::child_traversal_entry_bytes),
            )
            .chain(
                data.child_allocations
                    .iter()
                    .map(RootComponentRegistryStore::child_allocation_entry_bytes),
            )
            .chain(
                data.subtree_removals
                    .iter()
                    .map(RootComponentRegistryStore::subtree_removal_entry_bytes),
            )
            .chain(
                data.subtree_removal_history
                    .iter()
                    .map(RootComponentRegistryStore::subtree_removal_completed_leaf_entry_bytes),
            )
            .chain(
                data.component_drainings
                    .iter()
                    .map(charged_component_draining_entry_bytes),
            )
            .chain(
                data.parent_role_counts
                    .iter()
                    .map(RootComponentRegistryStore::parent_role_count_entry_bytes),
            )
            .sum()
    }

    fn exact_component_registry_entry_bytes(
        data: &RootComponentRegistryData,
        component: ComponentInstanceId,
    ) -> u64 {
        data.partitions
            .iter()
            .filter(|partition| partition.binding.component == component)
            .map(RootComponentRegistryStore::partition_entry_bytes)
            .chain(
                data.partitions
                    .iter()
                    .filter(|partition| partition.binding.component == component)
                    .map(|partition| {
                        RootComponentRegistryStore::principal_index_entry_bytes(
                            partition.binding.canister_id,
                            component,
                        )
                    }),
            )
            .chain(
                data.children
                    .iter()
                    .filter(|child| child.component == component)
                    .map(RootComponentRegistryStore::child_entry_bytes),
            )
            .chain(
                data.children
                    .iter()
                    .filter(|child| child.component == component)
                    .map(|child| {
                        RootComponentRegistryStore::principal_index_entry_bytes(
                            child.canister_id,
                            component,
                        )
                    }),
            )
            .chain(
                data.child_traversals
                    .iter()
                    .filter(|traversal| traversal.component == component)
                    .map(RootComponentRegistryStore::child_traversal_entry_bytes),
            )
            .chain(
                data.child_allocations
                    .iter()
                    .filter(|allocation| allocation.component == component)
                    .map(charged_child_allocation_entry_bytes),
            )
            .chain(
                data.subtree_removals
                    .iter()
                    .filter(|removal| removal.component == component)
                    .map(RootComponentRegistryStore::subtree_removal_entry_bytes),
            )
            .chain(
                data.subtree_removal_history
                    .iter()
                    .filter(|history| history.component == component)
                    .map(RootComponentRegistryStore::subtree_removal_completed_leaf_entry_bytes),
            )
            .chain(
                data.component_drainings
                    .iter()
                    .filter(|draining| draining.component == component)
                    .map(charged_component_draining_entry_bytes),
            )
            .chain(
                data.parent_role_counts
                    .iter()
                    .filter(|count| count.component == component)
                    .map(RootComponentRegistryStore::parent_role_count_entry_bytes),
            )
            .sum()
    }

    fn charged_child_allocation_entry_bytes(record: &RootComponentChildAllocationRecord) -> u64 {
        match &record.progress {
            RootComponentChildAllocationProgressRecord::Reserved => {
                RootComponentRegistryStore::child_allocation_entry_bytes(record)
            }
            RootComponentChildAllocationProgressRecord::CreationIntent(creation)
            | RootComponentChildAllocationProgressRecord::Created {
                effect: creation, ..
            } => creation.charged_entry_bytes,
            RootComponentChildAllocationProgressRecord::InstallIntent { installation, .. }
            | RootComponentChildAllocationProgressRecord::Installed { installation, .. }
            | RootComponentChildAllocationProgressRecord::Verified { installation, .. }
            | RootComponentChildAllocationProgressRecord::Committed { installation, .. } => {
                installation.charged_entry_bytes
            }
        }
    }

    #[test]
    fn preparation_is_exact_idempotent_and_conflict_closed() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let version = FleetRegistryVersion {
            authority: root.authority.clone(),
            revision: 4,
            content_hash: [5; 32],
        };
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        let store_bootstrap = RootStoreBootstrapRequest {
            manifest_payload_size_bytes: 128,
        };

        let prepared = ComponentRegistryOps::prepare(
            root.clone(),
            version.clone(),
            release_set,
            store_bootstrap.clone(),
        )
        .expect("prepare");
        let repeated =
            ComponentRegistryOps::prepare(root.clone(), version, release_set, store_bootstrap)
                .expect("exact retry");

        assert_eq!(prepared, repeated);
        assert_eq!(prepared.next_allocation_sequence, 1);
        assert_eq!(prepared.reserved_component_instances, 0);
        assert_eq!(prepared.committed_component_instances, 0);
        assert_eq!(prepared.managed_descendants, 0);
        assert_eq!(prepared.known_created_component_canisters, 0);
        assert_eq!(prepared.encoded_bytes, 0);

        let mut conflicting = root;
        conflicting.limits.maximum_component_instances += 1;
        assert!(
            ComponentRegistryOps::prepare(
                conflicting,
                repeated.prepared_against_registry,
                release_set,
                repeated.store_bootstrap,
            )
            .is_err()
        );
        let empty_inventory = ComponentRegistryOps::seal_initial_inventory([10; 32], 11)
            .expect("seal empty initial inventory");
        assert_eq!(empty_inventory.receipt.component_count, 0);
        assert_ne!(empty_inventory.receipt.inventory_hash, [0; 32]);
        assert!(empty_inventory.operation_ids.is_empty());
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    fn allocation_reservation_is_exact_idempotent_and_charges_registry_capacity() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        let version = FleetRegistryVersion {
            authority: root.authority.clone(),
            revision: 4,
            content_hash: [5; 32],
        };
        ComponentRegistryOps::prepare(
            root,
            version,
            release_set,
            RootStoreBootstrapRequest {
                manifest_payload_size_bytes: 128,
            },
        )
        .expect("prepare");
        let decision = TopLevelComponentAllocationDecision {
            allocation_sequence: 1,
            component: ComponentInstanceId::from_generated_bytes([10; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
        };
        let origin = ComponentProvisioningOrigin::FleetAdministrator {
            caller: candid::Principal::from_slice(&[11; 29]),
        };

        let reserved = ComponentRegistryOps::reserve_allocation(
            decision.clone(),
            [12; 32],
            origin.clone(),
            false,
        )
        .expect("reserve");
        let interrupted_snapshot = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(interrupted_snapshot);
        let repeated = ComponentRegistryOps::reserve_allocation(decision, [12; 32], origin, false)
            .expect("exact retry");

        assert_eq!(reserved, repeated);
        assert_eq!(reserved.allocation_sequence, 1);
        let status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(status.next_allocation_sequence, 2);
        assert_eq!(status.reserved_component_instances, 1);
        assert_eq!(status.committed_component_instances, 0);
        assert_eq!(status.known_created_component_canisters, 0);
        assert!(status.encoded_bytes > 0);
        assert_eq!(
            ComponentRegistryOps::component_spec_counts(&reserved.component_spec)
                .expect("Spec counts"),
            ComponentSpecInstanceCounts {
                reserved: 1,
                committed: 0,
            }
        );
        assert!(
            ComponentRegistryOps::seal_initial_inventory([20; 32], 21).is_err(),
            "a nonterminal allocation must prevent initial inventory sealing"
        );
        assert!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .initial_inventory
                .is_none()
        );

        let conflicting = TopLevelComponentAllocationDecision {
            allocation_sequence: 2,
            component: ComponentInstanceId::from_generated_bytes([13; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
        };
        assert!(
            ComponentRegistryOps::reserve_allocation(
                conflicting,
                [12; 32],
                ComponentProvisioningOrigin::FleetAdministrator {
                    caller: candid::Principal::from_slice(&[11; 29]),
                },
                false,
            )
            .is_err()
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    fn component_final_inventory_is_exact_durable_and_response_idempotent() {
        let (partition, draining, fleet) = import_empty_quiescent_component();
        let RootComponentDrainingAdvanceView::DescendantsEmpty {
            registry,
            descendant_content_hash,
        } = ComponentRegistryOps::advance_component_draining(
            partition.binding.component,
            draining.operation_id,
        )
        .expect("observe exact empty draining inventory")
        else {
            panic!("empty Component must have no draining subtree");
        };
        assert_eq!(registry, draining.registry);
        assert_eq!(
            descendant_content_hash,
            empty_component_descendant_content_hash(partition.binding.component)
        );

        let before_finalization = RootComponentRegistryStore::export();
        let mut conflicting_registry = registry.clone();
        conflicting_registry.revision += 1;
        ComponentRegistryOps::finalize_component_inventory(
            partition.binding.component,
            draining.operation_id,
            conflicting_registry.clone(),
            fleet.clone(),
            112,
        )
        .expect_err("final inventory must bind the exact current Registry head");
        ComponentRegistryOps::finalize_component_inventory(
            partition.binding.component,
            draining.operation_id,
            registry.clone(),
            fleet.clone(),
            110,
        )
        .expect_err("final inventory time cannot precede terminal quiescence");
        assert_eq!(RootComponentRegistryStore::export(), before_finalization);

        let inventory = ComponentRegistryOps::finalize_component_inventory(
            partition.binding.component,
            draining.operation_id,
            registry.clone(),
            fleet.clone(),
            112,
        )
        .expect("freeze exact final Component inventory");
        assert_final_inventory_receipt(&partition, &fleet, &inventory);

        let durable = restart_component_registry();
        let restarted = ComponentRegistryOps::component_draining(partition.binding.component)
            .expect("valid final inventory")
            .expect("durable draining authority");
        assert_eq!(restarted.final_inventory, Some(inventory.clone()));
        assert_eq!(
            ComponentRegistryOps::finalize_component_inventory(
                partition.binding.component,
                draining.operation_id,
                registry,
                fleet,
                999,
            )
            .expect("exact final-inventory retry returns the original receipt"),
            inventory
        );
        assert_eq!(RootComponentRegistryStore::export(), durable);
        ComponentRegistryOps::finalize_component_inventory(
            partition.binding.component,
            draining.operation_id,
            conflicting_registry,
            fleet_directory(&root_binding()),
            999,
        )
        .expect_err("final inventory rejects a different Registry head after commit");
        assert_eq!(RootComponentRegistryStore::export(), durable);
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            exact_registry_entry_bytes(&durable)
        );
        assert_eq!(
            partition.encoded_bytes,
            exact_component_registry_entry_bytes(&durable, partition.binding.component)
        );

        let mut corrupted = durable;
        corrupted.component_drainings[0]
            .final_inventory
            .as_mut()
            .expect("final inventory receipt")
            .inventory_hash = [0; 32];
        RootComponentRegistryStore::import(corrupted);
        ComponentRegistryOps::component_draining(partition.binding.component)
            .expect_err("final inventory hash must remain canonical");
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one test proves every durable top-level deletion transition and ledger invariant"
    )]
    fn component_deletion_is_prepared_durable_and_absence_idempotent() {
        let (partition, draining, fleet) = import_empty_quiescent_component();
        let inventory = ComponentRegistryOps::finalize_component_inventory(
            partition.binding.component,
            draining.operation_id,
            draining.registry,
            fleet,
            112,
        )
        .expect("freeze exact final Component inventory");
        let before_preparation = RootComponentRegistryStore::export();
        ComponentRegistryOps::prepare_component_deletion(
            partition.binding.component,
            draining.operation_id,
            [0; 32],
            113,
        )
        .expect_err("deletion must bind the exact final inventory hash");
        ComponentRegistryOps::prepare_component_deletion(
            partition.binding.component,
            draining.operation_id,
            inventory.inventory_hash,
            111,
        )
        .expect_err("deletion preparation cannot precede final inventory");
        assert_eq!(RootComponentRegistryStore::export(), before_preparation);

        let prepared = ComponentRegistryOps::prepare_component_deletion(
            partition.binding.component,
            draining.operation_id,
            inventory.inventory_hash,
            113,
        )
        .expect("prepare top-level Component deletion");
        let Some(RootComponentDeletionProgressView::DeleteIntent(intent)) = &prepared.deletion
        else {
            panic!("durable top-level deletion intent");
        };
        assert_eq!(intent.final_inventory, inventory);
        assert_eq!(
            intent.quiescence.stop.canister_id,
            partition.binding.canister_id
        );
        assert_eq!(
            intent.quiescence.stop.controller,
            partition.binding.fleet_subnet_root
        );
        assert_eq!(
            intent.quiescence.observed_module_hash,
            intent.quiescence.stop.expected_module_hash
        );
        assert_eq!(intent.prepared_at_ns, 113);

        let prepared_snapshot = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::prepare_component_deletion(
                partition.binding.component,
                draining.operation_id,
                inventory.inventory_hash,
                999,
            )
            .expect("exact deletion preparation retry"),
            prepared
        );
        assert_eq!(RootComponentRegistryStore::export(), prepared_snapshot);
        ComponentRegistryOps::mark_component_deleted(
            partition.binding.component,
            draining.operation_id,
            [0; 32],
            114,
        )
        .expect_err("absence receipt must bind the prepared inventory");
        ComponentRegistryOps::mark_component_deleted(
            partition.binding.component,
            draining.operation_id,
            inventory.inventory_hash,
            112,
        )
        .expect_err("absence observation cannot precede deletion intent");
        assert_eq!(RootComponentRegistryStore::export(), prepared_snapshot);

        let deleted = ComponentRegistryOps::mark_component_deleted(
            partition.binding.component,
            draining.operation_id,
            inventory.inventory_hash,
            114,
        )
        .expect("commit independently observed top-level absence");
        let Some(RootComponentDeletionProgressView::Deleted(receipt)) = &deleted.deletion else {
            panic!("durable top-level deleted receipt");
        };
        assert_eq!(receipt.deletion, *intent);
        assert_eq!(receipt.deleted_at_ns, 114);

        let deleted_snapshot = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::mark_component_deleted(
                partition.binding.component,
                draining.operation_id,
                inventory.inventory_hash,
                999,
            )
            .expect("exact deleted receipt retry"),
            deleted
        );
        assert_eq!(RootComponentRegistryStore::export(), deleted_snapshot);
        assert_eq!(
            ComponentRegistryOps::partition(partition.binding.component)
                .expect("valid retained partition")
                .expect("retained partition"),
            partition
        );
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            exact_registry_entry_bytes(&deleted_snapshot)
        );
        assert_eq!(
            partition.encoded_bytes,
            exact_component_registry_entry_bytes(&deleted_snapshot, partition.binding.component)
        );

        let mut corrupted = deleted_snapshot;
        let RootComponentDeletionProgressRecord::Deleted(receipt) = corrupted
            .component_drainings
            .first_mut()
            .and_then(|draining| draining.deletion.as_mut())
            .expect("deleted receipt")
        else {
            panic!("deleted progress");
        };
        receipt.deleted_at_ns = receipt.deletion.prepared_at_ns - 1;
        RootComponentRegistryStore::import(corrupted);
        ComponentRegistryOps::component_draining(partition.binding.component)
            .expect_err("deleted receipt time must remain monotonic");
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one test proves the durable fence across a multi-level Component tree"
    )]
    fn subtree_removal_fence_is_durable_scoped_and_capacity_bounded() {
        let fixture = import_active_component_tree();
        let initial = RootComponentRegistryStore::export();
        let registry = component_registry_head(&fixture.partition);

        ComponentRegistryOps::reserve_child_allocation(
            child_allocation_decision_for_parent(
                &fixture.partition,
                fixture.descendant.canister_id,
                &fixture.descendant.role,
                "project_machine",
            ),
            [69; 32],
            registry.clone(),
        )
        .expect("reserve in-flight descendant");
        let before_inflight_rejection = RootComponentRegistryStore::export();
        ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [70; 32],
            fixture.target.canister_id,
            registry.clone(),
            16_777_216,
        )
        .expect_err("in-flight descendant lifecycle must prevent fencing");
        assert_eq!(
            RootComponentRegistryStore::export(),
            before_inflight_rejection
        );
        RootComponentRegistryStore::import(initial.clone());

        ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [70; 32],
            fixture.target.canister_id,
            registry.clone(),
            fixture.partition.encoded_bytes,
        )
        .expect_err("subtree fence must fit before mutation");
        assert_eq!(RootComponentRegistryStore::export(), initial);

        let fenced = ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [70; 32],
            fixture.target.canister_id,
            registry.clone(),
            16_777_216,
        )
        .expect("durably fence target subtree");
        assert_eq!(fenced.target_canister_id, fixture.target.canister_id);
        assert_eq!(
            fenced.target_parent_canister_id,
            fixture.target.parent_canister_id
        );
        assert_eq!(
            fenced.progress,
            RootComponentSubtreeRemovalProgressView::Fenced
        );
        let durable_fence = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::subtree_removal(fixture.component, [70; 32])
                .expect("valid subtree removal")
                .expect("durable subtree removal"),
            fenced
        );
        assert_eq!(
            ComponentRegistryOps::begin_subtree_removal(
                fixture.component,
                [70; 32],
                fixture.target.canister_id,
                registry.clone(),
                16_777_216,
            )
            .expect("exact fence retry"),
            fenced
        );
        let current = ComponentRegistryOps::current().expect("Registry status");
        let partition = ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition");
        assert_eq!(
            partition.encoded_bytes,
            exact_component_registry_entry_bytes(&durable_fence, fixture.component)
        );
        assert_eq!(
            current.encoded_bytes,
            exact_registry_entry_bytes(&durable_fence)
        );

        for (operation_id, parent) in [([71; 32], &fixture.target), ([72; 32], &fixture.descendant)]
        {
            let before = RootComponentRegistryStore::export();
            ComponentRegistryOps::reserve_child_allocation(
                child_allocation_decision_for_parent(
                    &fixture.partition,
                    parent.canister_id,
                    &parent.role,
                    "project_machine",
                ),
                operation_id,
                registry.clone(),
            )
            .expect_err("fenced subtree member cannot reserve a new child");
            assert_eq!(RootComponentRegistryStore::export(), before);
        }

        let before_second_fence = RootComponentRegistryStore::export();
        ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [73; 32],
            fixture.unrelated.canister_id,
            registry.clone(),
            16_777_216,
        )
        .expect_err("one Component admits only one in-progress subtree removal");
        assert_eq!(RootComponentRegistryStore::export(), before_second_fence);

        ComponentRegistryOps::reserve_child_allocation(
            child_allocation_decision_for_parent(
                &fixture.partition,
                fixture.unrelated.canister_id,
                &fixture.unrelated.role,
                "project_machine",
            ),
            [74; 32],
            registry.clone(),
        )
        .expect("unrelated branch remains mutable");
        assert_eq!(
            ComponentRegistryOps::begin_subtree_removal(
                fixture.component,
                [70; 32],
                fixture.target.canister_id,
                registry,
                16_777_216,
            )
            .expect("fence retry survives unrelated progress"),
            fenced
        );

        let before_premature_stop = RootComponentRegistryStore::export();
        ComponentRegistryOps::prepare_subtree_leaf_stop(
            fixture.component,
            [70; 32],
            0,
            fixture.target.canister_id,
            fixture.target.parent_canister_id,
            16_777_216,
        )
        .expect_err("stop preparation requires one selected childless leaf");
        assert_eq!(RootComponentRegistryStore::export(), before_premature_stop);

        let before_traversal = RootComponentRegistryStore::export();
        let before_traversal_partition = ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition");
        ComponentRegistryOps::advance_subtree_removal(
            fixture.component,
            [70; 32],
            0,
            before_traversal_partition.encoded_bytes,
        )
        .expect_err("larger traversal receipt must fit before mutation");
        assert_eq!(RootComponentRegistryStore::export(), before_traversal);

        let selected = ComponentRegistryOps::advance_subtree_removal(
            fixture.component,
            [70; 32],
            0,
            16_777_216,
        )
        .expect("select first post-order leaf");
        assert_eq!(selected.traversal_steps, 2);
        assert!(matches!(
            &selected.progress,
            RootComponentSubtreeRemovalProgressView::LeafSelected { leaf }
                if leaf.canister_id == fixture.descendant.canister_id
                    && leaf.parent_canister_id == fixture.target.canister_id
        ));
        let selected_state = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::advance_subtree_removal(
                fixture.component,
                [70; 32],
                0,
                16_777_216,
            )
            .expect("stale traversal retry returns current progress"),
            selected
        );
        assert_eq!(RootComponentRegistryStore::export(), selected_state);

        let selected_partition = ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition");
        ComponentRegistryOps::prepare_subtree_leaf_stop(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            selected_partition.encoded_bytes,
        )
        .expect_err("larger stop intent must fit before mutation");
        assert_eq!(RootComponentRegistryStore::export(), selected_state);

        for (steps, leaf, parent) in [
            (
                selected.traversal_steps + 1,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
            ),
            (
                selected.traversal_steps,
                fixture.target.canister_id,
                fixture.target.parent_canister_id,
            ),
            (
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.parent_canister_id,
            ),
        ] {
            ComponentRegistryOps::prepare_subtree_leaf_stop(
                fixture.component,
                [70; 32],
                steps,
                leaf,
                parent,
                16_777_216,
            )
            .expect_err("stop preparation must bind the exact selected leaf observation");
            assert_eq!(RootComponentRegistryStore::export(), selected_state);
        }

        let prepared = ComponentRegistryOps::prepare_subtree_leaf_stop(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            16_777_216,
        )
        .expect("freeze exact leaf stop intent");
        assert!(matches!(
            &prepared.progress,
            RootComponentSubtreeRemovalProgressView::StopIntent(effect)
                if effect.leaf.canister_id == fixture.descendant.canister_id
                    && effect.leaf.parent_canister_id == fixture.target.canister_id
                    && effect.leaf.installed_artifact_hash
                        == fixture.descendant.installed_artifact_hash
                    && effect.controller == fixture.partition.binding.fleet_subnet_root
        ));
        let prepared_state = restart_component_registry();
        let mut corrupted_controller = prepared_state.clone();
        let RootComponentSubtreeRemovalProgressRecord::StopIntent(effect) =
            &mut corrupted_controller.subtree_removals[0].progress
        else {
            panic!("durable stop intent");
        };
        effect.controller = Principal::from_slice(&[99; 29]);
        RootComponentRegistryStore::import(corrupted_controller);
        ComponentRegistryOps::subtree_removal(fixture.component, [70; 32])
            .expect_err("stop intent must retain the exact protected root controller");
        RootComponentRegistryStore::import(prepared_state.clone());
        ComponentRegistryOps::prepare_subtree_leaf_stop(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.parent_canister_id,
            16_777_216,
        )
        .expect_err("durable stop intent rejects conflicting parent observation");
        assert_eq!(RootComponentRegistryStore::export(), prepared_state);
        assert_eq!(
            ComponentRegistryOps::prepare_subtree_leaf_stop(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                16_777_216,
            )
            .expect("exact stop preparation retry"),
            prepared
        );
        assert_eq!(RootComponentRegistryStore::export(), prepared_state);
        assert_eq!(
            ComponentRegistryOps::advance_subtree_removal(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                16_777_216,
            )
            .expect("traversal retry converges on stop intent"),
            prepared
        );
        assert_eq!(RootComponentRegistryStore::export(), prepared_state);

        let prepared_partition = ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition");
        ComponentRegistryOps::mark_subtree_leaf_stopped(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            [55; 32],
            prepared_partition.encoded_bytes,
        )
        .expect_err("larger stopped receipt must fit before mutation");
        assert_eq!(RootComponentRegistryStore::export(), prepared_state);

        let stopped = ComponentRegistryOps::mark_subtree_leaf_stopped(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            [55; 32],
            16_777_216,
        )
        .expect("commit independently observed stopped receipt");
        assert!(matches!(
            &stopped.progress,
            RootComponentSubtreeRemovalProgressView::Stopped(receipt)
                if receipt.stop.leaf.canister_id == fixture.descendant.canister_id
                    && receipt.stop.leaf.parent_canister_id == fixture.target.canister_id
                    && receipt.stop.controller == fixture.partition.binding.fleet_subnet_root
                    && receipt.observed_module_hash == [55; 32]
        ));
        let stopped_state = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::mark_subtree_leaf_stopped(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                [55; 32],
                16_777_216,
            )
            .expect("exact stopped receipt retry"),
            stopped
        );
        assert_eq!(RootComponentRegistryStore::export(), stopped_state);
        ComponentRegistryOps::mark_subtree_leaf_stopped(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            [56; 32],
            16_777_216,
        )
        .expect_err("stopped receipt rejects conflicting module observation");
        assert_eq!(RootComponentRegistryStore::export(), stopped_state);
        assert_eq!(
            ComponentRegistryOps::prepare_subtree_leaf_stop(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                16_777_216,
            )
            .expect("stop preparation converges on stopped receipt"),
            stopped
        );
        assert_eq!(RootComponentRegistryStore::export(), stopped_state);

        ComponentRegistryOps::prepare_subtree_leaf_delete(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            1,
        )
        .expect_err("deletion intent must fit before mutation");
        assert_eq!(RootComponentRegistryStore::export(), stopped_state);
        ComponentRegistryOps::prepare_subtree_leaf_delete(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.parent_canister_id,
            16_777_216,
        )
        .expect_err("deletion intent rejects a conflicting parent observation");
        assert_eq!(RootComponentRegistryStore::export(), stopped_state);

        let deletion = ComponentRegistryOps::prepare_subtree_leaf_delete(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            16_777_216,
        )
        .expect("freeze exact stopped receipt as deletion intent");
        assert!(matches!(
            &deletion.progress,
            RootComponentSubtreeRemovalProgressView::DeleteIntent(intent)
                if intent.stopped.stop.leaf.canister_id == fixture.descendant.canister_id
                    && intent.stopped.stop.leaf.parent_canister_id
                        == fixture.target.canister_id
                    && intent.stopped.stop.controller
                        == fixture.partition.binding.fleet_subnet_root
                    && intent.stopped.observed_module_hash == [55; 32]
        ));
        let deletion_state = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::prepare_subtree_leaf_delete(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                16_777_216,
            )
            .expect("exact deletion preparation retry"),
            deletion
        );
        assert_eq!(RootComponentRegistryStore::export(), deletion_state);

        ComponentRegistryOps::mark_subtree_leaf_deleted(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            1,
        )
        .expect_err("deleted receipt must fit before mutation");
        assert_eq!(RootComponentRegistryStore::export(), deletion_state);

        let deleted = ComponentRegistryOps::mark_subtree_leaf_deleted(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            16_777_216,
        )
        .expect("commit independently observed deleted receipt");
        assert!(matches!(
            &deleted.progress,
            RootComponentSubtreeRemovalProgressView::Deleted(receipt)
                if receipt.deletion.stopped.stop.leaf.canister_id
                    == fixture.descendant.canister_id
                    && receipt.deletion.stopped.observed_module_hash == [55; 32]
        ));
        let deleted_state = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::mark_subtree_leaf_deleted(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                16_777_216,
            )
            .expect("exact deleted receipt retry"),
            deleted
        );
        assert_eq!(RootComponentRegistryStore::export(), deleted_state);
        assert_eq!(
            ComponentRegistryOps::mark_subtree_leaf_stopped(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                [55; 32],
                16_777_216,
            )
            .expect("stale stopped retry converges on deleted receipt"),
            deleted
        );
        assert_eq!(RootComponentRegistryStore::export(), deleted_state);
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            exact_registry_entry_bytes(&deleted_state)
        );
        assert_eq!(
            ComponentRegistryOps::partition(fixture.component)
                .expect("partition read")
                .expect("active partition")
                .encoded_bytes,
            exact_component_registry_entry_bytes(&deleted_state, fixture.component)
        );

        let deleted_partition = ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition");
        let active_fleet_directory = fleet_directory(&root_binding());
        ComponentRegistryOps::remove_subtree_leaf_membership(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.parent_canister_id,
            deleted_partition.directory_synchronized_at_ns + 1,
            16_777_216,
            active_fleet_directory.clone(),
        )
        .expect_err("membership removal rejects a conflicting parent observation");
        assert_eq!(RootComponentRegistryStore::export(), deleted_state);
        ComponentRegistryOps::remove_subtree_leaf_membership(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            deleted_partition.directory_synchronized_at_ns + 1,
            1,
            active_fleet_directory.clone(),
        )
        .expect_err("membership-removal receipt must fit before mutation");
        assert_eq!(RootComponentRegistryStore::export(), deleted_state);

        let membership_removed = ComponentRegistryOps::remove_subtree_leaf_membership(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            deleted_partition.directory_synchronized_at_ns + 1,
            16_777_216,
            active_fleet_directory.clone(),
        )
        .expect("atomically remove deleted leaf membership");
        let RootComponentSubtreeRemovalProgressView::MembershipRemoved(receipt) =
            &membership_removed.progress
        else {
            panic!("durable membership-removal receipt");
        };
        assert_eq!(
            receipt.deleted.deletion.stopped.observed_module_hash,
            [55; 32]
        );
        assert_eq!(
            receipt.removed_from_registry,
            ComponentRegistryHead {
                component: fixture.component,
                revision: deleted_partition.revision,
                content_hash: deleted_partition.content_hash,
            }
        );
        assert_eq!(receipt.previous_committed_descendants, 4);
        assert_eq!(receipt.committed_descendants, 3);
        assert_eq!(receipt.reserved_descendants, 1);
        assert_eq!(receipt.parent_role_instances, 0);
        assert_eq!(receipt.root_managed_descendants, 4);
        assert_eq!(receipt.root_known_created_component_canisters, 4);
        assert_eq!(
            receipt.directory_authority_hash,
            component_directory_authority_hash(
                &deleted_partition.binding,
                receipt.registry.revision,
                receipt.registry.content_hash,
                receipt.directory_synchronized_at_ns,
                receipt.committed_descendants,
                &active_fleet_directory,
            )
            .expect("membership-removal Directory authority hash")
        );
        assert_eq!(
            RootComponentRegistryStore::child(fixture.component, fixture.descendant.canister_id),
            None
        );
        assert_eq!(
            RootComponentRegistryStore::child_traversal(
                fixture.component,
                fixture.target.canister_id,
                &fixture.descendant.role,
                fixture.descendant.canister_id,
            ),
            None
        );
        assert_eq!(
            RootComponentRegistryStore::component_for_principal(fixture.descendant.canister_id),
            None
        );
        assert_eq!(
            ComponentRegistryOps::parent_role_instances(
                fixture.component,
                fixture.target.canister_id,
                &fixture.descendant.role,
            )
            .expect("parent-role count"),
            0
        );
        let membership_removed_state = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::subtree_removal(fixture.component, [70; 32])
                .expect("valid membership-removal receipt")
                .expect("durable membership-removal receipt"),
            membership_removed
        );
        assert_eq!(
            ComponentRegistryOps::remove_subtree_leaf_membership(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                deleted_partition.directory_synchronized_at_ns + 1,
                16_777_216,
                active_fleet_directory.clone(),
            )
            .expect("exact membership-removal retry"),
            membership_removed
        );
        assert_eq!(
            RootComponentRegistryStore::export(),
            membership_removed_state
        );
        assert_eq!(
            ComponentRegistryOps::mark_subtree_leaf_deleted(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                16_777_216,
            )
            .expect("stale deleted retry converges on membership-removal receipt"),
            membership_removed
        );
        let current = ComponentRegistryOps::current().expect("Registry status");
        let partition = ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition");
        assert_eq!(current.managed_descendants, 4);
        assert_eq!(current.known_created_component_canisters, 4);
        assert_eq!(partition.committed_descendants, 3);
        assert_eq!(partition.encoded_bytes, receipt.registry_encoded_bytes);
        assert_eq!(
            current.encoded_bytes,
            exact_registry_entry_bytes(&membership_removed_state)
        );
        assert_eq!(
            partition.encoded_bytes,
            exact_component_registry_entry_bytes(&membership_removed_state, fixture.component)
        );

        let directory_authority = ComponentRuntimeDirectoryAuthority {
            fleet: active_fleet_directory,
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: partition.binding.clone(),
                    source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                    component_registry_revision: partition.revision,
                    component_registry_content_hash: partition.content_hash,
                    synchronized_at_ns: partition.directory_synchronized_at_ns,
                },
                descendant_count: partition.committed_descendants,
            },
        };
        let directory_authority_hash =
            ComponentRuntimeOps::directory_authority_hash(&directory_authority)
                .expect("current Directory authority hash");
        let owning_component = ComponentRuntimeDirectoryConvergenceEvidence {
            operation_id: [81; 32],
            binding: ManagedCanisterBinding::Component(partition.binding.clone()),
            covered_authority: directory_authority.clone(),
            covered_authority_hash: directory_authority_hash,
            activation: ComponentRuntimeActivationEvidence {
                directory_authority_hash: [82; 32],
                activated_at_ns: 83,
            },
        };
        let parent_binding =
            ComponentRegistryOps::registered_parent(fixture.component, fixture.target.canister_id)
                .expect("registered parent read")
                .expect("registered parent")
                .0;
        let parent = ComponentRuntimeDirectoryConvergenceEvidence {
            operation_id: [84; 32],
            binding: parent_binding,
            covered_authority: directory_authority.clone(),
            covered_authority_hash: directory_authority_hash,
            activation: ComponentRuntimeActivationEvidence {
                directory_authority_hash: [85; 32],
                activated_at_ns: 86,
            },
        };
        ComponentRegistryOps::mark_subtree_leaf_directory_synchronized(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            directory_authority.clone(),
            directory_authority_hash,
            Some(owning_component.clone()),
            Some(parent.clone()),
            1,
        )
        .expect_err("Directory convergence receipt must fit before mutation");
        assert_eq!(
            RootComponentRegistryStore::export(),
            membership_removed_state
        );

        let directory_synchronized =
            ComponentRegistryOps::mark_subtree_leaf_directory_synchronized(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                directory_authority.clone(),
                directory_authority_hash,
                Some(owning_component.clone()),
                Some(parent.clone()),
                16_777_216,
            )
            .expect("retain surviving-member Directory convergence");
        assert!(matches!(
            &directory_synchronized.progress,
            RootComponentSubtreeRemovalProgressView::DirectorySynchronized(receipt)
                if receipt.membership_removed.registry == receipt.covered_component_registry
                    && receipt.owning_component.as_ref().map(|evidence| evidence.canister_id)
                        == Some(partition.binding.canister_id)
                    && receipt.parent.as_ref().map(|evidence| evidence.canister_id)
                        == Some(fixture.target.canister_id)
                    && receipt.covered_authority_hash == directory_authority_hash
        ));
        let directory_synchronized_state = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::mark_subtree_leaf_directory_synchronized(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                directory_authority.clone(),
                directory_authority_hash,
                Some(owning_component.clone()),
                Some(parent.clone()),
                16_777_216,
            )
            .expect("exact Directory synchronization retry"),
            directory_synchronized
        );
        assert_eq!(
            RootComponentRegistryStore::export(),
            directory_synchronized_state
        );
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            exact_registry_entry_bytes(&directory_synchronized_state)
        );
        assert_eq!(
            ComponentRegistryOps::partition(fixture.component)
                .expect("partition read")
                .expect("active partition")
                .encoded_bytes,
            exact_component_registry_entry_bytes(&directory_synchronized_state, fixture.component)
        );

        let resumed = ComponentRegistryOps::finalize_subtree_leaf(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            directory_synchronized_state.partitions[0].encoded_bytes,
        )
        .expect("compact completed leaf and resume within the existing byte ceiling");
        assert_eq!(resumed.completed_leaves, 1);
        assert_eq!(resumed.maximum_completed_leaves, 4);
        assert!(matches!(
            &resumed.progress,
            RootComponentSubtreeRemovalProgressView::Traversing { cursor }
                if cursor.canister_id == fixture.target.canister_id
        ));
        let resumed_state = restart_component_registry();
        assert_eq!(resumed_state.subtree_removal_history.len(), 1);
        assert!(
            resumed_state.partitions[0].encoded_bytes
                <= directory_synchronized_state.partitions[0].encoded_bytes,
            "normalization must not require more Component Registry capacity"
        );
        let RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(
            expected_history_receipt,
        ) = &directory_synchronized_state.subtree_removals[0].progress
        else {
            panic!("Directory-synchronized history source");
        };
        let history = &resumed_state.subtree_removal_history[0];
        assert_eq!(history.leaf_canister_id, fixture.descendant.canister_id);
        assert_eq!(history.leaf_parent_canister_id, fixture.target.canister_id);
        assert_eq!(history.observed_module_hash, [55; 32]);
        assert_eq!(
            history.receipt_hash,
            subtree_directory_synchronized_receipt_hash(expected_history_receipt)
                .expect("canonical completed-leaf receipt hash")
        );
        assert!(
            RootComponentRegistryStore::subtree_removal_completed_leaf_entry_bytes(history)
                < RootComponentRegistryStore::subtree_removal_entry_bytes(
                    &directory_synchronized_state.subtree_removals[0]
                ),
            "normalized leaf history must remain smaller than its live source receipt"
        );
        assert_eq!(
            ComponentRegistryOps::finalize_subtree_leaf(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                16_777_216,
            )
            .expect("exact leaf-finalization retry"),
            resumed
        );
        assert_eq!(RootComponentRegistryStore::export(), resumed_state);
        assert_eq!(
            ComponentRegistryOps::prepare_subtree_leaf_stop(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                16_777_216,
            )
            .expect("stale stop preparation resolves through completed history"),
            resumed
        );
        assert_eq!(
            ComponentRegistryOps::mark_subtree_leaf_stopped(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                [55; 32],
                16_777_216,
            )
            .expect("stale stopped receipt resolves through completed history"),
            resumed
        );
        assert_eq!(
            ComponentRegistryOps::mark_subtree_leaf_directory_synchronized(
                fixture.component,
                [70; 32],
                selected.traversal_steps,
                fixture.descendant.canister_id,
                fixture.target.canister_id,
                directory_authority,
                directory_authority_hash,
                Some(owning_component),
                Some(parent),
                16_777_216,
            )
            .expect("stale Directory synchronization resolves through completed history"),
            resumed
        );
        assert_eq!(RootComponentRegistryStore::export(), resumed_state);
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            exact_registry_entry_bytes(&resumed_state)
        );
        assert_eq!(
            ComponentRegistryOps::partition(fixture.component)
                .expect("partition read")
                .expect("active partition")
                .encoded_bytes,
            exact_component_registry_entry_bytes(&resumed_state, fixture.component)
        );

        let next_selected = ComponentRegistryOps::advance_subtree_removal(
            fixture.component,
            [70; 32],
            resumed.traversal_steps,
            16_777_216,
        )
        .expect("select retained parent after completed child");
        assert!(next_selected.traversal_steps > resumed.traversal_steps);
        assert!(matches!(
            &next_selected.progress,
            RootComponentSubtreeRemovalProgressView::LeafSelected { leaf }
                if leaf.canister_id != fixture.target.canister_id
                    && leaf.parent_canister_id == fixture.target.canister_id
        ));
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    fn subtree_removal_traversal_is_bounded_deterministic_and_restart_safe() {
        let (fixture, expected_leaf) = import_deep_active_component_tree(65);
        let registry = component_registry_head(&fixture.partition);
        ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [80; 32],
            fixture.target.canister_id,
            registry,
            16_777_216,
        )
        .expect("fence deep subtree");

        let traversing = ComponentRegistryOps::advance_subtree_removal(
            fixture.component,
            [80; 32],
            0,
            16_777_216,
        )
        .expect("advance one bounded traversal batch");
        assert_eq!(
            traversing.traversal_steps,
            SUBTREE_REMOVAL_TRAVERSAL_BATCH_SIZE
        );
        assert!(matches!(
            &traversing.progress,
            RootComponentSubtreeRemovalProgressView::Traversing { cursor }
                if cursor.canister_id != expected_leaf
        ));

        let durable_midpoint = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::advance_subtree_removal(
                fixture.component,
                [80; 32],
                0,
                16_777_216,
            )
            .expect("stale traversal retry"),
            traversing
        );
        assert_eq!(RootComponentRegistryStore::export(), durable_midpoint);

        let selected = ComponentRegistryOps::advance_subtree_removal(
            fixture.component,
            [80; 32],
            traversing.traversal_steps,
            16_777_216,
        )
        .expect("resume traversal and select deepest leaf");
        assert_eq!(selected.traversal_steps, 67);
        assert!(matches!(
            &selected.progress,
            RootComponentSubtreeRemovalProgressView::LeafSelected { leaf }
                if leaf.canister_id == expected_leaf
        ));
        let terminal = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::subtree_removal(fixture.component, [80; 32])
                .expect("valid traversal status")
                .expect("durable traversal status"),
            selected
        );
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            exact_registry_entry_bytes(&terminal)
        );
        assert_eq!(
            ComponentRegistryOps::partition(fixture.component)
                .expect("partition read")
                .expect("active partition")
                .encoded_bytes,
            exact_component_registry_entry_bytes(&terminal, fixture.component)
        );
        let before_ahead_rejection = RootComponentRegistryStore::export();
        ComponentRegistryOps::advance_subtree_removal(
            fixture.component,
            [80; 32],
            selected.traversal_steps + 1,
            16_777_216,
        )
        .expect_err("future traversal expectation must fail");
        assert_eq!(RootComponentRegistryStore::export(), before_ahead_rejection);
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one test proves the complete durable Component draining fence"
    )]
    fn component_draining_fence_is_durable_capacity_bounded_and_stops_growth() {
        let fixture = import_active_component_tree();
        let initial = RootComponentRegistryStore::export();
        let previous_registry = component_registry_head(&fixture.partition);

        ComponentRegistryOps::reserve_child_allocation(
            child_allocation_decision_for_parent(
                &fixture.partition,
                fixture.unrelated.canister_id,
                &fixture.unrelated.role,
                "project_machine",
            ),
            [77; 32],
            previous_registry.clone(),
        )
        .expect("reserve in-flight descendant");
        let before_inflight_rejection = RootComponentRegistryStore::export();
        ComponentRegistryOps::begin_component_draining(
            fixture.component,
            [79; 32],
            previous_registry.clone(),
            100,
            16_777_216,
            fleet_directory(
                &before_inflight_rejection
                    .current
                    .as_ref()
                    .expect("Registry meta")
                    .root,
            ),
        )
        .expect_err("in-flight child lifecycle must prevent draining");
        assert_eq!(
            RootComponentRegistryStore::export(),
            before_inflight_rejection
        );
        RootComponentRegistryStore::import(initial.clone());

        ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [78; 32],
            fixture.target.canister_id,
            previous_registry.clone(),
            16_777_216,
        )
        .expect("fence in-progress subtree removal");
        let before_removal_rejection = RootComponentRegistryStore::export();
        ComponentRegistryOps::begin_component_draining(
            fixture.component,
            [79; 32],
            previous_registry.clone(),
            100,
            16_777_216,
            fleet_directory(
                &before_removal_rejection
                    .current
                    .as_ref()
                    .expect("Registry meta")
                    .root,
            ),
        )
        .expect_err("in-progress subtree removal must prevent draining");
        assert_eq!(
            RootComponentRegistryStore::export(),
            before_removal_rejection
        );
        RootComponentRegistryStore::import(initial.clone());

        ComponentRegistryOps::begin_component_draining(
            fixture.component,
            [79; 32],
            previous_registry.clone(),
            100,
            fixture.partition.encoded_bytes,
            fleet_directory(&initial.current.as_ref().expect("Registry meta").root),
        )
        .expect_err("draining receipt must fit before mutation");
        assert_eq!(RootComponentRegistryStore::export(), initial);

        let draining = ComponentRegistryOps::begin_component_draining(
            fixture.component,
            [79; 32],
            previous_registry.clone(),
            100,
            16_777_216,
            fleet_directory(&initial.current.as_ref().expect("Registry meta").root),
        )
        .expect("durably fence Component growth");
        assert_eq!(draining.previous_registry, previous_registry);
        assert_eq!(draining.registry.revision, fixture.partition.revision + 1);
        assert_eq!(draining.descendant_count, 4);
        assert_eq!(
            draining.descendant_content_hash,
            fixture.partition.descendant_content_hash
        );
        assert_ne!(draining.directory_authority_hash, [0; 32]);

        let durable = restart_component_registry();
        let partition = ComponentRegistryOps::partition(fixture.component)
            .expect("valid draining partition")
            .expect("draining partition");
        assert_eq!(partition.status, ComponentLifecycleStatus::Draining);
        assert_eq!(partition.revision, draining.registry.revision);
        assert_eq!(partition.content_hash, draining.registry.content_hash);
        assert_eq!(
            ComponentRegistryOps::component_for_principal(fixture.partition.binding.canister_id),
            Some(fixture.component)
        );
        assert_eq!(
            ComponentRegistryOps::component_draining(fixture.component)
                .expect("valid draining receipt")
                .expect("durable draining receipt"),
            draining
        );
        let mut corrupted = durable.clone();
        corrupted.component_drainings[0].descendant_content_hash = [0; 32];
        RootComponentRegistryStore::import(corrupted);
        ComponentRegistryOps::component_draining(fixture.component)
            .expect_err("draining receipt must retain its canonical descendant digest");
        RootComponentRegistryStore::import(durable.clone());
        assert_eq!(
            ComponentRegistryOps::begin_component_draining(
                fixture.component,
                [79; 32],
                draining.previous_registry.clone(),
                101,
                16_777_216,
                fleet_directory(&initial.current.as_ref().expect("Registry meta").root),
            )
            .expect("exact retry"),
            draining
        );
        assert_eq!(
            partition.encoded_bytes,
            exact_component_registry_entry_bytes(&durable, fixture.component)
        );
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            exact_registry_entry_bytes(&durable)
        );

        let before_growth = RootComponentRegistryStore::export();
        ComponentRegistryOps::reserve_child_allocation(
            child_allocation_decision_for_parent(
                &fixture.partition,
                fixture.unrelated.canister_id,
                &fixture.unrelated.role,
                "project_machine",
            ),
            [80; 32],
            draining.registry.clone(),
        )
        .expect_err("Draining Component cannot reserve a new child");
        assert_eq!(RootComponentRegistryStore::export(), before_growth);

        ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [81; 32],
            fixture.target.canister_id,
            draining.registry.clone(),
            16_777_216,
        )
        .expect_err("Draining Component must be quiescent before post-order removal");
        assert_eq!(RootComponentRegistryStore::export(), before_growth);

        let active_fleet_directory =
            fleet_directory(&initial.current.as_ref().expect("Registry meta").root);
        let directory_authority = ComponentRuntimeDirectoryAuthority {
            fleet: active_fleet_directory,
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: partition.binding.clone(),
                    source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                    component_registry_revision: partition.revision,
                    component_registry_content_hash: partition.content_hash,
                    synchronized_at_ns: partition.directory_synchronized_at_ns,
                },
                descendant_count: partition.committed_descendants,
            },
        };
        let authority_hash = ComponentRuntimeOps::directory_authority_hash(&directory_authority)
            .expect("draining Directory authority hash");
        let convergence = ComponentRuntimeDirectoryConvergenceEvidence {
            operation_id: [82; 32],
            binding: ManagedCanisterBinding::Component(partition.binding.clone()),
            covered_authority: directory_authority,
            covered_authority_hash: authority_hash,
            activation: ComponentRuntimeActivationEvidence {
                directory_authority_hash: [83; 32],
                activated_at_ns: 84,
            },
        };
        ComponentRegistryOps::prepare_component_quiescence(
            fixture.component,
            [79; 32],
            draining.registry.clone(),
            convergence.clone(),
            [85; 32],
            110,
            partition.encoded_bytes,
        )
        .expect_err("terminal quiescence receipt must fit before the stop intent is committed");
        assert_eq!(RootComponentRegistryStore::export(), before_growth);

        let prepared = ComponentRegistryOps::prepare_component_quiescence(
            fixture.component,
            [79; 32],
            draining.registry.clone(),
            convergence,
            [85; 32],
            110,
            16_777_216,
        )
        .expect("durably prepare qualified Component stop");
        assert!(matches!(
            &prepared.quiescence,
            Some(RootComponentQuiescenceProgressView::StopIntent(intent))
                if intent.registry == draining.registry
                    && intent.canister_id == fixture.partition.binding.canister_id
                    && intent.controller == fixture.partition.binding.fleet_subnet_root
                    && intent.expected_module_hash == [85; 32]
                    && intent.covered_authority_hash == authority_hash
        ));
        let prepared_state = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::partition(fixture.component)
                .expect("partition read")
                .expect("draining partition")
                .encoded_bytes,
            exact_component_registry_entry_bytes(&prepared_state, fixture.component)
        );
        let mut corrupted_reservation = prepared_state.clone();
        let Some(RootComponentQuiescenceProgressRecord::StopIntent(intent)) =
            &mut corrupted_reservation.component_drainings[0].quiescence
        else {
            panic!("durable Component stop intent");
        };
        intent.charged_entry_bytes += 1;
        RootComponentRegistryStore::import(corrupted_reservation);
        ComponentRegistryOps::component_draining(fixture.component)
            .expect_err("quiescence terminal byte reservation must remain canonical");
        RootComponentRegistryStore::import(prepared_state);
        let before_observation_rejection = RootComponentRegistryStore::export();
        ComponentRegistryOps::mark_component_quiescent(fixture.component, [79; 32], [86; 32], 110)
            .expect_err("observed module must match the durable stop intent");
        assert_eq!(
            RootComponentRegistryStore::export(),
            before_observation_rejection
        );
        let quiescent = ComponentRegistryOps::mark_component_quiescent(
            fixture.component,
            [79; 32],
            [85; 32],
            110,
        )
        .expect("commit independently observed Component quiescence");
        assert!(matches!(
            &quiescent.quiescence,
            Some(RootComponentQuiescenceProgressView::Quiescent(receipt))
                if receipt.stop.canister_id == fixture.partition.binding.canister_id
                    && receipt.observed_module_hash == [85; 32]
                    && receipt.quiesced_at_ns == 110
        ));
        let quiescent_state = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::mark_component_quiescent(
                fixture.component,
                [79; 32],
                [85; 32],
                111,
            )
            .expect("terminal quiescence retry"),
            quiescent
        );
        assert_eq!(RootComponentRegistryStore::export(), quiescent_state);

        ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [81; 32],
            fixture.target.canister_id,
            draining.registry.clone(),
            16_777_216,
        )
        .expect_err("caller-selected subtree removal must remain Active-only");
        assert_eq!(RootComponentRegistryStore::export(), quiescent_state);

        let pending = ComponentRegistryOps::advance_component_draining(
            fixture.component,
            draining.operation_id,
        )
        .expect("derive the first draining subtree");
        let pending_state = restart_component_registry();
        let restarted = ComponentRegistryOps::advance_component_draining(
            fixture.component,
            draining.operation_id,
        )
        .expect("derive the same draining subtree after restart");
        assert_eq!(RootComponentRegistryStore::export(), pending_state);
        let (
            RootComponentDrainingAdvanceView::DescendantSubtreePending {
                operation_id,
                target_canister_id,
                reserved_against_registry,
            },
            RootComponentDrainingAdvanceView::DescendantSubtreePending {
                operation_id: restarted_operation_id,
                target_canister_id: restarted_target,
                reserved_against_registry: restarted_registry,
            },
        ) = (pending, restarted)
        else {
            panic!("draining driver must select an unfenced direct subtree");
        };
        assert_eq!(target_canister_id, fixture.target.canister_id);
        assert_eq!(restarted_operation_id, operation_id);
        assert_eq!(restarted_target, target_canister_id);
        assert_eq!(restarted_registry, reserved_against_registry);

        let fenced = ComponentRegistryOps::begin_draining_subtree_removal(
            fixture.component,
            draining.operation_id,
            16_777_216,
        )
        .expect("driver fences its deterministic direct subtree");
        assert_eq!(fenced.operation_id, operation_id);
        assert_eq!(fenced.target_canister_id, target_canister_id);
        assert!(matches!(
            ComponentRegistryOps::advance_component_draining(
                fixture.component,
                draining.operation_id,
            )
            .expect("read the durable draining cursor"),
            RootComponentDrainingAdvanceView::DescendantRemoval(removal)
                if *removal == fenced
        ));
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one terminal-path test retains the complete monotonic removal setup and assertions"
    )]
    fn subtree_removal_target_finalization_is_terminal_and_releases_the_live_fence() {
        let fixture = import_active_component_tree();
        let operation_id = [91; 32];
        let initial = RootComponentRegistryStore::export();
        let draining = ComponentRegistryOps::begin_component_draining(
            fixture.component,
            [90; 32],
            component_registry_head(&fixture.partition),
            100,
            16_777_216,
            fleet_directory(&initial.current.as_ref().expect("Registry meta").root),
        )
        .expect("drain Component before terminal target removal");
        let partition = ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("draining partition");
        let directory_authority = ComponentRuntimeDirectoryAuthority {
            fleet: fleet_directory(&initial.current.as_ref().expect("Registry meta").root),
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: partition.binding.clone(),
                    source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                    component_registry_revision: partition.revision,
                    component_registry_content_hash: partition.content_hash,
                    synchronized_at_ns: partition.directory_synchronized_at_ns,
                },
                descendant_count: partition.committed_descendants,
            },
        };
        let directory_authority_hash =
            ComponentRuntimeOps::directory_authority_hash(&directory_authority)
                .expect("draining Directory authority");
        ComponentRegistryOps::prepare_component_quiescence(
            fixture.component,
            [90; 32],
            draining.registry.clone(),
            ComponentRuntimeDirectoryConvergenceEvidence {
                operation_id: [89; 32],
                binding: ManagedCanisterBinding::Component(partition.binding),
                covered_authority: directory_authority,
                covered_authority_hash: directory_authority_hash,
                activation: ComponentRuntimeActivationEvidence {
                    directory_authority_hash: [88; 32],
                    activated_at_ns: 87,
                },
            },
            [86; 32],
            101,
            16_777_216,
        )
        .expect("prepare Component quiescence");
        ComponentRegistryOps::mark_component_quiescent(fixture.component, [90; 32], [86; 32], 101)
            .expect("observe Component quiescent");
        let registry = draining.registry;
        ComponentRegistryOps::begin_subtree_removal_with_origin(
            fixture.component,
            operation_id,
            fixture.unrelated.canister_id,
            registry,
            16_777_216,
            SubtreeRemovalOrigin::DrainingDriver,
        )
        .expect("fence a direct leaf through the draining removal primitive");
        let selected = ComponentRegistryOps::advance_subtree_removal(
            fixture.component,
            operation_id,
            0,
            16_777_216,
        )
        .expect("select direct leaf target");
        assert!(matches!(
            &selected.progress,
            RootComponentSubtreeRemovalProgressView::LeafSelected { leaf }
                if leaf.canister_id == fixture.unrelated.canister_id
        ));

        ComponentRegistryOps::prepare_subtree_leaf_stop(
            fixture.component,
            operation_id,
            selected.traversal_steps,
            fixture.unrelated.canister_id,
            fixture.unrelated.parent_canister_id,
            16_777_216,
        )
        .expect("prepare target stop");
        ComponentRegistryOps::mark_subtree_leaf_stopped(
            fixture.component,
            operation_id,
            selected.traversal_steps,
            fixture.unrelated.canister_id,
            fixture.unrelated.parent_canister_id,
            [92; 32],
            16_777_216,
        )
        .expect("observe target stopped");
        ComponentRegistryOps::prepare_subtree_leaf_delete(
            fixture.component,
            operation_id,
            selected.traversal_steps,
            fixture.unrelated.canister_id,
            fixture.unrelated.parent_canister_id,
            16_777_216,
        )
        .expect("prepare target deletion");
        ComponentRegistryOps::mark_subtree_leaf_deleted(
            fixture.component,
            operation_id,
            selected.traversal_steps,
            fixture.unrelated.canister_id,
            fixture.unrelated.parent_canister_id,
            16_777_216,
        )
        .expect("observe target deleted");
        let before_membership = ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition");
        let active_fleet_directory = fleet_directory(&root_binding());
        ComponentRegistryOps::remove_subtree_leaf_membership(
            fixture.component,
            operation_id,
            selected.traversal_steps,
            fixture.unrelated.canister_id,
            fixture.unrelated.parent_canister_id,
            before_membership.directory_synchronized_at_ns + 1,
            16_777_216,
            active_fleet_directory.clone(),
        )
        .expect("remove target membership");
        assert!(matches!(
            ComponentRegistryOps::advance_component_draining(fixture.component, [90; 32])
                .expect("retain the removal cursor after target membership removal"),
            RootComponentDrainingAdvanceView::DescendantRemoval(removal)
                if removal.operation_id == operation_id
                    && matches!(
                        &removal.progress,
                        RootComponentSubtreeRemovalProgressView::MembershipRemoved(_)
                    )
        ));

        let partition = ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition");
        let directory_authority = ComponentRuntimeDirectoryAuthority {
            fleet: active_fleet_directory,
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: partition.binding.clone(),
                    source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                    component_registry_revision: partition.revision,
                    component_registry_content_hash: partition.content_hash,
                    synchronized_at_ns: partition.directory_synchronized_at_ns,
                },
                descendant_count: partition.committed_descendants,
            },
        };
        let directory_authority_hash =
            ComponentRuntimeOps::directory_authority_hash(&directory_authority)
                .expect("target Directory authority hash");
        let synchronized = ComponentRegistryOps::mark_subtree_leaf_directory_synchronized(
            fixture.component,
            operation_id,
            selected.traversal_steps,
            fixture.unrelated.canister_id,
            fixture.unrelated.parent_canister_id,
            directory_authority,
            directory_authority_hash,
            None,
            None,
            16_777_216,
        )
        .expect("retain local Directory authority without calling the quiescent owner");
        assert!(matches!(
            synchronized.progress,
            RootComponentSubtreeRemovalProgressView::DirectorySynchronized(receipt)
                if receipt.owning_component.is_none() && receipt.parent.is_none()
        ));

        let completed = ComponentRegistryOps::finalize_subtree_leaf(
            fixture.component,
            operation_id,
            selected.traversal_steps,
            fixture.unrelated.canister_id,
            fixture.unrelated.parent_canister_id,
            16_777_216,
        )
        .expect("finalize fenced target");
        assert_eq!(completed.completed_leaves, 1);
        assert!(matches!(
            &completed.progress,
            RootComponentSubtreeRemovalProgressView::Completed(receipt)
                if receipt.registry.component == fixture.component
                    && receipt.directory_authority_hash == directory_authority_hash
        ));
        let completed_state = restart_component_registry();
        assert_eq!(completed_state.subtree_removal_history.len(), 1);
        assert_eq!(
            ComponentRegistryOps::finalize_subtree_leaf(
                fixture.component,
                operation_id,
                selected.traversal_steps,
                fixture.unrelated.canister_id,
                fixture.unrelated.parent_canister_id,
                16_777_216,
            )
            .expect("exact terminal finalization retry"),
            completed
        );
        assert_eq!(RootComponentRegistryStore::export(), completed_state);
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            exact_registry_entry_bytes(&completed_state)
        );

        let partition = ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition");
        let RootComponentDrainingAdvanceView::DescendantSubtreePending {
            operation_id: next_operation_id,
            target_canister_id,
            reserved_against_registry,
        } = ComponentRegistryOps::advance_component_draining(fixture.component, [90; 32])
            .expect("advance beyond the completed removal cursor")
        else {
            panic!("completed draining subtree must release the driver cursor");
        };
        assert_eq!(target_canister_id, fixture.target.canister_id);
        assert_eq!(
            reserved_against_registry,
            ComponentRegistryHead {
                component: fixture.component,
                revision: partition.revision,
                content_hash: partition.content_hash,
            }
        );
        assert_eq!(
            ComponentRegistryOps::begin_draining_subtree_removal(
                fixture.component,
                [90; 32],
                16_777_216,
            )
            .expect("fence the next deterministic draining subtree")
            .operation_id,
            next_operation_id
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one test proves independent partition progress and exact shared root capacity"
    )]
    fn incomplete_component_operation_does_not_block_an_unrelated_partition() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        let component_a = ComponentInstanceId::from_generated_bytes([10; 32]);
        let component_b = ComponentInstanceId::from_generated_bytes([11; 32]);
        let parent_a = candid::Principal::from_slice(&[18; 29]);
        let parent_b = candid::Principal::from_slice(&[19; 29]);
        let partition_a = active_component_partition(&root, release_set, component_a, parent_a);
        let partition_b = active_component_partition(&root, release_set, component_b, parent_b);
        let initial_encoded_bytes = partition_a
            .encoded_bytes
            .checked_add(partition_b.encoded_bytes)
            .expect("initial Registry bytes");
        RootComponentRegistryStore::import(RootComponentRegistryData {
            current: Some(RootComponentRegistryMetaRecord {
                root: root.clone(),
                prepared_against_registry: FleetRegistryVersion {
                    authority: root.authority.clone(),
                    revision: 4,
                    content_hash: [5; 32],
                },
                release_set,
                store_bootstrap: RootStoreBootstrapRequest {
                    manifest_payload_size_bytes: 128,
                },
                next_allocation_sequence: 3,
                reserved_component_instances: 0,
                committed_component_instances: 2,
                managed_descendants: 0,
                known_created_component_canisters: 2,
                encoded_bytes: initial_encoded_bytes,
                initial_inventory: None,
            }),
            partitions: vec![partition_a.clone(), partition_b.clone()],
            ..RootComponentRegistryData::default()
        });

        let operation_a = [44; 32];
        let decision_a = child_allocation_decision(&partition_a, "project_instance");
        let registry_a = component_registry_head(&partition_a);
        ComponentRegistryOps::reserve_child_allocation(
            decision_a.clone(),
            operation_a,
            registry_a.clone(),
        )
        .expect("reserve Component A child");
        let incomplete_a = ComponentRegistryOps::begin_child_creation(
            component_a,
            operation_a,
            child_creation_plan(&root, 50),
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(51),
                reservation_intent_id: IntentId(52),
            },
        )
        .expect("record Component A creation intent");
        let partition_a_after_intent = ComponentRegistryOps::partition(component_a)
            .expect("Component A partition read")
            .expect("Component A partition");
        let before_failed_a = RootComponentRegistryStore::export();
        ComponentRegistryOps::mark_child_created(component_a, operation_a, parent_a)
            .expect_err("Component A cannot create over its registered parent");
        assert_eq!(RootComponentRegistryStore::export(), before_failed_a);

        let operation_b = [54; 32];
        let decision_b = child_allocation_decision(&partition_b, "project_instance");
        let registry_b = component_registry_head(&partition_b);
        ComponentRegistryOps::reserve_child_allocation(decision_b, operation_b, registry_b.clone())
            .expect("reserve unrelated Component B child");
        ComponentRegistryOps::begin_child_creation(
            component_b,
            operation_b,
            child_creation_plan(&root, 55),
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(56),
                reservation_intent_id: IntentId(57),
            },
        )
        .expect("record Component B creation intent");
        let child_b = candid::Principal::from_slice(&[58; 29]);
        let progressed_b =
            ComponentRegistryOps::mark_child_created(component_b, operation_b, child_b)
                .expect("Component B progresses independently");
        assert!(matches!(
            progressed_b.progress,
            RootComponentChildAllocationProgressView::Created { canister, .. }
                if canister == child_b
        ));

        let durable = restart_component_registry();
        let retried_a =
            ComponentRegistryOps::reserve_child_allocation(decision_a, operation_a, registry_a)
                .expect("retry preserves incomplete Component A intent");
        assert_eq!(retried_a, incomplete_a);
        assert_eq!(
            ComponentRegistryOps::partition(component_a)
                .expect("Component A partition read")
                .expect("Component A partition"),
            partition_a_after_intent
        );
        let current = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(current.managed_descendants, 2);
        assert_eq!(current.known_created_component_canisters, 3);
        assert!(
            exact_registry_entry_bytes(&durable) <= current.encoded_bytes,
            "persisted entries must fit inside their exact pre-effect charges"
        );
        assert_eq!(
            current.encoded_bytes,
            durable
                .partitions
                .iter()
                .map(|partition| partition.encoded_bytes)
                .sum::<u64>()
        );
        for component in [component_a, component_b] {
            let partition = ComponentRegistryOps::partition(component)
                .expect("partition read")
                .expect("active partition");
            assert_eq!(
                partition.encoded_bytes,
                exact_component_registry_entry_bytes(&durable, component)
            );
        }

        let mut capacity_bounded = durable;
        let status = capacity_bounded.current.as_mut().expect("Registry status");
        let managed_canisters = 1
            + status.reserved_component_instances
            + status.committed_component_instances
            + status.managed_descendants;
        assert_eq!(managed_canisters, 5);
        status.root.limits.maximum_managed_canisters = managed_canisters;
        RootComponentRegistryStore::import(capacity_bounded);
        let before_capacity_failure = RootComponentRegistryStore::export();
        let capacity_error = ComponentRegistryOps::reserve_child_allocation(
            child_allocation_decision(&partition_b, "project_machine"),
            [59; 32],
            registry_b,
        )
        .expect_err("Component A reservation remains charged to the shared root limit");
        assert!(capacity_error.is_public_resource_exhausted());
        assert_eq!(
            RootComponentRegistryStore::export(),
            before_capacity_failure
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one test follows the complete direct-child reserve-through-commit lifecycle"
    )]
    fn child_reservation_is_parent_indexed_idempotent_and_capacity_bounded() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let root_canister = root.fleet_subnet_root;
        let component = ComponentInstanceId::from_generated_bytes([10; 32]);
        let parent = candid::Principal::from_slice(&[18; 29]);
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        let binding = ComponentBinding {
            authority: root.authority.clone(),
            component,
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            canister_id: parent,
        };
        let mut partition = ComponentRegistryPartitionRecord {
            binding: binding.clone(),
            provisioning_origin: ComponentProvisioningOrigin::FleetAdministrator {
                caller: candid::Principal::from_slice(&[11; 29]),
            },
            release_set,
            status: ComponentLifecycleStatus::Active,
            revision: 2,
            content_hash: component_partition_content_hash(
                &binding,
                &ComponentProvisioningOrigin::FleetAdministrator {
                    caller: candid::Principal::from_slice(&[11; 29]),
                },
                release_set,
                ComponentLifecycleStatus::Active,
                2,
                empty_component_descendant_content_hash(component),
                0,
            )
            .expect("partition hash"),
            descendant_content_hash: empty_component_descendant_content_hash(component),
            directory_synchronized_at_ns: 33,
            reserved_descendants: 0,
            committed_descendants: 0,
            encoded_bytes: 0,
        };
        let component_principal_index_bytes =
            RootComponentRegistryStore::principal_index_entry_bytes(parent, component);
        for _ in 0..8 {
            let encoded_bytes = RootComponentRegistryStore::partition_entry_bytes(&partition)
                + component_principal_index_bytes;
            if partition.encoded_bytes == encoded_bytes {
                break;
            }
            partition.encoded_bytes = encoded_bytes;
        }
        assert_eq!(
            partition.encoded_bytes,
            RootComponentRegistryStore::partition_entry_bytes(&partition)
                + component_principal_index_bytes
        );
        let initial_encoded_bytes = partition.encoded_bytes;
        RootComponentRegistryStore::import(RootComponentRegistryData {
            current: Some(RootComponentRegistryMetaRecord {
                root: root.clone(),
                prepared_against_registry: FleetRegistryVersion {
                    authority: root.authority.clone(),
                    revision: 4,
                    content_hash: [5; 32],
                },
                release_set,
                store_bootstrap: RootStoreBootstrapRequest {
                    manifest_payload_size_bytes: 128,
                },
                next_allocation_sequence: 2,
                reserved_component_instances: 0,
                committed_component_instances: 1,
                managed_descendants: 0,
                known_created_component_canisters: 1,
                encoded_bytes: partition.encoded_bytes,
                initial_inventory: None,
            }),
            partitions: vec![partition.clone()],
            ..RootComponentRegistryData::default()
        });
        let decision = ComponentChildAllocationDecision {
            component,
            component_spec: binding.component_spec.clone(),
            spec_hash: binding.spec_hash,
            parent_canister_id: parent,
            parent_role: binding.role.clone(),
            child_role: CanisterRole::new("project_instance"),
            child_kind: ComponentChildKind::Instance,
            maximum_instances_per_parent: 10_000,
            maximum_descendants: 20_000,
            maximum_registry_bytes: 16_777_216,
        };
        let registry = ComponentRegistryHead {
            component,
            revision: partition.revision,
            content_hash: partition.content_hash,
        };

        let reserved = ComponentRegistryOps::reserve_child_allocation(
            decision.clone(),
            [44; 32],
            registry.clone(),
        )
        .expect("reserve child");
        let interrupted = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(interrupted);
        let repeated =
            ComponentRegistryOps::reserve_child_allocation(decision.clone(), [44; 32], registry)
                .expect("retry child reservation");

        assert_eq!(reserved, repeated);
        assert_eq!(
            ComponentRegistryOps::registered_parent(component, parent)
                .expect("registered parent")
                .expect("top-level parent")
                .0,
            ManagedCanisterBinding::Component(binding.clone())
        );
        assert_eq!(
            ComponentRegistryOps::parent_role_instances(component, parent, &decision.child_role,)
                .expect("parent-role count"),
            1
        );
        let partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("partition");
        assert_eq!(partition.reserved_descendants, 1);
        assert_eq!(partition.committed_descendants, 0);
        let current = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(current.managed_descendants, 1);
        assert_eq!(current.encoded_bytes, partition.encoded_bytes);
        assert!(partition.encoded_bytes > initial_encoded_bytes);

        let mut exhausted = decision.clone();
        exhausted.maximum_instances_per_parent = 1;
        let before = RootComponentRegistryStore::export();
        let error = ComponentRegistryOps::reserve_child_allocation(
            exhausted,
            [45; 32],
            repeated.reserved_against_registry.clone(),
        )
        .expect_err("per-parent capacity must reject reservation");
        assert!(error.is_public_resource_exhausted());
        assert_eq!(RootComponentRegistryStore::export(), before);

        let mut conflicting = decision.clone();
        conflicting.maximum_descendants -= 1;
        assert!(
            ComponentRegistryOps::reserve_child_allocation(
                conflicting,
                [44; 32],
                repeated.reserved_against_registry.clone(),
            )
            .is_err()
        );

        let plan = RootComponentCreationPlan {
            wasm_store: candid::Principal::from_slice(&[50; 29]),
            payload_hash: [51; 32],
            payload_size_bytes: 4_096,
            initial_cycles: Cycles::new(5_000_000_000_000),
            controller: root_canister,
        };
        let before_creation = RootComponentRegistryStore::export();
        let mut capacity_exhausted = before_creation.clone();
        let maximum_registry_bytes = capacity_exhausted
            .current
            .as_ref()
            .expect("Registry status")
            .encoded_bytes;
        capacity_exhausted
            .current
            .as_mut()
            .expect("Registry status")
            .root
            .limits
            .maximum_registry_bytes = maximum_registry_bytes;
        RootComponentRegistryStore::import(capacity_exhausted);
        let error =
            ComponentRegistryOps::validate_child_creation_capacity(component, [44; 32], &plan)
                .expect_err("creation must fit before the paid effect");
        assert!(error.is_public_resource_exhausted());
        assert!(matches!(
            ComponentRegistryOps::child_allocation(component, [44; 32])
                .expect("child allocation")
                .expect("reserved child")
                .progress,
            RootComponentChildAllocationProgressView::Reserved
        ));
        RootComponentRegistryStore::import(before_creation);

        ComponentRegistryOps::validate_child_creation_capacity(component, [44; 32], &plan)
            .expect("child creation capacity");
        let intent = ComponentRegistryOps::begin_child_creation(
            component,
            [44; 32],
            plan,
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(52),
                reservation_intent_id: IntentId(53),
            },
        )
        .expect("child creation intent");
        let intent_bytes = ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes;
        assert!(intent_bytes > current.encoded_bytes);
        assert!(matches!(
            intent.progress,
            RootComponentChildAllocationProgressView::CreationIntent(_)
        ));

        restart_component_registry();
        let canister = candid::Principal::from_slice(&[54; 29]);
        let created = ComponentRegistryOps::mark_child_created(component, [44; 32], canister)
            .expect("record created child");
        restart_component_registry();
        let repeated_created =
            ComponentRegistryOps::mark_child_created(component, [44; 32], canister)
                .expect("exact created child retry");

        assert_eq!(created, repeated_created);
        assert!(matches!(
            created.progress,
            RootComponentChildAllocationProgressView::Created {
                canister: created_canister,
                ..
            } if created_canister == canister
        ));
        let created_status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(created_status.known_created_component_canisters, 2);
        assert_eq!(created_status.managed_descendants, 1);
        assert_eq!(created_status.encoded_bytes, intent_bytes);
        assert_eq!(
            ComponentRegistryOps::partition(component)
                .expect("partition read")
                .expect("partition")
                .reserved_descendants,
            1
        );
        assert!(
            ComponentRegistryOps::mark_child_created(
                component,
                [44; 32],
                candid::Principal::from_slice(&[55; 29]),
            )
            .is_err()
        );
        let install_plan = RootComponentChildInstallPlan {
            raw_module_hash: [56; 32],
            chunk_hashes: vec![vec![57; 32], vec![58; 32]],
            binding: ComponentChildBinding {
                component: binding,
                parent_canister_id: parent,
                role: decision.child_role.clone(),
                canister_id: canister,
            },
            maximum_registry_bytes: decision.maximum_registry_bytes,
        };
        let before_install = RootComponentRegistryStore::export();
        let mut install_capacity_exhausted = before_install.clone();
        let maximum_registry_bytes = install_capacity_exhausted
            .current
            .as_ref()
            .expect("Registry status")
            .encoded_bytes;
        install_capacity_exhausted
            .current
            .as_mut()
            .expect("Registry status")
            .root
            .limits
            .maximum_registry_bytes = maximum_registry_bytes;
        RootComponentRegistryStore::import(install_capacity_exhausted);
        let error = ComponentRegistryOps::validate_child_install_capacity(
            component,
            [44; 32],
            &install_plan,
        )
        .expect_err("installation must fit before the paid effect");
        assert!(error.is_public_resource_exhausted());
        assert!(matches!(
            ComponentRegistryOps::child_allocation(component, [44; 32])
                .expect("child allocation")
                .expect("created child")
                .progress,
            RootComponentChildAllocationProgressView::Created { .. }
        ));
        RootComponentRegistryStore::import(before_install);

        ComponentRegistryOps::validate_child_install_capacity(component, [44; 32], &install_plan)
            .expect("child install capacity");
        let install_intent = ComponentRegistryOps::begin_child_install(
            component,
            [44; 32],
            install_plan.clone(),
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(59),
                reservation_intent_id: IntentId(60),
            },
        )
        .expect("child install intent");
        let install_intent_bytes = ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes;
        assert!(install_intent_bytes > intent_bytes);
        assert!(matches!(
            install_intent.progress,
            RootComponentChildAllocationProgressView::InstallIntent { .. }
        ));

        let mut conflicting_install = install_plan.clone();
        conflicting_install.raw_module_hash = [61; 32];
        assert!(
            ComponentRegistryOps::renew_child_install_intent(
                component,
                [44; 32],
                &conflicting_install,
                ReplayCostGuardSettlement {
                    quota_intent_id: IntentId(62),
                    reservation_intent_id: IntentId(63),
                },
            )
            .is_err()
        );
        restart_component_registry();
        let renewed = ComponentRegistryOps::renew_child_install_intent(
            component,
            [44; 32],
            &install_plan,
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(64),
                reservation_intent_id: IntentId(65),
            },
        )
        .expect("renew exact child install intent");
        let RootComponentChildAllocationProgressView::InstallIntent { installation, .. } =
            &renewed.progress
        else {
            panic!("renewed child install intent");
        };
        assert_eq!(installation.binding, install_plan.binding);
        assert_eq!(
            installation.cost_guard_settlement.quota_intent_id,
            IntentId(64)
        );

        let installed = ComponentRegistryOps::mark_child_installed(component, [44; 32])
            .expect("mark child installed");
        restart_component_registry();
        let installed_retry = ComponentRegistryOps::mark_child_installed(component, [44; 32])
            .expect("installed child retry");
        assert_eq!(installed, installed_retry);
        assert!(matches!(
            installed.progress,
            RootComponentChildAllocationProgressView::Installed { .. }
        ));
        let verified = ComponentRegistryOps::mark_child_verified(component, [44; 32])
            .expect("mark child verified");
        restart_component_registry();
        let verified_retry = ComponentRegistryOps::mark_child_verified(component, [44; 32])
            .expect("verified child retry");
        assert_eq!(verified, verified_retry);
        assert!(matches!(
            verified.progress,
            RootComponentChildAllocationProgressView::Verified { .. }
        ));
        let verified_status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(verified_status.known_created_component_canisters, 2);
        assert_eq!(verified_status.managed_descendants, 1);
        assert_eq!(verified_status.encoded_bytes, install_intent_bytes);
        let verified_partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("partition");
        assert_eq!(verified_partition.reserved_descendants, 1);
        assert_eq!(verified_partition.committed_descendants, 0);
        assert_eq!(verified_partition.encoded_bytes, install_intent_bytes);

        let committed = ComponentRegistryOps::commit_verified_child(
            component,
            [44; 32],
            66,
            fleet_directory(&root),
        )
        .expect("commit verified child");
        let committed_partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("partition");
        assert_eq!(committed_partition.revision, 3);
        assert_ne!(
            committed_partition.content_hash,
            verified_partition.content_hash
        );
        assert_eq!(committed_partition.directory_synchronized_at_ns, 66);
        assert_eq!(committed_partition.reserved_descendants, 0);
        assert_eq!(committed_partition.committed_descendants, 1);
        assert!(committed_partition.encoded_bytes <= install_intent_bytes);
        let RootComponentChildAllocationProgressView::Committed {
            commitment,
            installation,
            ..
        } = &committed.0.progress
        else {
            panic!("committed child progress");
        };
        assert_eq!(
            commitment.registry,
            ComponentRegistryHead {
                component,
                revision: committed_partition.revision,
                content_hash: committed_partition.content_hash,
            }
        );
        assert_eq!(
            commitment.registry_encoded_bytes,
            committed_partition.encoded_bytes
        );
        assert_eq!(commitment.reserved_descendants, 0);
        assert_eq!(commitment.committed_descendants, 1);
        assert_ne!(commitment.directory_authority_hash, [0; 32]);
        let child_directory_authority_hash = commitment.directory_authority_hash;
        assert_eq!(installation.binding, install_plan.binding);
        assert_eq!(committed.1, committed_partition);
        let committed_status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(committed_status.managed_descendants, 1);
        assert_eq!(committed_status.known_created_component_canisters, 2);
        assert_eq!(
            committed_status.encoded_bytes,
            committed_partition.encoded_bytes
        );
        assert_eq!(
            ComponentRegistryOps::parent_role_instances(component, parent, &decision.child_role,)
                .expect("parent-role count"),
            1
        );
        let registered_child = ComponentRegistryOps::registered_parent(component, canister)
            .expect("registered child")
            .expect("normalized child");
        assert_eq!(
            registered_child,
            (
                ManagedCanisterBinding::ComponentChild(install_plan.binding),
                ComponentLifecycleStatus::Prepared,
            )
        );
        let durable = restart_component_registry();
        assert_eq!(durable.children.len(), 1);
        assert_eq!(durable.child_traversals.len(), 1);
        let progressed_partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("partition");
        let progressed_reservation = ComponentRegistryOps::reserve_child_allocation(
            decision.clone(),
            [68; 32],
            ComponentRegistryHead {
                component,
                revision: progressed_partition.revision,
                content_hash: progressed_partition.content_hash,
            },
        )
        .expect("later child reservation");
        assert!(matches!(
            progressed_reservation.progress,
            RootComponentChildAllocationProgressView::Reserved
        ));
        let committed_retry = ComponentRegistryOps::commit_verified_child(
            component,
            [44; 32],
            67,
            fleet_directory(&root),
        )
        .expect("exact child commit retry");
        assert_eq!(committed_retry, committed);

        let retried_reservation = ComponentRegistryOps::reserve_child_allocation(
            decision.clone(),
            [44; 32],
            repeated.reserved_against_registry,
        )
        .expect("reservation retry preserves install progress");
        assert_eq!(retried_reservation, committed.0);
        let before_directory_receipt = ComponentRegistryOps::current().expect("Registry status");
        let before_directory_partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("partition");
        assert!(
            ComponentRegistryOps::mark_child_runtime_activated(
                component,
                [44; 32],
                child_directory_authority_hash,
            )
            .is_err()
        );
        let prepared = ComponentRegistryOps::mark_child_directory_prepared(
            component,
            [44; 32],
            child_directory_authority_hash,
        )
        .expect("mark child Directory prepared");
        restart_component_registry();
        let prepared_again = ComponentRegistryOps::mark_child_directory_prepared(
            component,
            [44; 32],
            child_directory_authority_hash,
        )
        .expect("repeat child Directory preparation receipt");
        assert_eq!(prepared_again, prepared);
        assert!(matches!(
            prepared.progress,
            RootComponentChildAllocationProgressView::Committed {
                commitment: RootComponentChildCommitmentView {
                    directory_prepared: true,
                    runtime_activated: false,
                    membership: None,
                    ..
                },
                ..
            }
        ));
        assert!(
            ComponentRegistryOps::activate_child_membership(
                component,
                [44; 32],
                69,
                fleet_directory(&root),
            )
            .is_err()
        );
        let activated = ComponentRegistryOps::mark_child_runtime_activated(
            component,
            [44; 32],
            child_directory_authority_hash,
        )
        .expect("mark child runtime activated");
        restart_component_registry();
        let activated_again = ComponentRegistryOps::mark_child_runtime_activated(
            component,
            [44; 32],
            child_directory_authority_hash,
        )
        .expect("repeat child runtime activation receipt");
        assert_eq!(activated_again, activated);
        assert!(matches!(
            activated.progress,
            RootComponentChildAllocationProgressView::Committed {
                commitment: RootComponentChildCommitmentView {
                    directory_prepared: true,
                    runtime_activated: true,
                    membership: None,
                    ..
                },
                ..
            }
        ));
        assert_eq!(
            ComponentRegistryOps::current().expect("Registry status"),
            before_directory_receipt
        );
        assert_eq!(
            ComponentRegistryOps::partition(component)
                .expect("partition read")
                .expect("partition"),
            before_directory_partition
        );
        let membership = ComponentRegistryOps::activate_child_membership(
            component,
            [44; 32],
            69,
            fleet_directory(&root),
        )
        .expect("activate child membership");
        restart_component_registry();
        let membership_again = ComponentRegistryOps::activate_child_membership(
            component,
            [44; 32],
            70,
            fleet_directory(&root),
        )
        .expect("repeat child membership activation");
        assert_eq!(membership_again, membership);
        assert_eq!(
            membership.1.revision,
            before_directory_partition.revision + 1
        );
        assert_eq!(membership.1.status, ComponentLifecycleStatus::Active);
        assert_eq!(
            membership.1.reserved_descendants,
            before_directory_partition.reserved_descendants
        );
        assert_eq!(
            membership.1.committed_descendants,
            before_directory_partition.committed_descendants
        );
        assert_ne!(
            membership.1.descendant_content_hash,
            before_directory_partition.descendant_content_hash
        );
        assert_eq!(membership.1.directory_synchronized_at_ns, 69);
        let RootComponentChildAllocationProgressView::Committed {
            commitment:
                RootComponentChildCommitmentView {
                    membership: Some(active_membership),
                    ..
                },
            ..
        } = &membership.0.progress
        else {
            panic!("active child membership receipt");
        };
        assert_eq!(
            active_membership.registry,
            ComponentRegistryHead {
                component,
                revision: membership.1.revision,
                content_hash: membership.1.content_hash,
            }
        );
        assert_eq!(
            active_membership.descendant_content_hash,
            membership.1.descendant_content_hash
        );
        assert_eq!(
            active_membership.registry_encoded_bytes,
            membership.1.encoded_bytes
        );
        assert!(!active_membership.directory_synchronized);
        assert_eq!(
            ComponentRegistryOps::registered_parent(component, canister)
                .expect("registered active child")
                .expect("active child row")
                .1,
            ComponentLifecycleStatus::Active
        );
        assert!(
            ComponentRegistryOps::mark_child_membership_synchronized(
                component,
                [44; 32],
                [u8::MAX; 32],
            )
            .is_err()
        );
        let terminal = ComponentRegistryOps::mark_child_membership_synchronized(
            component,
            [44; 32],
            active_membership.directory_authority_hash,
        )
        .expect("mark child membership synchronized");
        let terminal_snapshot = restart_component_registry();
        let terminal_again = ComponentRegistryOps::mark_child_membership_synchronized(
            component,
            [44; 32],
            active_membership.directory_authority_hash,
        )
        .expect("repeat child membership synchronization receipt");
        assert_eq!(terminal_again, terminal);
        assert!(matches!(
            terminal.progress,
            RootComponentChildAllocationProgressView::Committed {
                commitment: RootComponentChildCommitmentView {
                    membership: Some(RootComponentChildMembershipView {
                        directory_synchronized: true,
                        ..
                    }),
                    ..
                },
                ..
            }
        ));
        let terminal_partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("terminal active partition");
        assert_eq!(terminal_partition, membership.1);
        let exact_terminal_bytes = exact_registry_entry_bytes(&terminal_snapshot);
        assert_eq!(terminal_partition.encoded_bytes, exact_terminal_bytes);
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("terminal Registry status")
                .encoded_bytes,
            exact_terminal_bytes
        );
        let complete_directory = ComponentRegistryOps::directory_page(
            component,
            &ComponentDirectoryPageSelection {
                parent_canister_id: None,
                role: None,
                status: None,
                start_after: None,
            },
            100,
        )
        .expect("complete Component Directory page");
        assert_eq!(complete_directory.entries.len(), 1);
        assert_eq!(
            complete_directory.entries[0].binding.component,
            terminal_partition.binding
        );
        assert_eq!(
            complete_directory.entries[0].binding.parent_canister_id,
            parent
        );
        assert_eq!(
            complete_directory.entries[0].binding.role,
            decision.child_role
        );
        assert_eq!(complete_directory.entries[0].binding.canister_id, canister);
        assert_eq!(
            complete_directory.entries[0].status,
            ComponentLifecycleStatus::Active
        );
        assert!(complete_directory.next_cursor.is_none());
        let direct_active_children = ComponentRegistryOps::directory_page(
            component,
            &ComponentDirectoryPageSelection {
                parent_canister_id: Some(parent),
                role: Some(decision.child_role.clone()),
                status: Some(ComponentLifecycleStatus::Active),
                start_after: None,
            },
            100,
        )
        .expect("filtered direct-child Directory page");
        assert_eq!(direct_active_children.entries, complete_directory.entries);
        let after_only_child = ComponentRegistryOps::directory_page(
            component,
            &ComponentDirectoryPageSelection {
                parent_canister_id: Some(parent),
                role: Some(decision.child_role.clone()),
                status: Some(ComponentLifecycleStatus::Active),
                start_after: Some(ComponentDirectoryCanonicalCursor {
                    parent_canister_id: parent,
                    role: decision.child_role.clone(),
                    canister_id: canister,
                }),
            },
            100,
        )
        .expect("Directory page after only child");
        assert!(after_only_child.entries.is_empty());
        assert!(after_only_child.next_cursor.is_none());
        let prepared_children = ComponentRegistryOps::directory_page(
            component,
            &ComponentDirectoryPageSelection {
                parent_canister_id: Some(parent),
                role: Some(decision.child_role.clone()),
                status: Some(ComponentLifecycleStatus::Prepared),
                start_after: None,
            },
            100,
        )
        .expect("status-filtered Directory page");
        assert!(prepared_children.entries.is_empty());
        let later_reservation = ComponentRegistryOps::reserve_child_allocation(
            decision,
            [71; 32],
            ComponentRegistryHead {
                component,
                revision: terminal_partition.revision,
                content_hash: terminal_partition.content_hash,
            },
        )
        .expect("reserve later child after membership");
        assert!(matches!(
            later_reservation.progress,
            RootComponentChildAllocationProgressView::Reserved
        ));
        let membership_after_later_reservation = ComponentRegistryOps::activate_child_membership(
            component,
            [44; 32],
            72,
            fleet_directory(&root),
        )
        .expect("membership retry after later reservation");
        assert_eq!(membership_after_later_reservation.0, terminal);
        assert_eq!(membership_after_later_reservation.1, membership.1);
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one test follows the complete paid creation lifecycle and its exact retry invariants"
    )]
    fn creation_intent_reserves_terminal_bytes_and_created_retry_preserves_principal() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let version = FleetRegistryVersion {
            authority: root.authority.clone(),
            revision: 4,
            content_hash: [5; 32],
        };
        ComponentRegistryOps::prepare(
            root,
            version,
            FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [8; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
            },
            RootStoreBootstrapRequest {
                manifest_payload_size_bytes: 128,
            },
        )
        .expect("prepare");
        ComponentRegistryOps::reserve_allocation(
            TopLevelComponentAllocationDecision {
                allocation_sequence: 1,
                component: ComponentInstanceId::from_generated_bytes([10; 32]),
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [6; 32],
                role: CanisterRole::new("project_hub"),
            },
            [12; 32],
            ComponentProvisioningOrigin::FleetAdministrator {
                caller: candid::Principal::from_slice(&[11; 29]),
            },
            false,
        )
        .expect("reserve");
        let reserved_bytes = ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes;
        let plan = RootComponentCreationPlan {
            wasm_store: candid::Principal::from_slice(&[13; 29]),
            payload_hash: [14; 32],
            payload_size_bytes: 4_096,
            initial_cycles: Cycles::new(5_000_000_000_000),
            controller: candid::Principal::from_slice(&[15; 29]),
        };

        assert_creation_capacity_is_reserved_before_effect(&plan, reserved_bytes);

        ComponentRegistryOps::validate_creation_capacity([12; 32], &plan)
            .expect("creation capacity");
        let intent = ComponentRegistryOps::begin_creation(
            [12; 32],
            plan,
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(16),
                reservation_intent_id: IntentId(17),
            },
        )
        .expect("creation intent");
        let intent_bytes = ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes;
        assert!(intent_bytes > reserved_bytes);
        assert!(matches!(
            intent.progress,
            RootComponentAllocationProgressView::CreationIntent(_)
        ));
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .known_created_component_canisters,
            0,
            "creation intent without a known principal must not count"
        );

        let interrupted = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(interrupted);
        let canister = candid::Principal::from_slice(&[18; 29]);
        let created =
            ComponentRegistryOps::mark_created([12; 32], canister).expect("record created");
        let repeated =
            ComponentRegistryOps::mark_created([12; 32], canister).expect("exact created retry");

        assert_eq!(created, repeated);
        let status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(
            status.encoded_bytes, intent_bytes,
            "the intent must reserve terminal record capacity before the effect"
        );
        assert_eq!(
            status.known_created_component_canisters, 1,
            "a known created principal must be counted exactly once"
        );
        assert!(matches!(
            created.progress,
            RootComponentAllocationProgressView::Created {
                canister: created_canister,
                ..
            } if created_canister == canister
        ));
        assert!(
            ComponentRegistryOps::mark_created([12; 32], candid::Principal::from_slice(&[19; 29]),)
                .is_err()
        );
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .known_created_component_canisters,
            1,
            "a conflicting retry must not change the count"
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    fn install_intent_reserves_terminal_bytes_and_advances_idempotently() {
        let (root, created, canister) = prepared_created_allocation();
        let created_bytes = ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes;
        let plan = RootComponentInstallPlan {
            raw_module_hash: [20; 32],
            chunk_hashes: vec![vec![21; 32], vec![22; 32]],
            binding: ComponentBinding {
                authority: root.authority.clone(),
                component: created.component,
                component_spec: created.component_spec.clone(),
                spec_hash: created.spec_hash,
                role: created.role,
                placement_subnet: root.placement_subnet,
                fleet_subnet_root: root.fleet_subnet_root,
                canister_id: canister,
            },
            maximum_registry_bytes: 16_777_216,
        };

        let mut component_exhausted = plan.clone();
        component_exhausted.maximum_registry_bytes = 1;
        let capacity_error =
            ComponentRegistryOps::validate_install_capacity([12; 32], &component_exhausted)
                .expect_err("terminal Component partition must fit before installation");
        assert!(capacity_error.is_public_resource_exhausted());
        assert!(matches!(
            ComponentRegistryOps::allocation([12; 32])
                .expect("created allocation")
                .progress,
            RootComponentAllocationProgressView::Created { .. }
        ));

        let intent_bytes = advance_install_to_verified(&plan, created_bytes);

        let directory = fleet_directory(&root);
        let (committed, partition) = ComponentRegistryOps::commit_verified(
            [12; 32],
            31,
            plan.maximum_registry_bytes,
            directory.clone(),
        )
        .expect("commit verified Component");
        let interrupted = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(interrupted);
        let repeated = ComponentRegistryOps::commit_verified(
            [12; 32],
            32,
            plan.maximum_registry_bytes,
            directory,
        )
        .expect("exact commitment retry");
        assert_eq!(repeated, (committed.clone(), partition.clone()));
        assert!(matches!(
            committed.progress,
            RootComponentAllocationProgressView::Committed { .. }
        ));
        assert_eq!(partition.binding, plan.binding);
        assert_eq!(partition.status, ComponentLifecycleStatus::Prepared);
        assert_eq!(partition.revision, 1);
        assert_ne!(partition.content_hash, [0; 32]);
        assert_eq!(partition.directory_synchronized_at_ns, 31);
        assert_eq!(
            ComponentRegistryOps::component_for_principal(canister),
            Some(committed.component)
        );
        assert_eq!(
            ComponentRegistryOps::partition(committed.component)
                .expect("valid partition")
                .expect("committed partition"),
            partition
        );
        let status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(status.reserved_component_instances, 0);
        assert_eq!(status.committed_component_instances, 1);
        assert_eq!(status.managed_descendants, 0);
        assert_eq!(status.encoded_bytes, partition.encoded_bytes);
        assert!(status.encoded_bytes <= intent_bytes);
        assert_eq!(
            ComponentRegistryOps::component_spec_counts(&committed.component_spec)
                .expect("Spec counts"),
            ComponentSpecInstanceCounts {
                reserved: 0,
                committed: 1,
            }
        );
        assert_directory_preparation_receipt(
            &committed,
            &partition,
            fleet_directory(&root),
            plan.maximum_registry_bytes,
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    fn assert_directory_preparation_receipt(
        committed: &RootComponentAllocationView,
        prepared_partition: &ComponentRegistryPartitionView,
        directory: FleetDirectorySnapshot,
        maximum_component_registry_bytes: u64,
    ) {
        let RootComponentAllocationProgressView::Committed { commitment, .. } = &committed.progress
        else {
            panic!("committed allocation progress");
        };
        assert_ne!(commitment.directory_authority_hash, [0; 32]);
        assert_eq!(
            commitment.prepared_registry_encoded_bytes,
            prepared_partition.encoded_bytes
        );
        assert!(!commitment.directory_prepared);
        assert!(!commitment.runtime_activated);
        assert!(
            ComponentRegistryOps::mark_runtime_activated(
                [12; 32],
                commitment.directory_authority_hash,
            )
            .is_err()
        );
        let prepared = ComponentRegistryOps::mark_directory_prepared(
            [12; 32],
            commitment.directory_authority_hash,
        )
        .expect("mark Directory prepared");
        let prepared_again = ComponentRegistryOps::mark_directory_prepared(
            [12; 32],
            commitment.directory_authority_hash,
        )
        .expect("retry Directory receipt");
        assert_eq!(prepared_again, prepared);
        assert!(matches!(
            &prepared.progress,
            RootComponentAllocationProgressView::Committed {
                commitment: RootComponentCommitmentView {
                    directory_prepared: true,
                    runtime_activated: false,
                    ..
                },
                ..
            }
        ));
        let activated = ComponentRegistryOps::mark_runtime_activated(
            [12; 32],
            commitment.directory_authority_hash,
        )
        .expect("mark runtime activated");
        let activated_again = ComponentRegistryOps::mark_runtime_activated(
            [12; 32],
            commitment.directory_authority_hash,
        )
        .expect("retry runtime activation receipt");
        assert_eq!(activated_again, activated);
        assert!(matches!(
            &activated.progress,
            RootComponentAllocationProgressView::Committed {
                commitment: RootComponentCommitmentView {
                    directory_prepared: true,
                    runtime_activated: true,
                    membership: None,
                    ..
                },
                ..
            }
        ));
        assert_membership_receipt(
            &activated,
            prepared_partition,
            directory,
            maximum_component_registry_bytes,
        );
    }

    fn assert_membership_receipt(
        activated: &RootComponentAllocationView,
        prepared_partition: &ComponentRegistryPartitionView,
        directory: FleetDirectorySnapshot,
        maximum_component_registry_bytes: u64,
    ) {
        assert!(committed_membership(activated).is_none());
        let (membership_activated, active_partition) = ComponentRegistryOps::activate_membership(
            [12; 32],
            33,
            maximum_component_registry_bytes,
            directory.clone(),
        )
        .expect("activate Registry membership");
        let repeated_membership = ComponentRegistryOps::activate_membership(
            [12; 32],
            34,
            maximum_component_registry_bytes,
            directory.clone(),
        )
        .expect("repeat Registry membership activation");
        assert_eq!(
            repeated_membership,
            (membership_activated.clone(), active_partition.clone())
        );
        assert_eq!(active_partition.status, ComponentLifecycleStatus::Active);
        assert_eq!(active_partition.revision, 2);
        assert_eq!(active_partition.directory_synchronized_at_ns, 33);
        assert_ne!(
            active_partition.content_hash,
            prepared_partition.content_hash
        );
        assert_eq!(
            ComponentRegistryOps::prepared_partition([12; 32])
                .expect("reconstruct prepared partition"),
            *prepared_partition
        );
        assert_eq!(
            ComponentRegistryOps::commit_verified(
                [12; 32],
                35,
                maximum_component_registry_bytes,
                directory,
            )
            .expect("commit retry after membership activation")
            .1,
            *prepared_partition
        );

        let membership_synchronized = ComponentRegistryOps::mark_membership_synchronized(
            [12; 32],
            committed_membership(&membership_activated)
                .expect("membership receipt")
                .directory_authority_hash,
        )
        .expect("mark membership Directory synchronized");
        let synchronized_again = ComponentRegistryOps::mark_membership_synchronized(
            [12; 32],
            committed_membership(&membership_activated)
                .expect("membership receipt")
                .directory_authority_hash,
        )
        .expect("repeat membership Directory receipt");
        assert_eq!(synchronized_again, membership_synchronized);
        assert!(
            committed_membership(&membership_synchronized)
                .expect("membership receipt")
                .directory_synchronized
        );
        assert_initial_inventory_receipt();
    }

    fn assert_initial_inventory_receipt() {
        let sealed = ComponentRegistryOps::seal_initial_inventory([40; 32], 41)
            .expect("seal initial inventory");
        assert_eq!(sealed.operation_ids, vec![[12; 32]]);
        assert_eq!(sealed.receipt.fleet_activation_operation_id, [40; 32]);
        assert_eq!(sealed.receipt.component_count, 1);
        assert_ne!(sealed.receipt.inventory_hash, [0; 32]);
        assert_eq!(sealed.receipt.sealed_at_ns, 41);
        assert!(!sealed.receipt.directories_converged);
        assert!(!sealed.receipt.root_runtime_activated);
        let repeated = ComponentRegistryOps::seal_initial_inventory([40; 32], 42)
            .expect("retry initial inventory seal");
        assert_eq!(repeated, sealed);
        assert!(
            ComponentRegistryOps::reserve_allocation(
                TopLevelComponentAllocationDecision {
                    allocation_sequence: 2,
                    component: ComponentInstanceId::from_generated_bytes([42; 32]),
                    component_spec: "projects".parse().expect("Component Spec"),
                    spec_hash: [43; 32],
                    role: CanisterRole::new("project_hub"),
                },
                [44; 32],
                ComponentProvisioningOrigin::FleetAdministrator {
                    caller: candid::Principal::from_slice(&[11; 29]),
                },
                false,
            )
            .is_err(),
            "a Prepared root cannot extend its sealed initial inventory"
        );

        let converged = ComponentRegistryOps::mark_initial_inventory_directories_converged(
            [40; 32],
            sealed.receipt.inventory_hash,
        )
        .expect("mark initial Directories converged");
        assert!(converged.directories_converged);
        assert!(!converged.root_runtime_activated);
        let terminal = ComponentRegistryOps::mark_initial_inventory_root_runtime_activated(
            [40; 32],
            sealed.receipt.inventory_hash,
        )
        .expect("mark root runtime activated");
        assert!(terminal.directories_converged);
        assert!(terminal.root_runtime_activated);
        assert_eq!(
            ComponentRegistryOps::initial_inventory([40; 32]).expect("terminal initial inventory"),
            terminal
        );
        assert_child_reservation_preserves_membership_receipt();
        ComponentRegistryOps::reserve_allocation(
            TopLevelComponentAllocationDecision {
                allocation_sequence: 2,
                component: ComponentInstanceId::from_generated_bytes([42; 32]),
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [43; 32],
                role: CanisterRole::new("project_hub"),
            },
            [44; 32],
            ComponentProvisioningOrigin::FleetAdministrator {
                caller: candid::Principal::from_slice(&[11; 29]),
            },
            true,
        )
        .expect("active root admits dynamic allocation after terminal initial receipt");
    }

    fn assert_child_reservation_preserves_membership_receipt() {
        let allocation =
            ComponentRegistryOps::allocation([12; 32]).expect("committed Component allocation");
        let membership = committed_membership(&allocation)
            .expect("active membership receipt")
            .clone();
        let partition = ComponentRegistryOps::partition(allocation.component)
            .expect("valid active partition")
            .expect("active partition");
        ComponentRegistryOps::reserve_child_allocation(
            ComponentChildAllocationDecision {
                component: allocation.component,
                component_spec: allocation.component_spec,
                spec_hash: allocation.spec_hash,
                parent_canister_id: partition.binding.canister_id,
                parent_role: partition.binding.role.clone(),
                child_role: CanisterRole::new("project_instance"),
                child_kind: ComponentChildKind::Instance,
                maximum_instances_per_parent: 10_000,
                maximum_descendants: 20_000,
                maximum_registry_bytes: 16_777_216,
            },
            [50; 32],
            ComponentRegistryHead {
                component: allocation.component,
                revision: partition.revision,
                content_hash: partition.content_hash,
            },
        )
        .expect("reserve active Component child");

        let retried = ComponentRegistryOps::mark_membership_synchronized(
            [12; 32],
            membership.directory_authority_hash,
        )
        .expect("immutable membership receipt remains valid after child reservation");
        assert_eq!(
            committed_membership(&retried).expect("membership receipt"),
            &membership
        );
        assert_eq!(
            ComponentRegistryOps::partition(allocation.component)
                .expect("valid active partition")
                .expect("active partition")
                .reserved_descendants,
            1
        );
    }

    fn committed_membership(
        allocation: &RootComponentAllocationView,
    ) -> Option<&RootComponentMembershipView> {
        let RootComponentAllocationProgressView::Committed { commitment, .. } =
            &allocation.progress
        else {
            panic!("committed allocation progress");
        };
        commitment.membership.as_ref()
    }

    fn fleet_directory(root: &FleetSubnetRootBinding) -> FleetDirectorySnapshot {
        FleetDirectorySnapshot {
            provenance: FleetDirectoryProvenance {
                registry: FleetRegistryVersion {
                    authority: root.authority.clone(),
                    revision: 4,
                    content_hash: [5; 32],
                },
                source_fleet_subnet_root: root.fleet_subnet_root,
            },
            fleet_subnet_roots: vec![FleetSubnetRootDirectoryEntry {
                placement_subnet: root.placement_subnet,
                fleet_subnet_root: root.fleet_subnet_root,
                status: FleetSubnetRootStatus::Active,
            }],
        }
    }

    fn assert_final_inventory_receipt(
        partition: &ComponentRegistryPartitionView,
        fleet: &FleetDirectorySnapshot,
        inventory: &RootComponentFinalInventoryView,
    ) {
        assert_eq!(inventory.registry.component, partition.binding.component);
        assert_eq!(inventory.registry.revision, partition.revision);
        assert_eq!(inventory.registry.content_hash, partition.content_hash);
        assert_eq!(
            inventory.descendant_content_hash,
            empty_component_descendant_content_hash(partition.binding.component)
        );
        assert_eq!(inventory.registry_encoded_bytes, partition.encoded_bytes);
        assert_eq!(
            inventory.covered_fleet_registry_revision,
            fleet.provenance.registry.revision
        );
        assert_ne!(inventory.directory_authority_hash, [0; 32]);
        assert_ne!(inventory.inventory_hash, [0; 32]);
        assert_eq!(inventory.finalized_at_ns, 112);
    }

    fn import_empty_quiescent_component() -> (
        ComponentRegistryPartitionView,
        RootComponentDrainingView,
        FleetDirectorySnapshot,
    ) {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        let component = ComponentInstanceId::from_generated_bytes([97; 32]);
        let canister = candid::Principal::from_slice(&[98; 29]);
        let partition = active_component_partition(&root, release_set, component, canister);
        RootComponentRegistryStore::import(RootComponentRegistryData {
            current: Some(RootComponentRegistryMetaRecord {
                root: root.clone(),
                prepared_against_registry: FleetRegistryVersion {
                    authority: root.authority.clone(),
                    revision: 4,
                    content_hash: [5; 32],
                },
                release_set,
                store_bootstrap: RootStoreBootstrapRequest {
                    manifest_payload_size_bytes: 128,
                },
                next_allocation_sequence: 2,
                reserved_component_instances: 0,
                committed_component_instances: 1,
                managed_descendants: 0,
                known_created_component_canisters: 1,
                encoded_bytes: partition.encoded_bytes,
                initial_inventory: None,
            }),
            partitions: vec![partition.clone()],
            ..RootComponentRegistryData::default()
        });
        let fleet = fleet_directory(&root);
        let draining = ComponentRegistryOps::begin_component_draining(
            component,
            [99; 32],
            component_registry_head(&partition),
            100,
            16_777_216,
            fleet.clone(),
        )
        .expect("drain empty Component");
        let draining_partition = ComponentRegistryOps::partition(component)
            .expect("draining partition read")
            .expect("draining partition");
        let directory_authority = ComponentRuntimeDirectoryAuthority {
            fleet: fleet.clone(),
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: draining_partition.binding.clone(),
                    source_fleet_subnet_root: draining_partition.binding.fleet_subnet_root,
                    component_registry_revision: draining_partition.revision,
                    component_registry_content_hash: draining_partition.content_hash,
                    synchronized_at_ns: draining_partition.directory_synchronized_at_ns,
                },
                descendant_count: 0,
            },
        };
        let authority_hash = ComponentRuntimeOps::directory_authority_hash(&directory_authority)
            .expect("empty draining Directory authority hash");
        ComponentRegistryOps::prepare_component_quiescence(
            component,
            draining.operation_id,
            draining.registry.clone(),
            ComponentRuntimeDirectoryConvergenceEvidence {
                operation_id: [100; 32],
                binding: ManagedCanisterBinding::Component(draining_partition.binding),
                covered_authority: directory_authority,
                covered_authority_hash: authority_hash,
                activation: ComponentRuntimeActivationEvidence {
                    directory_authority_hash: [101; 32],
                    activated_at_ns: 102,
                },
            },
            [103; 32],
            110,
            16_777_216,
        )
        .expect("prepare empty Component quiescence");
        ComponentRegistryOps::mark_component_quiescent(
            component,
            draining.operation_id,
            [103; 32],
            111,
        )
        .expect("observe empty Component quiescence");
        let partition = ComponentRegistryOps::partition(component)
            .expect("quiescent partition read")
            .expect("quiescent partition");
        (partition, draining, fleet)
    }

    fn advance_install_to_verified(plan: &RootComponentInstallPlan, created_bytes: u64) -> u64 {
        ComponentRegistryOps::validate_install_capacity([12; 32], plan).expect("install capacity");
        let intent = ComponentRegistryOps::begin_install(
            [12; 32],
            plan.clone(),
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(23),
                reservation_intent_id: IntentId(24),
            },
        )
        .expect("install intent");
        let intent_bytes = ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes;
        assert!(intent_bytes > created_bytes);
        assert!(matches!(
            intent.progress,
            RootComponentAllocationProgressView::InstallIntent { .. }
        ));

        let mut conflicting = plan.clone();
        conflicting.raw_module_hash = [25; 32];
        assert!(
            ComponentRegistryOps::renew_install_intent(
                [12; 32],
                &conflicting,
                ReplayCostGuardSettlement {
                    quota_intent_id: IntentId(26),
                    reservation_intent_id: IntentId(27),
                },
            )
            .is_err()
        );

        let interrupted = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(interrupted);
        let renewed = ComponentRegistryOps::renew_install_intent(
            [12; 32],
            plan,
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(28),
                reservation_intent_id: IntentId(29),
            },
        )
        .expect("renew exact install intent");
        let RootComponentAllocationProgressView::InstallIntent { installation, .. } =
            &renewed.progress
        else {
            panic!("renewed install intent");
        };
        assert_eq!(installation.raw_module_hash, plan.raw_module_hash);
        assert_eq!(installation.binding, plan.binding);
        assert_eq!(
            installation.cost_guard_settlement.quota_intent_id,
            IntentId(28)
        );
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            intent_bytes
        );

        let installed = ComponentRegistryOps::mark_installed([12; 32]).expect("mark installed");
        let installed_retry =
            ComponentRegistryOps::mark_installed([12; 32]).expect("installed retry");
        assert_eq!(installed, installed_retry);
        assert!(matches!(
            installed.progress,
            RootComponentAllocationProgressView::Installed { .. }
        ));

        let verified = ComponentRegistryOps::mark_verified([12; 32]).expect("mark verified");
        let verified_retry = ComponentRegistryOps::mark_verified([12; 32]).expect("verified retry");
        assert_eq!(verified, verified_retry);
        assert!(matches!(
            verified.progress,
            RootComponentAllocationProgressView::Verified { .. }
        ));
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            intent_bytes,
            "the install intent must reserve terminal record capacity before the effect"
        );
        intent_bytes
    }

    fn prepared_created_allocation() -> (
        FleetSubnetRootBinding,
        RootComponentAllocationView,
        candid::Principal,
    ) {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        ComponentRegistryOps::prepare(
            root.clone(),
            FleetRegistryVersion {
                authority: root.authority.clone(),
                revision: 4,
                content_hash: [5; 32],
            },
            FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [8; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
            },
            RootStoreBootstrapRequest {
                manifest_payload_size_bytes: 128,
            },
        )
        .expect("prepare");
        ComponentRegistryOps::reserve_allocation(
            TopLevelComponentAllocationDecision {
                allocation_sequence: 1,
                component: ComponentInstanceId::from_generated_bytes([10; 32]),
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [6; 32],
                role: CanisterRole::new("project_hub"),
            },
            [12; 32],
            ComponentProvisioningOrigin::FleetAdministrator {
                caller: candid::Principal::from_slice(&[11; 29]),
            },
            false,
        )
        .expect("reserve");
        ComponentRegistryOps::begin_creation(
            [12; 32],
            RootComponentCreationPlan {
                wasm_store: candid::Principal::from_slice(&[13; 29]),
                payload_hash: [14; 32],
                payload_size_bytes: 4_096,
                initial_cycles: Cycles::new(5_000_000_000_000),
                controller: root.fleet_subnet_root,
            },
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(16),
                reservation_intent_id: IntentId(17),
            },
        )
        .expect("creation intent");
        let canister = candid::Principal::from_slice(&[18; 29]);
        let created = ComponentRegistryOps::mark_created([12; 32], canister)
            .expect("record created allocation");
        (root, created, canister)
    }

    fn assert_creation_capacity_is_reserved_before_effect(
        plan: &RootComponentCreationPlan,
        reserved_bytes: u64,
    ) {
        let before_creation = RootComponentRegistryStore::export();
        let mut exhausted = before_creation.clone();
        exhausted
            .current
            .as_mut()
            .expect("Registry meta")
            .root
            .limits
            .maximum_registry_bytes = reserved_bytes;
        RootComponentRegistryStore::import(exhausted);

        let capacity_error = ComponentRegistryOps::validate_creation_capacity([12; 32], plan)
            .expect_err("terminal creation evidence must fit before the paid effect");
        assert!(capacity_error.is_public_resource_exhausted());
        assert!(matches!(
            ComponentRegistryOps::allocation([12; 32])
                .expect("reserved allocation")
                .progress,
            RootComponentAllocationProgressView::Reserved
        ));
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            reserved_bytes
        );
        RootComponentRegistryStore::import(before_creation);
    }

    #[test]
    fn allocation_reservation_fails_before_mutation_when_registry_capacity_is_exhausted() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let mut root = root_binding();
        root.limits.maximum_registry_bytes = 1;
        let version = FleetRegistryVersion {
            authority: root.authority.clone(),
            revision: 4,
            content_hash: [5; 32],
        };
        ComponentRegistryOps::prepare(
            root,
            version,
            FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [8; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
            },
            RootStoreBootstrapRequest {
                manifest_payload_size_bytes: 128,
            },
        )
        .expect("prepare");

        let error = ComponentRegistryOps::reserve_allocation(
            TopLevelComponentAllocationDecision {
                allocation_sequence: 1,
                component: ComponentInstanceId::from_generated_bytes([10; 32]),
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [6; 32],
                role: CanisterRole::new("project_hub"),
            },
            [12; 32],
            ComponentProvisioningOrigin::FleetAdministrator {
                caller: candid::Principal::from_slice(&[11; 29]),
            },
            false,
        )
        .expect_err("Registry byte capacity must reject reservation");
        assert!(error.is_public_resource_exhausted());
        assert!(ComponentRegistryOps::allocation([12; 32]).is_none());

        let status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(status.next_allocation_sequence, 1);
        assert_eq!(status.reserved_component_instances, 0);
        assert_eq!(status.encoded_bytes, 0);
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    struct ActiveComponentTreeFixture {
        component: ComponentInstanceId,
        partition: ComponentRegistryPartitionRecord,
        target: ComponentRegistryChildRecord,
        descendant: ComponentRegistryChildRecord,
        unrelated: ComponentRegistryChildRecord,
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the fixture assembles one exact normalized multi-level Component tree"
    )]
    fn import_active_component_tree() -> ActiveComponentTreeFixture {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        let component = ComponentInstanceId::from_generated_bytes([10; 32]);
        let component_canister = candid::Principal::from_slice(&[18; 29]);
        let mut partition =
            active_component_partition(&root, release_set, component, component_canister);
        let target = ComponentRegistryChildRecord {
            component,
            canister_id: candid::Principal::from_slice(&[21; 29]),
            parent_canister_id: component_canister,
            role: CanisterRole::new("project_instance"),
            kind: ComponentChildKind::Instance,
            installed_artifact_hash: [31; 32],
            status: ComponentLifecycleStatus::Active,
        };
        let descendant = ComponentRegistryChildRecord {
            component,
            canister_id: candid::Principal::from_slice(&[22; 29]),
            parent_canister_id: target.canister_id,
            role: CanisterRole::new("project_ledger"),
            kind: ComponentChildKind::Singleton,
            installed_artifact_hash: [32; 32],
            status: ComponentLifecycleStatus::Active,
        };
        let unrelated = ComponentRegistryChildRecord {
            component,
            canister_id: candid::Principal::from_slice(&[23; 29]),
            parent_canister_id: component_canister,
            role: CanisterRole::new("project_instance"),
            kind: ComponentChildKind::Instance,
            installed_artifact_hash: [33; 32],
            status: ComponentLifecycleStatus::Active,
        };
        let alternate_descendant = ComponentRegistryChildRecord {
            component,
            canister_id: candid::Principal::from_slice(&[24; 29]),
            parent_canister_id: target.canister_id,
            role: CanisterRole::new("project_machine"),
            kind: ComponentChildKind::Singleton,
            installed_artifact_hash: [34; 32],
            status: ComponentLifecycleStatus::Active,
        };
        let children = vec![
            target.clone(),
            descendant.clone(),
            unrelated.clone(),
            alternate_descendant,
        ];
        let child_traversals = children
            .iter()
            .map(|child| ComponentRegistryChildTraversalRecord {
                component,
                parent_canister_id: child.parent_canister_id,
                role: child.role.clone(),
                canister_id: child.canister_id,
            })
            .collect();
        let parent_role_counts = vec![
            ComponentRegistryParentRoleCountRecord {
                component,
                parent_canister_id: component_canister,
                child_role: CanisterRole::new("project_instance"),
                instances: 2,
            },
            ComponentRegistryParentRoleCountRecord {
                component,
                parent_canister_id: target.canister_id,
                child_role: CanisterRole::new("project_ledger"),
                instances: 1,
            },
            ComponentRegistryParentRoleCountRecord {
                component,
                parent_canister_id: target.canister_id,
                child_role: CanisterRole::new("project_machine"),
                instances: 1,
            },
        ];
        partition.committed_descendants = 4;
        partition.descendant_content_hash = [77; 32];
        partition.content_hash = component_partition_content_hash(
            &partition.binding,
            &partition.provisioning_origin,
            partition.release_set,
            partition.status,
            partition.revision,
            partition.descendant_content_hash,
            partition.committed_descendants,
        )
        .expect("nonempty partition hash");
        partition.encoded_bytes = 0;
        let mut data = RootComponentRegistryData {
            partitions: vec![partition.clone()],
            children,
            child_traversals,
            parent_role_counts,
            ..RootComponentRegistryData::default()
        };
        for _ in 0..8 {
            data.partitions[0] = partition.clone();
            let encoded_bytes = exact_component_registry_entry_bytes(&data, component);
            if partition.encoded_bytes == encoded_bytes {
                break;
            }
            partition.encoded_bytes = encoded_bytes;
        }
        data.partitions[0] = partition.clone();
        assert_eq!(
            partition.encoded_bytes,
            exact_component_registry_entry_bytes(&data, component)
        );
        data.current = Some(RootComponentRegistryMetaRecord {
            root: root.clone(),
            prepared_against_registry: FleetRegistryVersion {
                authority: root.authority,
                revision: 4,
                content_hash: [5; 32],
            },
            release_set,
            store_bootstrap: RootStoreBootstrapRequest {
                manifest_payload_size_bytes: 128,
            },
            next_allocation_sequence: 2,
            reserved_component_instances: 0,
            committed_component_instances: 1,
            managed_descendants: 4,
            known_created_component_canisters: 5,
            encoded_bytes: partition.encoded_bytes,
            initial_inventory: None,
        });
        RootComponentRegistryStore::import(data);
        ActiveComponentTreeFixture {
            component,
            partition,
            target,
            descendant,
            unrelated,
        }
    }

    fn import_deep_active_component_tree(
        depth: usize,
    ) -> (ActiveComponentTreeFixture, candid::Principal) {
        assert!(depth > 0 && depth <= 100);
        let mut fixture = import_active_component_tree();
        let mut data = RootComponentRegistryStore::export();
        let mut parent_canister_id = fixture.descendant.canister_id;
        let role = CanisterRole::new("deep_node");

        for index in 0..depth {
            let principal_byte = 100_u8
                .checked_add(u8::try_from(index).expect("bounded deep-tree index"))
                .expect("bounded deep-tree principal");
            let canister_id = candid::Principal::from_slice(&[principal_byte; 29]);
            let child = ComponentRegistryChildRecord {
                component: fixture.component,
                canister_id,
                parent_canister_id,
                role: role.clone(),
                kind: ComponentChildKind::Instance,
                installed_artifact_hash: [principal_byte; 32],
                status: ComponentLifecycleStatus::Active,
            };
            data.children.push(child);
            data.child_traversals
                .push(ComponentRegistryChildTraversalRecord {
                    component: fixture.component,
                    parent_canister_id,
                    role: role.clone(),
                    canister_id,
                });
            data.parent_role_counts
                .push(ComponentRegistryParentRoleCountRecord {
                    component: fixture.component,
                    parent_canister_id,
                    child_role: role.clone(),
                    instances: 1,
                });
            parent_canister_id = canister_id;
        }

        let partition = data
            .partitions
            .iter_mut()
            .find(|partition| partition.binding.component == fixture.component)
            .expect("deep-tree partition");
        partition.committed_descendants = partition
            .committed_descendants
            .checked_add(u32::try_from(depth).expect("bounded deep-tree depth"))
            .expect("deep-tree descendant count");
        partition.descendant_content_hash = [78; 32];
        partition.content_hash = component_partition_content_hash(
            &partition.binding,
            &partition.provisioning_origin,
            partition.release_set,
            partition.status,
            partition.revision,
            partition.descendant_content_hash,
            partition.committed_descendants,
        )
        .expect("deep-tree partition hash");
        partition.encoded_bytes = 0;
        for _ in 0..8 {
            let encoded_bytes = exact_component_registry_entry_bytes(&data, fixture.component);
            let partition = data
                .partitions
                .iter_mut()
                .find(|partition| partition.binding.component == fixture.component)
                .expect("deep-tree partition");
            if partition.encoded_bytes == encoded_bytes {
                break;
            }
            partition.encoded_bytes = encoded_bytes;
        }
        fixture.partition = data
            .partitions
            .iter()
            .find(|partition| partition.binding.component == fixture.component)
            .expect("deep-tree partition")
            .clone();
        assert_eq!(
            fixture.partition.encoded_bytes,
            exact_component_registry_entry_bytes(&data, fixture.component)
        );
        let depth = u32::try_from(depth).expect("bounded deep-tree depth");
        let encoded_bytes = exact_registry_entry_bytes(&data);
        let current = data.current.as_mut().expect("deep-tree Registry status");
        current.managed_descendants = current
            .managed_descendants
            .checked_add(depth)
            .expect("deep-tree managed count");
        current.known_created_component_canisters = current
            .known_created_component_canisters
            .checked_add(depth)
            .expect("deep-tree known-created count");
        current.encoded_bytes = encoded_bytes;
        RootComponentRegistryStore::import(data);
        (fixture, parent_canister_id)
    }

    fn active_component_partition(
        root: &FleetSubnetRootBinding,
        release_set: FleetSubnetRootReleaseSet,
        component: ComponentInstanceId,
        canister_id: candid::Principal,
    ) -> ComponentRegistryPartitionRecord {
        let binding = ComponentBinding {
            authority: root.authority.clone(),
            component,
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            canister_id,
        };
        let provisioning_origin = ComponentProvisioningOrigin::FleetAdministrator {
            caller: candid::Principal::from_slice(&[11; 29]),
        };
        let descendant_content_hash = empty_component_descendant_content_hash(component);
        let mut partition = ComponentRegistryPartitionRecord {
            content_hash: component_partition_content_hash(
                &binding,
                &provisioning_origin,
                release_set,
                ComponentLifecycleStatus::Active,
                2,
                descendant_content_hash,
                0,
            )
            .expect("partition hash"),
            binding,
            provisioning_origin,
            release_set,
            status: ComponentLifecycleStatus::Active,
            revision: 2,
            descendant_content_hash,
            directory_synchronized_at_ns: 33,
            reserved_descendants: 0,
            committed_descendants: 0,
            encoded_bytes: 0,
        };
        let principal_index_bytes =
            RootComponentRegistryStore::principal_index_entry_bytes(canister_id, component);
        for _ in 0..8 {
            let encoded_bytes = RootComponentRegistryStore::partition_entry_bytes(&partition)
                + principal_index_bytes;
            if partition.encoded_bytes == encoded_bytes {
                break;
            }
            partition.encoded_bytes = encoded_bytes;
        }
        assert_eq!(
            partition.encoded_bytes,
            RootComponentRegistryStore::partition_entry_bytes(&partition) + principal_index_bytes
        );
        partition
    }

    fn child_allocation_decision(
        partition: &ComponentRegistryPartitionRecord,
        child_role: &'static str,
    ) -> ComponentChildAllocationDecision {
        child_allocation_decision_for_parent(
            partition,
            partition.binding.canister_id,
            &partition.binding.role,
            child_role,
        )
    }

    fn child_allocation_decision_for_parent(
        partition: &ComponentRegistryPartitionRecord,
        parent_canister_id: candid::Principal,
        parent_role: &CanisterRole,
        child_role: &'static str,
    ) -> ComponentChildAllocationDecision {
        ComponentChildAllocationDecision {
            component: partition.binding.component,
            component_spec: partition.binding.component_spec.clone(),
            spec_hash: partition.binding.spec_hash,
            parent_canister_id,
            parent_role: parent_role.clone(),
            child_role: CanisterRole::new(child_role),
            child_kind: ComponentChildKind::Instance,
            maximum_instances_per_parent: 10_000,
            maximum_descendants: 20_000,
            maximum_registry_bytes: 16_777_216,
        }
    }

    fn component_registry_head(
        partition: &ComponentRegistryPartitionRecord,
    ) -> ComponentRegistryHead {
        ComponentRegistryHead {
            component: partition.binding.component,
            revision: partition.revision,
            content_hash: partition.content_hash,
        }
    }

    fn child_creation_plan(
        root: &FleetSubnetRootBinding,
        evidence_seed: u8,
    ) -> RootComponentCreationPlan {
        RootComponentCreationPlan {
            wasm_store: candid::Principal::from_slice(&[evidence_seed; 29]),
            payload_hash: [evidence_seed; 32],
            payload_size_bytes: 4_096,
            initial_cycles: Cycles::new(5_000_000_000_000),
            controller: root.fleet_subnet_root,
        }
    }

    fn root_binding() -> FleetSubnetRootBinding {
        let coordinator_subnet = SubnetId::from_principal(candid::Principal::from_slice(&[2; 29]));
        FleetSubnetRootBinding {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                            fleet_id: FleetId::from_generated_bytes([1; 32]),
                        },
                        app: AppId::from("toko"),
                    },
                    coordinator_subnet,
                    coordinator: candid::Principal::from_slice(&[3; 29]),
                },
                epoch: 1,
            },
            placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[4; 29])),
            fleet_subnet_root: candid::Principal::from_slice(&[5; 29]),
            component_admissions: vec![ComponentSpecAdmission {
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [6; 32],
                maximum_root_instances: 10,
            }],
            component_topology_digest: ComponentTopologyDigest::from_bytes([7; 32]),
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 10,
                maximum_managed_canisters: 20_000,
                maximum_registry_bytes: 16_777_216,
                maximum_wasm_store_bytes: 268_435_456,
                cycles_funding: CyclesFundingBudget {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(1_000_000_000_000),
                },
            },
        }
    }
}
