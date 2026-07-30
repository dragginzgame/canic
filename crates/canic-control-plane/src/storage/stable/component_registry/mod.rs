//! Module: storage::stable::component_registry
//!
//! Responsibility: own one root's Component Registry meta, operations, partitions and indexes.
//! Does not own: Store, Fleet Registry, topology, admission, or lifecycle validation.
//! Boundary: ops commit only exact authority and records already validated by workflow.

#[cfg(feature = "root-control-plane")]
use canic_core::{
    cdk::structures::{
        DefaultMemoryImpl, btreemap::BTreeMap as StableBtreeMap, cell::Cell, memory::VirtualMemory,
        storable::Storable,
    },
    eager_static,
    role_contract::allocation::memory::control_plane::{
        ROOT_COMPONENT_ALLOCATIONS_ID, ROOT_COMPONENT_DRAINING_ID,
        ROOT_COMPONENT_PRINCIPAL_INDEX_ID, ROOT_COMPONENT_REGISTRY_ENTRIES_ID,
        ROOT_COMPONENT_REGISTRY_META_ID, ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_ID,
    },
};
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::config::schema::ComponentChildKind,
    control_plane_support::model::replay::ReplayCostGuardSettlement,
    dto::{
        component_registry::{
            ComponentLifecycleStatus, ComponentProvisioningOrigin, ComponentRegistryHead,
            ComponentRuntimeActivationEvidence,
        },
        fleet_registry::FleetRegistryVersion,
        root_store::RootStoreBootstrapRequest,
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentChildBinding, ComponentInstanceId,
        ComponentSpecId, FleetSubnetRootBinding, FleetSubnetRootReleaseSet,
    },
    impl_storable_bounded,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "root-control-plane")]
use std::{cell::RefCell, ops::Bound};

#[cfg(feature = "root-control-plane")]
const ROOT_COMPONENT_REGISTRY_STATE_MAX_BYTES: u32 = 65_536;
#[cfg(feature = "root-control-plane")]
const ROOT_COMPONENT_ALLOCATION_RECORD_MAX_BYTES: u32 = 4_096;
#[cfg(feature = "root-control-plane")]
const COMPONENT_REGISTRY_ENTRY_KEY_MAX_BYTES: u32 = 512;
#[cfg(feature = "root-control-plane")]
const COMPONENT_REGISTRY_ENTRY_RECORD_MAX_BYTES: u32 = 4_096;
#[cfg(feature = "root-control-plane")]
const SUBTREE_REMOVAL_HISTORY_KEY_MAX_BYTES: u32 = 256;
#[cfg(feature = "root-control-plane")]
const SUBTREE_REMOVAL_HISTORY_RECORD_MAX_BYTES: u32 = 1_024;
#[cfg(feature = "root-control-plane")]
const COMPONENT_DRAINING_RECORD_MAX_BYTES: u32 = 2_048;

#[cfg(feature = "root-control-plane")]
struct RootComponentRegistryState;
#[cfg(feature = "root-control-plane")]
struct RootComponentAllocations;
#[cfg(feature = "root-control-plane")]
struct ComponentRegistryEntries;
#[cfg(feature = "root-control-plane")]
struct ComponentRegistryPrincipalIndex;
#[cfg(feature = "root-control-plane")]
struct RootComponentSubtreeRemovalHistory;
#[cfg(feature = "root-control-plane")]
struct RootComponentDraining;

#[cfg(feature = "root-control-plane")]
eager_static! {
    static ROOT_COMPONENT_REGISTRY:
        RefCell<Cell<RootComponentRegistryStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            canic_core::ic_memory_key!(
                authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
                key = "canic.control_plane.root_component_registry.v1",
                ty = RootComponentRegistryState,
                id = ROOT_COMPONENT_REGISTRY_META_ID
            ),
            RootComponentRegistryStateRecord::default(),
        ));
}

#[cfg(feature = "root-control-plane")]
eager_static! {
    static ROOT_COMPONENT_DRAINING: RefCell<
        StableBtreeMap<
            RootComponentDrainingKey,
            RootComponentDrainingRecord,
            VirtualMemory<DefaultMemoryImpl>,
        >,
    > = RefCell::new(StableBtreeMap::init(
        canic_core::ic_memory_key!(
            authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            key = "canic.control_plane.root_component_draining.v1",
            ty = RootComponentDraining,
            id = ROOT_COMPONENT_DRAINING_ID
        ),
    ));
}

#[cfg(feature = "root-control-plane")]
eager_static! {
    static ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY: RefCell<
        StableBtreeMap<
            RootComponentSubtreeRemovalHistoryKey,
            RootComponentSubtreeRemovalCompletedLeafRecord,
            VirtualMemory<DefaultMemoryImpl>,
        >,
    > = RefCell::new(StableBtreeMap::init(
        canic_core::ic_memory_key!(
            authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            key = "canic.control_plane.root_component_subtree_removal_history.v1",
            ty = RootComponentSubtreeRemovalHistory,
            id = ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_ID
        ),
    ));
}

#[cfg(feature = "root-control-plane")]
eager_static! {
    static COMPONENT_REGISTRY_ENTRIES: RefCell<
        StableBtreeMap<
            ComponentRegistryEntryKey,
            ComponentRegistryEntryRecord,
            VirtualMemory<DefaultMemoryImpl>,
        >,
    > = RefCell::new(StableBtreeMap::init(
        canic_core::ic_memory_key!(
            authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            key = "canic.control_plane.component_registry_entries.v1",
            ty = ComponentRegistryEntries,
            id = ROOT_COMPONENT_REGISTRY_ENTRIES_ID
        ),
    ));
}

#[cfg(feature = "root-control-plane")]
eager_static! {
    static COMPONENT_REGISTRY_PRINCIPAL_INDEX: RefCell<
        StableBtreeMap<
            ComponentRegistryPrincipalKey,
            ComponentRegistryPrincipalIndexRecord,
            VirtualMemory<DefaultMemoryImpl>,
        >,
    > = RefCell::new(StableBtreeMap::init(
        canic_core::ic_memory_key!(
            authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            key = "canic.control_plane.component_registry_principal_index.v1",
            ty = ComponentRegistryPrincipalIndex,
            id = ROOT_COMPONENT_PRINCIPAL_INDEX_ID
        ),
    ));
}

#[cfg(feature = "root-control-plane")]
eager_static! {
    static ROOT_COMPONENT_ALLOCATIONS: RefCell<
        StableBtreeMap<
            RootComponentAllocationOperationKey,
            RootComponentAllocationRecord,
            VirtualMemory<DefaultMemoryImpl>,
        >,
    > = RefCell::new(StableBtreeMap::init(
        canic_core::ic_memory_key!(
            authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            key = "canic.control_plane.root_component_allocations.v1",
            ty = RootComponentAllocations,
            id = ROOT_COMPONENT_ALLOCATIONS_ID
        ),
    ));
}

///
/// RootComponentRegistryMetaRecord
///
/// Durable root authority and counters from which future Component allocations continue.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentRegistryMetaRecord {
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
    pub initial_inventory: Option<RootComponentInitialInventoryRecord>,
}

///
/// RootComponentInitialInventoryRecord
///
/// Immutable initial Component inventory plus terminal root-activation receipts.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentInitialInventoryRecord {
    pub fleet_activation_operation_id: [u8; 32],
    pub component_count: u32,
    pub inventory_hash: [u8; 32],
    pub sealed_at_ns: u64,
    pub directories_converged: bool,
    pub root_runtime_activated: bool,
}

///
/// RootComponentAllocationRecord
///
/// Durable exact top-level Component identity and capacity reservation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentAllocationRecord {
    pub operation_id: [u8; 32],
    pub allocation_sequence: u64,
    pub component: ComponentInstanceId,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub role: CanisterRole,
    pub provisioning_origin: ComponentProvisioningOrigin,
    pub release_set: FleetSubnetRootReleaseSet,
    pub progress: RootComponentAllocationProgressRecord,
}

impl RootComponentAllocationRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentAllocationRecord";
}

#[derive(Debug, Eq, PartialEq)]
struct RootComponentAllocationIdentity<'a> {
    operation_id: &'a [u8; 32],
    component: &'a ComponentInstanceId,
}

impl<'a> From<&'a RootComponentAllocationRecord> for RootComponentAllocationIdentity<'a> {
    fn from(record: &'a RootComponentAllocationRecord) -> Self {
        Self {
            operation_id: &record.operation_id,
            component: &record.component,
        }
    }
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(
    RootComponentAllocationRecord,
    ROOT_COMPONENT_ALLOCATION_RECORD_MAX_BYTES,
    false
);

///
/// RootComponentAllocationProgressRecord
///
/// Durable paid-effect boundary for one reserved top-level Component operation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentAllocationProgressRecord {
    Reserved,
    CreationIntent(RootComponentCreationEffectRecord),
    Created {
        effect: RootComponentCreationEffectRecord,
        canister: Principal,
    },
    InstallIntent {
        creation: RootComponentCreationEffectRecord,
        canister: Principal,
        installation: RootComponentInstallEffectRecord,
    },
    Installed {
        creation: RootComponentCreationEffectRecord,
        canister: Principal,
        installation: RootComponentInstallEffectRecord,
    },
    Verified {
        creation: RootComponentCreationEffectRecord,
        canister: Principal,
        installation: RootComponentInstallEffectRecord,
    },
    Committed {
        creation: RootComponentCreationEffectRecord,
        canister: Principal,
        installation: RootComponentInstallEffectRecord,
        commitment: RootComponentCommitmentRecord,
    },
}

///
/// RootComponentCreationEffectRecord
///
/// Exact artifact, settings and cost settlement frozen before Canister creation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentCreationEffectRecord {
    pub wasm_store: Principal,
    pub payload_hash: [u8; 32],
    pub payload_size_bytes: u64,
    pub initial_cycles: Cycles,
    pub controller: Principal,
    pub cost_guard_settlement: ReplayCostGuardSettlement,
    pub charged_entry_bytes: u64,
}

///
/// RootComponentInstallEffectRecord
///
/// Exact module source, target identity and cost settlement frozen before installation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentInstallEffectRecord {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentBinding,
    pub cost_guard_settlement: ReplayCostGuardSettlement,
    pub charged_entry_bytes: u64,
}

///
/// RootComponentCommitmentRecord
///
/// Durable link from one completed allocation operation to its Registry and Directory authority.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentCommitmentRecord {
    pub registry: ComponentRegistryHead,
    pub prepared_registry_encoded_bytes: u64,
    pub directory_synchronized_at_ns: u64,
    pub directory_authority_hash: [u8; 32],
    pub directory_prepared: bool,
    pub runtime_activated: bool,
    pub membership: Option<RootComponentMembershipRecord>,
}

///
/// RootComponentMembershipRecord
///
/// Immutable active-membership authority and terminal current-Directory receipt.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentMembershipRecord {
    pub registry_encoded_bytes: u64,
    pub directory_synchronized_at_ns: u64,
    pub directory_authority_hash: [u8; 32],
    pub directory_synchronized: bool,
}

///
/// ComponentRegistryPartitionRecord
///
/// Normalized authoritative top-level row and independent head for one Component tree.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRegistryPartitionRecord {
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

///
/// RootComponentDrainingRecord
///
/// One-per-Component draining fence with monotonic qualified-quiescence progress.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentDrainingRecord {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub previous_registry: ComponentRegistryHead,
    pub registry: ComponentRegistryHead,
    pub descendant_count: u32,
    pub descendant_content_hash: [u8; 32],
    pub directory_authority_hash: [u8; 32],
    pub started_at_ns: u64,
    pub quiescence: Option<RootComponentQuiescenceProgressRecord>,
}

///
/// RootComponentQuiescenceStopIntentRecord
///
/// Durable pre-effect authority and terminal-byte reservation for Component quiescence.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentQuiescenceStopIntentRecord {
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
/// RootComponentQuiescentReceiptRecord
///
/// Durable terminal evidence for one independently observed stopped Component.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentQuiescentReceiptRecord {
    pub stop: RootComponentQuiescenceStopIntentRecord,
    pub observed_module_hash: [u8; 32],
    pub quiesced_at_ns: u64,
}

///
/// RootComponentQuiescenceProgressRecord
///
/// Monotonic pre-effect or terminal quiescence state within one draining record.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentQuiescenceProgressRecord {
    StopIntent(RootComponentQuiescenceStopIntentRecord),
    Quiescent(RootComponentQuiescentReceiptRecord),
}

fn component_draining_identity_matches(
    left: &RootComponentDrainingRecord,
    right: &RootComponentDrainingRecord,
) -> bool {
    left.operation_id == right.operation_id
        && left.component == right.component
        && left.previous_registry == right.previous_registry
        && left.registry == right.registry
        && left.descendant_count == right.descendant_count
        && left.descendant_content_hash == right.descendant_content_hash
        && left.directory_authority_hash == right.directory_authority_hash
        && left.started_at_ns == right.started_at_ns
}

impl RootComponentDrainingRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentDrainingRecord";
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(
    RootComponentDrainingRecord,
    COMPONENT_DRAINING_RECORD_MAX_BYTES,
    false
);

#[derive(Debug, Eq, PartialEq)]
struct ComponentPartitionStableAuthority<'a> {
    binding: &'a ComponentBinding,
    provisioning_origin: &'a ComponentProvisioningOrigin,
    release_set: &'a FleetSubnetRootReleaseSet,
}

impl<'a> From<&'a ComponentRegistryPartitionRecord> for ComponentPartitionStableAuthority<'a> {
    fn from(partition: &'a ComponentRegistryPartitionRecord) -> Self {
        Self {
            binding: &partition.binding,
            provisioning_origin: &partition.provisioning_origin,
            release_set: &partition.release_set,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ComponentPartitionSnapshotAuthority<'a> {
    stable: ComponentPartitionStableAuthority<'a>,
    revision: u64,
    content_hash: &'a [u8; 32],
    descendant_content_hash: &'a [u8; 32],
    directory_synchronized_at_ns: u64,
    reserved_descendants: u32,
    committed_descendants: u32,
}

impl<'a> From<&'a ComponentRegistryPartitionRecord> for ComponentPartitionSnapshotAuthority<'a> {
    fn from(partition: &'a ComponentRegistryPartitionRecord) -> Self {
        Self {
            stable: ComponentPartitionStableAuthority::from(partition),
            revision: partition.revision,
            content_hash: &partition.content_hash,
            descendant_content_hash: &partition.descendant_content_hash,
            directory_synchronized_at_ns: partition.directory_synchronized_at_ns,
            reserved_descendants: partition.reserved_descendants,
            committed_descendants: partition.committed_descendants,
        }
    }
}

///
/// RootComponentChildAllocationRecord
///
/// Durable exact direct-child lifecycle operation inside one Component Registry partition.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildAllocationRecord {
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
    pub progress: RootComponentChildAllocationProgressRecord,
}

#[derive(Debug, Eq, PartialEq)]
struct RootComponentChildReservation<'a> {
    operation_id: &'a [u8; 32],
    component: &'a ComponentInstanceId,
    parent_canister_id: &'a Principal,
    parent_role: &'a CanisterRole,
    child_role: &'a CanisterRole,
    child_kind: &'a ComponentChildKind,
    maximum_instances_per_parent: u32,
    maximum_descendants: u32,
    maximum_registry_bytes: u64,
    reserved_against_registry: &'a ComponentRegistryHead,
    release_set: &'a FleetSubnetRootReleaseSet,
}

impl<'a> From<&'a RootComponentChildAllocationRecord> for RootComponentChildReservation<'a> {
    fn from(record: &'a RootComponentChildAllocationRecord) -> Self {
        Self {
            operation_id: &record.operation_id,
            component: &record.component,
            parent_canister_id: &record.parent_canister_id,
            parent_role: &record.parent_role,
            child_role: &record.child_role,
            child_kind: &record.child_kind,
            maximum_instances_per_parent: record.maximum_instances_per_parent,
            maximum_descendants: record.maximum_descendants,
            maximum_registry_bytes: record.maximum_registry_bytes,
            reserved_against_registry: &record.reserved_against_registry,
            release_set: &record.release_set,
        }
    }
}

impl RootComponentChildAllocationRecord {
    pub(crate) fn has_same_reservation(&self, other: &Self) -> bool {
        RootComponentChildReservation::from(self) == RootComponentChildReservation::from(other)
    }
}

///
/// RootComponentChildAllocationProgressRecord
///
/// Durable paid-effect boundary for one direct-child allocation operation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "stable child operations retain direct canonical records without indirection"
)]
pub enum RootComponentChildAllocationProgressRecord {
    Reserved,
    CreationIntent(RootComponentCreationEffectRecord),
    Created {
        effect: RootComponentCreationEffectRecord,
        canister: Principal,
    },
    InstallIntent {
        creation: RootComponentCreationEffectRecord,
        canister: Principal,
        installation: RootComponentChildInstallEffectRecord,
    },
    Installed {
        creation: RootComponentCreationEffectRecord,
        canister: Principal,
        installation: RootComponentChildInstallEffectRecord,
    },
    Verified {
        creation: RootComponentCreationEffectRecord,
        canister: Principal,
        installation: RootComponentChildInstallEffectRecord,
    },
    Committed {
        creation: RootComponentCreationEffectRecord,
        canister: Principal,
        installation: RootComponentChildInstallEffectRecord,
        commitment: RootComponentChildCommitmentRecord,
    },
}

///
/// RootComponentSubtreeRemovalRecord
///
/// Durable exact fence for one child-subtree removal operation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalRecord {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub target: ComponentRegistryChildRecord,
    pub reserved_against_registry: ComponentRegistryHead,
    pub maximum_completed_leaves: u32,
    pub completed_leaves: u32,
    pub traversal_steps: u32,
    pub progress: RootComponentSubtreeRemovalProgressRecord,
}

#[derive(Debug, Eq, PartialEq)]
struct RootComponentSubtreeFence<'a> {
    operation_id: &'a [u8; 32],
    component: &'a ComponentInstanceId,
    target: &'a ComponentRegistryChildRecord,
    reserved_against_registry: &'a ComponentRegistryHead,
    maximum_completed_leaves: u32,
}

impl<'a> From<&'a RootComponentSubtreeRemovalRecord> for RootComponentSubtreeFence<'a> {
    fn from(record: &'a RootComponentSubtreeRemovalRecord) -> Self {
        Self {
            operation_id: &record.operation_id,
            component: &record.component,
            target: &record.target,
            reserved_against_registry: &record.reserved_against_registry,
            maximum_completed_leaves: record.maximum_completed_leaves,
        }
    }
}

impl RootComponentSubtreeRemovalRecord {
    pub(crate) fn has_same_fence(&self, other: &Self) -> bool {
        RootComponentSubtreeFence::from(self) == RootComponentSubtreeFence::from(other)
    }
}

///
/// RootComponentSubtreeRemovalProgressRecord
///
/// Durable post-order removal progress after new subtree mutations are fenced.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "stable progress retains complete inline receipts for deterministic replay"
)]
pub enum RootComponentSubtreeRemovalProgressRecord {
    Fenced,
    Traversing {
        cursor: ComponentRegistryChildRecord,
    },
    LeafSelected {
        leaf: ComponentRegistryChildRecord,
    },
    StopIntent(RootComponentSubtreeStopEffectRecord),
    Stopped(RootComponentSubtreeStoppedEffectRecord),
    DeleteIntent(RootComponentSubtreeDeleteEffectRecord),
    Deleted(RootComponentSubtreeDeletedEffectRecord),
    MembershipRemoved(RootComponentSubtreeMembershipRemovedRecord),
    DirectorySynchronized(RootComponentSubtreeDirectorySynchronizedRecord),
    Completed(RootComponentSubtreeRemovalCompletedRecord),
}

///
/// RootComponentSubtreeStopEffectRecord
///
/// Exact registered leaf and sole root controller frozen before a stop call.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeStopEffectRecord {
    pub leaf: ComponentRegistryChildRecord,
    pub controller: Principal,
}

///
/// RootComponentSubtreeStoppedEffectRecord
///
/// Frozen stop authority plus the module hash independently observed stopped.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeStoppedEffectRecord {
    pub stop: RootComponentSubtreeStopEffectRecord,
    pub observed_module_hash: [u8; 32],
}

///
/// RootComponentSubtreeDeleteEffectRecord
///
/// Exact stopped receipt frozen before a destructive management call.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeDeleteEffectRecord {
    pub stopped: RootComponentSubtreeStoppedEffectRecord,
}

///
/// RootComponentSubtreeDeletedEffectRecord
///
/// Exact deletion authority retained after independently observed absence.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeDeletedEffectRecord {
    pub deletion: RootComponentSubtreeDeleteEffectRecord,
}

///
/// RootComponentSubtreeMembershipRemovedRecord
///
/// Exact Registry transition retained after the independently deleted leaf is unregistered.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeMembershipRemovedRecord {
    pub deleted: RootComponentSubtreeDeletedEffectRecord,
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
/// RootComponentSubtreeDirectoryConvergenceRecord
///
/// Compact stable proof that one surviving member covered the required Directory.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeDirectoryConvergenceRecord {
    pub operation_id: [u8; 32],
    pub canister_id: Principal,
    pub activation: ComponentRuntimeActivationEvidence,
}

///
/// RootComponentSubtreeDirectorySynchronizedRecord
///
/// Membership removal plus independently verified surviving-member convergence.
///
/// The owner is absent only under terminal top-level Component quiescence.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeDirectorySynchronizedRecord {
    pub membership_removed: RootComponentSubtreeMembershipRemovedRecord,
    pub covered_fleet_registry_revision: u64,
    pub covered_fleet_registry_content_hash: [u8; 32],
    pub covered_component_registry_revision: u64,
    pub covered_component_registry_content_hash: [u8; 32],
    pub covered_authority_hash: [u8; 32],
    pub owning_component: Option<RootComponentSubtreeDirectoryConvergenceRecord>,
    pub parent: Option<RootComponentSubtreeDirectoryConvergenceRecord>,
}

///
/// RootComponentSubtreeRemovalCompletedRecord
///
/// Compact terminal authority after the fenced target itself is finalized.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalCompletedRecord {
    pub registry: ComponentRegistryHead,
    pub directory_authority_hash: [u8; 32],
}

///
/// RootComponentSubtreeRemovalCompletedLeafRecord
///
/// Compact immutable operation/step-keyed history for one finalized leaf.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentSubtreeRemovalCompletedLeafRecord {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub traversal_steps: u32,
    pub leaf_canister_id: Principal,
    pub leaf_parent_canister_id: Principal,
    pub observed_module_hash: [u8; 32],
    pub registry: ComponentRegistryHead,
    pub directory_authority_hash: [u8; 32],
    pub receipt_hash: [u8; 32],
}

impl RootComponentSubtreeRemovalCompletedLeafRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentSubtreeRemovalCompletedLeafRecord";
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(
    RootComponentSubtreeRemovalCompletedLeafRecord,
    SUBTREE_REMOVAL_HISTORY_RECORD_MAX_BYTES,
    false
);

///
/// RootComponentChildInstallEffectRecord
///
/// Exact child module source, immutable binding and cost settlement frozen before installation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildInstallEffectRecord {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentChildBinding,
    pub cost_guard_settlement: ReplayCostGuardSettlement,
    pub charged_entry_bytes: u64,
}

///
/// RootComponentChildCommitmentRecord
///
/// Immutable child-commit Registry head plus later Directory and membership receipts.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildCommitmentRecord {
    pub registry: ComponentRegistryHead,
    pub descendant_content_hash: [u8; 32],
    pub registry_encoded_bytes: u64,
    pub reserved_descendants: u32,
    pub committed_descendants: u32,
    pub directory_synchronized_at_ns: u64,
    pub directory_authority_hash: [u8; 32],
    pub directory_prepared: bool,
    pub runtime_activated: bool,
    pub membership: Option<RootComponentChildMembershipRecord>,
}

///
/// RootComponentChildMembershipRecord
///
/// Immutable active child head plus terminal current-Directory receipt.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentChildMembershipRecord {
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
/// ComponentRegistryParentRoleCountRecord
///
/// Exact reserved plus committed non-removed count for one parent and direct-child role.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRegistryParentRoleCountRecord {
    pub component: ComponentInstanceId,
    pub parent_canister_id: Principal,
    pub child_role: CanisterRole,
    pub instances: u32,
}

///
/// ComponentRegistryChildRecord
///
/// Normalized authoritative child row retained at any depth in one Component tree.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRegistryChildRecord {
    pub component: ComponentInstanceId,
    pub canister_id: Principal,
    pub parent_canister_id: Principal,
    pub role: CanisterRole,
    pub kind: ComponentChildKind,
    pub installed_artifact_hash: [u8; 32],
    pub status: ComponentLifecycleStatus,
}

///
/// ComponentRegistryChildTraversalRecord
///
/// Compact parent/role traversal index value for one normalized child row.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRegistryChildTraversalRecord {
    pub component: ComponentInstanceId,
    pub parent_canister_id: Principal,
    pub role: CanisterRole,
    pub canister_id: Principal,
}

#[derive(Debug, Eq, PartialEq)]
struct ComponentParentRoleAuthority<'a> {
    component: &'a ComponentInstanceId,
    parent_canister_id: &'a Principal,
    child_role: &'a CanisterRole,
}

impl<'a> ComponentParentRoleAuthority<'a> {
    const fn from_count(record: &'a ComponentRegistryParentRoleCountRecord) -> Self {
        Self {
            component: &record.component,
            parent_canister_id: &record.parent_canister_id,
            child_role: &record.child_role,
        }
    }

    const fn from_allocation(record: &'a RootComponentChildAllocationRecord) -> Self {
        Self {
            component: &record.component,
            parent_canister_id: &record.parent_canister_id,
            child_role: &record.child_role,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ComponentChildIndexAuthority<'a> {
    component: &'a ComponentInstanceId,
    parent_canister_id: &'a Principal,
    role: &'a CanisterRole,
    canister_id: Principal,
}

impl<'a> ComponentChildIndexAuthority<'a> {
    const fn from_child(record: &'a ComponentRegistryChildRecord) -> Self {
        Self {
            component: &record.component,
            parent_canister_id: &record.parent_canister_id,
            role: &record.role,
            canister_id: record.canister_id,
        }
    }

    const fn from_traversal(record: &'a ComponentRegistryChildTraversalRecord) -> Self {
        Self {
            component: &record.component,
            parent_canister_id: &record.parent_canister_id,
            role: &record.role,
            canister_id: record.canister_id,
        }
    }

    const fn from_allocation(
        record: &'a RootComponentChildAllocationRecord,
        canister_id: Principal,
    ) -> Self {
        Self {
            component: &record.component,
            parent_canister_id: &record.parent_canister_id,
            role: &record.child_role,
            canister_id,
        }
    }
}

///
/// ComponentRegistryEntryRecord
///
/// One normalized partition, operation or index value in the Component-first Registry collection.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "stable Registry values retain direct canonical records without heap-indirection semantics"
)]
pub enum ComponentRegistryEntryRecord {
    Partition(ComponentRegistryPartitionRecord),
    Child(ComponentRegistryChildRecord),
    ChildTraversal(ComponentRegistryChildTraversalRecord),
    ChildAllocation(RootComponentChildAllocationRecord),
    SubtreeRemoval(RootComponentSubtreeRemovalRecord),
    ParentRoleCount(ComponentRegistryParentRoleCountRecord),
}

impl ComponentRegistryEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "ComponentRegistryEntryRecord";
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(
    ComponentRegistryEntryRecord,
    COMPONENT_REGISTRY_ENTRY_RECORD_MAX_BYTES,
    false
);

///
/// ComponentRegistryPrincipalIndexRecord
///
/// Durable principal-to-Component lookup value derived from committed partitions.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRegistryPrincipalIndexRecord {
    pub component: ComponentInstanceId,
}

impl ComponentRegistryPrincipalIndexRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "ComponentRegistryPrincipalIndexRecord";
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(ComponentRegistryPrincipalIndexRecord, 128, false);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct RootComponentAllocationOperationKey([u8; 32]);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct RootComponentDrainingKey([u8; 32]);

impl From<ComponentInstanceId> for RootComponentDrainingKey {
    fn from(value: ComponentInstanceId) -> Self {
        Self(*value.as_bytes())
    }
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(RootComponentDrainingKey, 64, false);

impl From<[u8; 32]> for RootComponentAllocationOperationKey {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(RootComponentAllocationOperationKey, 128, false);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct RootComponentSubtreeRemovalHistoryKey {
    component: [u8; 32],
    operation_id: [u8; 32],
    traversal_steps: u32,
}

impl RootComponentSubtreeRemovalHistoryKey {
    const fn new(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        traversal_steps: u32,
    ) -> Self {
        Self {
            component: *component.as_bytes(),
            operation_id,
            traversal_steps,
        }
    }
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(
    RootComponentSubtreeRemovalHistoryKey,
    SUBTREE_REMOVAL_HISTORY_KEY_MAX_BYTES,
    false
);

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct ComponentRegistryPrincipalKey(Vec<u8>);

impl From<Principal> for ComponentRegistryPrincipalKey {
    fn from(value: Principal) -> Self {
        Self(value.as_slice().to_vec())
    }
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(ComponentRegistryPrincipalKey, 128, false);

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct ComponentRegistryEntryKey {
    component: [u8; 32],
    index: ComponentRegistryEntryIndexKey,
}

impl ComponentRegistryEntryKey {
    const fn partition(component: ComponentInstanceId) -> Self {
        Self {
            component: *component.as_bytes(),
            index: ComponentRegistryEntryIndexKey::Partition,
        }
    }

    const fn child_allocation(component: ComponentInstanceId, operation_id: [u8; 32]) -> Self {
        Self {
            component: *component.as_bytes(),
            index: ComponentRegistryEntryIndexKey::ChildAllocation(operation_id),
        }
    }

    const fn child_allocation_range_start(component: ComponentInstanceId) -> Self {
        Self::child_allocation(component, [0; 32])
    }

    const fn child_allocation_range_end(component: ComponentInstanceId) -> Self {
        Self::child_allocation(component, [u8::MAX; 32])
    }

    const fn subtree_removal(component: ComponentInstanceId, operation_id: [u8; 32]) -> Self {
        Self {
            component: *component.as_bytes(),
            index: ComponentRegistryEntryIndexKey::SubtreeRemoval(operation_id),
        }
    }

    const fn subtree_removal_range_start(component: ComponentInstanceId) -> Self {
        Self::subtree_removal(component, [0; 32])
    }

    const fn subtree_removal_range_end(component: ComponentInstanceId) -> Self {
        Self::subtree_removal(component, [u8::MAX; 32])
    }

    fn child(component: ComponentInstanceId, canister_id: Principal) -> Self {
        Self {
            component: *component.as_bytes(),
            index: ComponentRegistryEntryIndexKey::Child(canister_id.as_slice().to_vec()),
        }
    }

    fn child_traversal(
        component: ComponentInstanceId,
        parent_canister_id: Principal,
        role: &CanisterRole,
        canister_id: Principal,
    ) -> Self {
        Self {
            component: *component.as_bytes(),
            index: ComponentRegistryEntryIndexKey::ChildTraversal {
                parent_canister_id: parent_canister_id.as_slice().to_vec(),
                role: role.clone(),
                canister_id: canister_id.as_slice().to_vec(),
            },
        }
    }

    fn child_traversal_parent_start(
        component: ComponentInstanceId,
        parent_canister_id: Principal,
    ) -> Self {
        Self {
            component: *component.as_bytes(),
            index: ComponentRegistryEntryIndexKey::ChildTraversal {
                parent_canister_id: parent_canister_id.as_slice().to_vec(),
                role: CanisterRole::new(""),
                canister_id: Vec::new(),
            },
        }
    }

    fn child_traversal_parent_end(
        component: ComponentInstanceId,
        parent_canister_id: Principal,
    ) -> Self {
        let mut parent_canister_id = parent_canister_id.as_slice().to_vec();
        parent_canister_id.push(0);
        Self {
            component: *component.as_bytes(),
            index: ComponentRegistryEntryIndexKey::ChildTraversal {
                parent_canister_id,
                role: CanisterRole::new(""),
                canister_id: Vec::new(),
            },
        }
    }

    fn child_traversal_parent_role_start(
        component: ComponentInstanceId,
        parent_canister_id: Principal,
        role: &CanisterRole,
    ) -> Self {
        Self {
            component: *component.as_bytes(),
            index: ComponentRegistryEntryIndexKey::ChildTraversal {
                parent_canister_id: parent_canister_id.as_slice().to_vec(),
                role: role.clone(),
                canister_id: Vec::new(),
            },
        }
    }

    fn child_traversal_parent_role_end(
        component: ComponentInstanceId,
        parent_canister_id: Principal,
        role: &CanisterRole,
    ) -> Self {
        Self {
            component: *component.as_bytes(),
            index: ComponentRegistryEntryIndexKey::ChildTraversal {
                parent_canister_id: parent_canister_id.as_slice().to_vec(),
                role: role.clone(),
                canister_id: vec![u8::MAX; Principal::MAX_LENGTH_IN_BYTES + 1],
            },
        }
    }

    const fn child_traversal_range_start(component: ComponentInstanceId) -> Self {
        Self {
            component: *component.as_bytes(),
            index: ComponentRegistryEntryIndexKey::ChildTraversal {
                parent_canister_id: Vec::new(),
                role: CanisterRole::new(""),
                canister_id: Vec::new(),
            },
        }
    }

    fn child_traversal_range_end(component: ComponentInstanceId) -> Self {
        Self {
            component: *component.as_bytes(),
            index: ComponentRegistryEntryIndexKey::ChildTraversal {
                parent_canister_id: vec![u8::MAX; Principal::MAX_LENGTH_IN_BYTES + 1],
                role: CanisterRole::new(""),
                canister_id: Vec::new(),
            },
        }
    }

    fn parent_role_count(
        component: ComponentInstanceId,
        parent_canister_id: Principal,
        child_role: &CanisterRole,
    ) -> Self {
        Self {
            component: *component.as_bytes(),
            index: ComponentRegistryEntryIndexKey::ParentRoleCount {
                parent_canister_id: parent_canister_id.as_slice().to_vec(),
                child_role: child_role.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
enum ComponentRegistryEntryIndexKey {
    Partition,
    Child(Vec<u8>),
    ChildTraversal {
        parent_canister_id: Vec<u8>,
        role: CanisterRole,
        canister_id: Vec<u8>,
    },
    ChildAllocation([u8; 32]),
    SubtreeRemoval([u8; 32]),
    ParentRoleCount {
        parent_canister_id: Vec<u8>,
        child_role: CanisterRole,
    },
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(
    ComponentRegistryEntryKey,
    COMPONENT_REGISTRY_ENTRY_KEY_MAX_BYTES,
    false
);

///
/// RootComponentRegistryStateRecord
///
/// Stable optional wrapper before the exact root authority is prepared.
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentRegistryStateRecord {
    pub current: Option<RootComponentRegistryMetaRecord>,
}

impl RootComponentRegistryStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentRegistryStateRecord";
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(
    RootComponentRegistryStateRecord,
    ROOT_COMPONENT_REGISTRY_STATE_MAX_BYTES,
    false
);

///
/// RootComponentRegistryData
///
/// Canonical export snapshot for root Component Registry meta authority.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RootComponentRegistryData {
    pub current: Option<RootComponentRegistryMetaRecord>,
    pub allocations: Vec<RootComponentAllocationRecord>,
    pub partitions: Vec<ComponentRegistryPartitionRecord>,
    pub children: Vec<ComponentRegistryChildRecord>,
    pub child_traversals: Vec<ComponentRegistryChildTraversalRecord>,
    pub child_allocations: Vec<RootComponentChildAllocationRecord>,
    pub subtree_removals: Vec<RootComponentSubtreeRemovalRecord>,
    pub subtree_removal_history: Vec<RootComponentSubtreeRemovalCompletedLeafRecord>,
    pub component_drainings: Vec<RootComponentDrainingRecord>,
    pub parent_role_counts: Vec<ComponentRegistryParentRoleCountRecord>,
}

impl RootComponentRegistryData {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentRegistryData";
}

///
/// RootComponentRegistryCommitOutcome
///
/// Result of preparing the one root-local Component Registry authority.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootComponentRegistryCommitOutcome {
    Committed,
    Existing,
}

///
/// RootComponentRegistryCommitError
///
/// Rejection when preparation conflicts with already durable authority.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootComponentRegistryCommitError {
    ConflictingState,
}

///
/// RootComponentAllocationCommitError
///
/// Stable-store rejection for one top-level Component identity reservation.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootComponentAllocationCommitError {
    ComponentIdentityConflict,
    ComponentPrincipalConflict,
    ConflictingChildEntry,
    ConflictingPartition,
    ConflictingOperation,
    ConflictingState,
    MissingOperation,
    ParentPrincipalConflict,
    Uninitialized,
}

/// Narrow stable owner for root-local Component Registry meta authority.
pub struct RootComponentRegistryStore;

#[cfg(feature = "root-control-plane")]
impl RootComponentRegistryStore {
    pub(crate) fn prepare(
        record: RootComponentRegistryMetaRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            match state.current.as_ref() {
                None => {
                    state.current = Some(record);
                    cell.set(state);
                    Ok(RootComponentRegistryCommitOutcome::Committed)
                }
                Some(existing) if existing == &record => {
                    Ok(RootComponentRegistryCommitOutcome::Existing)
                }
                Some(_) => Err(RootComponentRegistryCommitError::ConflictingState),
            }
        })
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn export() -> RootComponentRegistryData {
        ROOT_COMPONENT_REGISTRY.with_borrow(|cell| RootComponentRegistryData {
            current: cell.get().current.clone(),
            allocations: ROOT_COMPONENT_ALLOCATIONS
                .with_borrow(|map| map.iter().map(|entry| entry.value()).collect()),
            partitions: COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.iter()
                    .filter_map(|entry| match entry.value() {
                        ComponentRegistryEntryRecord::Partition(record) => Some(record),
                        ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    })
                    .collect()
            }),
            children: COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.iter()
                    .filter_map(|entry| match entry.value() {
                        ComponentRegistryEntryRecord::Child(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    })
                    .collect()
            }),
            child_traversals: COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.iter()
                    .filter_map(|entry| match entry.value() {
                        ComponentRegistryEntryRecord::ChildTraversal(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    })
                    .collect()
            }),
            child_allocations: COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.iter()
                    .filter_map(|entry| match entry.value() {
                        ComponentRegistryEntryRecord::ChildAllocation(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    })
                    .collect()
            }),
            subtree_removals: COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.iter()
                    .filter_map(|entry| match entry.value() {
                        ComponentRegistryEntryRecord::SubtreeRemoval(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    })
                    .collect()
            }),
            subtree_removal_history: ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY
                .with_borrow(|map| map.iter().map(|entry| entry.value()).collect()),
            component_drainings: ROOT_COMPONENT_DRAINING
                .with_borrow(|map| map.iter().map(|entry| entry.value()).collect()),
            parent_role_counts: COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.iter()
                    .filter_map(|entry| match entry.value() {
                        ComponentRegistryEntryRecord::ParentRoleCount(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_) => None,
                    })
                    .collect()
            }),
        })
    }

    #[must_use]
    pub(crate) fn current() -> Option<RootComponentRegistryMetaRecord> {
        ROOT_COMPONENT_REGISTRY.with_borrow(|cell| cell.get().current.clone())
    }

    #[must_use]
    pub(crate) fn allocations() -> Vec<RootComponentAllocationRecord> {
        ROOT_COMPONENT_ALLOCATIONS
            .with_borrow(|map| map.iter().map(|entry| entry.value()).collect())
    }

    #[must_use]
    pub(crate) fn partitions() -> Vec<ComponentRegistryPartitionRecord> {
        COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
            map.iter()
                .filter_map(|entry| match entry.value() {
                    ComponentRegistryEntryRecord::Partition(record) => Some(record),
                    ComponentRegistryEntryRecord::Child(_)
                    | ComponentRegistryEntryRecord::ChildTraversal(_)
                    | ComponentRegistryEntryRecord::ChildAllocation(_)
                    | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                    | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                })
                .collect()
        })
    }

    pub(crate) fn replace_meta(
        expected: &RootComponentRegistryMetaRecord,
        next: RootComponentRegistryMetaRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if current != expected {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            state.current = Some(next);
            cell.set(state);
            Ok(())
        })
    }

    #[must_use]
    pub(crate) fn allocation(operation_id: [u8; 32]) -> Option<RootComponentAllocationRecord> {
        ROOT_COMPONENT_ALLOCATIONS
            .with_borrow(|map| map.get(&RootComponentAllocationOperationKey::from(operation_id)))
    }

    #[must_use]
    pub(crate) fn allocation_counts(component_spec: &ComponentSpecId) -> (usize, usize) {
        ROOT_COMPONENT_ALLOCATIONS.with_borrow(|map| {
            map.iter().fold((0, 0), |(reserved, committed), entry| {
                let record = entry.value();
                if &record.component_spec != component_spec {
                    return (reserved, committed);
                }
                if matches!(
                    record.progress,
                    RootComponentAllocationProgressRecord::Committed { .. }
                ) {
                    (reserved, committed + 1)
                } else {
                    (reserved + 1, committed)
                }
            })
        })
    }

    #[must_use]
    pub(crate) fn partition(
        component: ComponentInstanceId,
    ) -> Option<ComponentRegistryPartitionRecord> {
        COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
            match map.get(&ComponentRegistryEntryKey::partition(component)) {
                Some(ComponentRegistryEntryRecord::Partition(record)) => Some(record),
                Some(
                    ComponentRegistryEntryRecord::Child(_)
                    | ComponentRegistryEntryRecord::ChildTraversal(_)
                    | ComponentRegistryEntryRecord::ChildAllocation(_)
                    | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                    | ComponentRegistryEntryRecord::ParentRoleCount(_),
                )
                | None => None,
            }
        })
    }

    #[must_use]
    pub(crate) fn component_draining(
        component: ComponentInstanceId,
    ) -> Option<RootComponentDrainingRecord> {
        ROOT_COMPONENT_DRAINING
            .with_borrow(|map| map.get(&RootComponentDrainingKey::from(component)))
    }

    #[must_use]
    pub(crate) fn child_allocation(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Option<RootComponentChildAllocationRecord> {
        COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
            match map.get(&ComponentRegistryEntryKey::child_allocation(
                component,
                operation_id,
            )) {
                Some(ComponentRegistryEntryRecord::ChildAllocation(record)) => Some(record),
                Some(
                    ComponentRegistryEntryRecord::Partition(_)
                    | ComponentRegistryEntryRecord::Child(_)
                    | ComponentRegistryEntryRecord::ChildTraversal(_)
                    | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                    | ComponentRegistryEntryRecord::ParentRoleCount(_),
                )
                | None => None,
            }
        })
    }

    #[must_use]
    pub(crate) fn subtree_removal(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Option<RootComponentSubtreeRemovalRecord> {
        COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
            match map.get(&ComponentRegistryEntryKey::subtree_removal(
                component,
                operation_id,
            )) {
                Some(ComponentRegistryEntryRecord::SubtreeRemoval(record)) => Some(record),
                Some(
                    ComponentRegistryEntryRecord::Partition(_)
                    | ComponentRegistryEntryRecord::Child(_)
                    | ComponentRegistryEntryRecord::ChildTraversal(_)
                    | ComponentRegistryEntryRecord::ChildAllocation(_)
                    | ComponentRegistryEntryRecord::ParentRoleCount(_),
                )
                | None => None,
            }
        })
    }

    #[must_use]
    pub(crate) fn subtree_removal_completed_leaf(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        traversal_steps: u32,
    ) -> Option<RootComponentSubtreeRemovalCompletedLeafRecord> {
        ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY.with_borrow(|map| {
            map.get(&RootComponentSubtreeRemovalHistoryKey::new(
                component,
                operation_id,
                traversal_steps,
            ))
        })
    }

    #[must_use]
    pub(crate) fn subtree_removals(
        component: ComponentInstanceId,
    ) -> Vec<RootComponentSubtreeRemovalRecord> {
        COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
            map.range((
                Bound::Included(ComponentRegistryEntryKey::subtree_removal_range_start(
                    component,
                )),
                Bound::Included(ComponentRegistryEntryKey::subtree_removal_range_end(
                    component,
                )),
            ))
            .filter_map(|entry| match entry.value() {
                ComponentRegistryEntryRecord::SubtreeRemoval(record) => Some(record),
                ComponentRegistryEntryRecord::Partition(_)
                | ComponentRegistryEntryRecord::Child(_)
                | ComponentRegistryEntryRecord::ChildTraversal(_)
                | ComponentRegistryEntryRecord::ChildAllocation(_)
                | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
            })
            .collect()
        })
    }

    #[must_use]
    pub(crate) fn child_allocations(
        component: ComponentInstanceId,
    ) -> Vec<RootComponentChildAllocationRecord> {
        COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
            map.range((
                Bound::Included(ComponentRegistryEntryKey::child_allocation_range_start(
                    component,
                )),
                Bound::Included(ComponentRegistryEntryKey::child_allocation_range_end(
                    component,
                )),
            ))
            .filter_map(|entry| match entry.value() {
                ComponentRegistryEntryRecord::ChildAllocation(record) => Some(record),
                ComponentRegistryEntryRecord::Partition(_)
                | ComponentRegistryEntryRecord::Child(_)
                | ComponentRegistryEntryRecord::ChildTraversal(_)
                | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
            })
            .collect()
        })
    }

    #[must_use]
    pub(crate) fn child(
        component: ComponentInstanceId,
        canister_id: Principal,
    ) -> Option<ComponentRegistryChildRecord> {
        COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
            match map.get(&ComponentRegistryEntryKey::child(component, canister_id)) {
                Some(ComponentRegistryEntryRecord::Child(record)) => Some(record),
                Some(
                    ComponentRegistryEntryRecord::Partition(_)
                    | ComponentRegistryEntryRecord::ChildTraversal(_)
                    | ComponentRegistryEntryRecord::ChildAllocation(_)
                    | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                    | ComponentRegistryEntryRecord::ParentRoleCount(_),
                )
                | None => None,
            }
        })
    }

    #[must_use]
    pub(crate) fn child_traversal(
        component: ComponentInstanceId,
        parent_canister_id: Principal,
        role: &CanisterRole,
        canister_id: Principal,
    ) -> Option<ComponentRegistryChildTraversalRecord> {
        COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
            match map.get(&ComponentRegistryEntryKey::child_traversal(
                component,
                parent_canister_id,
                role,
                canister_id,
            )) {
                Some(ComponentRegistryEntryRecord::ChildTraversal(record)) => Some(record),
                Some(
                    ComponentRegistryEntryRecord::Partition(_)
                    | ComponentRegistryEntryRecord::Child(_)
                    | ComponentRegistryEntryRecord::ChildAllocation(_)
                    | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                    | ComponentRegistryEntryRecord::ParentRoleCount(_),
                )
                | None => None,
            }
        })
    }

    #[must_use]
    pub(crate) fn child_traversals_page(
        component: ComponentInstanceId,
        parent_canister_id: Option<Principal>,
        role: Option<&CanisterRole>,
        start_after: Option<(&Principal, &CanisterRole, &Principal)>,
        limit: usize,
    ) -> Vec<ComponentRegistryChildTraversalRecord> {
        COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
            let lower = match start_after {
                Some((parent_canister_id, role, canister_id)) => {
                    Bound::Excluded(ComponentRegistryEntryKey::child_traversal(
                        component,
                        *parent_canister_id,
                        role,
                        *canister_id,
                    ))
                }
                None => Bound::Included(match (parent_canister_id, role) {
                    (Some(parent_canister_id), Some(role)) => {
                        ComponentRegistryEntryKey::child_traversal_parent_role_start(
                            component,
                            parent_canister_id,
                            role,
                        )
                    }
                    (Some(parent_canister_id), None) => {
                        ComponentRegistryEntryKey::child_traversal_parent_start(
                            component,
                            parent_canister_id,
                        )
                    }
                    (None, Some(_) | None) => {
                        ComponentRegistryEntryKey::child_traversal_range_start(component)
                    }
                }),
            };
            let upper = match (parent_canister_id, role) {
                (Some(parent_canister_id), Some(role)) => {
                    Bound::Included(ComponentRegistryEntryKey::child_traversal_parent_role_end(
                        component,
                        parent_canister_id,
                        role,
                    ))
                }
                (Some(parent_canister_id), None) => {
                    Bound::Excluded(ComponentRegistryEntryKey::child_traversal_parent_end(
                        component,
                        parent_canister_id,
                    ))
                }
                (None, Some(_) | None) => Bound::Excluded(
                    ComponentRegistryEntryKey::child_traversal_range_end(component),
                ),
            };
            map.range((lower, upper))
                .filter_map(|entry| match entry.value() {
                    ComponentRegistryEntryRecord::ChildTraversal(record) => Some(record),
                    ComponentRegistryEntryRecord::Partition(_)
                    | ComponentRegistryEntryRecord::Child(_)
                    | ComponentRegistryEntryRecord::ChildAllocation(_)
                    | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                    | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                })
                .take(limit)
                .collect()
        })
    }

    #[must_use]
    pub(crate) fn parent_role_count(
        component: ComponentInstanceId,
        parent_canister_id: Principal,
        child_role: &CanisterRole,
    ) -> Option<ComponentRegistryParentRoleCountRecord> {
        COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
            match map.get(&ComponentRegistryEntryKey::parent_role_count(
                component,
                parent_canister_id,
                child_role,
            )) {
                Some(ComponentRegistryEntryRecord::ParentRoleCount(record)) => Some(record),
                Some(
                    ComponentRegistryEntryRecord::Partition(_)
                    | ComponentRegistryEntryRecord::Child(_)
                    | ComponentRegistryEntryRecord::ChildTraversal(_)
                    | ComponentRegistryEntryRecord::ChildAllocation(_)
                    | ComponentRegistryEntryRecord::SubtreeRemoval(_),
                )
                | None => None,
            }
        })
    }

    #[must_use]
    pub(crate) fn component_for_principal(canister: Principal) -> Option<ComponentInstanceId> {
        COMPONENT_REGISTRY_PRINCIPAL_INDEX
            .with_borrow(|map| map.get(&ComponentRegistryPrincipalKey::from(canister)))
            .map(|record| record.component)
    }

    pub(crate) fn reserve_allocation(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        record: RootComponentAllocationRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentAllocationCommitError> {
        let key = RootComponentAllocationOperationKey::from(record.operation_id);
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;

            if let Some(existing) = ROOT_COMPONENT_ALLOCATIONS.with_borrow(|map| map.get(&key)) {
                return if existing == record {
                    Ok(RootComponentRegistryCommitOutcome::Existing)
                } else {
                    Err(RootComponentAllocationCommitError::ConflictingOperation)
                };
            }
            if current != expected_meta {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            if ROOT_COMPONENT_ALLOCATIONS.with_borrow(|map| {
                map.iter()
                    .any(|entry| entry.value().component == record.component)
            }) {
                return Err(RootComponentAllocationCommitError::ComponentIdentityConflict);
            }

            ROOT_COMPONENT_ALLOCATIONS.with_borrow_mut(|map| {
                map.insert(key, record);
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one stable transaction compares and commits all child-reservation indexes"
    )]
    pub(crate) fn reserve_child_allocation(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_partition: &ComponentRegistryPartitionRecord,
        next_partition: ComponentRegistryPartitionRecord,
        record: RootComponentChildAllocationRecord,
        expected_parent_role_count: Option<&ComponentRegistryParentRoleCountRecord>,
        next_parent_role_count: ComponentRegistryParentRoleCountRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentAllocationCommitError> {
        let component = record.component;
        let operation_key =
            ComponentRegistryEntryKey::child_allocation(component, record.operation_id);
        let count_key = ComponentRegistryEntryKey::parent_role_count(
            component,
            record.parent_canister_id,
            &record.child_role,
        );
        let partition_component_matches = expected_partition.binding.component == component
            && next_partition.binding.component == component;
        let parent_role_matches = ComponentParentRoleAuthority::from_count(&next_parent_role_count)
            == ComponentParentRoleAuthority::from_allocation(&record);
        if !partition_component_matches || !parent_role_matches {
            return Err(RootComponentAllocationCommitError::ConflictingChildEntry);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if let Some(existing) =
                COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| map.get(&operation_key))
            {
                return match existing {
                    ComponentRegistryEntryRecord::ChildAllocation(existing)
                        if existing.has_same_reservation(&record) =>
                    {
                        Ok(RootComponentRegistryCommitOutcome::Existing)
                    }
                    ComponentRegistryEntryRecord::Partition(_)
                    | ComponentRegistryEntryRecord::Child(_)
                    | ComponentRegistryEntryRecord::ChildTraversal(_)
                    | ComponentRegistryEntryRecord::ChildAllocation(_)
                    | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                    | ComponentRegistryEntryRecord::ParentRoleCount(_) => {
                        Err(RootComponentAllocationCommitError::ConflictingOperation)
                    }
                };
            }
            if current_meta != expected_meta {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let current_partition = COMPONENT_REGISTRY_ENTRIES
                .with_borrow(|map| {
                    map.get(&ComponentRegistryEntryKey::partition(component))
                        .and_then(|entry| match entry {
                            ComponentRegistryEntryRecord::Partition(record) => Some(record),
                            ComponentRegistryEntryRecord::Child(_)
                            | ComponentRegistryEntryRecord::ChildTraversal(_)
                            | ComponentRegistryEntryRecord::ChildAllocation(_)
                            | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                            | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                        })
                })
                .ok_or(RootComponentAllocationCommitError::ConflictingPartition)?;
            if &current_partition != expected_partition {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let current_count = COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.get(&count_key).and_then(|entry| match entry {
                    ComponentRegistryEntryRecord::ParentRoleCount(record) => Some(record),
                    ComponentRegistryEntryRecord::Partition(_)
                    | ComponentRegistryEntryRecord::Child(_)
                    | ComponentRegistryEntryRecord::ChildTraversal(_)
                    | ComponentRegistryEntryRecord::ChildAllocation(_)
                    | ComponentRegistryEntryRecord::SubtreeRemoval(_) => None,
                })
            });
            if current_count.as_ref() != expected_parent_role_count {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            if COMPONENT_REGISTRY_PRINCIPAL_INDEX
                .with_borrow(|map| {
                    map.get(&ComponentRegistryPrincipalKey::from(
                        record.parent_canister_id,
                    ))
                })
                .map(|indexed| indexed.component)
                != Some(component)
            {
                return Err(RootComponentAllocationCommitError::ParentPrincipalConflict);
            }

            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    operation_key,
                    ComponentRegistryEntryRecord::ChildAllocation(record),
                );
                map.insert(
                    count_key,
                    ComponentRegistryEntryRecord::ParentRoleCount(next_parent_role_count),
                );
                map.insert(
                    ComponentRegistryEntryKey::partition(component),
                    ComponentRegistryEntryRecord::Partition(next_partition),
                );
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn begin_subtree_removal(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_partition: &ComponentRegistryPartitionRecord,
        next_partition: ComponentRegistryPartitionRecord,
        expected_target: &ComponentRegistryChildRecord,
        record: RootComponentSubtreeRemovalRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentAllocationCommitError> {
        let component = record.component;
        let operation_key =
            ComponentRegistryEntryKey::subtree_removal(component, record.operation_id);
        let partition_key = ComponentRegistryEntryKey::partition(component);
        let target_key = ComponentRegistryEntryKey::child(component, record.target.canister_id);
        if expected_partition.binding.component != component
            || next_partition.binding.component != component
            || next_partition.binding != expected_partition.binding
            || next_partition.provisioning_origin != expected_partition.provisioning_origin
            || next_partition.release_set != expected_partition.release_set
            || next_partition.status != expected_partition.status
            || next_partition.revision != expected_partition.revision
            || next_partition.content_hash != expected_partition.content_hash
            || next_partition.descendant_content_hash != expected_partition.descendant_content_hash
            || next_partition.directory_synchronized_at_ns
                != expected_partition.directory_synchronized_at_ns
            || next_partition.reserved_descendants != expected_partition.reserved_descendants
            || next_partition.committed_descendants != expected_partition.committed_descendants
            || &record.target != expected_target
            || record.target.component != component
        {
            return Err(RootComponentAllocationCommitError::ConflictingChildEntry);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if let Some(existing) =
                COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| map.get(&operation_key))
            {
                return match existing {
                    ComponentRegistryEntryRecord::SubtreeRemoval(existing)
                        if existing.has_same_fence(&record) =>
                    {
                        Ok(RootComponentRegistryCommitOutcome::Existing)
                    }
                    ComponentRegistryEntryRecord::Partition(_)
                    | ComponentRegistryEntryRecord::Child(_)
                    | ComponentRegistryEntryRecord::ChildTraversal(_)
                    | ComponentRegistryEntryRecord::ChildAllocation(_)
                    | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                    | ComponentRegistryEntryRecord::ParentRoleCount(_) => {
                        Err(RootComponentAllocationCommitError::ConflictingOperation)
                    }
                };
            }
            if current_meta != expected_meta {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let (current_partition, current_target) =
                COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                    let partition = map.get(&partition_key).and_then(|entry| match entry {
                        ComponentRegistryEntryRecord::Partition(record) => Some(record),
                        ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    });
                    let target = map.get(&target_key).and_then(|entry| match entry {
                        ComponentRegistryEntryRecord::Child(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    });
                    (partition, target)
                });
            if current_partition.as_ref() != Some(expected_partition)
                || current_target.as_ref() != Some(expected_target)
            {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }

            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    operation_key,
                    ComponentRegistryEntryRecord::SubtreeRemoval(record),
                );
                map.insert(
                    partition_key,
                    ComponentRegistryEntryRecord::Partition(next_partition),
                );
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn replace_subtree_removal(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_partition: &ComponentRegistryPartitionRecord,
        next_partition: ComponentRegistryPartitionRecord,
        expected_record: &RootComponentSubtreeRemovalRecord,
        next_record: RootComponentSubtreeRemovalRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let component = expected_record.component;
        let operation_key =
            ComponentRegistryEntryKey::subtree_removal(component, expected_record.operation_id);
        let partition_key = ComponentRegistryEntryKey::partition(component);
        if !next_record.has_same_fence(expected_record)
            || next_record.traversal_steps < expected_record.traversal_steps
            || next_partition.binding != expected_partition.binding
            || next_partition.provisioning_origin != expected_partition.provisioning_origin
            || next_partition.release_set != expected_partition.release_set
            || next_partition.status != expected_partition.status
            || next_partition.revision != expected_partition.revision
            || next_partition.content_hash != expected_partition.content_hash
            || next_partition.descendant_content_hash != expected_partition.descendant_content_hash
            || next_partition.directory_synchronized_at_ns
                != expected_partition.directory_synchronized_at_ns
            || next_partition.reserved_descendants != expected_partition.reserved_descendants
            || next_partition.committed_descendants != expected_partition.committed_descendants
        {
            return Err(RootComponentAllocationCommitError::ConflictingChildEntry);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if current_meta != expected_meta {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let (current_partition, current_record) =
                COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                    let partition = map.get(&partition_key).and_then(|entry| match entry {
                        ComponentRegistryEntryRecord::Partition(record) => Some(record),
                        ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    });
                    let record = map.get(&operation_key).and_then(|entry| match entry {
                        ComponentRegistryEntryRecord::SubtreeRemoval(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    });
                    (partition, record)
                });
            if current_partition.as_ref() != Some(expected_partition)
                || current_record.as_ref() != Some(expected_record)
            {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }

            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    operation_key,
                    ComponentRegistryEntryRecord::SubtreeRemoval(next_record),
                );
                map.insert(
                    partition_key,
                    ComponentRegistryEntryRecord::Partition(next_partition),
                );
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(())
        })
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one atomic stable mutation validates and commits cursor, history and byte authority"
    )]
    pub(crate) fn finalize_subtree_removal_leaf(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_partition: &ComponentRegistryPartitionRecord,
        next_partition: ComponentRegistryPartitionRecord,
        expected_record: &RootComponentSubtreeRemovalRecord,
        next_record: RootComponentSubtreeRemovalRecord,
        completed_leaf: RootComponentSubtreeRemovalCompletedLeafRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let component = expected_record.component;
        let operation_key =
            ComponentRegistryEntryKey::subtree_removal(component, expected_record.operation_id);
        let partition_key = ComponentRegistryEntryKey::partition(component);
        let history_key = RootComponentSubtreeRemovalHistoryKey::new(
            component,
            expected_record.operation_id,
            expected_record.traversal_steps,
        );
        let RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(expected_receipt) =
            &expected_record.progress
        else {
            return Err(RootComponentAllocationCommitError::ConflictingChildEntry);
        };
        let leaf = &expected_receipt
            .membership_removed
            .deleted
            .deletion
            .stopped
            .stop
            .leaf;
        let expected_completed = expected_record.completed_leaves.checked_add(1);
        let progress_transition_is_valid = match &next_record.progress {
            RootComponentSubtreeRemovalProgressRecord::Traversing { cursor } => {
                leaf.canister_id != expected_record.target.canister_id
                    && cursor.component == component
                    && cursor.canister_id == leaf.parent_canister_id
            }
            RootComponentSubtreeRemovalProgressRecord::Completed(completed) => {
                leaf.canister_id == expected_record.target.canister_id
                    && completed.registry.component == component
                    && completed.registry.revision
                        == expected_receipt.covered_component_registry_revision
                    && completed.registry.content_hash
                        == expected_receipt.covered_component_registry_content_hash
                    && completed.directory_authority_hash == expected_receipt.covered_authority_hash
            }
            RootComponentSubtreeRemovalProgressRecord::Fenced
            | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
            | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
            | RootComponentSubtreeRemovalProgressRecord::Stopped(_)
            | RootComponentSubtreeRemovalProgressRecord::DeleteIntent(_)
            | RootComponentSubtreeRemovalProgressRecord::Deleted(_)
            | RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(_)
            | RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(_) => false,
        };
        let mut expected_next_meta = expected_meta.clone();
        expected_next_meta.encoded_bytes = next_meta.encoded_bytes;
        if !next_record.has_same_fence(expected_record)
            || next_record.maximum_completed_leaves != expected_record.maximum_completed_leaves
            || Some(next_record.completed_leaves) != expected_completed
            || next_record.completed_leaves > next_record.maximum_completed_leaves
            || next_record.traversal_steps != expected_record.traversal_steps
            || !progress_transition_is_valid
            || completed_leaf.operation_id != expected_record.operation_id
            || completed_leaf.component != component
            || completed_leaf.traversal_steps != expected_record.traversal_steps
            || completed_leaf.leaf_canister_id != leaf.canister_id
            || completed_leaf.leaf_parent_canister_id != leaf.parent_canister_id
            || completed_leaf.observed_module_hash
                != expected_receipt
                    .membership_removed
                    .deleted
                    .deletion
                    .stopped
                    .observed_module_hash
            || completed_leaf.registry.component != component
            || completed_leaf.registry.revision
                != expected_receipt.covered_component_registry_revision
            || completed_leaf.registry.content_hash
                != expected_receipt.covered_component_registry_content_hash
            || completed_leaf.directory_authority_hash != expected_receipt.covered_authority_hash
            || completed_leaf.receipt_hash == [0; 32]
            || ComponentPartitionStableAuthority::from(&next_partition)
                != ComponentPartitionStableAuthority::from(expected_partition)
            || next_meta != expected_next_meta
        {
            return Err(RootComponentAllocationCommitError::ConflictingChildEntry);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if current_meta != expected_meta
                || ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY
                    .with_borrow(|map| map.contains_key(&history_key))
            {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let (current_partition, current_record) =
                COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                    let partition = map.get(&partition_key).and_then(|entry| match entry {
                        ComponentRegistryEntryRecord::Partition(record) => Some(record),
                        ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    });
                    let record = map.get(&operation_key).and_then(|entry| match entry {
                        ComponentRegistryEntryRecord::SubtreeRemoval(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    });
                    (partition, record)
                });
            if current_partition.as_ref() != Some(expected_partition)
                || current_record.as_ref() != Some(expected_record)
            {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }

            ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY.with_borrow_mut(|map| {
                map.insert(history_key, completed_leaf);
            });
            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    operation_key,
                    ComponentRegistryEntryRecord::SubtreeRemoval(next_record),
                );
                map.insert(
                    partition_key,
                    ComponentRegistryEntryRecord::Partition(next_partition),
                );
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(())
        })
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "one stable mutation compare-and-commits every normalized removal authority"
    )]
    pub(crate) fn remove_subtree_leaf_membership(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_partition: &ComponentRegistryPartitionRecord,
        next_partition: ComponentRegistryPartitionRecord,
        expected_record: &RootComponentSubtreeRemovalRecord,
        next_record: RootComponentSubtreeRemovalRecord,
        expected_child: &ComponentRegistryChildRecord,
        expected_traversal: &ComponentRegistryChildTraversalRecord,
        expected_parent_role_count: &ComponentRegistryParentRoleCountRecord,
        next_parent_role_count: Option<ComponentRegistryParentRoleCountRecord>,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let component = expected_record.component;
        let operation_key =
            ComponentRegistryEntryKey::subtree_removal(component, expected_record.operation_id);
        let partition_key = ComponentRegistryEntryKey::partition(component);
        let child_key = ComponentRegistryEntryKey::child(component, expected_child.canister_id);
        let traversal_key = ComponentRegistryEntryKey::child_traversal(
            component,
            expected_traversal.parent_canister_id,
            &expected_traversal.role,
            expected_traversal.canister_id,
        );
        let count_key = ComponentRegistryEntryKey::parent_role_count(
            component,
            expected_child.parent_canister_id,
            &expected_child.role,
        );
        let principal_key = ComponentRegistryPrincipalKey::from(expected_child.canister_id);
        let child_authority = ComponentChildIndexAuthority::from_child(expected_child);
        let traversal_authority = ComponentChildIndexAuthority::from_traversal(expected_traversal);
        let parent_role_authority =
            ComponentParentRoleAuthority::from_count(expected_parent_role_count);
        let next_parent_role_is_valid = next_parent_role_count.as_ref().is_none_or(|next| {
            ComponentParentRoleAuthority::from_count(next) == parent_role_authority
                && next.instances.checked_add(1) == Some(expected_parent_role_count.instances)
        });
        let root_count_transition_is_valid = expected_meta.managed_descendants.checked_sub(1)
            == Some(next_meta.managed_descendants)
            && expected_meta
                .known_created_component_canisters
                .checked_sub(1)
                == Some(next_meta.known_created_component_canisters);
        let mut expected_next_meta = expected_meta.clone();
        expected_next_meta.managed_descendants = next_meta.managed_descendants;
        expected_next_meta.known_created_component_canisters =
            next_meta.known_created_component_canisters;
        expected_next_meta.encoded_bytes = next_meta.encoded_bytes;
        let partition_transition_is_valid = expected_partition.revision.checked_add(1)
            == Some(next_partition.revision)
            && expected_partition.committed_descendants.checked_sub(1)
                == Some(next_partition.committed_descendants)
            && next_partition.directory_synchronized_at_ns
                > expected_partition.directory_synchronized_at_ns
            && next_partition.content_hash != [0; 32]
            && next_partition.descendant_content_hash != [0; 32]
            && next_partition.reserved_descendants == expected_partition.reserved_descendants
            && next_partition.status == expected_partition.status;
        let progress_transition_is_valid = match (&expected_record.progress, &next_record.progress)
        {
            (
                RootComponentSubtreeRemovalProgressRecord::Deleted(deleted),
                RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt),
            ) => {
                &receipt.deleted == deleted
                    && receipt.removed_from_registry
                        == (ComponentRegistryHead {
                            component,
                            revision: expected_partition.revision,
                            content_hash: expected_partition.content_hash,
                        })
                    && receipt.previous_descendant_content_hash
                        == expected_partition.descendant_content_hash
                    && receipt.previous_committed_descendants
                        == expected_partition.committed_descendants
                    && receipt.registry
                        == (ComponentRegistryHead {
                            component,
                            revision: next_partition.revision,
                            content_hash: next_partition.content_hash,
                        })
                    && receipt.descendant_content_hash == next_partition.descendant_content_hash
                    && receipt.registry_encoded_bytes == next_partition.encoded_bytes
                    && receipt.reserved_descendants == next_partition.reserved_descendants
                    && receipt.committed_descendants == next_partition.committed_descendants
                    && receipt.directory_synchronized_at_ns
                        == next_partition.directory_synchronized_at_ns
                    && receipt.directory_authority_hash != [0; 32]
                    && receipt.parent_role_instances
                        == next_parent_role_count
                            .as_ref()
                            .map_or(0, |count| count.instances)
                    && receipt.root_managed_descendants == next_meta.managed_descendants
                    && receipt.root_known_created_component_canisters
                        == next_meta.known_created_component_canisters
            }
            _ => false,
        };
        if !next_record.has_same_fence(expected_record)
            || next_record.traversal_steps != expected_record.traversal_steps
            || ComponentPartitionStableAuthority::from(&next_partition)
                != ComponentPartitionStableAuthority::from(expected_partition)
            || child_authority != traversal_authority
            || parent_role_authority
                != (ComponentParentRoleAuthority {
                    component: &expected_child.component,
                    parent_canister_id: &expected_child.parent_canister_id,
                    child_role: &expected_child.role,
                })
            || expected_parent_role_count.instances == 0
            || (expected_parent_role_count.instances == 1) != next_parent_role_count.is_none()
            || !next_parent_role_is_valid
            || !root_count_transition_is_valid
            || next_meta != expected_next_meta
            || !partition_transition_is_valid
            || !progress_transition_is_valid
        {
            return Err(RootComponentAllocationCommitError::ConflictingChildEntry);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if current_meta != expected_meta {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let (
                current_partition,
                current_record,
                current_child,
                current_traversal,
                current_count,
            ) = COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                let partition = map.get(&partition_key).and_then(|entry| match entry {
                    ComponentRegistryEntryRecord::Partition(record) => Some(record),
                    _ => None,
                });
                let record = map.get(&operation_key).and_then(|entry| match entry {
                    ComponentRegistryEntryRecord::SubtreeRemoval(record) => Some(record),
                    _ => None,
                });
                let child = map.get(&child_key).and_then(|entry| match entry {
                    ComponentRegistryEntryRecord::Child(record) => Some(record),
                    _ => None,
                });
                let traversal = map.get(&traversal_key).and_then(|entry| match entry {
                    ComponentRegistryEntryRecord::ChildTraversal(record) => Some(record),
                    _ => None,
                });
                let count = map.get(&count_key).and_then(|entry| match entry {
                    ComponentRegistryEntryRecord::ParentRoleCount(record) => Some(record),
                    _ => None,
                });
                (partition, record, child, traversal, count)
            });
            if current_partition.as_ref() != Some(expected_partition)
                || current_record.as_ref() != Some(expected_record)
                || current_child.as_ref() != Some(expected_child)
                || current_traversal.as_ref() != Some(expected_traversal)
                || current_count.as_ref() != Some(expected_parent_role_count)
            {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            if COMPONENT_REGISTRY_PRINCIPAL_INDEX
                .with_borrow(|map| map.get(&principal_key))
                .map(|indexed| indexed.component)
                != Some(component)
            {
                return Err(RootComponentAllocationCommitError::ComponentPrincipalConflict);
            }

            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    operation_key,
                    ComponentRegistryEntryRecord::SubtreeRemoval(next_record),
                );
                map.insert(
                    partition_key,
                    ComponentRegistryEntryRecord::Partition(next_partition),
                );
                map.remove(&child_key);
                map.remove(&traversal_key);
                match next_parent_role_count {
                    Some(next) => {
                        map.insert(
                            count_key,
                            ComponentRegistryEntryRecord::ParentRoleCount(next),
                        );
                    }
                    None => {
                        map.remove(&count_key);
                    }
                }
            });
            COMPONENT_REGISTRY_PRINCIPAL_INDEX.with_borrow_mut(|map| {
                map.remove(&principal_key);
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(())
        })
    }

    pub(crate) fn replace_child_allocation(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_partition: &ComponentRegistryPartitionRecord,
        next_partition: ComponentRegistryPartitionRecord,
        expected_record: &RootComponentChildAllocationRecord,
        next_record: RootComponentChildAllocationRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let component = expected_record.component;
        let operation_key =
            ComponentRegistryEntryKey::child_allocation(component, expected_record.operation_id);
        if !next_record.has_same_reservation(expected_record)
            || ComponentPartitionSnapshotAuthority::from(&next_partition)
                != ComponentPartitionSnapshotAuthority::from(expected_partition)
        {
            return Err(RootComponentAllocationCommitError::ConflictingChildEntry);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if current_meta != expected_meta {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let current_partition = COMPONENT_REGISTRY_ENTRIES
                .with_borrow(|map| {
                    map.get(&ComponentRegistryEntryKey::partition(component))
                        .and_then(|entry| match entry {
                            ComponentRegistryEntryRecord::Partition(record) => Some(record),
                            ComponentRegistryEntryRecord::Child(_)
                            | ComponentRegistryEntryRecord::ChildTraversal(_)
                            | ComponentRegistryEntryRecord::ChildAllocation(_)
                            | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                            | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                        })
                })
                .ok_or(RootComponentAllocationCommitError::ConflictingPartition)?;
            let current_record = COMPONENT_REGISTRY_ENTRIES
                .with_borrow(|map| {
                    map.get(&operation_key).and_then(|entry| match entry {
                        ComponentRegistryEntryRecord::ChildAllocation(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    })
                })
                .ok_or(RootComponentAllocationCommitError::MissingOperation)?;
            if &current_partition != expected_partition || &current_record != expected_record {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }

            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    operation_key,
                    ComponentRegistryEntryRecord::ChildAllocation(next_record),
                );
                map.insert(
                    ComponentRegistryEntryKey::partition(component),
                    ComponentRegistryEntryRecord::Partition(next_partition),
                );
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(())
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one compare-and-commit advances the child row, partition and operation receipt"
    )]
    pub(crate) fn activate_child_membership(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_partition: &ComponentRegistryPartitionRecord,
        next_partition: ComponentRegistryPartitionRecord,
        expected_record: &RootComponentChildAllocationRecord,
        next_record: RootComponentChildAllocationRecord,
        expected_child: &ComponentRegistryChildRecord,
        next_child: ComponentRegistryChildRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let component = expected_record.component;
        let operation_key =
            ComponentRegistryEntryKey::child_allocation(component, expected_record.operation_id);
        let partition_key = ComponentRegistryEntryKey::partition(component);
        let child_key = ComponentRegistryEntryKey::child(component, expected_child.canister_id);
        if !next_record.has_same_reservation(expected_record)
            || next_partition.binding != expected_partition.binding
            || next_partition.provisioning_origin != expected_partition.provisioning_origin
            || next_partition.release_set != expected_partition.release_set
            || next_partition.status != expected_partition.status
            || next_partition.reserved_descendants != expected_partition.reserved_descendants
            || next_partition.committed_descendants != expected_partition.committed_descendants
            || expected_child.component != component
            || next_child.component != component
            || next_child.canister_id != expected_child.canister_id
            || next_child.parent_canister_id != expected_child.parent_canister_id
            || next_child.role != expected_child.role
            || next_child.kind != expected_child.kind
            || next_child.installed_artifact_hash != expected_child.installed_artifact_hash
            || expected_child.status != ComponentLifecycleStatus::Prepared
            || next_child.status != ComponentLifecycleStatus::Active
        {
            return Err(RootComponentAllocationCommitError::ConflictingChildEntry);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if current_meta != expected_meta {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let (current_partition, current_record, current_child) = COMPONENT_REGISTRY_ENTRIES
                .with_borrow(|map| {
                    let partition = map.get(&partition_key).and_then(|entry| match entry {
                        ComponentRegistryEntryRecord::Partition(record) => Some(record),
                        ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    });
                    let record = map.get(&operation_key).and_then(|entry| match entry {
                        ComponentRegistryEntryRecord::ChildAllocation(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    });
                    let child = map.get(&child_key).and_then(|entry| match entry {
                        ComponentRegistryEntryRecord::Child(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    });
                    (partition, record, child)
                });
            if current_partition.as_ref() != Some(expected_partition)
                || current_record.as_ref() != Some(expected_record)
                || current_child.as_ref() != Some(expected_child)
            {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }

            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    operation_key,
                    ComponentRegistryEntryRecord::ChildAllocation(next_record),
                );
                map.insert(
                    partition_key,
                    ComponentRegistryEntryRecord::Partition(next_partition),
                );
                map.insert(child_key, ComponentRegistryEntryRecord::Child(next_child));
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(())
        })
    }

    pub(crate) fn replace_allocation(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_record: &RootComponentAllocationRecord,
        next_record: RootComponentAllocationRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let key = RootComponentAllocationOperationKey::from(expected_record.operation_id);
        if next_record.operation_id != expected_record.operation_id {
            return Err(RootComponentAllocationCommitError::ConflictingOperation);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if current_meta != expected_meta {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let current_record = ROOT_COMPONENT_ALLOCATIONS
                .with_borrow(|map| map.get(&key))
                .ok_or(RootComponentAllocationCommitError::MissingOperation)?;
            if &current_record != expected_record {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }

            ROOT_COMPONENT_ALLOCATIONS.with_borrow_mut(|map| {
                map.insert(key, next_record);
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(())
        })
    }

    pub(crate) fn commit_component(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_record: &RootComponentAllocationRecord,
        next_record: RootComponentAllocationRecord,
        partition: ComponentRegistryPartitionRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let operation_key = RootComponentAllocationOperationKey::from(expected_record.operation_id);
        let component = partition.binding.component;
        let principal_key = ComponentRegistryPrincipalKey::from(partition.binding.canister_id);
        if next_record.operation_id != expected_record.operation_id {
            return Err(RootComponentAllocationCommitError::ConflictingOperation);
        }
        if next_record.component != expected_record.component
            || component != expected_record.component
        {
            return Err(RootComponentAllocationCommitError::ComponentIdentityConflict);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if current_meta != expected_meta {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let current_record = ROOT_COMPONENT_ALLOCATIONS
                .with_borrow(|map| map.get(&operation_key))
                .ok_or(RootComponentAllocationCommitError::MissingOperation)?;
            if &current_record != expected_record {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            if COMPONENT_REGISTRY_ENTRIES
                .with_borrow(|map| map.get(&ComponentRegistryEntryKey::partition(component)))
                .is_some()
            {
                return Err(RootComponentAllocationCommitError::ConflictingPartition);
            }
            if COMPONENT_REGISTRY_PRINCIPAL_INDEX
                .with_borrow(|map| map.get(&principal_key))
                .is_some()
            {
                return Err(RootComponentAllocationCommitError::ComponentPrincipalConflict);
            }

            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryEntryKey::partition(component),
                    ComponentRegistryEntryRecord::Partition(partition),
                );
            });
            COMPONENT_REGISTRY_PRINCIPAL_INDEX.with_borrow_mut(|map| {
                map.insert(
                    principal_key,
                    ComponentRegistryPrincipalIndexRecord { component },
                );
            });
            ROOT_COMPONENT_ALLOCATIONS.with_borrow_mut(|map| {
                map.insert(operation_key, next_record);
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(())
        })
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "one stable mutation compare-and-commits every normalized child authority"
    )]
    pub(crate) fn commit_child(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_partition: &ComponentRegistryPartitionRecord,
        next_partition: ComponentRegistryPartitionRecord,
        expected_record: &RootComponentChildAllocationRecord,
        next_record: RootComponentChildAllocationRecord,
        child: ComponentRegistryChildRecord,
        traversal: ComponentRegistryChildTraversalRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let component = expected_record.component;
        let operation_key =
            ComponentRegistryEntryKey::child_allocation(component, expected_record.operation_id);
        let partition_key = ComponentRegistryEntryKey::partition(component);
        let child_key = ComponentRegistryEntryKey::child(component, child.canister_id);
        let traversal_key = ComponentRegistryEntryKey::child_traversal(
            component,
            traversal.parent_canister_id,
            &traversal.role,
            traversal.canister_id,
        );
        let principal_key = ComponentRegistryPrincipalKey::from(child.canister_id);
        let expected_child_authority =
            ComponentChildIndexAuthority::from_allocation(expected_record, child.canister_id);
        let child_authority = ComponentChildIndexAuthority::from_child(&child);
        let traversal_authority = ComponentChildIndexAuthority::from_traversal(&traversal);
        if !next_record.has_same_reservation(expected_record)
            || ComponentPartitionStableAuthority::from(&next_partition)
                != ComponentPartitionStableAuthority::from(expected_partition)
            || child_authority != expected_child_authority
            || traversal_authority != expected_child_authority
        {
            return Err(RootComponentAllocationCommitError::ConflictingChildEntry);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if current_meta != expected_meta {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let current_partition = COMPONENT_REGISTRY_ENTRIES
                .with_borrow(|map| {
                    map.get(&partition_key).and_then(|entry| match entry {
                        ComponentRegistryEntryRecord::Partition(record) => Some(record),
                        ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    })
                })
                .ok_or(RootComponentAllocationCommitError::ConflictingPartition)?;
            let current_record = COMPONENT_REGISTRY_ENTRIES
                .with_borrow(|map| {
                    map.get(&operation_key).and_then(|entry| match entry {
                        ComponentRegistryEntryRecord::ChildAllocation(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    })
                })
                .ok_or(RootComponentAllocationCommitError::MissingOperation)?;
            if &current_partition != expected_partition || &current_record != expected_record {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            if COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.get(&child_key).is_some() || map.get(&traversal_key).is_some()
            }) {
                return Err(RootComponentAllocationCommitError::ConflictingChildEntry);
            }
            if COMPONENT_REGISTRY_PRINCIPAL_INDEX
                .with_borrow(|map| map.get(&principal_key))
                .is_some()
            {
                return Err(RootComponentAllocationCommitError::ComponentPrincipalConflict);
            }
            if COMPONENT_REGISTRY_PRINCIPAL_INDEX
                .with_borrow(|map| {
                    map.get(&ComponentRegistryPrincipalKey::from(
                        expected_record.parent_canister_id,
                    ))
                })
                .map(|indexed| indexed.component)
                != Some(component)
            {
                return Err(RootComponentAllocationCommitError::ParentPrincipalConflict);
            }

            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    operation_key,
                    ComponentRegistryEntryRecord::ChildAllocation(next_record),
                );
                map.insert(
                    partition_key,
                    ComponentRegistryEntryRecord::Partition(next_partition),
                );
                map.insert(child_key, ComponentRegistryEntryRecord::Child(child));
                map.insert(
                    traversal_key,
                    ComponentRegistryEntryRecord::ChildTraversal(traversal),
                );
            });
            COMPONENT_REGISTRY_PRINCIPAL_INDEX.with_borrow_mut(|map| {
                map.insert(
                    principal_key,
                    ComponentRegistryPrincipalIndexRecord { component },
                );
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(())
        })
    }

    pub(crate) fn replace_component_partition(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_record: &RootComponentAllocationRecord,
        next_record: RootComponentAllocationRecord,
        expected_partition: &ComponentRegistryPartitionRecord,
        next_partition: ComponentRegistryPartitionRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let operation_key = RootComponentAllocationOperationKey::from(expected_record.operation_id);
        let component = expected_partition.binding.component;
        if RootComponentAllocationIdentity::from(&next_record)
            != RootComponentAllocationIdentity::from(expected_record)
            || next_partition.binding.component != component
            || ComponentPartitionStableAuthority::from(&next_partition)
                != ComponentPartitionStableAuthority::from(expected_partition)
        {
            return Err(RootComponentAllocationCommitError::ComponentIdentityConflict);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if current_meta != expected_meta {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let current_record = ROOT_COMPONENT_ALLOCATIONS
                .with_borrow(|map| map.get(&operation_key))
                .ok_or(RootComponentAllocationCommitError::MissingOperation)?;
            let current_partition = COMPONENT_REGISTRY_ENTRIES
                .with_borrow(|map| {
                    map.get(&ComponentRegistryEntryKey::partition(component))
                        .and_then(|entry| match entry {
                            ComponentRegistryEntryRecord::Partition(record) => Some(record),
                            ComponentRegistryEntryRecord::Child(_)
                            | ComponentRegistryEntryRecord::ChildTraversal(_)
                            | ComponentRegistryEntryRecord::ChildAllocation(_)
                            | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                            | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                        })
                })
                .ok_or(RootComponentAllocationCommitError::ConflictingPartition)?;
            if &current_record != expected_record || &current_partition != expected_partition {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }

            ROOT_COMPONENT_ALLOCATIONS.with_borrow_mut(|map| {
                map.insert(operation_key, next_record);
            });
            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryEntryKey::partition(component),
                    ComponentRegistryEntryRecord::Partition(next_partition),
                );
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(())
        })
    }

    pub(crate) fn begin_component_draining(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_partition: &ComponentRegistryPartitionRecord,
        next_partition: ComponentRegistryPartitionRecord,
        record: RootComponentDrainingRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let component = expected_partition.binding.component;
        let key = RootComponentDrainingKey::from(component);
        let expected_registry = ComponentRegistryHead {
            component,
            revision: expected_partition.revision,
            content_hash: expected_partition.content_hash,
        };
        let next_registry = ComponentRegistryHead {
            component,
            revision: next_partition.revision,
            content_hash: next_partition.content_hash,
        };
        if ComponentPartitionStableAuthority::from(&next_partition)
            != ComponentPartitionStableAuthority::from(expected_partition)
            || expected_partition.status != ComponentLifecycleStatus::Active
            || next_partition.status != ComponentLifecycleStatus::Draining
            || next_partition.descendant_content_hash != expected_partition.descendant_content_hash
            || next_partition.reserved_descendants != expected_partition.reserved_descendants
            || next_partition.committed_descendants != expected_partition.committed_descendants
            || expected_partition.revision.checked_add(1) != Some(next_partition.revision)
            || next_partition.directory_synchronized_at_ns != record.started_at_ns
            || record.started_at_ns <= expected_partition.directory_synchronized_at_ns
            || record.operation_id == [0; 32]
            || record.component != component
            || record.previous_registry != expected_registry
            || record.registry != next_registry
            || record.descendant_count != expected_partition.committed_descendants
            || record.descendant_content_hash != expected_partition.descendant_content_hash
            || record.directory_authority_hash == [0; 32]
            || record.quiescence.is_some()
        {
            return Err(RootComponentAllocationCommitError::ConflictingPartition);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if current_meta != expected_meta
                || ROOT_COMPONENT_DRAINING.with_borrow(|map| map.contains_key(&key))
            {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let current_partition = COMPONENT_REGISTRY_ENTRIES
                .with_borrow(|map| {
                    map.get(&ComponentRegistryEntryKey::partition(component))
                        .and_then(|entry| match entry {
                            ComponentRegistryEntryRecord::Partition(partition) => Some(partition),
                            ComponentRegistryEntryRecord::Child(_)
                            | ComponentRegistryEntryRecord::ChildTraversal(_)
                            | ComponentRegistryEntryRecord::ChildAllocation(_)
                            | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                            | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                        })
                })
                .ok_or(RootComponentAllocationCommitError::ConflictingPartition)?;
            if &current_partition != expected_partition {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }

            ROOT_COMPONENT_DRAINING.with_borrow_mut(|map| {
                map.insert(key, record);
            });
            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryEntryKey::partition(component),
                    ComponentRegistryEntryRecord::Partition(next_partition),
                );
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(())
        })
    }

    pub(crate) fn prepare_component_quiescence(
        expected_meta: &RootComponentRegistryMetaRecord,
        next_meta: RootComponentRegistryMetaRecord,
        expected_partition: &ComponentRegistryPartitionRecord,
        next_partition: ComponentRegistryPartitionRecord,
        expected_record: &RootComponentDrainingRecord,
        next_record: RootComponentDrainingRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let component = expected_partition.binding.component;
        let key = RootComponentDrainingKey::from(component);
        let next_intent = match &next_record.quiescence {
            Some(RootComponentQuiescenceProgressRecord::StopIntent(intent)) => intent,
            None | Some(RootComponentQuiescenceProgressRecord::Quiescent(_)) => {
                return Err(RootComponentAllocationCommitError::ConflictingPartition);
            }
        };
        if expected_record.quiescence.is_some()
            || ComponentPartitionSnapshotAuthority::from(&next_partition)
                != ComponentPartitionSnapshotAuthority::from(expected_partition)
            || expected_partition.status != ComponentLifecycleStatus::Draining
            || next_partition.status != ComponentLifecycleStatus::Draining
            || !component_draining_identity_matches(expected_record, &next_record)
            || next_intent.registry != expected_record.registry
            || next_intent.descendant_count != expected_record.descendant_count
            || next_intent.descendant_content_hash != expected_record.descendant_content_hash
            || next_intent.canister_id != expected_partition.binding.canister_id
            || next_intent.controller != expected_partition.binding.fleet_subnet_root
            || next_intent.charged_entry_bytes < Self::component_draining_entry_bytes(&next_record)
        {
            return Err(RootComponentAllocationCommitError::ConflictingPartition);
        }

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            let current_record = ROOT_COMPONENT_DRAINING
                .with_borrow(|map| map.get(&key))
                .ok_or(RootComponentAllocationCommitError::ConflictingState)?;
            let current_partition = COMPONENT_REGISTRY_ENTRIES
                .with_borrow(|map| {
                    map.get(&ComponentRegistryEntryKey::partition(component))
                        .and_then(|entry| match entry {
                            ComponentRegistryEntryRecord::Partition(partition) => Some(partition),
                            ComponentRegistryEntryRecord::Child(_)
                            | ComponentRegistryEntryRecord::ChildTraversal(_)
                            | ComponentRegistryEntryRecord::ChildAllocation(_)
                            | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                            | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                        })
                })
                .ok_or(RootComponentAllocationCommitError::ConflictingPartition)?;
            if current_meta != expected_meta
                || current_record != *expected_record
                || current_partition != *expected_partition
            {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }

            ROOT_COMPONENT_DRAINING.with_borrow_mut(|map| {
                map.insert(key, next_record);
            });
            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryEntryKey::partition(component),
                    ComponentRegistryEntryRecord::Partition(next_partition),
                );
            });
            state.current = Some(next_meta);
            cell.set(state);
            Ok(())
        })
    }

    pub(crate) fn mark_component_quiescent(
        expected_record: &RootComponentDrainingRecord,
        next_record: RootComponentDrainingRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let component = expected_record.component;
        let key = RootComponentDrainingKey::from(component);
        let expected_intent = match &expected_record.quiescence {
            Some(RootComponentQuiescenceProgressRecord::StopIntent(intent)) => intent,
            None | Some(RootComponentQuiescenceProgressRecord::Quiescent(_)) => {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
        };
        let next_receipt = match &next_record.quiescence {
            Some(RootComponentQuiescenceProgressRecord::Quiescent(receipt)) => receipt,
            None | Some(RootComponentQuiescenceProgressRecord::StopIntent(_)) => {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
        };
        if !component_draining_identity_matches(expected_record, &next_record)
            || next_receipt.stop != *expected_intent
            || next_receipt.observed_module_hash != expected_intent.expected_module_hash
            || next_receipt.quiesced_at_ns < expected_intent.prepared_at_ns
            || Self::component_draining_entry_bytes(&next_record)
                > expected_intent.charged_entry_bytes
        {
            return Err(RootComponentAllocationCommitError::ConflictingState);
        }

        ROOT_COMPONENT_DRAINING.with_borrow_mut(|map| {
            let current = map
                .get(&key)
                .ok_or(RootComponentAllocationCommitError::ConflictingState)?;
            if current != *expected_record {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            map.insert(key, next_record);
            Ok(())
        })
    }

    #[must_use]
    pub(crate) fn allocation_entry_bytes(record: &RootComponentAllocationRecord) -> u64 {
        let key = RootComponentAllocationOperationKey::from(record.operation_id);
        (key.to_bytes().len() + record.to_bytes().len()) as u64
    }

    #[must_use]
    pub(crate) const fn allocation_record_max_bytes() -> u64 {
        ROOT_COMPONENT_ALLOCATION_RECORD_MAX_BYTES as u64
    }

    #[must_use]
    pub(crate) fn partition_entry_bytes(record: &ComponentRegistryPartitionRecord) -> u64 {
        let key = ComponentRegistryEntryKey::partition(record.binding.component);
        let value = ComponentRegistryEntryRecord::Partition(record.clone());
        (key.to_bytes().len() + value.to_bytes().len()) as u64
    }

    #[must_use]
    pub(crate) fn child_allocation_entry_bytes(record: &RootComponentChildAllocationRecord) -> u64 {
        let key =
            ComponentRegistryEntryKey::child_allocation(record.component, record.operation_id);
        let value = ComponentRegistryEntryRecord::ChildAllocation(record.clone());
        (key.to_bytes().len() + value.to_bytes().len()) as u64
    }

    #[must_use]
    pub(crate) fn subtree_removal_entry_bytes(record: &RootComponentSubtreeRemovalRecord) -> u64 {
        let key = ComponentRegistryEntryKey::subtree_removal(record.component, record.operation_id);
        let value = ComponentRegistryEntryRecord::SubtreeRemoval(record.clone());
        (key.to_bytes().len() + value.to_bytes().len()) as u64
    }

    #[must_use]
    pub(crate) fn subtree_removal_completed_leaf_entry_bytes(
        record: &RootComponentSubtreeRemovalCompletedLeafRecord,
    ) -> u64 {
        let key = RootComponentSubtreeRemovalHistoryKey::new(
            record.component,
            record.operation_id,
            record.traversal_steps,
        );
        (key.to_bytes().len() + record.to_bytes().len()) as u64
    }

    #[must_use]
    pub(crate) fn component_draining_entry_bytes(record: &RootComponentDrainingRecord) -> u64 {
        let key = RootComponentDrainingKey::from(record.component);
        (key.to_bytes().len() + record.to_bytes().len()) as u64
    }

    #[must_use]
    pub(crate) fn child_entry_bytes(record: &ComponentRegistryChildRecord) -> u64 {
        let key = ComponentRegistryEntryKey::child(record.component, record.canister_id);
        let value = ComponentRegistryEntryRecord::Child(record.clone());
        (key.to_bytes().len() + value.to_bytes().len()) as u64
    }

    #[must_use]
    pub(crate) fn child_traversal_entry_bytes(
        record: &ComponentRegistryChildTraversalRecord,
    ) -> u64 {
        let key = ComponentRegistryEntryKey::child_traversal(
            record.component,
            record.parent_canister_id,
            &record.role,
            record.canister_id,
        );
        let value = ComponentRegistryEntryRecord::ChildTraversal(record.clone());
        (key.to_bytes().len() + value.to_bytes().len()) as u64
    }

    #[must_use]
    pub(crate) fn parent_role_count_entry_bytes(
        record: &ComponentRegistryParentRoleCountRecord,
    ) -> u64 {
        let key = ComponentRegistryEntryKey::parent_role_count(
            record.component,
            record.parent_canister_id,
            &record.child_role,
        );
        let value = ComponentRegistryEntryRecord::ParentRoleCount(record.clone());
        (key.to_bytes().len() + value.to_bytes().len()) as u64
    }

    #[must_use]
    pub(crate) fn principal_index_entry_bytes(
        canister: Principal,
        component: ComponentInstanceId,
    ) -> u64 {
        let key = ComponentRegistryPrincipalKey::from(canister);
        let value = ComponentRegistryPrincipalIndexRecord { component };
        (key.to_bytes().len() + value.to_bytes().len()) as u64
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "test snapshot import repopulates every normalized stable domain explicitly"
    )]
    pub(crate) fn import(data: RootComponentRegistryData) {
        ROOT_COMPONENT_ALLOCATIONS.with_borrow_mut(StableBtreeMap::clear_new);
        ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY.with_borrow_mut(StableBtreeMap::clear_new);
        ROOT_COMPONENT_DRAINING.with_borrow_mut(StableBtreeMap::clear_new);
        for record in data.allocations {
            ROOT_COMPONENT_ALLOCATIONS.with_borrow_mut(|map| {
                map.insert(
                    RootComponentAllocationOperationKey::from(record.operation_id),
                    record,
                );
            });
        }
        COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(StableBtreeMap::clear_new);
        COMPONENT_REGISTRY_PRINCIPAL_INDEX.with_borrow_mut(StableBtreeMap::clear_new);
        for record in data.partitions {
            let component = record.binding.component;
            let canister = record.binding.canister_id;
            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryEntryKey::partition(component),
                    ComponentRegistryEntryRecord::Partition(record),
                );
            });
            COMPONENT_REGISTRY_PRINCIPAL_INDEX.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryPrincipalKey::from(canister),
                    ComponentRegistryPrincipalIndexRecord { component },
                );
            });
        }
        for record in data.children {
            let component = record.component;
            let canister = record.canister_id;
            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryEntryKey::child(component, canister),
                    ComponentRegistryEntryRecord::Child(record),
                );
            });
            COMPONENT_REGISTRY_PRINCIPAL_INDEX.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryPrincipalKey::from(canister),
                    ComponentRegistryPrincipalIndexRecord { component },
                );
            });
        }
        for record in data.child_traversals {
            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryEntryKey::child_traversal(
                        record.component,
                        record.parent_canister_id,
                        &record.role,
                        record.canister_id,
                    ),
                    ComponentRegistryEntryRecord::ChildTraversal(record),
                );
            });
        }
        for record in data.child_allocations {
            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryEntryKey::child_allocation(
                        record.component,
                        record.operation_id,
                    ),
                    ComponentRegistryEntryRecord::ChildAllocation(record),
                );
            });
        }
        for record in data.subtree_removals {
            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryEntryKey::subtree_removal(
                        record.component,
                        record.operation_id,
                    ),
                    ComponentRegistryEntryRecord::SubtreeRemoval(record),
                );
            });
        }
        for record in data.subtree_removal_history {
            ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY.with_borrow_mut(|map| {
                map.insert(
                    RootComponentSubtreeRemovalHistoryKey::new(
                        record.component,
                        record.operation_id,
                        record.traversal_steps,
                    ),
                    record,
                );
            });
        }
        for record in data.component_drainings {
            ROOT_COMPONENT_DRAINING.with_borrow_mut(|map| {
                map.insert(RootComponentDrainingKey::from(record.component), record);
            });
        }
        for record in data.parent_role_counts {
            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryEntryKey::parent_role_count(
                        record.component,
                        record.parent_canister_id,
                        &record.child_role,
                    ),
                    ComponentRegistryEntryRecord::ParentRoleCount(record),
                );
            });
        }
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            cell.set(RootComponentRegistryStateRecord {
                current: data.current,
            });
        });
    }
}

#[cfg(all(test, feature = "root-control-plane"))]
mod tests {
    use super::*;

    #[test]
    fn child_traversal_pages_follow_canonical_parent_role_canister_order() {
        let component = ComponentInstanceId::from_generated_bytes([1; 32]);
        let other_component = ComponentInstanceId::from_generated_bytes([2; 32]);
        let first_parent = Principal::from_slice(&[3; 29]);
        let second_parent = Principal::from_slice(&[4; 29]);
        let first = ComponentRegistryChildTraversalRecord {
            component,
            parent_canister_id: first_parent,
            role: CanisterRole::new("instance"),
            canister_id: Principal::from_slice(&[5; 29]),
        };
        let second = ComponentRegistryChildTraversalRecord {
            component,
            parent_canister_id: first_parent,
            role: CanisterRole::new("ledger"),
            canister_id: Principal::from_slice(&[6; 29]),
        };
        let third = ComponentRegistryChildTraversalRecord {
            component,
            parent_canister_id: second_parent,
            role: CanisterRole::new("machine"),
            canister_id: Principal::from_slice(&[7; 29]),
        };
        RootComponentRegistryStore::import(RootComponentRegistryData {
            child_traversals: vec![
                third.clone(),
                ComponentRegistryChildTraversalRecord {
                    component: other_component,
                    parent_canister_id: first_parent,
                    role: CanisterRole::new("instance"),
                    canister_id: Principal::from_slice(&[8; 29]),
                },
                second.clone(),
                first.clone(),
            ],
            ..RootComponentRegistryData::default()
        });

        let first_page =
            RootComponentRegistryStore::child_traversals_page(component, None, None, None, 2);
        assert_eq!(first_page, vec![first, second.clone()]);
        let second_page = RootComponentRegistryStore::child_traversals_page(
            component,
            None,
            None,
            Some((
                &second.parent_canister_id,
                &second.role,
                &second.canister_id,
            )),
            2,
        );
        assert_eq!(second_page, vec![third.clone()]);
        assert_eq!(
            RootComponentRegistryStore::child_traversals_page(
                component,
                Some(first_parent),
                Some(&second.role),
                None,
                2,
            ),
            vec![second]
        );
        assert_eq!(
            RootComponentRegistryStore::child_traversals_page(
                component,
                Some(second_parent),
                None,
                None,
                2,
            ),
            vec![third]
        );

        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }
}
