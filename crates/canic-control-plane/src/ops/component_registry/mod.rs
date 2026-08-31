//! Module: ops::component_registry
//!
//! Responsibility: read and commit Component Registry authority and lifecycle progress.
//! Does not own: Store side effects, Fleet Registry, topology, admission, or orchestration.
//! Boundary: converts stable records into read-only views before workflow use.

mod child_activation;
mod child_allocation;
mod component_retirement;
mod directory_refresh;
mod initial_inventory;
mod root_retirement;
mod subtree_retirement;
mod top_level_activation;
mod top_level_allocation;

use root_retirement::{
    root_store_binding_finalization_hash, root_store_deletion_hash, root_store_reclamation_hash,
};

#[cfg(test)]
use crate::ids::WasmStoreGcMode;
use crate::{
    ids::WasmStoreBinding,
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
        RootComponentMembershipRemovedRecord, RootComponentQuiescenceProgressRecord,
        RootComponentQuiescenceStopIntentRecord, RootComponentQuiescentReceiptRecord,
        RootComponentRegistryCommitError, RootComponentRegistryMetaRecord,
        RootComponentRegistryStore, RootComponentSubtreeDeleteEffectRecord,
        RootComponentSubtreeDeletedEffectRecord, RootComponentSubtreeDirectoryConvergenceRecord,
        RootComponentSubtreeDirectorySynchronizedRecord,
        RootComponentSubtreeMembershipRemovedRecord, RootComponentSubtreeRemovalBeginCommit,
        RootComponentSubtreeRemovalCompletedLeafRecord, RootComponentSubtreeRemovalCompletedRecord,
        RootComponentSubtreeRemovalProgressRecord, RootComponentSubtreeRemovalRecord,
        RootComponentSubtreeStopEffectRecord, RootComponentSubtreeStoppedEffectRecord,
        RootFleetSubnetDeletionPreparationIntentRecord, RootFleetSubnetDeletionPreparationRecord,
        RootFleetSubnetDrainingRecord, RootFleetSubnetFinalInventoryRecord,
        RootFleetSubnetRemovalPublicationRecord,
        RootFleetSubnetStoreBindingFinalizationIntentRecord,
        RootFleetSubnetStoreBindingFinalizationRecord, RootFleetSubnetStoreDeletionIntentRecord,
        RootFleetSubnetStoreDeletionRecord, RootFleetSubnetStoreReclamationIntentRecord,
        RootFleetSubnetStoreReclamationRecord,
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
        RootComponentMembershipRemovedView, RootComponentMembershipView,
        RootComponentQuiescenceProgressView, RootComponentQuiescenceStopIntentView,
        RootComponentQuiescentReceiptView, RootComponentRegistryView,
        RootComponentSubtreeDeleteEffectView, RootComponentSubtreeDeletedEffectView,
        RootComponentSubtreeDirectoryConvergenceView,
        RootComponentSubtreeDirectorySynchronizedView, RootComponentSubtreeMembershipRemovedView,
        RootComponentSubtreeRemovalCompletedView, RootComponentSubtreeRemovalNodeView,
        RootComponentSubtreeRemovalProgressView, RootComponentSubtreeRemovalView,
        RootComponentSubtreeStopEffectView, RootComponentSubtreeStoppedEffectView,
        RootFleetSubnetDeletionPreparationIntentView, RootFleetSubnetDeletionPreparationView,
        RootFleetSubnetDrainingView, RootFleetSubnetFinalInventoryView,
        RootFleetSubnetRemovalPublicationView, RootFleetSubnetStoreBindingFinalizationIntentView,
        RootFleetSubnetStoreBindingFinalizationView, RootFleetSubnetStoreDeletionIntentView,
        RootFleetSubnetStoreDeletionView, RootFleetSubnetStoreReclamationIntentView,
        RootFleetSubnetStoreReclamationView,
    },
};
use candid::CandidType;
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::{
        config::schema::ComponentChildKind,
        error::InternalError,
        model::replay::ReplayCostGuardSettlement,
        ops::{
            component_runtime::ComponentRuntimeOps,
            root_draining_reservation::FleetSubnetRootDrainingReservationOps,
        },
    },
    dto::{
        component_provisioning::ComponentGroupDirectory,
        component_registry::{
            ComponentDirectoryHead, ComponentDirectoryProvenance, ComponentLifecycleStatus,
            ComponentProvisioningOrigin, ComponentRegistryHead, ComponentRuntimeDirectoryAuthority,
            ComponentRuntimeDirectoryConvergenceEvidence,
        },
        fleet_registry::{FleetDirectorySnapshot, FleetRegistryVersion},
        fleet_subnet_root::FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES,
        root_store::RootStoreBootstrapRequest,
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentChildBinding, ComponentInstanceId,
        ComponentSpecId, FleetSubnetRootBinding, FleetSubnetRootReleaseSet, IntentId,
        ManagedCanisterBinding,
    },
    role_contract::ProtocolProfileDigest,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const SUBTREE_REMOVAL_TRAVERSAL_BATCH_SIZE: u32 = 64;
const COMPONENT_DRAINING_SUBTREE_OPERATION_DOMAIN: &[u8] =
    b"canic.component-draining.subtree-operation.v1";
const COMPONENT_FINAL_INVENTORY_HASH_DOMAIN: &[u8] = b"canic.component.final-inventory.v1";
const COMPONENT_MEMBERSHIP_REMOVAL_HASH_DOMAIN: &[u8] = b"canic.component.membership-removal.v1";
const ROOT_TERMINAL_COMPONENT_HISTORY_HASH_DOMAIN: &[u8] =
    b"canic.fleet-subnet-root.terminal-component-history.v1";
const ROOT_STORE_FINAL_CATALOG_HASH_DOMAIN: &[u8] = b"canic.fleet-subnet-root.store-catalog.v1";
const ROOT_FINAL_INVENTORY_HASH_DOMAIN: &[u8] = b"canic.fleet-subnet-root.final-inventory.v1";
const ROOT_STORE_RECLAMATION_HASH_DOMAIN: &[u8] = b"canic.fleet-subnet-root.store-reclamation.v1";
const ROOT_STORE_BINDING_FINALIZATION_HASH_DOMAIN: &[u8] =
    b"canic.fleet-subnet-root.store-binding-finalization.v1";
const ROOT_STORE_DELETION_HASH_DOMAIN: &[u8] = b"canic.fleet-subnet-root.store-deletion.v1";
const SECONDS_PER_DAY: u128 = 86_400;

fn deletion_retained_cycles_target(
    idle_cycles_burned_per_day: u128,
    freezing_threshold_seconds: u128,
) -> Option<u128> {
    idle_cycles_burned_per_day
        .checked_mul(freezing_threshold_seconds)?
        .div_ceil(SECONDS_PER_DAY)
        .checked_add(FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SubtreeRemovalOrigin {
    Ordinary,
    DrainingDriver,
}

#[derive(Clone, Copy)]
struct ComponentDirectoryAuthorityInput<'a> {
    synchronized_at_ns: u64,
    fleet: &'a FleetDirectorySnapshot,
    component_group: Option<&'a ComponentGroupDirectory>,
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
/// PeerComponentInstanceCounts
///
/// Root-local live target counts attributed to one exact requester Component.
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PeerComponentInstanceCounts {
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

///
/// RootFleetSubnetFinalInventoryPlan
///
/// Exact terminal Component authority frozen before the Store quiescence effect.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetSubnetFinalInventoryPlan {
    pub operation_id: [u8; 32],
    pub registry: FleetRegistryVersion,
    pub removed_component_instances: u32,
    pub terminal_component_history_hash: [u8; 32],
    pub root_registry_encoded_bytes: u64,
}

#[derive(CandidType)]
struct RootTerminalComponentHistoryHashEntry {
    allocation_sequence: u64,
    component: ComponentInstanceId,
    allocation_operation_id: [u8; 32],
    draining_operation_id: [u8; 32],
    membership_removal_hash: [u8; 32],
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

#[derive(CandidType)]
struct RootComponentMembershipRemovalHashAuthority {
    operation_id: [u8; 32],
    component: ComponentInstanceId,
    final_inventory_hash: [u8; 32],
    deleted_at_ns: u64,
    allocation_operation_id: [u8; 32],
    remaining_spec_committed_instances: u32,
    root_committed_component_instances: u32,
    root_known_created_component_canisters: u32,
    root_registry_encoded_bytes: u64,
    removed_at_ns: u64,
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

    const fn invalid() -> InternalError {
        InternalError::invariant()
    }
}

struct RootComponentDeletionAuthority<'a> {
    draining: &'a RootComponentDrainingRecord,
    progress: &'a RootComponentDeletionProgressRecord,
}

#[derive(Debug, Eq, PartialEq)]
struct ComponentAllocationPartitionAuthority<'a> {
    component: ComponentInstanceId,
    component_spec: &'a ComponentSpecId,
    spec_hash: [u8; 32],
    role: &'a CanisterRole,
    protocol_profile_digest: ProtocolProfileDigest,
    provisioning_origin: &'a ComponentProvisioningOrigin,
    release_set: FleetSubnetRootReleaseSet,
    binding: &'a ComponentBinding,
    canister: Principal,
}

impl<'a> ComponentAllocationPartitionAuthority<'a> {
    const fn from_partition(partition: &'a ComponentRegistryPartitionRecord) -> Self {
        Self {
            component: partition.binding.component,
            component_spec: &partition.binding.component_spec,
            spec_hash: partition.binding.spec_hash,
            role: &partition.binding.role,
            protocol_profile_digest: partition.protocol_profile_digest,
            provisioning_origin: &partition.provisioning_origin,
            release_set: partition.release_set,
            binding: &partition.binding,
            canister: partition.binding.canister_id,
        }
    }

    const fn from_committed_allocation(record: &'a RootComponentAllocationRecord) -> Option<Self> {
        let RootComponentAllocationProgressRecord::Committed {
            canister,
            installation,
            ..
        } = &record.progress
        else {
            return None;
        };
        Some(Self::from_allocation(record, installation, *canister))
    }

    const fn from_removed_allocation(record: &'a RootComponentAllocationRecord) -> Option<Self> {
        let RootComponentAllocationProgressRecord::Removed {
            canister,
            installation,
            ..
        } = &record.progress
        else {
            return None;
        };
        Some(Self::from_allocation(record, installation, *canister))
    }

    const fn from_allocation(
        record: &'a RootComponentAllocationRecord,
        installation: &'a RootComponentInstallEffectRecord,
        canister: Principal,
    ) -> Self {
        Self {
            component: record.component,
            component_spec: &record.component_spec,
            spec_hash: record.spec_hash,
            role: &record.role,
            protocol_profile_digest: installation.protocol_profile_digest,
            provisioning_origin: &record.provisioning_origin,
            release_set: record.release_set,
            binding: &installation.binding,
            canister,
        }
    }
}

struct ComponentMembershipRemovalRecords {
    next_meta: RootComponentRegistryMetaRecord,
    next_allocation: RootComponentAllocationRecord,
    next_draining: RootComponentDrainingRecord,
}

impl RootComponentDeletionAuthority<'_> {
    fn validate(&self) -> Result<(), InternalError> {
        let final_inventory = self
            .draining
            .final_inventory
            .as_ref()
            .ok_or_else(Self::invalid)?;
        let quiescence = terminal_component_quiescence(self.draining).ok_or_else(Self::invalid)?;
        let (intent, deleted_at_ns, removed_at_ns) = match self.progress {
            RootComponentDeletionProgressRecord::DeleteIntent(intent) => (intent, None, None),
            RootComponentDeletionProgressRecord::Deleted(receipt) => {
                (&receipt.deletion, Some(receipt.deleted_at_ns), None)
            }
            RootComponentDeletionProgressRecord::MembershipRemoved(receipt) => (
                &receipt.deleted.deletion,
                Some(receipt.deleted.deleted_at_ns),
                Some(receipt.removed_at_ns),
            ),
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
        if removed_at_ns
            .zip(deleted_at_ns)
            .is_some_and(|(removed_at_ns, deleted_at_ns)| removed_at_ns < deleted_at_ns)
        {
            return Err(Self::invalid());
        }
        if RootComponentRegistryStore::component_draining_entry_bytes(self.draining)
            > quiescence.stop.charged_entry_bytes
        {
            return Err(Self::invalid());
        }
        Ok(())
    }

    const fn invalid() -> InternalError {
        InternalError::invariant()
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
    pub protocol_profile_digest: ProtocolProfileDigest,
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentBinding,
    pub maximum_registry_bytes: u64,
}

#[derive(Debug, Eq, PartialEq)]
struct RootComponentInstallAuthority<'a> {
    raw_module_hash: [u8; 32],
    protocol_profile_digest: ProtocolProfileDigest,
    chunk_hashes: &'a [Vec<u8>],
    binding: &'a ComponentBinding,
}

impl<'a> From<&'a RootComponentInstallPlan> for RootComponentInstallAuthority<'a> {
    fn from(plan: &'a RootComponentInstallPlan) -> Self {
        Self {
            raw_module_hash: plan.raw_module_hash,
            protocol_profile_digest: plan.protocol_profile_digest,
            chunk_hashes: &plan.chunk_hashes,
            binding: &plan.binding,
        }
    }
}

impl<'a> From<&'a RootComponentInstallEffectView> for RootComponentInstallAuthority<'a> {
    fn from(effect: &'a RootComponentInstallEffectView) -> Self {
        Self {
            raw_module_hash: effect.raw_module_hash,
            protocol_profile_digest: effect.protocol_profile_digest,
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
    pub protocol_profile_digest: ProtocolProfileDigest,
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentChildBinding,
    pub maximum_registry_bytes: u64,
}

#[derive(Debug, Eq, PartialEq)]
struct RootComponentChildInstallAuthority<'a> {
    raw_module_hash: [u8; 32],
    protocol_profile_digest: ProtocolProfileDigest,
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
            protocol_profile_digest: plan.protocol_profile_digest,
            chunk_hashes: &plan.chunk_hashes,
            binding: &plan.binding,
        }
    }
}

impl<'a> From<&'a RootComponentChildInstallEffectView> for RootComponentChildInstallAuthority<'a> {
    fn from(effect: &'a RootComponentChildInstallEffectView) -> Self {
        Self {
            raw_module_hash: effect.raw_module_hash,
            protocol_profile_digest: effect.protocol_profile_digest,
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

    /// Return every current top-level Component Canister in canonical principal order.
    pub(crate) fn root_component_canisters() -> Result<Vec<Principal>, InternalError> {
        let partitions = RootComponentRegistryStore::partitions();
        let mut canisters = BTreeSet::new();
        for partition in partitions {
            validate_partition_record(&partition)?;
            if !canisters.insert(partition.binding.canister_id) {
                return Err(InternalError::invariant());
            }
        }
        Ok(canisters.into_iter().collect())
    }

    /// Return every current top-level Component partition in canonical identity order.
    pub(crate) fn root_component_partitions()
    -> Result<Vec<ComponentRegistryPartitionView>, InternalError> {
        let mut partitions = RootComponentRegistryStore::partitions();
        partitions.sort_by_key(|partition| partition.binding.component);
        partitions
            .into_iter()
            .map(|partition| {
                validate_partition_record(&partition)?;
                Ok(partition_record_to_view(partition))
            })
            .collect()
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
            root_draining: None,
        };
        RootComponentRegistryStore::prepare(record.clone()).map_err(|error| match error {
            RootComponentRegistryCommitError::ConflictingState => InternalError::conflict(),
        })?;
        Ok(record_to_view(record))
    }

    /// Prove that no retained Component operation can still change admission participants.
    pub(crate) fn require_admission_catalog_stable() -> Result<(), InternalError> {
        for allocation in RootComponentRegistryStore::allocations() {
            if allocation.operation_id == [0; 32]
                || !matches!(
                    allocation.progress,
                    RootComponentAllocationProgressRecord::Committed { .. }
                        | RootComponentAllocationProgressRecord::Removed { .. }
                )
            {
                return Err(InternalError::conflict());
            }
        }
        for draining in RootComponentRegistryStore::component_drainings() {
            match RootComponentRegistryStore::partition(draining.component) {
                Some(partition) => {
                    validate_partition_record(&partition)?;
                    validate_component_draining_record(&partition, &draining)?;
                    return Err(InternalError::conflict());
                }
                None => validate_removed_component_authority(&draining)?,
            }
        }
        for component in RootComponentRegistryStore::registry_components() {
            for allocation in RootComponentRegistryStore::child_allocations(component) {
                validate_child_allocation_record(&allocation)?;
                if !matches!(
                    allocation.progress,
                    RootComponentChildAllocationProgressRecord::Committed { .. }
                ) {
                    return Err(InternalError::conflict());
                }
            }
            for removal in RootComponentRegistryStore::subtree_removals(component) {
                validate_subtree_removal_record(&removal)?;
                if !matches!(
                    removal.progress,
                    RootComponentSubtreeRemovalProgressRecord::Completed(_)
                ) {
                    return Err(InternalError::conflict());
                }
            }
        }
        Ok(())
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
            return Err(InternalError::invalid_input());
        }
        if selection.start_after.as_ref().is_some_and(|cursor| {
            selection
                .parent_canister_id
                .is_some_and(|parent| cursor.parent_canister_id != parent)
        }) {
            return Err(InternalError::invalid_input());
        }
        if selection.start_after.as_ref().is_some_and(|cursor| {
            selection.parent_canister_id.is_some()
                && selection
                    .role
                    .as_ref()
                    .is_some_and(|role| role != &cursor.role)
        }) {
            return Err(InternalError::invalid_input());
        }

        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
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
                .ok_or_else(InternalError::invariant)?;
            validate_child_record(&partition, &child)?;
            if ComponentTreeNodeIdentity::from_traversal(&traversal)
                != ComponentTreeNodeIdentity::from_child(&child)
                || RootComponentRegistryStore::component_for_principal(traversal.parent_canister_id)
                    != Some(component)
            {
                return Err(InternalError::invariant());
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
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let RootComponentAllocationProgressRecord::Committed { commitment, .. } = &record.progress
        else {
            return Err(InternalError::conflict());
        };
        exact_committed_partition(&record, commitment).map(partition_record_to_view)
    }

    pub(crate) fn committed_child_authority(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        fleet_directory: &FleetDirectorySnapshot,
        component_group: Option<&ComponentGroupDirectory>,
    ) -> Result<
        (
            RootComponentChildAllocationView,
            ComponentRegistryPartitionView,
        ),
        InternalError,
    > {
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let RootComponentChildAllocationProgressRecord::Committed { commitment, .. } =
            &record.progress
        else {
            return Err(InternalError::conflict());
        };
        let committed = exact_committed_child_partition(&record, commitment)?;
        validate_child_directory_authority_hash(
            &committed,
            fleet_directory,
            component_group,
            commitment,
        )?;
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
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::invariant)?;
        validate_partition_record(&partition)?;
        if partition.binding.canister_id == canister {
            return Ok(Some((
                ManagedCanisterBinding::Component(partition.binding),
                partition.status,
            )));
        }
        let child = RootComponentRegistryStore::child(component, canister)
            .ok_or_else(InternalError::invariant)?;
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
            return Err(InternalError::invariant());
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

    /// Resolve the immutable install/runtime operation owned by one active managed binding.
    pub(crate) fn managed_runtime_operation_id(
        binding: &ManagedCanisterBinding,
    ) -> Result<[u8; 32], InternalError> {
        let (component, canister) = match binding {
            ManagedCanisterBinding::Component(binding) => {
                let partition = RootComponentRegistryStore::partition(binding.component)
                    .ok_or_else(InternalError::invariant)?;
                validate_partition_record(&partition)?;
                if &partition.binding != binding {
                    return Err(InternalError::conflict());
                }
                let allocation = committed_component_allocation(&partition)?;
                return Ok(allocation.operation_id);
            }
            ManagedCanisterBinding::ComponentChild(binding) => {
                (binding.component.component, binding.canister_id)
            }
        };

        let Some((registered, status)) = Self::registered_parent(component, canister)? else {
            return Err(InternalError::unavailable());
        };
        if &registered != binding || status != ComponentLifecycleStatus::Active {
            return Err(InternalError::conflict());
        }
        let mut matches = RootComponentRegistryStore::child_allocations(component)
            .into_iter()
            .filter(|record| {
                matches!(
                    &record.progress,
                    RootComponentChildAllocationProgressRecord::Committed {
                        canister: committed,
                        ..
                    } if *committed == canister
                )
            });
        let record = matches.next().ok_or_else(InternalError::invariant)?;
        if matches.next().is_some() {
            return Err(InternalError::invariant());
        }
        validate_child_allocation_record(&record)?;
        Ok(record.operation_id)
    }
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
        root_draining: record.root_draining.map(root_draining_record_to_view),
    }
}

fn validate_root_draining_record(
    current: &RootComponentRegistryMetaRecord,
    record: &RootFleetSubnetDrainingRecord,
) -> Result<(), InternalError> {
    if !record.is_valid_for_current(current) {
        return Err(InternalError::invariant());
    }
    if FleetSubnetRootDrainingReservationOps::content_hash(&record.reservation)?
        != record.reservation.reservation_hash
    {
        return Err(InternalError::invariant());
    }
    if let Some(reclamation) = record.store_reclamation.as_ref() {
        let expected_hash = root_store_reclamation_hash(reclamation)?;
        if reclamation.reclamation_hash != expected_hash {
            return Err(InternalError::invariant());
        }
    }
    if let Some(finalization) = record.store_binding_finalization.as_ref() {
        let expected_hash = root_store_binding_finalization_hash(finalization)?;
        if finalization.finalization_hash != expected_hash {
            return Err(InternalError::invariant());
        }
    }
    if let Some(deletion) = record.store_deletion.as_ref() {
        let expected_hash = root_store_deletion_hash(deletion)?;
        if deletion.deletion_hash != expected_hash {
            return Err(InternalError::invariant());
        }
    }
    Ok(())
}

fn terminal_root_inventory_plan(
    current: &RootComponentRegistryMetaRecord,
    draining: &RootFleetSubnetDrainingRecord,
    operation_id: [u8; 32],
    expected_registry: &FleetRegistryVersion,
) -> Result<RootFleetSubnetFinalInventoryPlan, InternalError> {
    if draining.funding_fenced_at_ns.is_none() {
        return Err(InternalError::conflict());
    }
    ensure_terminal_root_request(draining, operation_id, expected_registry)?;
    ensure_terminal_root_counters(current)?;
    ensure_terminal_root_indexes_are_empty()?;
    let history = terminal_root_component_history()?;
    let expected_next_sequence = u64::from(history.removed_component_instances)
        .checked_add(1)
        .ok_or_else(InternalError::invariant)?;
    if current.next_allocation_sequence != expected_next_sequence {
        return Err(InternalError::invariant());
    }

    let retained_registry_components: BTreeSet<_> =
        RootComponentRegistryStore::registry_components()
            .into_iter()
            .collect();
    if !retained_registry_components.is_subset(&history.components) {
        return Err(InternalError::invariant());
    }
    if history.registry_bytes != current.encoded_bytes {
        return Err(InternalError::invariant());
    }

    Ok(RootFleetSubnetFinalInventoryPlan {
        operation_id,
        registry: expected_registry.clone(),
        removed_component_instances: history.removed_component_instances,
        terminal_component_history_hash: history.hash,
        root_registry_encoded_bytes: current.encoded_bytes,
    })
}

struct TerminalRootComponentHistory {
    removed_component_instances: u32,
    hash: [u8; 32],
    registry_bytes: u64,
    components: BTreeSet<ComponentInstanceId>,
}

fn terminal_root_component_history() -> Result<TerminalRootComponentHistory, InternalError> {
    let mut allocations = RootComponentRegistryStore::allocations();
    allocations.sort_by_key(|allocation| allocation.allocation_sequence);
    let removed_component_instances =
        u32::try_from(allocations.len()).map_err(|_| InternalError::invariant())?;
    let mut drainings: BTreeMap<_, _> = RootComponentRegistryStore::component_drainings()
        .into_iter()
        .map(|record| (record.component, record))
        .collect();
    if drainings.len() != allocations.len() {
        return Err(InternalError::invariant());
    }

    let mut hash_entries = Vec::with_capacity(allocations.len());
    let mut registry_bytes = 0_u64;
    let mut components = BTreeSet::new();
    for (index, allocation) in allocations.iter().enumerate() {
        ensure_terminal_allocation_sequence(index, allocation)?;
        let draining = drainings
            .remove(&allocation.component)
            .ok_or_else(InternalError::invariant)?;
        validate_removed_component_authority(&draining)?;
        let receipt = removed_component_membership_receipt(&draining)?;
        registry_bytes = registry_bytes
            .checked_add(terminal_component_registry_bytes(allocation, &draining)?)
            .ok_or_else(InternalError::invariant)?;
        components.insert(allocation.component);
        hash_entries.push(RootTerminalComponentHistoryHashEntry {
            allocation_sequence: allocation.allocation_sequence,
            component: allocation.component,
            allocation_operation_id: allocation.operation_id,
            draining_operation_id: draining.operation_id,
            membership_removal_hash: receipt.removal_hash,
        });
    }
    Ok(TerminalRootComponentHistory {
        removed_component_instances,
        hash: terminal_component_history_hash(&hash_entries)?,
        registry_bytes,
        components,
    })
}

fn ensure_terminal_allocation_sequence(
    index: usize,
    allocation: &RootComponentAllocationRecord,
) -> Result<(), InternalError> {
    let sequence = u64::try_from(index)
        .ok()
        .and_then(|index| index.checked_add(1))
        .ok_or_else(InternalError::invariant)?;
    let allocation_is_terminal = [
        allocation.allocation_sequence == sequence,
        matches!(
            allocation.progress,
            RootComponentAllocationProgressRecord::Removed { .. }
        ),
    ]
    .into_iter()
    .all(|valid| valid);
    if !allocation_is_terminal {
        return Err(InternalError::unavailable());
    }
    Ok(())
}

fn ensure_terminal_root_request(
    draining: &RootFleetSubnetDrainingRecord,
    operation_id: [u8; 32],
    expected_registry: &FleetRegistryVersion,
) -> Result<(), InternalError> {
    if operation_id == [0; 32] {
        return Err(InternalError::invalid_input());
    }
    if operation_id != draining.operation_id {
        return Err(InternalError::conflict());
    }
    let publication_is_current_or_later = ComponentRegistryOps::registry_covers_preparation(
        &draining.active_registry,
        expected_registry,
    );
    if !publication_is_current_or_later {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn ensure_terminal_root_counters(
    current: &RootComponentRegistryMetaRecord,
) -> Result<(), InternalError> {
    let counters_are_empty = [
        current.reserved_component_instances,
        current.committed_component_instances,
        current.managed_descendants,
        current.known_created_component_canisters,
    ]
    .into_iter()
    .all(|count| count == 0);
    if !counters_are_empty {
        return Err(InternalError::unavailable());
    }
    Ok(())
}

fn ensure_terminal_root_indexes_are_empty() -> Result<(), InternalError> {
    let indexes_are_empty = [
        RootComponentRegistryStore::partitions().is_empty(),
        RootComponentRegistryStore::principal_inventory_is_empty(),
    ]
    .into_iter()
    .all(|empty| empty);
    if !indexes_are_empty {
        return Err(InternalError::unavailable());
    }
    Ok(())
}

fn terminal_component_registry_bytes(
    allocation: &RootComponentAllocationRecord,
    draining: &RootComponentDrainingRecord,
) -> Result<u64, InternalError> {
    let component = allocation.component;
    let child_allocations = RootComponentRegistryStore::child_allocations(component);
    let subtree_removals = RootComponentRegistryStore::subtree_removals(component);
    let subtree_history = RootComponentRegistryStore::subtree_removal_history(component);
    let mut entries = std::iter::once(RootComponentRegistryStore::allocation_entry_bytes(
        allocation,
    ))
    .chain(
        child_allocations
            .iter()
            .map(charged_child_allocation_entry_bytes),
    )
    .chain(
        subtree_removals
            .iter()
            .map(RootComponentRegistryStore::subtree_removal_entry_bytes),
    )
    .chain(
        subtree_history
            .iter()
            .map(RootComponentRegistryStore::subtree_removal_completed_leaf_entry_bytes),
    )
    .chain(std::iter::once(charged_component_draining_entry_bytes(
        draining,
    )));
    entries.try_fold(0_u64, |total, bytes| {
        total
            .checked_add(bytes)
            .ok_or_else(InternalError::invariant)
    })
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

const fn removed_component_membership_receipt(
    draining: &RootComponentDrainingRecord,
) -> Result<&RootComponentMembershipRemovedRecord, InternalError> {
    match draining.deletion.as_ref() {
        Some(RootComponentDeletionProgressRecord::MembershipRemoved(receipt)) => Ok(receipt),
        Some(
            RootComponentDeletionProgressRecord::DeleteIntent(_)
            | RootComponentDeletionProgressRecord::Deleted(_),
        )
        | None => Err(InternalError::invariant()),
    }
}

fn terminal_component_history_hash(
    entries: &[RootTerminalComponentHistoryHashEntry],
) -> Result<[u8; 32], InternalError> {
    let payload = candid::encode_one(entries).map_err(|_error| InternalError::invariant())?;
    Ok(domain_hash(
        ROOT_TERMINAL_COMPONENT_HISTORY_HASH_DOMAIN,
        &payload,
    ))
}

fn domain_hash(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    hasher.finalize().into()
}

const fn ensure_root_accepts_top_level_allocation(
    current: &RootComponentRegistryMetaRecord,
) -> Result<(), InternalError> {
    if current.root_draining.is_some() {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn root_draining_record_to_view(
    record: RootFleetSubnetDrainingRecord,
) -> RootFleetSubnetDrainingView {
    RootFleetSubnetDrainingView {
        operation_id: record.operation_id,
        fleet_subnet_root: record.fleet_subnet_root,
        placement_subnet: record.placement_subnet,
        active_registry: record.active_registry,
        reservation_hash: record.reservation.reservation_hash,
        component_topology_digest: record.component_topology_digest,
        active_release_set: record.active_release_set,
        next_allocation_sequence: record.next_allocation_sequence,
        reserved_component_instances: record.reserved_component_instances,
        committed_component_instances: record.committed_component_instances,
        managed_descendants: record.managed_descendants,
        known_created_component_canisters: record.known_created_component_canisters,
        root_registry_encoded_bytes: record.root_registry_encoded_bytes,
        started_at_ns: record.started_at_ns,
        funding_fenced_at_ns: record.funding_fenced_at_ns,
        final_inventory: record
            .final_inventory
            .map(root_final_inventory_record_to_view),
        removal_publication: record
            .removal_publication
            .map(root_removal_publication_record_to_view),
        store_reclamation_intent: record
            .store_reclamation_intent
            .map(root_store_reclamation_intent_record_to_view),
        store_reclamation: record
            .store_reclamation
            .map(root_store_reclamation_record_to_view),
        store_binding_finalization_intent: record
            .store_binding_finalization_intent
            .map(root_store_binding_finalization_intent_record_to_view),
        store_binding_finalization: record
            .store_binding_finalization
            .map(root_store_binding_finalization_record_to_view),
        store_deletion_intent: record
            .store_deletion_intent
            .map(root_store_deletion_intent_record_to_view),
        store_deletion: record
            .store_deletion
            .map(root_store_deletion_record_to_view),
        root_deletion_preparation_intent: record
            .root_deletion_preparation_intent
            .map(root_deletion_preparation_intent_record_to_view),
        root_deletion_preparation: record
            .root_deletion_preparation
            .map(root_deletion_preparation_record_to_view),
    }
}

fn root_final_inventory_record_to_view(
    record: RootFleetSubnetFinalInventoryRecord,
) -> RootFleetSubnetFinalInventoryView {
    RootFleetSubnetFinalInventoryView {
        operation_id: record.operation_id,
        fleet_subnet_root: record.fleet_subnet_root,
        placement_subnet: record.placement_subnet,
        registry: record.registry,
        component_topology_digest: record.component_topology_digest,
        active_release_set: record.active_release_set,
        next_allocation_sequence: record.next_allocation_sequence,
        removed_component_instances: record.removed_component_instances,
        terminal_component_history_hash: record.terminal_component_history_hash,
        root_registry_encoded_bytes: record.root_registry_encoded_bytes,
        wasm_store: record.wasm_store,
        wasm_store_catalog_hash: record.wasm_store_catalog_hash,
        wasm_store_catalog_entries: record.wasm_store_catalog_entries,
        wasm_store_occupied_bytes: record.wasm_store_occupied_bytes,
        wasm_store_template_count: record.wasm_store_template_count,
        wasm_store_release_count: record.wasm_store_release_count,
        wasm_store_gc_prepared_at_secs: record.wasm_store_gc_prepared_at_secs,
        finalized_at_ns: record.finalized_at_ns,
        inventory_hash: record.inventory_hash,
    }
}

fn root_removal_publication_record_to_view(
    record: RootFleetSubnetRemovalPublicationRecord,
) -> RootFleetSubnetRemovalPublicationView {
    RootFleetSubnetRemovalPublicationView {
        operation_id: record.operation_id,
        final_inventory_hash: record.final_inventory_hash,
        previous_registry: record.previous_registry,
        registry: record.registry,
        recorded_at_ns: record.recorded_at_ns,
    }
}

const fn root_store_reclamation_intent_record_to_view(
    record: RootFleetSubnetStoreReclamationIntentRecord,
) -> RootFleetSubnetStoreReclamationIntentView {
    RootFleetSubnetStoreReclamationIntentView {
        operation_id: record.operation_id,
        final_inventory_hash: record.final_inventory_hash,
        wasm_store: record.wasm_store,
        prepared_at_ns: record.prepared_at_ns,
    }
}

const fn root_store_reclamation_record_to_view(
    record: RootFleetSubnetStoreReclamationRecord,
) -> RootFleetSubnetStoreReclamationView {
    RootFleetSubnetStoreReclamationView {
        operation_id: record.operation_id,
        fleet_subnet_root: record.fleet_subnet_root,
        wasm_store: record.wasm_store,
        final_inventory_hash: record.final_inventory_hash,
        reclaimed_store_bytes: record.reclaimed_store_bytes,
        reclaimed_catalog_entries: record.reclaimed_catalog_entries,
        reclaimed_template_count: record.reclaimed_template_count,
        reclaimed_release_count: record.reclaimed_release_count,
        gc_prepared_at_secs: record.gc_prepared_at_secs,
        gc_started_at_secs: record.gc_started_at_secs,
        gc_completed_at_secs: record.gc_completed_at_secs,
        gc_runs_completed: record.gc_runs_completed,
        completed_at_ns: record.completed_at_ns,
        reclamation_hash: record.reclamation_hash,
    }
}

fn root_store_binding_finalization_intent_record_to_view(
    record: RootFleetSubnetStoreBindingFinalizationIntentRecord,
) -> RootFleetSubnetStoreBindingFinalizationIntentView {
    RootFleetSubnetStoreBindingFinalizationIntentView {
        operation_id: record.operation_id,
        final_inventory_hash: record.final_inventory_hash,
        reclamation_hash: record.reclamation_hash,
        wasm_store: record.wasm_store,
        binding: WasmStoreBinding::owned(record.binding),
        source_generation: record.source_generation,
        prepared_at_ns: record.prepared_at_ns,
    }
}

fn root_store_binding_finalization_record_to_view(
    record: RootFleetSubnetStoreBindingFinalizationRecord,
) -> RootFleetSubnetStoreBindingFinalizationView {
    RootFleetSubnetStoreBindingFinalizationView {
        operation_id: record.operation_id,
        fleet_subnet_root: record.fleet_subnet_root,
        wasm_store: record.wasm_store,
        binding: WasmStoreBinding::owned(record.binding),
        final_inventory_hash: record.final_inventory_hash,
        reclamation_hash: record.reclamation_hash,
        source_generation: record.source_generation,
        finalized_generation: record.finalized_generation,
        finalized_at_secs: record.finalized_at_secs,
        completed_at_ns: record.completed_at_ns,
        finalization_hash: record.finalization_hash,
    }
}

fn root_store_deletion_intent_record_to_view(
    record: RootFleetSubnetStoreDeletionIntentRecord,
) -> RootFleetSubnetStoreDeletionIntentView {
    RootFleetSubnetStoreDeletionIntentView {
        operation_id: record.operation_id,
        binding_finalization_hash: record.binding_finalization_hash,
        wasm_store: record.wasm_store,
        binding: WasmStoreBinding::owned(record.binding),
        observed_module_hash: record.observed_module_hash,
        observed_controllers: record.observed_controllers,
        observed_cycles_before_reclamation: record.observed_cycles_before_reclamation,
        retained_cycles_target: record.retained_cycles_target,
        observed_cycles_after_reclamation: record.observed_cycles_after_reclamation,
        cycles_reclaimed_at_ns: record.cycles_reclaimed_at_ns,
        prepared_at_ns: record.prepared_at_ns,
    }
}

fn root_store_deletion_record_to_view(
    record: RootFleetSubnetStoreDeletionRecord,
) -> RootFleetSubnetStoreDeletionView {
    RootFleetSubnetStoreDeletionView {
        operation_id: record.operation_id,
        fleet_subnet_root: record.fleet_subnet_root,
        wasm_store: record.wasm_store,
        binding: WasmStoreBinding::owned(record.binding),
        binding_finalization_hash: record.binding_finalization_hash,
        observed_module_hash: record.observed_module_hash,
        observed_controllers: record.observed_controllers,
        observed_cycles_before_reclamation: record.observed_cycles_before_reclamation,
        retained_cycles_target: record.retained_cycles_target,
        observed_cycles_after_reclamation: record.observed_cycles_after_reclamation,
        cycles_reclaimed_at_ns: record.cycles_reclaimed_at_ns,
        prepared_at_ns: record.prepared_at_ns,
        observed_absent_at_ns: record.observed_absent_at_ns,
        completed_at_ns: record.completed_at_ns,
        deletion_hash: record.deletion_hash,
    }
}

const fn root_deletion_preparation_intent_record_to_view(
    record: RootFleetSubnetDeletionPreparationIntentRecord,
) -> RootFleetSubnetDeletionPreparationIntentView {
    RootFleetSubnetDeletionPreparationIntentView {
        operation_id: record.operation_id,
        coordinator: record.coordinator,
        final_inventory_hash: record.final_inventory_hash,
        store_deletion_hash: record.store_deletion_hash,
        observed_cycles_before_reclamation: record.observed_cycles_before_reclamation,
        retained_cycles_target: record.retained_cycles_target,
        observed_reserved_cycles: record.observed_reserved_cycles,
        observed_idle_cycles_burned_per_day: record.observed_idle_cycles_burned_per_day,
        observed_freezing_threshold_seconds: record.observed_freezing_threshold_seconds,
        coordinator_intent_hash: record.coordinator_intent_hash,
        observed_cycles_after_reclamation: record.observed_cycles_after_reclamation,
        cycles_reclaimed_at_ns: record.cycles_reclaimed_at_ns,
        prepared_at_ns: record.prepared_at_ns,
    }
}

const fn root_deletion_preparation_record_to_view(
    record: RootFleetSubnetDeletionPreparationRecord,
) -> RootFleetSubnetDeletionPreparationView {
    RootFleetSubnetDeletionPreparationView {
        operation_id: record.operation_id,
        fleet_subnet_root: record.fleet_subnet_root,
        coordinator: record.coordinator,
        final_inventory_hash: record.final_inventory_hash,
        store_deletion_hash: record.store_deletion_hash,
        observed_cycles_before_reclamation: record.observed_cycles_before_reclamation,
        retained_cycles_target: record.retained_cycles_target,
        observed_reserved_cycles: record.observed_reserved_cycles,
        observed_idle_cycles_burned_per_day: record.observed_idle_cycles_burned_per_day,
        observed_freezing_threshold_seconds: record.observed_freezing_threshold_seconds,
        observed_cycles_after_reclamation: record.observed_cycles_after_reclamation,
        cycles_reclaimed_at_ns: record.cycles_reclaimed_at_ns,
        coordinator_intent_hash: record.coordinator_intent_hash,
        coordinator_readiness_hash: record.coordinator_readiness_hash,
        prepared_at_ns: record.prepared_at_ns,
        completed_at_ns: record.completed_at_ns,
    }
}

fn root_final_inventory_record_matches_response(
    record: &RootFleetSubnetFinalInventoryRecord,
    response: &canic_core::dto::fleet_subnet_root::FleetSubnetRootFinalInventoryResponse,
) -> bool {
    [
        response.operation_id == record.operation_id,
        response.fleet_subnet_root == record.fleet_subnet_root,
        response.placement_subnet == record.placement_subnet,
        response.registry == record.registry,
        response.component_topology_digest == record.component_topology_digest,
        response.active_release_set == record.active_release_set,
        response.next_allocation_sequence == record.next_allocation_sequence,
        response.removed_component_instances == record.removed_component_instances,
        response.terminal_component_history_hash == record.terminal_component_history_hash,
        response.root_registry_encoded_bytes == record.root_registry_encoded_bytes,
        response.wasm_store == record.wasm_store,
        response.wasm_store_catalog_hash == record.wasm_store_catalog_hash,
        response.wasm_store_catalog_entries == record.wasm_store_catalog_entries,
        response.wasm_store_occupied_bytes == record.wasm_store_occupied_bytes,
        response.wasm_store_template_count == record.wasm_store_template_count,
        response.wasm_store_release_count == record.wasm_store_release_count,
        response.wasm_store_gc_prepared_at_secs == record.wasm_store_gc_prepared_at_secs,
        response.finalized_at_ns == record.finalized_at_ns,
        response.inventory_hash == record.inventory_hash,
    ]
    .into_iter()
    .all(|valid| valid)
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
            RootComponentAllocationProgressRecord::Removed {
                creation,
                canister,
                installation,
                commitment,
            } => RootComponentAllocationProgressView::Removed {
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
        protocol_profile_digest: effect.protocol_profile_digest,
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
        protocol_profile_digest: record.protocol_profile_digest,
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
        application_init_args: record.application_init_args,
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
            RootComponentDeletionProgressRecord::MembershipRemoved(receipt) => {
                RootComponentDeletionProgressView::MembershipRemoved(
                    RootComponentMembershipRemovedView {
                        deleted: RootComponentDeletedReceiptView {
                            deletion: component_deletion_intent_record_to_view(
                                receipt.deleted.deletion,
                            ),
                            deleted_at_ns: receipt.deleted.deleted_at_ns,
                        },
                        allocation_operation_id: receipt.allocation_operation_id,
                        remaining_spec_committed_instances: receipt
                            .remaining_spec_committed_instances,
                        root_committed_component_instances: receipt
                            .root_committed_component_instances,
                        root_known_created_component_canisters: receipt
                            .root_known_created_component_canisters,
                        root_registry_encoded_bytes: receipt.root_registry_encoded_bytes,
                        removed_at_ns: receipt.removed_at_ns,
                        removal_hash: receipt.removal_hash,
                    },
                )
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
        protocol_profile_digest: effect.protocol_profile_digest,
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
        return Err(InternalError::invariant());
    }
    let current_partition_bytes = RootComponentRegistryStore::partition_entry_bytes(current);
    let current_count_bytes = current_count
        .map(RootComponentRegistryStore::parent_role_count_entry_bytes)
        .unwrap_or_default();
    let allocation_bytes = RootComponentRegistryStore::child_allocation_entry_bytes(allocation);
    let next_count_bytes = RootComponentRegistryStore::parent_role_count_entry_bytes(next_count);
    let mut next = current.clone();
    next.reserved_descendants = next
        .reserved_descendants
        .checked_add(1)
        .ok_or_else(InternalError::resource_exhausted)?;

    for _ in 0..8 {
        let next_partition_bytes = RootComponentRegistryStore::partition_entry_bytes(&next);
        let next_total = next_partition_bytes
            .checked_add(allocation_bytes)
            .and_then(|value| value.checked_add(next_count_bytes))
            .ok_or_else(InternalError::resource_exhausted)?;
        let current_total = current_partition_bytes
            .checked_add(current_count_bytes)
            .ok_or_else(InternalError::resource_exhausted)?;
        let delta = next_total
            .checked_sub(current_total)
            .ok_or_else(InternalError::invariant)?;
        let encoded_bytes = current
            .encoded_bytes
            .checked_add(delta)
            .ok_or_else(InternalError::resource_exhausted)?;
        if next.encoded_bytes == encoded_bytes {
            return Ok((next, delta));
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant())
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
            .ok_or_else(InternalError::resource_exhausted)?;
        let registry_delta = next_total
            .checked_sub(current_partition_bytes)
            .ok_or_else(InternalError::invariant)?;
        let encoded_bytes = current
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(InternalError::resource_exhausted)?;
        if next.encoded_bytes == encoded_bytes {
            return Ok((next, registry_delta));
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant())
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
        .ok_or_else(InternalError::resource_exhausted)?;
    let component_without_current = partition
        .encoded_bytes
        .checked_sub(current_total)
        .ok_or_else(InternalError::invariant)?;
    let root_without_current = current
        .encoded_bytes
        .checked_sub(current_total)
        .ok_or_else(InternalError::invariant)?;
    let next_record_bytes = RootComponentRegistryStore::subtree_removal_entry_bytes(next_record);
    let mut next_partition = partition.clone();

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next_partition)
            .checked_add(next_record_bytes)
            .ok_or_else(InternalError::resource_exhausted)?;
        let next_component_bytes = component_without_current
            .checked_add(next_total)
            .ok_or_else(InternalError::resource_exhausted)?;
        if next_partition.encoded_bytes == next_component_bytes {
            if next_component_bytes > maximum_component_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            let next_root_bytes = root_without_current
                .checked_add(next_total)
                .ok_or_else(InternalError::resource_exhausted)?;
            if next_root_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            let mut next_meta = current.clone();
            next_meta.encoded_bytes = next_root_bytes;
            return Ok((next_partition, next_meta));
        }
        next_partition.encoded_bytes = next_component_bytes;
    }
    Err(InternalError::invariant())
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
        .ok_or_else(InternalError::invariant)?;
    let root_without_partition = current
        .encoded_bytes
        .checked_sub(current_partition_bytes)
        .ok_or_else(InternalError::invariant)?;
    let draining_bytes = RootComponentRegistryStore::component_draining_entry_bytes(record);

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next_partition)
            .checked_add(draining_bytes)
            .ok_or_else(InternalError::resource_exhausted)?;
        let next_component_bytes = component_without_partition
            .checked_add(next_total)
            .ok_or_else(InternalError::resource_exhausted)?;
        if next_partition.encoded_bytes == next_component_bytes {
            if next_component_bytes > maximum_component_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            let next_root_bytes = root_without_partition
                .checked_add(next_total)
                .ok_or_else(InternalError::resource_exhausted)?;
            if next_root_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            let mut next_meta = current.clone();
            next_meta.encoded_bytes = next_root_bytes;
            return Ok((next_partition, next_meta));
        }
        next_partition.encoded_bytes = next_component_bytes;
    }
    Err(InternalError::invariant())
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
        terminal.deletion = Some(RootComponentDeletionProgressRecord::MembershipRemoved(
            RootComponentMembershipRemovedRecord {
                deleted: RootComponentDeletedReceiptRecord {
                    deletion: RootComponentDeletionIntentRecord {
                        final_inventory,
                        quiescence,
                        prepared_at_ns: u64::MAX,
                    },
                    deleted_at_ns: u64::MAX,
                },
                allocation_operation_id: [u8::MAX; 32],
                remaining_spec_committed_instances: u32::MAX,
                root_committed_component_instances: u32::MAX,
                root_known_created_component_canisters: u32::MAX,
                root_registry_encoded_bytes: u64::MAX,
                removed_at_ns: u64::MAX,
                removal_hash: [u8::MAX; 32],
            },
        ));
        let next = RootComponentRegistryStore::component_draining_entry_bytes(&terminal);
        if next == charged_entry_bytes {
            return Ok(next);
        }
        charged_entry_bytes = next;
    }
    Err(InternalError::invariant())
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
        .ok_or_else(InternalError::invariant)?;
    let root_without_mutated_entries = current
        .encoded_bytes
        .checked_sub(current_partition_bytes)
        .and_then(|bytes| bytes.checked_sub(current_draining_bytes))
        .ok_or_else(InternalError::invariant)?;
    let next_draining_bytes = charged_component_draining_entry_bytes(next_draining);
    let mut next_partition = partition.clone();
    for _ in 0..8 {
        let next_mutated_bytes = RootComponentRegistryStore::partition_entry_bytes(&next_partition)
            .checked_add(next_draining_bytes)
            .ok_or_else(InternalError::resource_exhausted)?;
        let next_component_bytes = component_without_mutated_entries
            .checked_add(next_mutated_bytes)
            .ok_or_else(InternalError::resource_exhausted)?;
        if next_partition.encoded_bytes == next_component_bytes {
            if next_component_bytes > maximum_component_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            let next_root_bytes = root_without_mutated_entries
                .checked_add(next_mutated_bytes)
                .ok_or_else(InternalError::resource_exhausted)?;
            if next_root_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            let mut next_meta = current.clone();
            next_meta.encoded_bytes = next_root_bytes;
            return Ok((next_partition, next_meta));
        }
        next_partition.encoded_bytes = next_component_bytes;
    }
    Err(InternalError::invariant())
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
        .ok_or_else(InternalError::resource_exhausted)?;
    let component_without_current = partition
        .encoded_bytes
        .checked_sub(current_total)
        .ok_or_else(InternalError::invariant)?;
    let root_without_current = current
        .encoded_bytes
        .checked_sub(current_total)
        .ok_or_else(InternalError::invariant)?;
    let next_record_bytes = RootComponentRegistryStore::subtree_removal_entry_bytes(next_record);
    let history_bytes =
        RootComponentRegistryStore::subtree_removal_completed_leaf_entry_bytes(completed_leaf);
    let mut next_partition = partition.clone();

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next_partition)
            .checked_add(next_record_bytes)
            .and_then(|bytes| bytes.checked_add(history_bytes))
            .ok_or_else(InternalError::resource_exhausted)?;
        let next_component_bytes = component_without_current
            .checked_add(next_total)
            .ok_or_else(InternalError::resource_exhausted)?;
        if next_partition.encoded_bytes == next_component_bytes {
            if next_component_bytes > maximum_component_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            let next_root_bytes = root_without_current
                .checked_add(next_total)
                .ok_or_else(InternalError::resource_exhausted)?;
            if next_root_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            let mut next_meta = current.clone();
            next_meta.encoded_bytes = next_root_bytes;
            return Ok((next_partition, next_meta));
        }
        next_partition.encoded_bytes = next_component_bytes;
    }
    Err(InternalError::invariant())
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
        .ok_or_else(InternalError::resource_exhausted)?;
    let component_without_current = partition
        .encoded_bytes
        .checked_sub(current_total)
        .ok_or_else(InternalError::invariant)?;
    let root_without_current = current
        .encoded_bytes
        .checked_sub(current_total)
        .ok_or_else(InternalError::invariant)?;
    let next_count_bytes =
        next_parent_role_count.map_or(0, RootComponentRegistryStore::parent_role_count_entry_bytes);

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(next_partition)
            .checked_add(RootComponentRegistryStore::subtree_removal_entry_bytes(
                next_record,
            ))
            .and_then(|bytes| bytes.checked_add(next_count_bytes))
            .ok_or_else(InternalError::resource_exhausted)?;
        let next_component_bytes = component_without_current
            .checked_add(next_total)
            .ok_or_else(InternalError::resource_exhausted)?;
        let next_root_bytes = root_without_current
            .checked_add(next_total)
            .ok_or_else(InternalError::resource_exhausted)?;
        let RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant());
        };
        if next_partition.encoded_bytes == next_component_bytes
            && receipt.registry_encoded_bytes == next_component_bytes
        {
            if next_component_bytes > maximum_component_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            if next_root_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            next_meta.encoded_bytes = next_root_bytes;
            return Ok(());
        }
        next_partition.encoded_bytes = next_component_bytes;
        receipt.registry_encoded_bytes = next_component_bytes;
    }
    Err(InternalError::invariant())
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
        return Err(InternalError::conflict());
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
        return Err(InternalError::invariant());
    }
    let current_total = current_partition_bytes
        .checked_add(current_record_bytes)
        .ok_or_else(InternalError::resource_exhausted)?;
    let mut next = partition.clone();

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next)
            .checked_add(charged_entry_bytes)
            .ok_or_else(InternalError::resource_exhausted)?;
        let registry_delta = next_total
            .checked_sub(current_total)
            .ok_or_else(InternalError::invariant)?;
        let encoded_bytes = partition
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(InternalError::resource_exhausted)?;
        if next.encoded_bytes == encoded_bytes {
            if encoded_bytes > record.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            let root_encoded_bytes = current
                .encoded_bytes
                .checked_add(registry_delta)
                .ok_or_else(InternalError::resource_exhausted)?;
            if root_encoded_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            return Ok((next, registry_delta));
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant())
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
        return Err(InternalError::invariant());
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
            return Err(InternalError::conflict());
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
        return Err(InternalError::conflict());
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
            return Err(InternalError::conflict());
        }
    };
    let installation = RootComponentChildInstallEffectRecord {
        raw_module_hash: plan.raw_module_hash,
        protocol_profile_digest: plan.protocol_profile_digest,
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
        protocol_profile_digest: plan.protocol_profile_digest,
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
        .ok_or_else(InternalError::resource_exhausted)
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
            return Err(InternalError::conflict());
        }
    };
    let current_total = RootComponentRegistryStore::partition_entry_bytes(partition)
        .checked_add(current_reserved_bytes)
        .ok_or_else(InternalError::resource_exhausted)?;
    let mut next = partition.clone();

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next)
            .checked_add(charged_entry_bytes)
            .ok_or_else(InternalError::resource_exhausted)?;
        let registry_delta = next_total
            .checked_sub(current_total)
            .ok_or_else(InternalError::invariant)?;
        let encoded_bytes = partition
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(InternalError::resource_exhausted)?;
        if next.encoded_bytes == encoded_bytes {
            if encoded_bytes > record.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            let root_encoded_bytes = current
                .encoded_bytes
                .checked_add(registry_delta)
                .ok_or_else(InternalError::resource_exhausted)?;
            if root_encoded_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted());
            }
            return Ok((next, registry_delta));
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant())
}

fn validate_child_install_effect_record(
    effect: &RootComponentChildInstallEffectRecord,
    plan: &RootComponentChildInstallPlan,
) -> Result<(), InternalError> {
    if effect.raw_module_hash != plan.raw_module_hash
        || effect.protocol_profile_digest != plan.protocol_profile_digest
        || effect.chunk_hashes != plan.chunk_hashes
        || effect.binding != plan.binding
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn advance_child_install_phase(
    component: ComponentInstanceId,
    operation_id: [u8; 32],
    verified: bool,
) -> Result<RootComponentChildAllocationView, InternalError> {
    let current = RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
    let partition =
        RootComponentRegistryStore::partition(component).ok_or_else(InternalError::unavailable)?;
    let record = RootComponentRegistryStore::child_allocation(component, operation_id)
        .ok_or_else(InternalError::unavailable)?;
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
            return Err(InternalError::conflict());
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
            return Err(InternalError::conflict());
        }
    };
    let mut maximum = record.clone();
    let installation = RootComponentInstallEffectRecord {
        raw_module_hash: plan.raw_module_hash,
        protocol_profile_digest: plan.protocol_profile_digest,
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
            plan.protocol_profile_digest,
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
        protocol_profile_digest: plan.protocol_profile_digest,
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
        .ok_or_else(InternalError::resource_exhausted)?;
    if charged > plan.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted());
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
    directory: ComponentDirectoryAuthorityInput<'_>,
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
        return Err(InternalError::conflict());
    }
    let child = ComponentRegistryChildRecord {
        component: record.component,
        canister_id: canister,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        kind: record.child_kind,
        installed_artifact_hash: installation.raw_module_hash,
        protocol_profile_digest: installation.protocol_profile_digest,
        status: ComponentLifecycleStatus::Prepared,
    };
    validate_child_record(partition, &child)?;

    let revision = partition
        .revision
        .checked_add(1)
        .ok_or_else(InternalError::resource_exhausted)?;
    let reserved_descendants = partition
        .reserved_descendants
        .checked_sub(1)
        .ok_or_else(InternalError::invariant)?;
    let committed_descendants = partition
        .committed_descendants
        .checked_add(1)
        .ok_or_else(InternalError::resource_exhausted)?;
    let descendant_content_hash = committed_component_descendant_content_hash(
        partition.descendant_content_hash,
        partition.committed_descendants,
        revision,
        &child,
    )?;
    let content_hash = component_partition_content_hash(
        &partition.binding,
        partition.protocol_profile_digest,
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
    let directory_authority_hash = component_directory_authority_hash_with_group(
        &partition.binding,
        revision,
        content_hash,
        directory.synchronized_at_ns,
        committed_descendants,
        directory.fleet,
        directory.component_group,
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
            directory_synchronized_at_ns: directory.synchronized_at_ns,
            directory_authority_hash,
            directory_prepared: false,
            runtime_activated: false,
            membership: None,
        },
    };
    let mut next_partition = ComponentRegistryPartitionRecord {
        binding: partition.binding.clone(),
        protocol_profile_digest: partition.protocol_profile_digest,
        provisioning_origin: partition.provisioning_origin.clone(),
        release_set: partition.release_set,
        status: partition.status,
        revision,
        content_hash,
        descendant_content_hash,
        directory_synchronized_at_ns: directory.synchronized_at_ns,
        reserved_descendants,
        committed_descendants,
        encoded_bytes: partition.encoded_bytes,
    };
    let current_total = RootComponentRegistryStore::partition_entry_bytes(partition)
        .checked_add(installation.charged_entry_bytes)
        .ok_or_else(InternalError::resource_exhausted)?;
    let child_bytes = RootComponentRegistryStore::child_entry_bytes(&child);
    let traversal_bytes = RootComponentRegistryStore::child_traversal_entry_bytes(&traversal);
    let index_bytes =
        RootComponentRegistryStore::principal_index_entry_bytes(canister, record.component);

    for _ in 0..8 {
        let terminal_bytes = RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
            .checked_add(child_bytes)
            .and_then(|value| value.checked_add(traversal_bytes))
            .and_then(|value| value.checked_add(index_bytes))
            .ok_or_else(InternalError::resource_exhausted)?;
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next_partition)
            .checked_add(terminal_bytes)
            .ok_or_else(InternalError::resource_exhausted)?;
        let released_precharge = current_total
            .checked_sub(next_total)
            .ok_or_else(InternalError::invariant)?;
        let encoded_bytes = partition
            .encoded_bytes
            .checked_sub(released_precharge)
            .ok_or_else(InternalError::invariant)?;
        let RootComponentChildAllocationProgressRecord::Committed { commitment, .. } =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant());
        };
        if next_partition.encoded_bytes == encoded_bytes
            && commitment.registry_encoded_bytes == encoded_bytes
        {
            return Ok((next_record, next_partition, child, traversal));
        }
        next_partition.encoded_bytes = encoded_bytes;
        commitment.registry_encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant())
}

fn persist_child_membership_activation(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentChildAllocationRecord,
    child: &ComponentRegistryChildRecord,
    directory_synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
    component_group: Option<&ComponentGroupDirectory>,
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
        return Err(InternalError::invariant());
    };
    let (next_record, active_partition, active_child) = active_child_membership_records(
        record,
        commitment,
        partition,
        child,
        directory_synchronized_at_ns,
        fleet_directory,
        component_group,
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
        .ok_or_else(InternalError::resource_exhausted)?;
    if terminal_bytes > installation.charged_entry_bytes {
        return Err(InternalError::invariant());
    }
    if active_partition.encoded_bytes > record.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted());
    }
    let encoded_bytes = current
        .encoded_bytes
        .checked_sub(partition.encoded_bytes)
        .and_then(|value| value.checked_add(active_partition.encoded_bytes))
        .ok_or_else(InternalError::invariant)?;
    if encoded_bytes > current.root.limits.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted());
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
    component_group: Option<&ComponentGroupDirectory>,
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
        return Err(InternalError::invariant());
    };
    let revision = partition
        .revision
        .checked_add(1)
        .ok_or_else(InternalError::resource_exhausted)?;
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
        partition.protocol_profile_digest,
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
    let directory_authority_hash = component_directory_authority_hash_with_group(
        &partition.binding,
        revision,
        content_hash,
        directory_synchronized_at_ns,
        partition.committed_descendants,
        fleet_directory,
        component_group,
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
        protocol_profile_digest: partition.protocol_profile_digest,
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
        .ok_or_else(InternalError::resource_exhausted)?;

    for _ in 0..8 {
        let next_variable_bytes =
            RootComponentRegistryStore::partition_entry_bytes(&active_partition)
                .checked_add(RootComponentRegistryStore::child_allocation_entry_bytes(
                    &next_record,
                ))
                .and_then(|value| {
                    value.checked_add(RootComponentRegistryStore::child_entry_bytes(&active_child))
                })
                .ok_or_else(InternalError::resource_exhausted)?;
        let encoded_bytes = partition
            .encoded_bytes
            .checked_sub(previous_variable_bytes)
            .and_then(|value| value.checked_add(next_variable_bytes))
            .ok_or_else(InternalError::invariant)?;
        let RootComponentChildAllocationProgressRecord::Committed { commitment, .. } =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant());
        };
        let membership = commitment
            .membership
            .as_mut()
            .ok_or_else(InternalError::invariant)?;
        if active_partition.encoded_bytes == encoded_bytes
            && membership.registry_encoded_bytes == encoded_bytes
        {
            return Ok((next_record, active_partition, active_child));
        }
        active_partition.encoded_bytes = encoded_bytes;
        membership.registry_encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant())
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
        installation.protocol_profile_digest,
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
            component_group: None,
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
        protocol_profile_digest: installation.protocol_profile_digest,
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
            .ok_or_else(InternalError::resource_exhausted)?;
        let RootComponentAllocationProgressRecord::Committed { commitment, .. } =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant());
        };
        if partition.encoded_bytes == encoded_bytes
            && commitment.prepared_registry_encoded_bytes == encoded_bytes
        {
            return Ok((next_record, partition));
        }
        partition.encoded_bytes = encoded_bytes;
        commitment.prepared_registry_encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant())
}

fn active_membership_records(
    record: &RootComponentAllocationRecord,
    commitment: &RootComponentCommitmentRecord,
    directory_synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
    component_group: Option<&ComponentGroupDirectory>,
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
        return Err(InternalError::invariant());
    };
    let revision = commitment
        .registry
        .revision
        .checked_add(1)
        .ok_or_else(InternalError::resource_exhausted)?;
    let content_hash = component_partition_content_hash(
        &installation.binding,
        installation.protocol_profile_digest,
        &record.provisioning_origin,
        record.release_set,
        ComponentLifecycleStatus::Active,
        revision,
        empty_component_descendant_content_hash(record.component),
        0,
    )?;
    let directory_authority_hash = component_directory_authority_hash_with_group(
        &installation.binding,
        revision,
        content_hash,
        directory_synchronized_at_ns,
        0,
        fleet_directory,
        component_group,
    )?;
    let mut next_record = record.clone();
    let mut active = ComponentRegistryPartitionRecord {
        binding: installation.binding.clone(),
        protocol_profile_digest: installation.protocol_profile_digest,
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
            .ok_or_else(InternalError::resource_exhausted)?;
        let RootComponentAllocationProgressRecord::Committed { commitment, .. } =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant());
        };
        let membership = commitment
            .membership
            .as_mut()
            .ok_or_else(InternalError::invariant)?;
        if active.encoded_bytes == encoded_bytes
            && membership.registry_encoded_bytes == encoded_bytes
        {
            return Ok((next_record, active));
        }
        active.encoded_bytes = encoded_bytes;
        membership.registry_encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant())
}

fn component_directory_authority_hash(
    binding: &ComponentBinding,
    revision: u64,
    content_hash: [u8; 32],
    synchronized_at_ns: u64,
    descendant_count: u32,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<[u8; 32], InternalError> {
    component_directory_authority_hash_with_group(
        binding,
        revision,
        content_hash,
        synchronized_at_ns,
        descendant_count,
        fleet_directory,
        None,
    )
}

fn component_directory_authority_hash_with_group(
    binding: &ComponentBinding,
    revision: u64,
    content_hash: [u8; 32],
    synchronized_at_ns: u64,
    descendant_count: u32,
    fleet_directory: &FleetDirectorySnapshot,
    component_group: Option<&ComponentGroupDirectory>,
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
        component_group: component_group.cloned(),
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
        return Err(InternalError::invariant());
    };
    let current = RootComponentRegistryStore::partition(record.component)
        .ok_or_else(InternalError::invariant)?;
    validate_partition_record(&current)?;
    let child = RootComponentRegistryStore::child(record.component, *canister)
        .ok_or_else(InternalError::invariant)?;
    validate_child_record(&current, &child)?;
    let traversal = ComponentRegistryChildTraversalRecord {
        component: record.component,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        canister_id: *canister,
    };
    let committed = ComponentRegistryPartitionRecord {
        binding: installation.binding.component.clone(),
        protocol_profile_digest: current.protocol_profile_digest,
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
        return Err(InternalError::invariant());
    }
    Ok(committed)
}

fn exact_active_child_partition(
    record: &RootComponentChildAllocationRecord,
    commitment: &RootComponentChildCommitmentRecord,
    membership: &RootComponentChildMembershipRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let current = RootComponentRegistryStore::partition(record.component)
        .ok_or_else(InternalError::invariant)?;
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
        return Err(InternalError::invariant());
    };
    let child = RootComponentRegistryStore::child(record.component, *canister)
        .ok_or_else(InternalError::invariant)?;
    validate_child_record(current, &child)?;
    let historical = ComponentRegistryPartitionRecord {
        binding: current.binding.clone(),
        protocol_profile_digest: current.protocol_profile_digest,
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
        return Err(InternalError::invariant());
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
        return Err(InternalError::invariant());
    };
    let current = RootComponentRegistryStore::partition(record.component)
        .ok_or_else(InternalError::invariant)?;
    let prepared = ComponentRegistryPartitionRecord {
        binding: installation.binding.clone(),
        protocol_profile_digest: installation.protocol_profile_digest,
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
        return Err(InternalError::invariant());
    }
    validate_partition_snapshot(&prepared)?;
    match &commitment.membership {
        None if current == prepared => {}
        Some(membership) => {
            let _active = validate_active_partition(record, commitment, membership, &current)?;
        }
        None => {
            return Err(InternalError::invariant());
        }
    }
    Ok(prepared)
}

fn exact_active_partition(
    record: &RootComponentAllocationRecord,
    commitment: &RootComponentCommitmentRecord,
    membership: &RootComponentMembershipRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let current = RootComponentRegistryStore::partition(record.component)
        .ok_or_else(InternalError::invariant)?;
    validate_active_partition(record, commitment, membership, &current)
}

fn validate_active_partition(
    record: &RootComponentAllocationRecord,
    commitment: &RootComponentCommitmentRecord,
    membership: &RootComponentMembershipRecord,
    current: &ComponentRegistryPartitionRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let expected_revision = commitment
        .registry
        .revision
        .checked_add(1)
        .ok_or_else(InternalError::resource_exhausted)?;
    let historical = ComponentRegistryPartitionRecord {
        binding: current.binding.clone(),
        protocol_profile_digest: current.protocol_profile_digest,
        provisioning_origin: current.provisioning_origin.clone(),
        release_set: current.release_set,
        status: ComponentLifecycleStatus::Active,
        revision: expected_revision,
        content_hash: component_partition_content_hash(
            &current.binding,
            current.protocol_profile_digest,
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
        return Err(InternalError::invariant());
    }
    validate_partition_record(current)?;
    Ok(historical)
}

fn validate_membership_directory_authority_hash(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
    component_group: Option<&ComponentGroupDirectory>,
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
        component_group: component_group.cloned(),
    };
    if ComponentRuntimeOps::directory_authority_hash(&authority)?
        != membership.directory_authority_hash
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_child_directory_authority_hash(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
    component_group: Option<&ComponentGroupDirectory>,
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
        component_group: component_group.cloned(),
    };
    if ComponentRuntimeOps::directory_authority_hash(&authority)?
        != commitment.directory_authority_hash
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_child_membership_directory_authority_hash(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
    component_group: Option<&ComponentGroupDirectory>,
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
        component_group: component_group.cloned(),
    };
    if ComponentRuntimeOps::directory_authority_hash(&authority)?
        != membership.directory_authority_hash
    {
        return Err(InternalError::invariant());
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
        component_group: None,
    };
    if ComponentRuntimeOps::directory_authority_hash(&authority)?
        != commitment.directory_authority_hash
    {
        return Err(InternalError::invariant());
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

const fn require_ordinary_component_lifecycle(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<(), InternalError> {
    if component_uses_grouped_lifecycle(partition) {
        return Err(InternalError::conflict());
    }
    Ok(())
}

const fn validate_ordinary_component_lifecycle(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<(), InternalError> {
    if component_uses_grouped_lifecycle(partition) {
        return Err(InternalError::invariant());
    }
    Ok(())
}

const fn component_uses_grouped_lifecycle(partition: &ComponentRegistryPartitionRecord) -> bool {
    matches!(
        partition.provisioning_origin,
        ComponentProvisioningOrigin::ComponentGroup { .. }
    )
}

fn validate_component_draining_record(
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentDrainingRecord,
) -> Result<(), InternalError> {
    validate_ordinary_component_lifecycle(partition)?;
    let previous_content_hash = component_partition_content_hash(
        &partition.binding,
        partition.protocol_profile_digest,
        &partition.provisioning_origin,
        partition.release_set,
        ComponentLifecycleStatus::Active,
        record.previous_registry.revision,
        record.descendant_content_hash,
        record.descendant_count,
    )?;
    let draining_content_hash = component_partition_content_hash(
        &partition.binding,
        partition.protocol_profile_digest,
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
        return Err(InternalError::invariant());
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
        return Err(InternalError::conflict());
    }
    if !component_partition_is_empty_and_draining(partition) {
        return Err(InternalError::conflict());
    }
    Ok(())
}

const fn ensure_component_final_inventory_time(
    partition: &ComponentRegistryPartitionRecord,
    quiesced_at_ns: u64,
    finalized_at_ns: u64,
) -> Result<(), InternalError> {
    if component_final_inventory_time_is_monotonic(partition, quiesced_at_ns, finalized_at_ns) {
        return Ok(());
    }
    Err(InternalError::invalid_input())
}

fn ensure_component_final_inventory_indexes_are_empty(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<(), InternalError> {
    if component_final_inventory_indexes_are_empty(partition) {
        return Ok(());
    }
    Err(InternalError::invariant())
}

fn ensure_component_final_inventory_fleet_authority(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<(), InternalError> {
    if fleet_directory.provenance.source_fleet_subnet_root != partition.binding.fleet_subnet_root {
        return Err(InternalError::conflict());
    }
    if fleet_directory.provenance.registry.revision == 0 {
        return Err(InternalError::conflict());
    }
    if fleet_directory.provenance.registry.content_hash == [0; 32] {
        return Err(InternalError::conflict());
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
    Err(InternalError::conflict())
}

fn ensure_component_deletion_inventory(
    progress: &RootComponentDeletionProgressRecord,
    expected_inventory_hash: [u8; 32],
) -> Result<(), InternalError> {
    let intent = match progress {
        RootComponentDeletionProgressRecord::DeleteIntent(intent) => intent,
        RootComponentDeletionProgressRecord::Deleted(receipt) => &receipt.deletion,
        RootComponentDeletionProgressRecord::MembershipRemoved(receipt) => {
            &receipt.deleted.deletion
        }
    };
    if intent.final_inventory.inventory_hash == expected_inventory_hash {
        return Ok(());
    }
    Err(InternalError::conflict())
}

fn committed_component_allocation(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<RootComponentAllocationRecord, InternalError> {
    let mut allocations = RootComponentRegistryStore::allocations()
        .into_iter()
        .filter(|allocation| allocation.component == partition.binding.component);
    let allocation = allocations.next().ok_or_else(InternalError::invariant)?;
    if allocations.next().is_some() {
        return Err(InternalError::invariant());
    }
    if ComponentAllocationPartitionAuthority::from_committed_allocation(&allocation)
        != Some(ComponentAllocationPartitionAuthority::from_partition(
            partition,
        ))
    {
        return Err(InternalError::invariant());
    }
    let RootComponentAllocationProgressRecord::Committed { commitment, .. } = &allocation.progress
    else {
        unreachable!("authority comparison accepts only committed allocations");
    };
    let membership_is_terminal = commitment.runtime_activated
        && commitment
            .membership
            .as_ref()
            .is_some_and(|membership| membership.directory_synchronized);
    if !commitment.directory_prepared || !membership_is_terminal {
        return Err(InternalError::invariant());
    }
    Ok(allocation)
}

fn removed_component_allocation(
    allocation: &RootComponentAllocationRecord,
) -> Result<RootComponentAllocationRecord, InternalError> {
    let RootComponentAllocationProgressRecord::Committed {
        creation,
        canister,
        installation,
        commitment,
    } = &allocation.progress
    else {
        return Err(InternalError::conflict());
    };
    let mut removed = allocation.clone();
    removed.progress = RootComponentAllocationProgressRecord::Removed {
        creation: creation.clone(),
        canister: *canister,
        installation: installation.clone(),
        commitment: commitment.clone(),
    };
    if RootComponentRegistryStore::allocation_entry_bytes(&removed)
        > RootComponentRegistryStore::allocation_record_max_bytes() + 128
    {
        return Err(InternalError::invariant());
    }
    Ok(removed)
}

fn component_membership_removal_records(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    allocation: &RootComponentAllocationRecord,
    draining: &RootComponentDrainingRecord,
    deleted: &RootComponentDeletedReceiptRecord,
    removed_at_ns: u64,
) -> Result<ComponentMembershipRemovalRecords, InternalError> {
    let next_allocation = removed_component_allocation(allocation)?;
    let (_, committed) = RootComponentRegistryStore::allocation_counts(&allocation.component_spec);
    let remaining_spec_committed_instances = committed
        .checked_sub(1)
        .and_then(|count| u32::try_from(count).ok())
        .ok_or_else(InternalError::invariant)?;
    let mut next_meta = current.clone();
    next_meta.committed_component_instances = next_meta
        .committed_component_instances
        .checked_sub(1)
        .ok_or_else(InternalError::invariant)?;
    next_meta.known_created_component_canisters = next_meta
        .known_created_component_canisters
        .checked_sub(1)
        .ok_or_else(InternalError::invariant)?;
    next_meta.encoded_bytes =
        removed_component_root_registry_bytes(current, partition, allocation, &next_allocation)?;

    let mut receipt = RootComponentMembershipRemovedRecord {
        deleted: deleted.clone(),
        allocation_operation_id: allocation.operation_id,
        remaining_spec_committed_instances,
        root_committed_component_instances: next_meta.committed_component_instances,
        root_known_created_component_canisters: next_meta.known_created_component_canisters,
        root_registry_encoded_bytes: next_meta.encoded_bytes,
        removed_at_ns,
        removal_hash: [0; 32],
    };
    receipt.removal_hash = component_membership_removal_hash(draining, &receipt)?;
    let mut next_draining = draining.clone();
    next_draining.deletion = Some(RootComponentDeletionProgressRecord::MembershipRemoved(
        receipt,
    ));
    if RootComponentRegistryStore::component_draining_entry_bytes(&next_draining)
        > deleted.deletion.quiescence.stop.charged_entry_bytes
    {
        return Err(InternalError::invariant());
    }
    Ok(ComponentMembershipRemovalRecords {
        next_meta,
        next_allocation,
        next_draining,
    })
}

fn removed_component_allocation_for_receipt(
    component: ComponentInstanceId,
    receipt: &RootComponentMembershipRemovedRecord,
) -> Result<RootComponentAllocationRecord, InternalError> {
    let mut allocations = RootComponentRegistryStore::allocations()
        .into_iter()
        .filter(|allocation| allocation.component == component);
    let allocation = allocations.next().ok_or_else(InternalError::invariant)?;
    if allocations.next().is_some() || allocation.operation_id != receipt.allocation_operation_id {
        return Err(InternalError::invariant());
    }
    Ok(allocation)
}

fn removed_component_root_registry_bytes(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    allocation: &RootComponentAllocationRecord,
    removed_allocation: &RootComponentAllocationRecord,
) -> Result<u64, InternalError> {
    let partition_bytes = RootComponentRegistryStore::partition_entry_bytes(partition);
    let principal_bytes = RootComponentRegistryStore::principal_index_entry_bytes(
        partition.binding.canister_id,
        partition.binding.component,
    );
    current
        .encoded_bytes
        .checked_sub(partition_bytes)
        .and_then(|bytes| bytes.checked_sub(principal_bytes))
        .and_then(|bytes| {
            bytes.checked_sub(RootComponentRegistryStore::allocation_entry_bytes(
                allocation,
            ))
        })
        .and_then(|bytes| {
            bytes.checked_add(RootComponentRegistryStore::allocation_entry_bytes(
                removed_allocation,
            ))
        })
        .ok_or_else(InternalError::invariant)
}

fn validate_removed_component_authority(
    draining: &RootComponentDrainingRecord,
) -> Result<(), InternalError> {
    let Some(progress) = draining.deletion.as_ref() else {
        return Err(InternalError::invariant());
    };
    let RootComponentDeletionProgressRecord::MembershipRemoved(receipt) = progress else {
        return Err(InternalError::invariant());
    };
    let allocation = removed_component_allocation_for_receipt(draining.component, receipt)?;
    let partition = removed_component_partition(&allocation, receipt)?;
    if ComponentAllocationPartitionAuthority::from_removed_allocation(&allocation)
        != Some(ComponentAllocationPartitionAuthority::from_partition(
            &partition,
        ))
    {
        return Err(InternalError::invariant());
    }
    let partition_is_absent = RootComponentRegistryStore::partition(draining.component).is_none();
    let live_inventory_is_empty =
        RootComponentRegistryStore::component_live_inventory_is_empty(draining.component);
    let principal_inventory_is_empty =
        RootComponentRegistryStore::component_principal_inventory_is_empty(draining.component);
    if ![
        partition_is_absent,
        live_inventory_is_empty,
        principal_inventory_is_empty,
    ]
    .into_iter()
    .all(|empty| empty)
    {
        return Err(InternalError::invariant());
    }
    if component_membership_removal_hash(draining, receipt)? != receipt.removal_hash {
        return Err(InternalError::invariant());
    }
    validate_partition_shape(&partition)?;
    validate_removed_component_final_inventory(&partition, draining, receipt)?;
    RootComponentDeletionAuthority { draining, progress }.validate()
}

fn removed_component_partition(
    allocation: &RootComponentAllocationRecord,
    receipt: &RootComponentMembershipRemovedRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let RootComponentAllocationProgressRecord::Removed { installation, .. } = &allocation.progress
    else {
        return Err(InternalError::invariant());
    };
    let inventory = &receipt.deleted.deletion.final_inventory;
    Ok(ComponentRegistryPartitionRecord {
        binding: installation.binding.clone(),
        protocol_profile_digest: installation.protocol_profile_digest,
        provisioning_origin: allocation.provisioning_origin.clone(),
        release_set: allocation.release_set,
        status: ComponentLifecycleStatus::Draining,
        revision: inventory.registry.revision,
        content_hash: inventory.registry.content_hash,
        descendant_content_hash: inventory.descendant_content_hash,
        directory_synchronized_at_ns: inventory.directory_synchronized_at_ns,
        reserved_descendants: 0,
        committed_descendants: 0,
        encoded_bytes: inventory.registry_encoded_bytes,
    })
}

fn validate_removed_component_final_inventory(
    partition: &ComponentRegistryPartitionRecord,
    draining: &RootComponentDrainingRecord,
    receipt: &RootComponentMembershipRemovedRecord,
) -> Result<(), InternalError> {
    let inventory = &receipt.deleted.deletion.final_inventory;
    let inventory_shape_is_exact =
        RootComponentFinalInventorySnapshotAuthority::from_inventory(inventory)
            == RootComponentFinalInventorySnapshotAuthority::from_partition(partition);
    let inventory_hash_is_exact =
        inventory.inventory_hash == component_final_inventory_hash(partition, inventory)?;
    let time_is_monotonic = receipt.removed_at_ns >= receipt.deleted.deleted_at_ns;
    let cursor_is_terminal = component_draining_cursor_is_terminal(draining);
    if ![
        inventory_shape_is_exact,
        inventory_hash_is_exact,
        time_is_monotonic,
        cursor_is_terminal,
    ]
    .into_iter()
    .all(|valid| valid)
    {
        return Err(InternalError::invariant());
    }
    ensure_component_lifecycle_history_is_terminal(partition)
}

fn component_membership_removal_hash(
    draining: &RootComponentDrainingRecord,
    receipt: &RootComponentMembershipRemovedRecord,
) -> Result<[u8; 32], InternalError> {
    let payload = candid::encode_one(RootComponentMembershipRemovalHashAuthority {
        operation_id: draining.operation_id,
        component: draining.component,
        final_inventory_hash: receipt.deleted.deletion.final_inventory.inventory_hash,
        deleted_at_ns: receipt.deleted.deleted_at_ns,
        allocation_operation_id: receipt.allocation_operation_id,
        remaining_spec_committed_instances: receipt.remaining_spec_committed_instances,
        root_committed_component_instances: receipt.root_committed_component_instances,
        root_known_created_component_canisters: receipt.root_known_created_component_canisters,
        root_registry_encoded_bytes: receipt.root_registry_encoded_bytes,
        removed_at_ns: receipt.removed_at_ns,
    })
    .map_err(|_error| InternalError::invariant())?;
    let mut hasher = Sha256::new();
    hasher.update(COMPONENT_MEMBERSHIP_REMOVAL_HASH_DOMAIN);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn component_has_terminal_quiescence(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<bool, InternalError> {
    let draining = RootComponentRegistryStore::component_draining(partition.binding.component)
        .ok_or_else(InternalError::invariant)?;
    validate_component_draining_record(partition, &draining)?;
    Ok(matches!(
        draining.quiescence,
        Some(RootComponentQuiescenceProgressRecord::Quiescent(_))
    ))
}

fn validate_partition_snapshot(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<(), InternalError> {
    validate_partition_shape(partition)?;
    if RootComponentRegistryStore::component_for_principal(partition.binding.canister_id)
        != Some(partition.binding.component)
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_partition_shape(
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
        partition.protocol_profile_digest,
        &partition.provisioning_origin,
        partition.release_set,
        partition.status,
        partition.revision,
        partition.descendant_content_hash,
        partition.committed_descendants,
    )?;
    let head_is_versioned = partition.revision > 0;
    let directory_is_synchronized = partition.directory_synchronized_at_ns > 0;
    let content_is_canonical = partition.descendant_content_hash != [0; 32]
        && partition.protocol_profile_digest.as_bytes() != &[0; 32]
        && descendant_hash_matches_count
        && partition.content_hash == expected_content_hash;
    if !head_is_versioned || !directory_is_synchronized || !content_is_canonical {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_child_record(
    partition: &ComponentRegistryPartitionRecord,
    child: &ComponentRegistryChildRecord,
) -> Result<(), InternalError> {
    if child.protocol_profile_digest.as_bytes() == &[0; 32]
        || !ComponentTreeBoundary::from_partition(partition).admits(child)
    {
        return Err(InternalError::invariant());
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
        return Err(InternalError::invariant());
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
        return Err(InternalError::conflict());
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
        return Err(InternalError::conflict());
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
            Err(InternalError::conflict())
        };
    }
    let (binding, status) = ComponentRegistryOps::registered_parent(component, parent_canister_id)?
        .ok_or_else(InternalError::invariant)?;
    if status != ComponentLifecycleStatus::Active {
        return Err(InternalError::conflict());
    }
    let evidence = evidence.ok_or_else(InternalError::unavailable)?;
    let (coverage, record) = subtree_directory_convergence_record(partition, &binding, evidence)?;
    if &coverage != expected_coverage {
        return Err(InternalError::conflict());
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
        return Err(InternalError::invariant());
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
        return Err(InternalError::invariant());
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
        RootComponentRegistryStore::child(record.component, record.target.canister_id)
            .ok_or_else(InternalError::invariant)?;
    validate_registered_child_record(partition, &current_target)?;
    if current_target != record.target {
        return Err(InternalError::invariant());
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
        .ok_or_else(InternalError::invariant)?;
    validate_registered_child_record(partition, &current_node)?;
    if &current_node != node {
        return Err(InternalError::invariant());
    }
    let traversal_limit = partition
        .committed_descendants
        .checked_add(1)
        .ok_or_else(InternalError::resource_exhausted)?;
    if !canister_is_in_subtree(
        partition,
        node.canister_id,
        record.target.canister_id,
        traversal_limit,
    )? {
        return Err(InternalError::invariant());
    }
    if must_be_leaf && first_registered_child(partition, node.canister_id)?.is_some() {
        return Err(InternalError::invariant());
    }
    Ok(())
}

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
        partition.protocol_profile_digest,
        &partition.provisioning_origin,
        partition.release_set,
        partition.status,
        receipt.registry.revision,
        receipt.descendant_content_hash,
        receipt.committed_descendants,
    )?;
    let expected_previous_content_hash = component_partition_content_hash(
        &partition.binding,
        partition.protocol_profile_digest,
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
                .ok_or_else(InternalError::invariant)?
        && current_parent_role_instances >= receipt.parent_role_instances;
    if !receipt_is_canonical
        || (!exact_head_is_current && !head_was_advanced)
        || !removed_indexes_are_absent
        || first_registered_child(partition, leaf.canister_id)?.is_some()
    {
        return Err(InternalError::invariant());
    }
    if record.target.canister_id != leaf.canister_id {
        let current_target =
            RootComponentRegistryStore::child(record.component, record.target.canister_id)
                .ok_or_else(InternalError::invariant)?;
        validate_registered_child_record(partition, &current_target)?;
        if current_target != record.target {
            return Err(InternalError::invariant());
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
        return Err(InternalError::invariant());
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
    .ok_or_else(InternalError::invariant)?;
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
        return Err(InternalError::invariant());
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
    let payload = canic_core::cdk::serialize::serialize(receipt)
        .map_err(|_error| InternalError::invariant())?;
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

    let parent = RootComponentRegistryStore::child(component, leaf.parent_canister_id)
        .ok_or_else(InternalError::invariant)?;
    validate_registered_child_record(partition, &parent)?;
    if parent.status != ComponentLifecycleStatus::Active {
        return Err(InternalError::conflict());
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
        return Err(InternalError::invariant());
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
        return Err(InternalError::conflict());
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
            .ok_or_else(InternalError::invariant)?;
    validate_registered_child_record(partition, &child)?;
    let expected_identity = ComponentTreeNodeIdentity::new(
        partition.binding.component,
        parent_canister_id,
        &child.role,
        child.canister_id,
    );
    if ComponentTreeNodeIdentity::from_traversal(&traversal) != expected_identity {
        return Err(InternalError::invariant());
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
            return Err(InternalError::invariant());
        }
    }
    for removal in RootComponentRegistryStore::subtree_removals(partition.binding.component) {
        validate_subtree_removal_record(&removal)?;
        validate_subtree_removal_progress(partition, &removal)?;
        if !matches!(
            removal.progress,
            RootComponentSubtreeRemovalProgressRecord::Completed(_)
        ) {
            return Err(InternalError::invariant());
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
            .ok_or_else(InternalError::invariant)?;
        validate_registered_child_record(partition, &child)?;
        current = child.parent_canister_id;
    }
    Err(InternalError::invariant())
}

fn validate_child_traversal_record(
    component: ComponentInstanceId,
    traversal: &ComponentRegistryChildTraversalRecord,
) -> Result<(), InternalError> {
    if !ComponentTreeNodeIdentity::from_traversal(traversal).is_valid_for(component) {
        return Err(InternalError::invariant());
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
        protocol_profile_digest: child.protocol_profile_digest,
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
        return Err(InternalError::invariant());
    }
    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "the arguments are the exact ordered fields of the canonical partition hash"
)]
fn component_partition_content_hash(
    binding: &ComponentBinding,
    protocol_profile_digest: ProtocolProfileDigest,
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
        protocol_profile_digest,
        provisioning_origin.clone(),
        release_set,
        status,
        revision,
        descendant_content_hash,
        committed_descendants,
    ))
    .map_err(|_error| InternalError::invariant())?;
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
    .map_err(|_error| InternalError::invariant())?;
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
        return Err(InternalError::invariant());
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
        child.protocol_profile_digest,
        child.status,
    ))
    .map_err(|_error| InternalError::invariant())?;
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
        return Err(InternalError::invariant());
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
        child.protocol_profile_digest,
        ComponentLifecycleStatus::Prepared,
        child.status,
    ))
    .map_err(|_error| InternalError::invariant())?;
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
        return Err(InternalError::invariant());
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
        child.protocol_profile_digest,
        child.status,
    ))
    .map_err(|_error| InternalError::invariant())?;
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
            return Err(InternalError::conflict());
        }
    };
    let without_current = current
        .encoded_bytes
        .checked_sub(current_reserved_bytes)
        .ok_or_else(InternalError::invariant)?;
    let next_encoded_bytes = without_current
        .checked_add(charged_entry_bytes)
        .ok_or_else(InternalError::resource_exhausted)?;
    if next_encoded_bytes > current.root.limits.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted());
    }
    Ok(next_encoded_bytes)
}

fn validate_install_effect_record(
    effect: &RootComponentInstallEffectRecord,
    plan: &RootComponentInstallPlan,
) -> Result<(), InternalError> {
    if effect.raw_module_hash != plan.raw_module_hash
        || effect.protocol_profile_digest != plan.protocol_profile_digest
        || effect.chunk_hashes != plan.chunk_hashes
        || effect.binding != plan.binding
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn advance_install_phase(
    operation_id: [u8; 32],
    verified: bool,
) -> Result<RootComponentAllocationView, InternalError> {
    let current = RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
    let record = RootComponentRegistryStore::allocation(operation_id)
        .ok_or_else(InternalError::unavailable)?;
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
            return Err(InternalError::conflict());
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
        return Err(InternalError::resource_exhausted());
    }
    let current_entry_bytes = RootComponentRegistryStore::allocation_entry_bytes(record);
    let without_current = current
        .encoded_bytes
        .checked_sub(current_entry_bytes)
        .ok_or_else(InternalError::invariant)?;
    let next_encoded_bytes = without_current
        .checked_add(charged_entry_bytes)
        .ok_or_else(InternalError::resource_exhausted)?;
    if next_encoded_bytes > current.root.limits.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted());
    }
    Ok(next_encoded_bytes)
}

fn validate_charged_record_size(
    record: &RootComponentAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<(), InternalError> {
    let entry_bytes = RootComponentRegistryStore::allocation_entry_bytes(record);
    if entry_bytes > charged_entry_bytes {
        return Err(InternalError::invariant());
    }
    Ok(())
}

const fn map_allocation_commit_error(error: RootComponentAllocationCommitError) -> InternalError {
    match error {
        RootComponentAllocationCommitError::ComponentIdentityConflict
        | RootComponentAllocationCommitError::ComponentPrincipalConflict
        | RootComponentAllocationCommitError::ParentPrincipalConflict => {
            InternalError::public(canic_core::diagnostics::codes::AUTHORITY_CONFLICT)
        }
        RootComponentAllocationCommitError::ConflictingChildEntry
        | RootComponentAllocationCommitError::ConflictingPartition => {
            InternalError::public(canic_core::diagnostics::codes::COLLECTION_CONFLICT)
        }
        RootComponentAllocationCommitError::ConflictingOperation => {
            InternalError::public(canic_core::diagnostics::codes::REQUEST_CONFLICT)
        }
        RootComponentAllocationCommitError::ConflictingState => {
            InternalError::public(canic_core::diagnostics::codes::STATE_CONFLICT)
        }
        RootComponentAllocationCommitError::MissingOperation => {
            InternalError::public(canic_core::diagnostics::codes::REQUEST_UNAVAILABLE)
        }
        RootComponentAllocationCommitError::Uninitialized => {
            InternalError::public(canic_core::diagnostics::codes::STATE_UNAVAILABLE)
        }
    }
}

#[cfg(test)]
mod tests;
