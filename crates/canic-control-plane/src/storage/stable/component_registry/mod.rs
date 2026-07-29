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
        ROOT_COMPONENT_ALLOCATIONS_ID, ROOT_COMPONENT_PRINCIPAL_INDEX_ID,
        ROOT_COMPONENT_REGISTRY_ENTRIES_ID, ROOT_COMPONENT_REGISTRY_META_ID,
    },
};
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::config::schema::ComponentChildKind,
    control_plane_support::model::replay::ReplayCostGuardSettlement,
    dto::{
        component_registry::{
            ComponentLifecycleStatus, ComponentProvisioningOrigin, ComponentRegistryHead,
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
use std::cell::RefCell;

#[cfg(feature = "root-control-plane")]
const ROOT_COMPONENT_REGISTRY_STATE_MAX_BYTES: u32 = 65_536;
#[cfg(feature = "root-control-plane")]
const ROOT_COMPONENT_ALLOCATION_RECORD_MAX_BYTES: u32 = 4_096;
#[cfg(feature = "root-control-plane")]
const COMPONENT_REGISTRY_ENTRY_KEY_MAX_BYTES: u32 = 512;
#[cfg(feature = "root-control-plane")]
const COMPONENT_REGISTRY_ENTRY_RECORD_MAX_BYTES: u32 = 4_096;

#[cfg(feature = "root-control-plane")]
struct RootComponentRegistryState;
#[cfg(feature = "root-control-plane")]
struct RootComponentAllocations;
#[cfg(feature = "root-control-plane")]
struct ComponentRegistryEntries;
#[cfg(feature = "root-control-plane")]
struct ComponentRegistryPrincipalIndex;

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

impl RootComponentChildAllocationRecord {
    pub(crate) fn has_same_reservation(&self, other: &Self) -> bool {
        self.operation_id == other.operation_id
            && self.component == other.component
            && self.parent_canister_id == other.parent_canister_id
            && self.parent_role == other.parent_role
            && self.child_role == other.child_role
            && self.child_kind == other.child_kind
            && self.maximum_instances_per_parent == other.maximum_instances_per_parent
            && self.maximum_descendants == other.maximum_descendants
            && self.maximum_registry_bytes == other.maximum_registry_bytes
            && self.reserved_against_registry == other.reserved_against_registry
            && self.release_set == other.release_set
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
    pub membership: Option<RootComponentMembershipRecord>,
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

impl From<[u8; 32]> for RootComponentAllocationOperationKey {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(RootComponentAllocationOperationKey, 128, false);

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
                        | ComponentRegistryEntryRecord::ParentRoleCount(_) => None,
                    })
                    .collect()
            }),
            parent_role_counts: COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.iter()
                    .filter_map(|entry| match entry.value() {
                        ComponentRegistryEntryRecord::ParentRoleCount(record) => Some(record),
                        ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::Child(_)
                        | ComponentRegistryEntryRecord::ChildTraversal(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_) => None,
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
                    | ComponentRegistryEntryRecord::ParentRoleCount(_),
                )
                | None => None,
            }
        })
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
                    | ComponentRegistryEntryRecord::ParentRoleCount(_),
                )
                | None => None,
            }
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
                    | ComponentRegistryEntryRecord::ParentRoleCount(_),
                )
                | None => None,
            }
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
                    | ComponentRegistryEntryRecord::ChildAllocation(_),
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
        if expected_partition.binding.component != component
            || next_partition.binding.component != component
            || next_parent_role_count.component != component
            || next_parent_role_count.parent_canister_id != record.parent_canister_id
            || next_parent_role_count.child_role != record.child_role
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
                    ComponentRegistryEntryRecord::ChildAllocation(existing)
                        if existing.has_same_reservation(&record) =>
                    {
                        Ok(RootComponentRegistryCommitOutcome::Existing)
                    }
                    ComponentRegistryEntryRecord::Partition(_)
                    | ComponentRegistryEntryRecord::Child(_)
                    | ComponentRegistryEntryRecord::ChildTraversal(_)
                    | ComponentRegistryEntryRecord::ChildAllocation(_)
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
                    | ComponentRegistryEntryRecord::ChildAllocation(_) => None,
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
            || next_partition.binding != expected_partition.binding
            || next_partition.provisioning_origin != expected_partition.provisioning_origin
            || next_partition.release_set != expected_partition.release_set
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
            let current_partition = COMPONENT_REGISTRY_ENTRIES
                .with_borrow(|map| {
                    map.get(&ComponentRegistryEntryKey::partition(component))
                        .and_then(|entry| match entry {
                            ComponentRegistryEntryRecord::Partition(record) => Some(record),
                            ComponentRegistryEntryRecord::Child(_)
                            | ComponentRegistryEntryRecord::ChildTraversal(_)
                            | ComponentRegistryEntryRecord::ChildAllocation(_)
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
        if !next_record.has_same_reservation(expected_record)
            || next_partition.binding != expected_partition.binding
            || next_partition.provisioning_origin != expected_partition.provisioning_origin
            || next_partition.release_set != expected_partition.release_set
            || child.component != component
            || traversal.component != component
            || child.parent_canister_id != expected_record.parent_canister_id
            || traversal.parent_canister_id != expected_record.parent_canister_id
            || child.role != expected_record.child_role
            || traversal.role != expected_record.child_role
            || traversal.canister_id != child.canister_id
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
        if next_record.operation_id != expected_record.operation_id
            || next_record.component != expected_record.component
            || next_partition.binding.component != component
            || next_partition.binding != expected_partition.binding
            || next_partition.provisioning_origin != expected_partition.provisioning_origin
            || next_partition.release_set != expected_partition.release_set
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
    pub(crate) fn import(data: RootComponentRegistryData) {
        ROOT_COMPONENT_ALLOCATIONS.with_borrow_mut(StableBtreeMap::clear_new);
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
