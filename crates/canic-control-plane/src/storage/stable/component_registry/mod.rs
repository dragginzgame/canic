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
        ROOT_COMPONENT_REGISTRY_META_ID, ROOT_COMPONENT_REGISTRY_PARTITIONS_ID,
    },
};
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::model::replay::ReplayCostGuardSettlement,
    dto::{
        component_registry::{
            ComponentLifecycleStatus, ComponentProvisioningOrigin, ComponentRegistryHead,
        },
        fleet_registry::FleetRegistryVersion,
        root_store::RootStoreBootstrapRequest,
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentInstanceId, ComponentSpecId,
        FleetSubnetRootBinding, FleetSubnetRootReleaseSet,
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
const COMPONENT_REGISTRY_PARTITION_RECORD_MAX_BYTES: u32 = 4_096;

#[cfg(feature = "root-control-plane")]
struct RootComponentRegistryState;
#[cfg(feature = "root-control-plane")]
struct RootComponentAllocations;
#[cfg(feature = "root-control-plane")]
struct ComponentRegistryPartitions;
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
    static COMPONENT_REGISTRY_PARTITIONS: RefCell<
        StableBtreeMap<
            ComponentRegistryPartitionKey,
            ComponentRegistryPartitionRecord,
            VirtualMemory<DefaultMemoryImpl>,
        >,
    > = RefCell::new(StableBtreeMap::init(
        canic_core::ic_memory_key!(
            authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            key = "canic.control_plane.component_registry_partitions.v1",
            ty = ComponentRegistryPartitions,
            id = ROOT_COMPONENT_REGISTRY_PARTITIONS_ID
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
    pub encoded_bytes: u64,
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
    pub directory_synchronized_at_ns: u64,
    pub directory_authority_hash: [u8; 32],
    pub directory_prepared: bool,
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
    pub directory_synchronized_at_ns: u64,
    pub encoded_bytes: u64,
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(
    ComponentRegistryPartitionRecord,
    COMPONENT_REGISTRY_PARTITION_RECORD_MAX_BYTES,
    false
);

impl ComponentRegistryPartitionRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "ComponentRegistryPartitionRecord";
}

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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct ComponentRegistryPartitionKey([u8; 32]);

impl From<ComponentInstanceId> for ComponentRegistryPartitionKey {
    fn from(value: ComponentInstanceId) -> Self {
        Self(*value.as_bytes())
    }
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(ComponentRegistryPartitionKey, 128, false);

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
    ConflictingPartition,
    ConflictingOperation,
    ConflictingState,
    MissingOperation,
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
            partitions: COMPONENT_REGISTRY_PARTITIONS
                .with_borrow(|map| map.iter().map(|entry| entry.value()).collect()),
        })
    }

    #[must_use]
    pub(crate) fn current() -> Option<RootComponentRegistryMetaRecord> {
        ROOT_COMPONENT_REGISTRY.with_borrow(|cell| cell.get().current.clone())
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
        COMPONENT_REGISTRY_PARTITIONS
            .with_borrow(|map| map.get(&ComponentRegistryPartitionKey::from(component)))
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
            if COMPONENT_REGISTRY_PARTITIONS
                .with_borrow(|map| map.get(&ComponentRegistryPartitionKey::from(component)))
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

            COMPONENT_REGISTRY_PARTITIONS.with_borrow_mut(|map| {
                map.insert(ComponentRegistryPartitionKey::from(component), partition);
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
        let key = ComponentRegistryPartitionKey::from(record.binding.component);
        (key.to_bytes().len() + record.to_bytes().len()) as u64
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
        COMPONENT_REGISTRY_PARTITIONS.with_borrow_mut(StableBtreeMap::clear_new);
        COMPONENT_REGISTRY_PRINCIPAL_INDEX.with_borrow_mut(StableBtreeMap::clear_new);
        for record in data.partitions {
            let component = record.binding.component;
            let canister = record.binding.canister_id;
            COMPONENT_REGISTRY_PARTITIONS.with_borrow_mut(|map| {
                map.insert(ComponentRegistryPartitionKey::from(component), record);
            });
            COMPONENT_REGISTRY_PRINCIPAL_INDEX.with_borrow_mut(|map| {
                map.insert(
                    ComponentRegistryPrincipalKey::from(canister),
                    ComponentRegistryPrincipalIndexRecord { component },
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
