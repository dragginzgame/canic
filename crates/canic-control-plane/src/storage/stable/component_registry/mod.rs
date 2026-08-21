//! Module: storage::stable::component_registry
//!
//! Responsibility: own one root's Component Registry meta, operations, partitions and indexes.
//! Does not own: Store, Fleet Registry, topology, admission, or lifecycle validation.
//! Boundary: ops commit only exact authority and records already validated by workflow.

#[cfg(feature = "root-control-plane")]
use canic_core::dto::fleet_registry::{FleetSubnetRootEntry, FleetSubnetRootStatus};
#[cfg(feature = "root-control-plane")]
use canic_core::impl_storable_bounded;
#[cfg(feature = "root-control-plane")]
use canic_core::{
    cdk::structures::{
        DefaultMemoryImpl, btreemap::BTreeMap as StableBtreeMap, cell::Cell, memory::VirtualMemory,
        storable::Storable,
    },
    dto::fleet_subnet_root::FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES,
    eager_static,
    role_contract::allocation::memory::control_plane::{
        ROOT_COMPONENT_ALLOCATIONS_ID, ROOT_COMPONENT_DRAINING_ID,
        ROOT_COMPONENT_PRINCIPAL_INDEX_ID, ROOT_COMPONENT_REGISTRY_ENTRIES_ID,
        ROOT_COMPONENT_REGISTRY_STATE_ID, ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_ID,
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
        fleet_registry::{FleetRegistryVersion, FleetSubnetRootDrainingReservationResponse},
        root_store::RootStoreBootstrapRequest,
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentChildBinding, ComponentInstanceId,
        ComponentSpecId, ComponentTopologyDigest, FleetSubnetRootBinding,
        FleetSubnetRootReleaseSet, SubnetId,
    },
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
const COMPONENT_DRAINING_RECORD_MAX_BYTES: u32 = 8_192;
#[cfg(feature = "root-control-plane")]
const SECONDS_PER_DAY: u128 = 86_400;

#[cfg(feature = "root-control-plane")]
fn root_deletion_retained_cycles_target(
    idle_cycles_burned_per_day: u128,
    freezing_threshold_seconds: u128,
) -> Option<u128> {
    idle_cycles_burned_per_day
        .checked_mul(freezing_threshold_seconds)?
        .div_ceil(SECONDS_PER_DAY)
        .checked_add(FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES)
}

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
                key = "canic.control_plane.root.component.registry_state.v1",
                ty = RootComponentRegistryState,
                id = ROOT_COMPONENT_REGISTRY_STATE_ID
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
            key = "canic.control_plane.root.component.draining.v1",
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
            key = "canic.control_plane.root.component.subtree_removal_history.v1",
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
            key = "canic.control_plane.root.component.registry_entries.v1",
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
            key = "canic.control_plane.root.component.principal_index.v1",
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
            key = "canic.control_plane.root.component.allocations.v1",
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
    pub root_draining: Option<RootFleetSubnetDrainingRecord>,
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
/// RootFleetSubnetDrainingRecord
///
/// Durable root-local cutoff for new top-level Component allocation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetSubnetDrainingRecord {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub placement_subnet: SubnetId,
    pub active_registry: FleetRegistryVersion,
    pub reservation: FleetSubnetRootDrainingReservationResponse,
    pub component_topology_digest: ComponentTopologyDigest,
    pub active_release_set: FleetSubnetRootReleaseSet,
    pub next_allocation_sequence: u64,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    pub managed_descendants: u32,
    pub known_created_component_canisters: u32,
    pub root_registry_encoded_bytes: u64,
    pub started_at_ns: u64,
    pub final_inventory_intent: Option<RootFleetSubnetFinalInventoryIntentRecord>,
    pub final_inventory: Option<RootFleetSubnetFinalInventoryRecord>,
    pub removal_publication: Option<RootFleetSubnetRemovalPublicationRecord>,
    pub store_reclamation_intent: Option<RootFleetSubnetStoreReclamationIntentRecord>,
    pub store_reclamation: Option<RootFleetSubnetStoreReclamationRecord>,
    pub store_binding_finalization_intent:
        Option<RootFleetSubnetStoreBindingFinalizationIntentRecord>,
    pub store_binding_finalization: Option<RootFleetSubnetStoreBindingFinalizationRecord>,
    pub store_deletion_intent: Option<RootFleetSubnetStoreDeletionIntentRecord>,
    pub store_deletion: Option<RootFleetSubnetStoreDeletionRecord>,
    pub root_deletion_preparation_intent: Option<RootFleetSubnetDeletionPreparationIntentRecord>,
    pub root_deletion_preparation: Option<RootFleetSubnetDeletionPreparationRecord>,
}

///
/// RootFleetSubnetFinalInventoryIntentRecord
///
/// Durable terminal Component authority frozen before the Store write-fence effect.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetSubnetFinalInventoryIntentRecord {
    pub operation_id: [u8; 32],
    pub registry: FleetRegistryVersion,
    pub removed_component_instances: u32,
    pub terminal_component_history_hash: [u8; 32],
    pub root_registry_encoded_bytes: u64,
    pub prepared_at_ns: u64,
}

///
/// RootFleetSubnetFinalInventoryRecord
///
/// Durable terminal Component history and retained write-fenced Store authority.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetSubnetFinalInventoryRecord {
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

///
/// RootFleetSubnetRemovalPublicationRecord
///
/// Durable local evidence of the Coordinator's exact logical root-removal commit.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetSubnetRemovalPublicationRecord {
    pub operation_id: [u8; 32],
    pub final_inventory_hash: [u8; 32],
    pub previous_registry: FleetRegistryVersion,
    pub registry: FleetRegistryVersion,
    pub recorded_at_ns: u64,
}

/// Durable authority frozen before the logically removed root's Store begins destructive GC.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetSubnetStoreReclamationIntentRecord {
    pub operation_id: [u8; 32],
    pub final_inventory_hash: [u8; 32],
    pub wasm_store: Principal,
    pub prepared_at_ns: u64,
}

/// Durable terminal proof that one logically removed root Store completed exact GC.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetSubnetStoreReclamationRecord {
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

/// Durable authority frozen before a reclaimed Store is removed from publication binding slots.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetSubnetStoreBindingFinalizationIntentRecord {
    pub operation_id: [u8; 32],
    pub final_inventory_hash: [u8; 32],
    pub reclamation_hash: [u8; 32],
    pub wasm_store: Principal,
    pub binding: String,
    pub source_generation: u64,
    pub prepared_at_ns: u64,
}

/// Durable proof that the reclaimed Store no longer occupies a publication binding slot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetSubnetStoreBindingFinalizationRecord {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub binding: String,
    pub final_inventory_hash: [u8; 32],
    pub reclamation_hash: [u8; 32],
    pub source_generation: u64,
    pub finalized_generation: u64,
    pub finalized_at_secs: u64,
    pub completed_at_ns: u64,
    pub finalization_hash: [u8; 32],
}

/// Durable authority frozen before stopping and physically deleting the reclaimed Store.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetSubnetStoreDeletionIntentRecord {
    pub operation_id: [u8; 32],
    pub binding_finalization_hash: [u8; 32],
    pub wasm_store: Principal,
    pub binding: String,
    pub observed_module_hash: [u8; 32],
    pub observed_controllers: Vec<Principal>,
    pub observed_cycles_before_reclamation: u128,
    pub retained_cycles_target: u128,
    pub observed_cycles_after_reclamation: Option<u128>,
    pub cycles_reclaimed_at_ns: Option<u64>,
    pub prepared_at_ns: u64,
}

/// Durable proof that the reclaimed Store was independently observed absent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetSubnetStoreDeletionRecord {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub binding: String,
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

/// Durable authority frozen before a removed root returns cycles to its Coordinator.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetSubnetDeletionPreparationIntentRecord {
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

/// Durable local proof that a removed root is ready for its external deletion executor.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetSubnetDeletionPreparationRecord {
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

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetDrainingRecord {
    pub(crate) fn is_valid_for_current(&self, meta: &RootComponentRegistryMetaRecord) -> bool {
        let source_is_exact = RootFleetSubnetDrainingSourceAuthority::from_record(self)
            == RootFleetSubnetDrainingSourceAuthority::from_meta(meta);
        let registry_is_covered =
            registry_covers_preparation(&meta.prepared_against_registry, &self.active_registry);
        let operation_is_valid = self.operation_id != [0; 32];
        let expected_root = FleetSubnetRootEntry {
            placement_subnet: meta.root.placement_subnet,
            fleet_subnet_root: meta.root.fleet_subnet_root,
            component_admissions: meta.root.component_admissions.clone(),
            component_topology_digest: meta.root.component_topology_digest,
            active_release_set: meta.release_set,
            limits: meta.root.limits.clone(),
            funding: meta.root.funding.clone(),
            status: FleetSubnetRootStatus::Active,
        };
        let reservation_is_valid = [
            self.reservation.request.operation_id == self.operation_id,
            self.reservation.request.expected_registry == self.active_registry,
            self.reservation.request.expected_root == expected_root,
            self.reservation.coordinator == meta.root.authority.binding.coordinator,
            self.reservation.prepared_at_ns > 0,
            self.reservation.reservation_hash != [0; 32],
        ]
        .into_iter()
        .all(|valid| valid);
        let time_is_valid = self.started_at_ns > 0;
        let sequence_is_valid = self.next_allocation_sequence > 0;
        let bytes_are_bounded =
            self.root_registry_encoded_bytes <= meta.root.limits.maximum_registry_bytes;
        let final_inventory_intent_is_valid = self
            .final_inventory_intent
            .as_ref()
            .is_none_or(|intent| intent.is_valid_for_current(meta, self));
        let final_inventory_is_valid = self
            .final_inventory
            .as_ref()
            .is_none_or(|inventory| inventory.is_valid_for_current(meta, self));
        let removal_publication_is_valid = self
            .removal_publication
            .as_ref()
            .is_none_or(|publication| publication.is_valid_for_current(self));
        let store_reclamation_intent_is_valid = self
            .store_reclamation_intent
            .as_ref()
            .is_none_or(|intent| intent.is_valid_for_current(self));
        let store_reclamation_is_valid = self
            .store_reclamation
            .as_ref()
            .is_none_or(|reclamation| reclamation.is_valid_for_current(self));
        let store_binding_finalization_intent_is_valid = self
            .store_binding_finalization_intent
            .as_ref()
            .is_none_or(|intent| intent.is_valid_for_current(self));
        let store_binding_finalization_is_valid = self
            .store_binding_finalization
            .as_ref()
            .is_none_or(|finalization| finalization.is_valid_for_current(self));
        let store_deletion_intent_is_valid = self
            .store_deletion_intent
            .as_ref()
            .is_none_or(|intent| intent.is_valid_for_current(self));
        let store_deletion_is_valid = self
            .store_deletion
            .as_ref()
            .is_none_or(|deletion| deletion.is_valid_for_current(self));
        let root_deletion_preparation_intent_is_valid = self
            .root_deletion_preparation_intent
            .as_ref()
            .is_none_or(|intent| intent.is_valid_for_current(self));
        let root_deletion_preparation_is_valid = self
            .root_deletion_preparation
            .as_ref()
            .is_none_or(|preparation| preparation.is_valid_for_current(self));
        [
            source_is_exact,
            registry_is_covered,
            operation_is_valid,
            reservation_is_valid,
            time_is_valid,
            sequence_is_valid,
            bytes_are_bounded,
            final_inventory_intent_is_valid,
            final_inventory_is_valid,
            removal_publication_is_valid,
            store_reclamation_intent_is_valid,
            store_reclamation_is_valid,
            store_binding_finalization_intent_is_valid,
            store_binding_finalization_is_valid,
            store_deletion_intent_is_valid,
            store_deletion_is_valid,
            root_deletion_preparation_intent_is_valid,
            root_deletion_preparation_is_valid,
        ]
        .into_iter()
        .all(|valid| valid)
    }

    fn matches_begin_meta(&self, meta: &RootComponentRegistryMetaRecord) -> bool {
        let inventory_is_exact = RootFleetSubnetDrainingInventoryAuthority::from_record(self)
            == RootFleetSubnetDrainingInventoryAuthority::from_meta(meta);
        [
            self.is_valid_for_current(meta),
            inventory_is_exact,
            self.final_inventory_intent.is_none(),
            self.final_inventory.is_none(),
            self.removal_publication.is_none(),
            self.store_reclamation_intent.is_none(),
            self.store_reclamation.is_none(),
            self.store_binding_finalization_intent.is_none(),
            self.store_binding_finalization.is_none(),
            self.store_deletion_intent.is_none(),
            self.store_deletion.is_none(),
            self.root_deletion_preparation_intent.is_none(),
            self.root_deletion_preparation.is_none(),
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetFinalInventoryIntentRecord {
    fn is_valid_for_current(
        &self,
        meta: &RootComponentRegistryMetaRecord,
        draining: &RootFleetSubnetDrainingRecord,
    ) -> bool {
        let removed_component_instances = draining.next_allocation_sequence.saturating_sub(1);
        [
            self.operation_id == draining.operation_id,
            registry_covers_preparation(&draining.active_registry, &self.registry),
            u64::from(self.removed_component_instances) == removed_component_instances,
            self.terminal_component_history_hash != [0; 32],
            self.root_registry_encoded_bytes == meta.encoded_bytes,
            self.prepared_at_ns >= draining.started_at_ns,
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetFinalInventoryRecord {
    fn is_valid_for_current(
        &self,
        meta: &RootComponentRegistryMetaRecord,
        draining: &RootFleetSubnetDrainingRecord,
    ) -> bool {
        let removed_component_instances = self.next_allocation_sequence.saturating_sub(1);
        let intent_is_exact = draining
            .final_inventory_intent
            .as_ref()
            .is_some_and(|intent| {
                [
                    self.operation_id == intent.operation_id,
                    self.registry == intent.registry,
                    self.removed_component_instances == intent.removed_component_instances,
                    self.terminal_component_history_hash == intent.terminal_component_history_hash,
                    self.root_registry_encoded_bytes == intent.root_registry_encoded_bytes,
                    self.finalized_at_ns >= intent.prepared_at_ns,
                ]
                .into_iter()
                .all(|valid| valid)
            });
        [
            self.operation_id == draining.operation_id,
            self.fleet_subnet_root == draining.fleet_subnet_root,
            self.placement_subnet == draining.placement_subnet,
            registry_covers_preparation(&draining.active_registry, &self.registry),
            self.component_topology_digest == draining.component_topology_digest,
            self.active_release_set == draining.active_release_set,
            self.next_allocation_sequence == draining.next_allocation_sequence,
            u64::from(self.removed_component_instances) == removed_component_instances,
            self.terminal_component_history_hash != [0; 32],
            self.root_registry_encoded_bytes == meta.encoded_bytes,
            self.wasm_store != Principal::anonymous(),
            self.wasm_store_catalog_hash != [0; 32],
            self.wasm_store_gc_prepared_at_secs > 0,
            self.finalized_at_ns >= draining.started_at_ns,
            self.inventory_hash != [0; 32],
            intent_is_exact,
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetRemovalPublicationRecord {
    fn is_valid_for_current(&self, draining: &RootFleetSubnetDrainingRecord) -> bool {
        let Some(inventory) = draining.final_inventory.as_ref() else {
            return false;
        };
        let registry_transition_is_exact = [
            self.previous_registry.authority == self.registry.authority,
            self.previous_registry.content_hash != [0; 32],
            self.registry.content_hash != [0; 32],
            self.previous_registry
                .revision
                .checked_add(1)
                .is_some_and(|revision| revision == self.registry.revision),
        ]
        .into_iter()
        .all(|valid| valid);
        [
            self.operation_id == draining.operation_id,
            self.final_inventory_hash == inventory.inventory_hash,
            registry_covers_preparation(&inventory.registry, &self.previous_registry),
            registry_transition_is_exact,
            self.recorded_at_ns >= inventory.finalized_at_ns,
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetStoreReclamationIntentRecord {
    fn is_valid_for_current(&self, draining: &RootFleetSubnetDrainingRecord) -> bool {
        let Some(inventory) = draining.final_inventory.as_ref() else {
            return false;
        };
        let Some(publication) = draining.removal_publication.as_ref() else {
            return false;
        };
        [
            self.operation_id == draining.operation_id,
            self.final_inventory_hash == inventory.inventory_hash,
            self.wasm_store == inventory.wasm_store,
            self.prepared_at_ns >= publication.recorded_at_ns,
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetStoreReclamationRecord {
    fn is_valid_for_current(&self, draining: &RootFleetSubnetDrainingRecord) -> bool {
        let Some(inventory) = draining.final_inventory.as_ref() else {
            return false;
        };
        let Some(publication) = draining.removal_publication.as_ref() else {
            return false;
        };
        let Some(intent) = draining.store_reclamation_intent.as_ref() else {
            return false;
        };
        [
            self.operation_id == intent.operation_id,
            self.fleet_subnet_root == draining.fleet_subnet_root,
            self.wasm_store == intent.wasm_store,
            self.final_inventory_hash == intent.final_inventory_hash,
            self.reclaimed_store_bytes == inventory.wasm_store_occupied_bytes,
            self.reclaimed_catalog_entries == inventory.wasm_store_catalog_entries,
            self.reclaimed_template_count == inventory.wasm_store_template_count,
            self.reclaimed_release_count == inventory.wasm_store_release_count,
            self.gc_prepared_at_secs == inventory.wasm_store_gc_prepared_at_secs,
            self.gc_started_at_secs >= self.gc_prepared_at_secs,
            self.gc_completed_at_secs >= self.gc_started_at_secs,
            self.gc_runs_completed == 1,
            self.completed_at_ns >= intent.prepared_at_ns,
            self.completed_at_ns >= publication.recorded_at_ns,
            self.reclamation_hash != [0; 32],
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetStoreBindingFinalizationIntentRecord {
    fn is_valid_for_current(&self, draining: &RootFleetSubnetDrainingRecord) -> bool {
        let Some(reclamation) = draining.store_reclamation.as_ref() else {
            return false;
        };
        [
            self.operation_id == draining.operation_id,
            self.final_inventory_hash == reclamation.final_inventory_hash,
            self.reclamation_hash == reclamation.reclamation_hash,
            self.wasm_store == reclamation.wasm_store,
            self.binding.as_str() == self.wasm_store.to_text(),
            self.source_generation > 0,
            self.prepared_at_ns >= reclamation.completed_at_ns,
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetStoreBindingFinalizationRecord {
    fn is_valid_for_current(&self, draining: &RootFleetSubnetDrainingRecord) -> bool {
        let Some(intent) = draining.store_binding_finalization_intent.as_ref() else {
            return false;
        };
        let expected_finalized_generation = intent.source_generation.checked_add(3);
        [
            self.operation_id == intent.operation_id,
            self.fleet_subnet_root == draining.fleet_subnet_root,
            self.wasm_store == intent.wasm_store,
            self.binding == intent.binding,
            self.final_inventory_hash == intent.final_inventory_hash,
            self.reclamation_hash == intent.reclamation_hash,
            self.source_generation == intent.source_generation,
            Some(self.finalized_generation) == expected_finalized_generation,
            self.finalized_at_secs > 0,
            self.completed_at_ns >= intent.prepared_at_ns,
            self.finalization_hash != [0; 32],
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetStoreDeletionIntentRecord {
    fn has_same_preparation_authority(&self, other: &Self) -> bool {
        [
            self.operation_id == other.operation_id,
            self.binding_finalization_hash == other.binding_finalization_hash,
            self.wasm_store == other.wasm_store,
            self.binding == other.binding,
            self.observed_module_hash == other.observed_module_hash,
            self.observed_controllers == other.observed_controllers,
            self.observed_cycles_before_reclamation == other.observed_cycles_before_reclamation,
            self.retained_cycles_target == other.retained_cycles_target,
            self.prepared_at_ns == other.prepared_at_ns,
        ]
        .into_iter()
        .all(|valid| valid)
    }

    fn is_valid_for_current(&self, draining: &RootFleetSubnetDrainingRecord) -> bool {
        let Some(finalization) = draining.store_binding_finalization.as_ref() else {
            return false;
        };
        let cycle_reclamation_is_valid = match (
            self.observed_cycles_after_reclamation,
            self.cycles_reclaimed_at_ns,
        ) {
            (None, None) => true,
            (Some(observed_after), Some(reclaimed_at_ns)) => [
                observed_after <= self.observed_cycles_before_reclamation,
                observed_after <= self.retained_cycles_target,
                reclaimed_at_ns >= self.prepared_at_ns,
            ]
            .into_iter()
            .all(|valid| valid),
            _ => false,
        };
        [
            self.operation_id == draining.operation_id,
            self.binding_finalization_hash == finalization.finalization_hash,
            self.wasm_store == finalization.wasm_store,
            self.binding == finalization.binding,
            self.observed_module_hash != [0; 32],
            canonical_controllers(&self.observed_controllers),
            self.observed_controllers
                .contains(&draining.fleet_subnet_root),
            self.observed_cycles_before_reclamation > 0,
            self.retained_cycles_target > 0,
            cycle_reclamation_is_valid,
            self.prepared_at_ns >= finalization.completed_at_ns,
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetStoreDeletionRecord {
    fn is_valid_for_current(&self, draining: &RootFleetSubnetDrainingRecord) -> bool {
        let Some(intent) = draining.store_deletion_intent.as_ref() else {
            return false;
        };
        [
            self.operation_id == intent.operation_id,
            self.fleet_subnet_root == draining.fleet_subnet_root,
            self.wasm_store == intent.wasm_store,
            self.binding == intent.binding,
            self.binding_finalization_hash == intent.binding_finalization_hash,
            self.observed_module_hash == intent.observed_module_hash,
            self.observed_controllers == intent.observed_controllers,
            self.observed_cycles_before_reclamation == intent.observed_cycles_before_reclamation,
            self.retained_cycles_target == intent.retained_cycles_target,
            Some(self.observed_cycles_after_reclamation)
                == intent.observed_cycles_after_reclamation,
            Some(self.cycles_reclaimed_at_ns) == intent.cycles_reclaimed_at_ns,
            self.prepared_at_ns == intent.prepared_at_ns,
            self.observed_absent_at_ns >= self.cycles_reclaimed_at_ns,
            self.completed_at_ns >= self.observed_absent_at_ns,
            self.deletion_hash != [0; 32],
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetDeletionPreparationIntentRecord {
    fn has_same_preparation_authority(&self, other: &Self) -> bool {
        [
            self.operation_id == other.operation_id,
            self.coordinator == other.coordinator,
            self.final_inventory_hash == other.final_inventory_hash,
            self.store_deletion_hash == other.store_deletion_hash,
            self.observed_cycles_before_reclamation == other.observed_cycles_before_reclamation,
            self.retained_cycles_target == other.retained_cycles_target,
            self.observed_reserved_cycles == other.observed_reserved_cycles,
            self.observed_idle_cycles_burned_per_day == other.observed_idle_cycles_burned_per_day,
            self.observed_freezing_threshold_seconds == other.observed_freezing_threshold_seconds,
            self.prepared_at_ns == other.prepared_at_ns,
        ]
        .into_iter()
        .all(|valid| valid)
    }

    fn is_valid_for_current(&self, draining: &RootFleetSubnetDrainingRecord) -> bool {
        let Some(inventory) = draining.final_inventory.as_ref() else {
            return false;
        };
        let Some(deletion) = draining.store_deletion.as_ref() else {
            return false;
        };
        let reclamation_is_valid = match (
            self.coordinator_intent_hash,
            self.observed_cycles_after_reclamation,
            self.cycles_reclaimed_at_ns,
        ) {
            (None, None, None) => true,
            (Some(intent_hash), Some(observed_after), Some(reclaimed_at_ns)) => [
                intent_hash != [0; 32],
                observed_after <= self.observed_cycles_before_reclamation,
                observed_after <= self.retained_cycles_target,
                reclaimed_at_ns >= self.prepared_at_ns,
            ]
            .into_iter()
            .all(|valid| valid),
            _ => false,
        };
        [
            self.operation_id == draining.operation_id,
            self.coordinator == draining.active_registry.authority.binding.coordinator,
            self.final_inventory_hash == inventory.inventory_hash,
            self.store_deletion_hash == deletion.deletion_hash,
            self.observed_cycles_before_reclamation > 0,
            self.retained_cycles_target > 0,
            root_deletion_retained_cycles_target(
                self.observed_idle_cycles_burned_per_day,
                self.observed_freezing_threshold_seconds,
            ) == Some(self.retained_cycles_target),
            self.observed_reserved_cycles == 0,
            reclamation_is_valid,
            self.prepared_at_ns >= deletion.completed_at_ns,
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetDeletionPreparationRecord {
    fn is_valid_for_current(&self, draining: &RootFleetSubnetDrainingRecord) -> bool {
        let Some(intent) = draining.root_deletion_preparation_intent.as_ref() else {
            return false;
        };
        [
            self.operation_id == intent.operation_id,
            self.fleet_subnet_root == draining.fleet_subnet_root,
            self.coordinator == intent.coordinator,
            self.final_inventory_hash == intent.final_inventory_hash,
            self.store_deletion_hash == intent.store_deletion_hash,
            self.observed_cycles_before_reclamation == intent.observed_cycles_before_reclamation,
            self.retained_cycles_target == intent.retained_cycles_target,
            self.observed_reserved_cycles == intent.observed_reserved_cycles,
            self.observed_idle_cycles_burned_per_day == intent.observed_idle_cycles_burned_per_day,
            self.observed_freezing_threshold_seconds == intent.observed_freezing_threshold_seconds,
            Some(self.observed_cycles_after_reclamation)
                == intent.observed_cycles_after_reclamation,
            Some(self.cycles_reclaimed_at_ns) == intent.cycles_reclaimed_at_ns,
            Some(self.coordinator_intent_hash) == intent.coordinator_intent_hash,
            self.coordinator_readiness_hash != [0; 32],
            self.prepared_at_ns == intent.prepared_at_ns,
            self.completed_at_ns >= self.cycles_reclaimed_at_ns,
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
fn canonical_controllers(controllers: &[Principal]) -> bool {
    !controllers.is_empty() && controllers.windows(2).all(|pair| pair[0] < pair[1])
}

#[cfg(feature = "root-control-plane")]
fn registry_covers_preparation(
    prepared: &FleetRegistryVersion,
    current: &FleetRegistryVersion,
) -> bool {
    let authority_is_exact = prepared.authority == current.authority;
    let revision_is_covered = match prepared.revision.cmp(&current.revision) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Equal => prepared.content_hash == current.content_hash,
        std::cmp::Ordering::Greater => false,
    };
    let hashes_are_present = prepared.content_hash != [0; 32] && current.content_hash != [0; 32];
    [authority_is_exact, revision_is_covered, hashes_are_present]
        .into_iter()
        .all(|valid| valid)
}

#[cfg(feature = "root-control-plane")]
#[derive(Debug, Eq, PartialEq)]
struct RootFleetSubnetDrainingSourceAuthority {
    fleet_subnet_root: Principal,
    placement_subnet: SubnetId,
    component_topology_digest: ComponentTopologyDigest,
    active_release_set: FleetSubnetRootReleaseSet,
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetDrainingSourceAuthority {
    const fn from_record(record: &RootFleetSubnetDrainingRecord) -> Self {
        Self {
            fleet_subnet_root: record.fleet_subnet_root,
            placement_subnet: record.placement_subnet,
            component_topology_digest: record.component_topology_digest,
            active_release_set: record.active_release_set,
        }
    }

    const fn from_meta(meta: &RootComponentRegistryMetaRecord) -> Self {
        Self {
            fleet_subnet_root: meta.root.fleet_subnet_root,
            placement_subnet: meta.root.placement_subnet,
            component_topology_digest: meta.root.component_topology_digest,
            active_release_set: meta.release_set,
        }
    }
}

#[cfg(feature = "root-control-plane")]
#[derive(Debug, Eq, PartialEq)]
struct RootFleetSubnetDrainingInventoryAuthority {
    next_allocation_sequence: u64,
    reserved_component_instances: u32,
    committed_component_instances: u32,
    managed_descendants: u32,
    known_created_component_canisters: u32,
    root_registry_encoded_bytes: u64,
}

#[cfg(feature = "root-control-plane")]
impl RootFleetSubnetDrainingInventoryAuthority {
    const fn from_record(record: &RootFleetSubnetDrainingRecord) -> Self {
        Self {
            next_allocation_sequence: record.next_allocation_sequence,
            reserved_component_instances: record.reserved_component_instances,
            committed_component_instances: record.committed_component_instances,
            managed_descendants: record.managed_descendants,
            known_created_component_canisters: record.known_created_component_canisters,
            root_registry_encoded_bytes: record.root_registry_encoded_bytes,
        }
    }

    const fn from_meta(meta: &RootComponentRegistryMetaRecord) -> Self {
        Self {
            next_allocation_sequence: meta.next_allocation_sequence,
            reserved_component_instances: meta.reserved_component_instances,
            committed_component_instances: meta.committed_component_instances,
            managed_descendants: meta.managed_descendants,
            known_created_component_canisters: meta.known_created_component_canisters,
            root_registry_encoded_bytes: meta.encoded_bytes,
        }
    }
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
#[derive(Debug, Eq, PartialEq)]
struct RootComponentAllocationIdentity<'a> {
    operation_id: &'a [u8; 32],
    component: &'a ComponentInstanceId,
}

#[cfg(feature = "root-control-plane")]
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
    Removed {
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
    pub protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest,
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
    pub protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest,
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
    pub subtree_operation_id: Option<[u8; 32]>,
    pub final_inventory: Option<RootComponentFinalInventoryRecord>,
    pub deletion: Option<RootComponentDeletionProgressRecord>,
}

///
/// RootComponentFinalInventoryRecord
///
/// Durable exact empty-inventory authority frozen before top-level Component deletion.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentFinalInventoryRecord {
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
/// RootComponentDeletionIntentRecord
///
/// Complete final-inventory and quiescence authority frozen before top-level deletion.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentDeletionIntentRecord {
    pub final_inventory: RootComponentFinalInventoryRecord,
    pub quiescence: RootComponentQuiescentReceiptRecord,
    pub prepared_at_ns: u64,
}

///
/// RootComponentDeletedReceiptRecord
///
/// Durable top-level workload-deletion authority retained after Canister recycling.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentDeletedReceiptRecord {
    pub deletion: RootComponentDeletionIntentRecord,
    pub deleted_at_ns: u64,
}

///
/// RootComponentMembershipRemovedRecord
///
/// Terminal local-membership and settled root/Spec accounting authority.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentMembershipRemovedRecord {
    pub deleted: RootComponentDeletedReceiptRecord,
    pub allocation_operation_id: [u8; 32],
    pub remaining_spec_committed_instances: u32,
    pub root_committed_component_instances: u32,
    pub root_known_created_component_canisters: u32,
    pub root_registry_encoded_bytes: u64,
    pub removed_at_ns: u64,
    pub removal_hash: [u8; 32],
}

///
/// RootComponentDeletionProgressRecord
///
/// Monotonic deletion and local-membership state within one draining record.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentDeletionProgressRecord {
    DeleteIntent(RootComponentDeletionIntentRecord),
    Deleted(RootComponentDeletedReceiptRecord),
    MembershipRemoved(RootComponentMembershipRemovedRecord),
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

#[cfg(feature = "root-control-plane")]
#[derive(Debug, Eq, PartialEq)]
struct RootComponentDrainingFenceAuthority<'a> {
    operation_id: [u8; 32],
    component: ComponentInstanceId,
    previous_registry: &'a ComponentRegistryHead,
    registry: &'a ComponentRegistryHead,
    descendant_count: u32,
    descendant_content_hash: [u8; 32],
    directory_authority_hash: [u8; 32],
    started_at_ns: u64,
}

#[cfg(feature = "root-control-plane")]
impl<'a> From<&'a RootComponentDrainingRecord> for RootComponentDrainingFenceAuthority<'a> {
    fn from(record: &'a RootComponentDrainingRecord) -> Self {
        Self {
            operation_id: record.operation_id,
            component: record.component,
            previous_registry: &record.previous_registry,
            registry: &record.registry,
            descendant_count: record.descendant_count,
            descendant_content_hash: record.descendant_content_hash,
            directory_authority_hash: record.directory_authority_hash,
            started_at_ns: record.started_at_ns,
        }
    }
}

#[cfg(feature = "root-control-plane")]
#[derive(Debug, Eq, PartialEq)]
struct RootComponentDrainingIdentityAuthority<'a> {
    fence: RootComponentDrainingFenceAuthority<'a>,
    subtree_operation_id: Option<[u8; 32]>,
    final_inventory: &'a Option<RootComponentFinalInventoryRecord>,
    deletion: &'a Option<RootComponentDeletionProgressRecord>,
}

#[cfg(feature = "root-control-plane")]
impl<'a> From<&'a RootComponentDrainingRecord> for RootComponentDrainingIdentityAuthority<'a> {
    fn from(record: &'a RootComponentDrainingRecord) -> Self {
        Self {
            fence: RootComponentDrainingFenceAuthority::from(record),
            subtree_operation_id: record.subtree_operation_id,
            final_inventory: &record.final_inventory,
            deletion: &record.deletion,
        }
    }
}

#[cfg(feature = "root-control-plane")]
#[derive(Debug, Eq, PartialEq)]
struct RootComponentDrainingDeletionBaseAuthority<'a> {
    fence: RootComponentDrainingFenceAuthority<'a>,
    quiescence: &'a Option<RootComponentQuiescenceProgressRecord>,
    subtree_operation_id: Option<[u8; 32]>,
    final_inventory: &'a Option<RootComponentFinalInventoryRecord>,
}

#[cfg(feature = "root-control-plane")]
impl<'a> From<&'a RootComponentDrainingRecord> for RootComponentDrainingDeletionBaseAuthority<'a> {
    fn from(record: &'a RootComponentDrainingRecord) -> Self {
        Self {
            fence: RootComponentDrainingFenceAuthority::from(record),
            quiescence: &record.quiescence,
            subtree_operation_id: record.subtree_operation_id,
            final_inventory: &record.final_inventory,
        }
    }
}

#[cfg(feature = "root-control-plane")]
fn component_draining_identity_matches(
    left: &RootComponentDrainingRecord,
    right: &RootComponentDrainingRecord,
) -> bool {
    RootComponentDrainingIdentityAuthority::from(left)
        == RootComponentDrainingIdentityAuthority::from(right)
}

#[cfg(feature = "root-control-plane")]
fn component_draining_deletion_base_matches(
    left: &RootComponentDrainingRecord,
    right: &RootComponentDrainingRecord,
) -> bool {
    RootComponentDrainingDeletionBaseAuthority::from(left)
        == RootComponentDrainingDeletionBaseAuthority::from(right)
}

#[cfg(feature = "root-control-plane")]
fn component_draining_fence_matches(
    left: &RootComponentDrainingRecord,
    right: &RootComponentDrainingRecord,
) -> bool {
    RootComponentDrainingFenceAuthority::from(left)
        == RootComponentDrainingFenceAuthority::from(right)
}

#[cfg(feature = "root-control-plane")]
const fn component_draining_has_no_progress(record: &RootComponentDrainingRecord) -> bool {
    if record.quiescence.is_some() {
        return false;
    }
    if record.subtree_operation_id.is_some() {
        return false;
    }
    if record.final_inventory.is_some() {
        return false;
    }
    record.deletion.is_none()
}

#[cfg(feature = "root-control-plane")]
fn component_final_inventory_transition_is_valid(
    expected: &RootComponentDrainingRecord,
    next: &RootComponentDrainingRecord,
    charged_entry_bytes: u64,
) -> bool {
    if !component_draining_fence_matches(expected, next) {
        return false;
    }
    if expected.quiescence != next.quiescence {
        return false;
    }
    if expected.subtree_operation_id != next.subtree_operation_id {
        return false;
    }
    if expected.deletion != next.deletion {
        return false;
    }
    if expected.final_inventory.is_some() {
        return false;
    }
    if next.final_inventory.is_none() {
        return false;
    }
    RootComponentRegistryStore::component_draining_entry_bytes(next) <= charged_entry_bytes
}

#[cfg(feature = "root-control-plane")]
fn component_draining_cursor_transition_is_valid(
    expected: &RootComponentDrainingRecord,
    next: &RootComponentDrainingRecord,
    removal: &RootComponentSubtreeRemovalRecord,
) -> bool {
    if expected.component != removal.component {
        return false;
    }
    if !component_draining_fence_matches(expected, next) {
        return false;
    }
    if expected.quiescence != next.quiescence {
        return false;
    }
    if expected.final_inventory != next.final_inventory {
        return false;
    }
    if expected.deletion != next.deletion {
        return false;
    }
    if next.subtree_operation_id != Some(removal.operation_id) {
        return false;
    }
    if removal.operation_id == [0; 32] {
        return false;
    }
    let charged_entry_bytes = next
        .quiescence
        .as_ref()
        .map(|progress| match progress {
            RootComponentQuiescenceProgressRecord::StopIntent(intent) => intent.charged_entry_bytes,
            RootComponentQuiescenceProgressRecord::Quiescent(receipt) => {
                receipt.stop.charged_entry_bytes
            }
        })
        .unwrap_or_default();
    RootComponentRegistryStore::component_draining_entry_bytes(next) <= charged_entry_bytes
}

#[cfg(feature = "root-control-plane")]
fn component_deletion_intent_transition_is_valid(
    expected: &RootComponentDrainingRecord,
    next: &RootComponentDrainingRecord,
) -> bool {
    if !component_draining_deletion_base_matches(expected, next) {
        return false;
    }
    if expected.deletion.is_some() {
        return false;
    }
    let Some(final_inventory) = &expected.final_inventory else {
        return false;
    };
    let Some(RootComponentQuiescenceProgressRecord::Quiescent(quiescence)) = &expected.quiescence
    else {
        return false;
    };
    let Some(RootComponentDeletionProgressRecord::DeleteIntent(intent)) = &next.deletion else {
        return false;
    };
    let authority_is_exact =
        intent.final_inventory == *final_inventory && intent.quiescence == *quiescence;
    let time_is_monotonic = intent.prepared_at_ns >= final_inventory.finalized_at_ns;
    let fits_precharged_entry = RootComponentRegistryStore::component_draining_entry_bytes(next)
        <= quiescence.stop.charged_entry_bytes;
    authority_is_exact && time_is_monotonic && fits_precharged_entry
}

#[cfg(feature = "root-control-plane")]
fn component_deleted_transition_is_valid(
    expected: &RootComponentDrainingRecord,
    next: &RootComponentDrainingRecord,
) -> bool {
    if !component_draining_deletion_base_matches(expected, next) {
        return false;
    }
    let Some(RootComponentDeletionProgressRecord::DeleteIntent(intent)) = &expected.deletion else {
        return false;
    };
    let Some(RootComponentDeletionProgressRecord::Deleted(receipt)) = &next.deletion else {
        return false;
    };
    let receipt_is_exact = receipt.deletion == *intent;
    let time_is_monotonic = receipt.deleted_at_ns >= intent.prepared_at_ns;
    let fits_precharged_entry = RootComponentRegistryStore::component_draining_entry_bytes(next)
        <= intent.quiescence.stop.charged_entry_bytes;
    receipt_is_exact && time_is_monotonic && fits_precharged_entry
}

#[cfg(feature = "root-control-plane")]
fn component_membership_removed_transition_is_valid(
    expected: &RootComponentDrainingRecord,
    next: &RootComponentDrainingRecord,
    allocation_operation_id: [u8; 32],
    next_meta: &RootComponentRegistryMetaRecord,
) -> bool {
    if !component_draining_deletion_base_matches(expected, next) {
        return false;
    }
    let Some(RootComponentDeletionProgressRecord::Deleted(deleted)) = &expected.deletion else {
        return false;
    };
    let Some(RootComponentDeletionProgressRecord::MembershipRemoved(receipt)) = &next.deletion
    else {
        return false;
    };
    let receipt_authority_is_exact =
        receipt.deleted == *deleted && receipt.allocation_operation_id == allocation_operation_id;
    let receipt_is_hashed = receipt.removal_hash != [0; 32];
    let settled_root_authority_is_exact =
        RootComponentRemovalSettlementAuthority::from_receipt(receipt)
            == RootComponentRemovalSettlementAuthority::from_meta(next_meta);
    let time_is_monotonic = receipt.removed_at_ns >= deleted.deleted_at_ns;
    let fits_precharged_entry = RootComponentRegistryStore::component_draining_entry_bytes(next)
        <= deleted.deletion.quiescence.stop.charged_entry_bytes;
    [
        receipt_authority_is_exact,
        receipt_is_hashed,
        settled_root_authority_is_exact,
        time_is_monotonic,
        fits_precharged_entry,
    ]
    .into_iter()
    .all(|valid| valid)
}

#[cfg(feature = "root-control-plane")]
#[derive(Debug, Eq, PartialEq)]
struct RootComponentRemovalSettlementAuthority {
    committed_component_instances: u32,
    known_created_component_canisters: u32,
    encoded_bytes: u64,
}

#[cfg(feature = "root-control-plane")]
impl RootComponentRemovalSettlementAuthority {
    const fn from_receipt(receipt: &RootComponentMembershipRemovedRecord) -> Self {
        Self {
            committed_component_instances: receipt.root_committed_component_instances,
            known_created_component_canisters: receipt.root_known_created_component_canisters,
            encoded_bytes: receipt.root_registry_encoded_bytes,
        }
    }

    const fn from_meta(meta: &RootComponentRegistryMetaRecord) -> Self {
        Self {
            committed_component_instances: meta.committed_component_instances,
            known_created_component_canisters: meta.known_created_component_canisters,
            encoded_bytes: meta.encoded_bytes,
        }
    }
}

#[cfg(feature = "root-control-plane")]
fn component_allocation_removed_transition_is_valid(
    expected: &RootComponentAllocationRecord,
    next: &RootComponentAllocationRecord,
) -> bool {
    let RootComponentAllocationProgressRecord::Committed {
        creation,
        canister,
        installation,
        commitment,
    } = &expected.progress
    else {
        return false;
    };
    let mut terminal = expected.clone();
    terminal.progress = RootComponentAllocationProgressRecord::Removed {
        creation: creation.clone(),
        canister: *canister,
        installation: installation.clone(),
        commitment: commitment.clone(),
    };
    &terminal == next
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

#[cfg(feature = "root-control-plane")]
#[derive(Debug, Eq, PartialEq)]
struct ComponentPartitionStableAuthority<'a> {
    binding: &'a ComponentBinding,
    provisioning_origin: &'a ComponentProvisioningOrigin,
    release_set: &'a FleetSubnetRootReleaseSet,
}

#[cfg(feature = "root-control-plane")]
impl<'a> From<&'a ComponentRegistryPartitionRecord> for ComponentPartitionStableAuthority<'a> {
    fn from(partition: &'a ComponentRegistryPartitionRecord) -> Self {
        Self {
            binding: &partition.binding,
            provisioning_origin: &partition.provisioning_origin,
            release_set: &partition.release_set,
        }
    }
}

#[cfg(feature = "root-control-plane")]
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

#[cfg(feature = "root-control-plane")]
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
    pub application_init_args: Option<Vec<u8>>,
    pub reserved_against_registry: ComponentRegistryHead,
    pub release_set: FleetSubnetRootReleaseSet,
    pub progress: RootComponentChildAllocationProgressRecord,
}

#[cfg(feature = "root-control-plane")]
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
    application_init_args: &'a Option<Vec<u8>>,
    reserved_against_registry: &'a ComponentRegistryHead,
    release_set: &'a FleetSubnetRootReleaseSet,
}

#[cfg(feature = "root-control-plane")]
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
            application_init_args: &record.application_init_args,
            reserved_against_registry: &record.reserved_against_registry,
            release_set: &record.release_set,
        }
    }
}

#[cfg(feature = "root-control-plane")]
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

#[cfg(feature = "root-control-plane")]
#[derive(Debug, Eq, PartialEq)]
struct RootComponentSubtreeFence<'a> {
    operation_id: &'a [u8; 32],
    component: &'a ComponentInstanceId,
    target: &'a ComponentRegistryChildRecord,
    reserved_against_registry: &'a ComponentRegistryHead,
    maximum_completed_leaves: u32,
}

#[cfg(feature = "root-control-plane")]
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

#[cfg(feature = "root-control-plane")]
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
/// Exact workload-deletion authority retained after Canister recycling.
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
    pub protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest,
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
    pub protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest,
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

#[cfg(feature = "root-control-plane")]
#[derive(Debug, Eq, PartialEq)]
struct ComponentParentRoleAuthority<'a> {
    component: &'a ComponentInstanceId,
    parent_canister_id: &'a Principal,
    child_role: &'a CanisterRole,
}

#[cfg(feature = "root-control-plane")]
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

#[cfg(feature = "root-control-plane")]
#[derive(Debug, Eq, PartialEq)]
struct ComponentChildIndexAuthority<'a> {
    component: &'a ComponentInstanceId,
    parent_canister_id: &'a Principal,
    role: &'a CanisterRole,
    canister_id: Principal,
}

#[cfg(feature = "root-control-plane")]
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

#[cfg(feature = "root-control-plane")]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct RootComponentAllocationOperationKey([u8; 32]);

#[cfg(feature = "root-control-plane")]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct RootComponentDrainingKey([u8; 32]);

#[cfg(feature = "root-control-plane")]
impl From<ComponentInstanceId> for RootComponentDrainingKey {
    fn from(value: ComponentInstanceId) -> Self {
        Self(*value.as_bytes())
    }
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(RootComponentDrainingKey, 128, false);

#[cfg(feature = "root-control-plane")]
impl From<[u8; 32]> for RootComponentAllocationOperationKey {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(RootComponentAllocationOperationKey, 128, false);

#[cfg(feature = "root-control-plane")]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct RootComponentSubtreeRemovalHistoryKey {
    component: [u8; 32],
    operation_id: [u8; 32],
    traversal_steps: u32,
}

#[cfg(feature = "root-control-plane")]
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

#[cfg(feature = "root-control-plane")]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct ComponentRegistryPrincipalKey(Vec<u8>);

#[cfg(feature = "root-control-plane")]
impl From<Principal> for ComponentRegistryPrincipalKey {
    fn from(value: Principal) -> Self {
        Self(value.as_slice().to_vec())
    }
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(ComponentRegistryPrincipalKey, 128, false);

#[cfg(feature = "root-control-plane")]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct ComponentRegistryEntryKey {
    component: [u8; 32],
    index: ComponentRegistryEntryIndexKey,
}

#[cfg(feature = "root-control-plane")]
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

#[cfg(feature = "root-control-plane")]
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

#[cfg(feature = "root-control-plane")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootComponentRegistryCommitOutcome {
    Committed,
    Existing,
}

///
/// RootComponentSubtreeRemovalBeginCommit
///
/// Exact stable compare-and-commit authority for one subtree fence and optional drain cursor.
///

#[cfg(feature = "root-control-plane")]
pub struct RootComponentSubtreeRemovalBeginCommit<'a> {
    pub expected_meta: &'a RootComponentRegistryMetaRecord,
    pub next_meta: RootComponentRegistryMetaRecord,
    pub expected_partition: &'a ComponentRegistryPartitionRecord,
    pub next_partition: ComponentRegistryPartitionRecord,
    pub expected_target: &'a ComponentRegistryChildRecord,
    pub record: RootComponentSubtreeRemovalRecord,
    pub expected_draining: Option<&'a RootComponentDrainingRecord>,
    pub next_draining: Option<RootComponentDrainingRecord>,
}

///
/// RootComponentMembershipRemovalCommit
///
/// Exact stable compare-and-commit authority for terminal top-level membership removal.
///

#[cfg(feature = "root-control-plane")]
pub struct RootComponentMembershipRemovalCommit<'a> {
    pub expected_meta: &'a RootComponentRegistryMetaRecord,
    pub next_meta: RootComponentRegistryMetaRecord,
    pub expected_partition: &'a ComponentRegistryPartitionRecord,
    pub expected_allocation: &'a RootComponentAllocationRecord,
    pub next_allocation: RootComponentAllocationRecord,
    pub expected_draining: &'a RootComponentDrainingRecord,
    pub next_draining: RootComponentDrainingRecord,
}

#[cfg(feature = "root-control-plane")]
#[derive(Debug, Eq, PartialEq)]
struct RootComponentRemovalPartitionAuthority<'a> {
    registry: ComponentRegistryHead,
    descendant_content_hash: [u8; 32],
    registry_encoded_bytes: u64,
    directory_synchronized_at_ns: u64,
    binding: &'a ComponentBinding,
    provisioning_origin: &'a ComponentProvisioningOrigin,
    release_set: FleetSubnetRootReleaseSet,
}

#[cfg(feature = "root-control-plane")]
impl<'a> RootComponentRemovalPartitionAuthority<'a> {
    const fn from_partition(partition: &'a ComponentRegistryPartitionRecord) -> Self {
        Self {
            registry: ComponentRegistryHead {
                component: partition.binding.component,
                revision: partition.revision,
                content_hash: partition.content_hash,
            },
            descendant_content_hash: partition.descendant_content_hash,
            registry_encoded_bytes: partition.encoded_bytes,
            directory_synchronized_at_ns: partition.directory_synchronized_at_ns,
            binding: &partition.binding,
            provisioning_origin: &partition.provisioning_origin,
            release_set: partition.release_set,
        }
    }

    fn from_commit(commit: &'a RootComponentMembershipRemovalCommit<'a>) -> Option<Self> {
        let receipt = commit.receipt()?;
        let RootComponentAllocationProgressRecord::Committed {
            canister,
            installation,
            ..
        } = &commit.expected_allocation.progress
        else {
            return None;
        };
        if *canister != installation.binding.canister_id {
            return None;
        }
        let inventory = &receipt.deleted.deletion.final_inventory;
        Some(Self {
            registry: inventory.registry.clone(),
            descendant_content_hash: inventory.descendant_content_hash,
            registry_encoded_bytes: inventory.registry_encoded_bytes,
            directory_synchronized_at_ns: inventory.directory_synchronized_at_ns,
            binding: &installation.binding,
            provisioning_origin: &commit.expected_allocation.provisioning_origin,
            release_set: commit.expected_allocation.release_set,
        })
    }
}

#[cfg(feature = "root-control-plane")]
impl RootComponentMembershipRemovalCommit<'_> {
    const fn receipt(&self) -> Option<&RootComponentMembershipRemovedRecord> {
        match &self.next_draining.deletion {
            Some(RootComponentDeletionProgressRecord::MembershipRemoved(receipt)) => Some(receipt),
            None
            | Some(
                RootComponentDeletionProgressRecord::DeleteIntent(_)
                | RootComponentDeletionProgressRecord::Deleted(_),
            ) => None,
        }
    }

    fn meta_transition_is_valid(&self) -> bool {
        let partition_bytes =
            RootComponentRegistryStore::partition_entry_bytes(self.expected_partition);
        let principal_bytes = RootComponentRegistryStore::principal_index_entry_bytes(
            self.expected_partition.binding.canister_id,
            self.expected_partition.binding.component,
        );
        let previous_allocation_bytes =
            RootComponentRegistryStore::allocation_entry_bytes(self.expected_allocation);
        let next_allocation_bytes =
            RootComponentRegistryStore::allocation_entry_bytes(&self.next_allocation);
        let Some(next_encoded_bytes) = self
            .expected_meta
            .encoded_bytes
            .checked_sub(partition_bytes)
            .and_then(|bytes| bytes.checked_sub(principal_bytes))
            .and_then(|bytes| bytes.checked_sub(previous_allocation_bytes))
            .and_then(|bytes| bytes.checked_add(next_allocation_bytes))
        else {
            return false;
        };
        let mut expected_next = self.expected_meta.clone();
        let Some(committed_component_instances) =
            expected_next.committed_component_instances.checked_sub(1)
        else {
            return false;
        };
        let Some(known_created_component_canisters) = expected_next
            .known_created_component_canisters
            .checked_sub(1)
        else {
            return false;
        };
        expected_next.committed_component_instances = committed_component_instances;
        expected_next.known_created_component_canisters = known_created_component_canisters;
        expected_next.encoded_bytes = next_encoded_bytes;
        self.next_meta == expected_next
    }

    fn shape_is_valid(&self) -> bool {
        let Some(receipt) = self.receipt() else {
            return false;
        };
        let component = self.expected_partition.binding.component;
        let component_identity_is_exact = [
            self.expected_allocation.component,
            self.expected_draining.component,
            self.next_draining.component,
        ]
        .into_iter()
        .all(|candidate| candidate == component);
        let allocation_operation_is_exact =
            receipt.allocation_operation_id == self.expected_allocation.operation_id;
        let partition_is_empty_and_draining = self.expected_partition.status
            == ComponentLifecycleStatus::Draining
            && self.expected_partition.reserved_descendants == 0
            && self.expected_partition.committed_descendants == 0;
        let partition_authority_is_exact =
            RootComponentRemovalPartitionAuthority::from_commit(self)
                == Some(RootComponentRemovalPartitionAuthority::from_partition(
                    self.expected_partition,
                ));
        let meta_transition_is_valid = self.meta_transition_is_valid();
        let allocation_transition_is_valid = component_allocation_removed_transition_is_valid(
            self.expected_allocation,
            &self.next_allocation,
        );
        let draining_transition_is_valid = component_membership_removed_transition_is_valid(
            self.expected_draining,
            &self.next_draining,
            self.expected_allocation.operation_id,
            &self.next_meta,
        );
        [
            component_identity_is_exact,
            allocation_operation_is_exact,
            partition_is_empty_and_draining,
            partition_authority_is_exact,
            meta_transition_is_valid,
            allocation_transition_is_valid,
            draining_transition_is_valid,
        ]
        .into_iter()
        .all(|valid| valid)
    }
}

#[cfg(feature = "root-control-plane")]
impl RootComponentSubtreeRemovalBeginCommit<'_> {
    fn draining_transition_is_valid(&self) -> bool {
        match (self.expected_draining, &self.next_draining) {
            (None, None) => true,
            (Some(expected), Some(next)) => {
                component_draining_cursor_transition_is_valid(expected, next, &self.record)
            }
            (None, Some(_)) | (Some(_), None) => false,
        }
    }

    fn shape_is_valid(&self) -> bool {
        let component = self.record.component;
        self.draining_transition_is_valid()
            && self.expected_partition.binding.component == component
            && self.next_partition.binding.component == component
            && self.next_partition.binding == self.expected_partition.binding
            && self.next_partition.provisioning_origin
                == self.expected_partition.provisioning_origin
            && self.next_partition.release_set == self.expected_partition.release_set
            && self.next_partition.status == self.expected_partition.status
            && self.next_partition.revision == self.expected_partition.revision
            && self.next_partition.content_hash == self.expected_partition.content_hash
            && self.next_partition.descendant_content_hash
                == self.expected_partition.descendant_content_hash
            && self.next_partition.directory_synchronized_at_ns
                == self.expected_partition.directory_synchronized_at_ns
            && self.next_partition.reserved_descendants
                == self.expected_partition.reserved_descendants
            && self.next_partition.committed_descendants
                == self.expected_partition.committed_descendants
            && &self.record.target == self.expected_target
            && self.record.target.component == component
    }
}

///
/// RootComponentRegistryCommitError
///
/// Rejection when preparation conflicts with already durable authority.
///

#[cfg(feature = "root-control-plane")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootComponentRegistryCommitError {
    ConflictingState,
}

///
/// RootComponentAllocationCommitError
///
/// Stable-store rejection for one top-level Component identity reservation.
///

#[cfg(feature = "root-control-plane")]
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
#[cfg(feature = "root-control-plane")]
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

    pub(crate) fn begin_root_draining(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetDrainingRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        if !record.matches_begin_meta(expected) {
            return Err(RootComponentRegistryCommitError::ConflictingState);
        }
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if current.root_draining.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            if current != expected || current.root_draining.is_some() {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn prepare_root_final_inventory(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetFinalInventoryIntentRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.final_inventory_intent.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let transition_is_exact = [
                current == expected,
                draining.final_inventory_intent.is_none(),
                draining.final_inventory.is_none(),
                record.is_valid_for_current(current, draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .final_inventory_intent = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn finalize_root_inventory(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetFinalInventoryRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.final_inventory.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let transition_is_exact = [
                current == expected,
                draining.final_inventory.is_none(),
                record.is_valid_for_current(current, draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .final_inventory = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn record_root_removal_publication(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetRemovalPublicationRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.removal_publication.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let transition_is_exact = [
                current == expected,
                draining.removal_publication.is_none(),
                record.is_valid_for_current(draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .removal_publication = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn prepare_root_store_reclamation(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetStoreReclamationIntentRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.store_reclamation_intent.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let transition_is_exact = [
                current == expected,
                draining.store_reclamation_intent.is_none(),
                draining.store_reclamation.is_none(),
                record.is_valid_for_current(draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .store_reclamation_intent = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn record_root_store_reclamation(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetStoreReclamationRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.store_reclamation.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let transition_is_exact = [
                current == expected,
                draining.store_reclamation_intent.is_some(),
                draining.store_reclamation.is_none(),
                record.is_valid_for_current(draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .store_reclamation = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn prepare_root_store_binding_finalization(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetStoreBindingFinalizationIntentRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.store_binding_finalization_intent.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let transition_is_exact = [
                current == expected,
                draining.store_binding_finalization_intent.is_none(),
                draining.store_binding_finalization.is_none(),
                record.is_valid_for_current(draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .store_binding_finalization_intent = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn record_root_store_binding_finalization(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetStoreBindingFinalizationRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.store_binding_finalization.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let transition_is_exact = [
                current == expected,
                draining.store_binding_finalization_intent.is_some(),
                draining.store_binding_finalization.is_none(),
                record.is_valid_for_current(draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .store_binding_finalization = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn prepare_root_store_deletion(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetStoreDeletionIntentRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.store_deletion_intent.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let transition_is_exact = [
                current == expected,
                draining.store_binding_finalization.is_some(),
                draining.store_deletion_intent.is_none(),
                draining.store_deletion.is_none(),
                record.is_valid_for_current(draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .store_deletion_intent = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn record_root_store_cycle_reclamation(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetStoreDeletionIntentRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.store_deletion_intent.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let Some(previous) = draining.store_deletion_intent.as_ref() else {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            };
            let transition_is_exact = [
                current == expected,
                previous.has_same_preparation_authority(&record),
                previous.observed_cycles_after_reclamation.is_none(),
                previous.cycles_reclaimed_at_ns.is_none(),
                record.observed_cycles_after_reclamation.is_some(),
                record.cycles_reclaimed_at_ns.is_some(),
                draining.store_deletion.is_none(),
                record.is_valid_for_current(draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .store_deletion_intent = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn record_root_store_deletion(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetStoreDeletionRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.store_deletion.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let transition_is_exact = [
                current == expected,
                draining.store_deletion_intent.is_some(),
                draining.store_deletion.is_none(),
                record.is_valid_for_current(draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .store_deletion = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn prepare_root_deletion(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetDeletionPreparationIntentRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.root_deletion_preparation_intent.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let transition_is_exact = [
                current == expected,
                draining.store_deletion.is_some(),
                draining.root_deletion_preparation_intent.is_none(),
                draining.root_deletion_preparation.is_none(),
                record.is_valid_for_current(draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .root_deletion_preparation_intent = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn record_root_deletion_cycle_reclamation(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetDeletionPreparationIntentRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.root_deletion_preparation_intent.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let Some(previous) = draining.root_deletion_preparation_intent.as_ref() else {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            };
            let transition_is_exact = [
                current == expected,
                previous.has_same_preparation_authority(&record),
                previous.coordinator_intent_hash.is_none(),
                previous.observed_cycles_after_reclamation.is_none(),
                previous.cycles_reclaimed_at_ns.is_none(),
                record.coordinator_intent_hash.is_some(),
                record.observed_cycles_after_reclamation.is_some(),
                record.cycles_reclaimed_at_ns.is_some(),
                draining.root_deletion_preparation.is_none(),
                record.is_valid_for_current(draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .root_deletion_preparation_intent = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
        })
    }

    pub(crate) fn record_root_deletion_preparation(
        expected: &RootComponentRegistryMetaRecord,
        record: RootFleetSubnetDeletionPreparationRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current = state
                .current
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            let draining = current
                .root_draining
                .as_ref()
                .ok_or(RootComponentRegistryCommitError::ConflictingState)?;
            if draining.root_deletion_preparation.as_ref() == Some(&record) {
                return Ok(RootComponentRegistryCommitOutcome::Existing);
            }
            let transition_is_exact = [
                current == expected,
                draining.root_deletion_preparation_intent.is_some(),
                draining.root_deletion_preparation.is_none(),
                record.is_valid_for_current(draining),
            ]
            .into_iter()
            .all(|valid| valid);
            if !transition_is_exact {
                return Err(RootComponentRegistryCommitError::ConflictingState);
            }
            let mut next = current.clone();
            next.root_draining
                .as_mut()
                .expect("validated root draining authority")
                .root_deletion_preparation = Some(record);
            state.current = Some(next);
            cell.set(state);
            Ok(RootComponentRegistryCommitOutcome::Committed)
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
    pub(crate) fn component_drainings() -> Vec<RootComponentDrainingRecord> {
        ROOT_COMPONENT_DRAINING.with_borrow(|map| map.iter().map(|entry| entry.value()).collect())
    }

    #[must_use]
    pub(crate) fn registry_components() -> Vec<ComponentInstanceId> {
        COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
            map.iter()
                .map(|entry| ComponentInstanceId::from_generated_bytes(entry.key().component))
                .collect()
        })
    }

    #[must_use]
    pub(crate) fn principal_inventory_is_empty() -> bool {
        COMPONENT_REGISTRY_PRINCIPAL_INDEX.with_borrow(StableBtreeMap::is_empty)
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
                } else if matches!(
                    record.progress,
                    RootComponentAllocationProgressRecord::Removed { .. }
                ) {
                    (reserved, committed)
                } else {
                    (reserved + 1, committed)
                }
            })
        })
    }

    #[must_use]
    pub(crate) fn peer_allocation_counts(
        requester: &ComponentBinding,
        target_component_spec: &ComponentSpecId,
    ) -> (usize, usize) {
        ROOT_COMPONENT_ALLOCATIONS.with_borrow(|map| {
            map.iter().fold((0, 0), |(reserved, committed), entry| {
                let record = entry.value();
                let recorded_requester = match &record.provisioning_origin {
                    ComponentProvisioningOrigin::Component { requester, .. } => requester.as_ref(),
                    ComponentProvisioningOrigin::FleetServiceComponent { requester, .. } => {
                        &requester.component
                    }
                    ComponentProvisioningOrigin::FleetAdministrator { .. }
                    | ComponentProvisioningOrigin::ComponentGroup { .. } => {
                        return (reserved, committed);
                    }
                };
                if recorded_requester != requester
                    || &record.component_spec != target_component_spec
                {
                    return (reserved, committed);
                }
                if matches!(
                    record.progress,
                    RootComponentAllocationProgressRecord::Committed { .. }
                ) {
                    (reserved, committed + 1)
                } else if matches!(
                    record.progress,
                    RootComponentAllocationProgressRecord::Removed { .. }
                ) {
                    (reserved, committed)
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
    pub(crate) fn subtree_removal_history(
        component: ComponentInstanceId,
    ) -> Vec<RootComponentSubtreeRemovalCompletedLeafRecord> {
        ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY.with_borrow(|map| {
            let start = RootComponentSubtreeRemovalHistoryKey::new(component, [0; 32], 0);
            let end =
                RootComponentSubtreeRemovalHistoryKey::new(component, [u8::MAX; 32], u32::MAX);
            map.range((Bound::Included(start), Bound::Included(end)))
                .map(|entry| entry.value())
                .collect()
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

    #[must_use]
    pub(crate) fn component_live_inventory_is_empty(component: ComponentInstanceId) -> bool {
        COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
            map.range((
                Bound::Included(ComponentRegistryEntryKey::partition(component)),
                Bound::Unbounded,
            ))
            .take_while(|entry| entry.key().component == *component.as_bytes())
            .all(|entry| {
                matches!(
                    entry.value(),
                    ComponentRegistryEntryRecord::Partition(_)
                        | ComponentRegistryEntryRecord::ChildAllocation(_)
                        | ComponentRegistryEntryRecord::SubtreeRemoval(_)
                )
            })
        })
    }

    #[must_use]
    pub(crate) fn component_principal_inventory_is_exact(
        component: ComponentInstanceId,
        top_level_canister: Principal,
    ) -> bool {
        Self::component_for_principal(top_level_canister) == Some(component)
            && COMPONENT_REGISTRY_PRINCIPAL_INDEX.with_borrow(|map| {
                map.iter()
                    .filter(|entry| entry.value().component == component)
                    .count()
                    == 1
            })
    }

    #[must_use]
    pub(crate) fn component_principal_inventory_is_empty(component: ComponentInstanceId) -> bool {
        COMPONENT_REGISTRY_PRINCIPAL_INDEX
            .with_borrow(|map| map.iter().all(|entry| entry.value().component != component))
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
            let root_accepts_allocations = current.root_draining.is_none();
            let draining_fence_is_preserved = next_meta.root_draining == current.root_draining;
            if !root_accepts_allocations || !draining_fence_is_preserved {
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

    #[expect(
        clippy::too_many_lines,
        reason = "one atomic compare-and-commit verifies and writes the subtree fence, partition and optional drain cursor"
    )]
    pub(crate) fn begin_subtree_removal(
        commit: RootComponentSubtreeRemovalBeginCommit<'_>,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentAllocationCommitError> {
        if !commit.shape_is_valid() {
            return Err(RootComponentAllocationCommitError::ConflictingChildEntry);
        }
        let RootComponentSubtreeRemovalBeginCommit {
            expected_meta,
            next_meta,
            expected_partition,
            next_partition,
            expected_target,
            record,
            expected_draining,
            next_draining,
        } = commit;
        let component = record.component;
        let operation_key =
            ComponentRegistryEntryKey::subtree_removal(component, record.operation_id);
        let partition_key = ComponentRegistryEntryKey::partition(component);
        let target_key = ComponentRegistryEntryKey::child(component, record.target.canister_id);

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
            if let Some(expected_draining) = expected_draining {
                let draining_key = RootComponentDrainingKey::from(component);
                let current_draining = ROOT_COMPONENT_DRAINING
                    .with_borrow(|map| map.get(&draining_key))
                    .ok_or(RootComponentAllocationCommitError::ConflictingState)?;
                if current_draining != *expected_draining {
                    return Err(RootComponentAllocationCommitError::ConflictingState);
                }
                if let Some(previous_operation_id) = expected_draining.subtree_operation_id
                    && previous_operation_id != record.operation_id
                {
                    let previous_key = ComponentRegistryEntryKey::subtree_removal(
                        component,
                        previous_operation_id,
                    );
                    let previous_is_completed = COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                        matches!(
                            map.get(&previous_key),
                            Some(ComponentRegistryEntryRecord::SubtreeRemoval(previous))
                                if matches!(
                                    previous.progress,
                                    RootComponentSubtreeRemovalProgressRecord::Completed(_)
                                )
                        )
                    });
                    if !previous_is_completed {
                        return Err(RootComponentAllocationCommitError::ConflictingState);
                    }
                }
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
            if let Some(next_draining) = next_draining {
                ROOT_COMPONENT_DRAINING.with_borrow_mut(|map| {
                    map.insert(RootComponentDrainingKey::from(component), next_draining);
                });
            }
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
            || !component_draining_has_no_progress(&record)
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

    pub(crate) fn mark_component_final_inventory(
        expected_record: &RootComponentDrainingRecord,
        next_record: RootComponentDrainingRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let component = expected_record.component;
        let key = RootComponentDrainingKey::from(component);
        let charged_entry_bytes = match &expected_record.quiescence {
            Some(RootComponentQuiescenceProgressRecord::Quiescent(receipt)) => {
                receipt.stop.charged_entry_bytes
            }
            None | Some(RootComponentQuiescenceProgressRecord::StopIntent(_)) => {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
        };
        if !component_final_inventory_transition_is_valid(
            expected_record,
            &next_record,
            charged_entry_bytes,
        ) {
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

    pub(crate) fn prepare_component_deletion(
        expected_record: &RootComponentDrainingRecord,
        next_record: RootComponentDrainingRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        if !component_deletion_intent_transition_is_valid(expected_record, &next_record) {
            return Err(RootComponentAllocationCommitError::ConflictingState);
        }
        Self::replace_component_draining(expected_record, next_record)
    }

    pub(crate) fn mark_component_deleted(
        expected_record: &RootComponentDrainingRecord,
        next_record: RootComponentDrainingRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        if !component_deleted_transition_is_valid(expected_record, &next_record) {
            return Err(RootComponentAllocationCommitError::ConflictingState);
        }
        Self::replace_component_draining(expected_record, next_record)
    }

    pub(crate) fn remove_component_membership(
        commit: RootComponentMembershipRemovalCommit<'_>,
    ) -> Result<(), RootComponentAllocationCommitError> {
        if !commit.shape_is_valid() {
            return Err(RootComponentAllocationCommitError::ConflictingState);
        }
        let receipt = commit
            .receipt()
            .ok_or(RootComponentAllocationCommitError::ConflictingState)?;
        let (_, current_spec_committed) =
            Self::allocation_counts(&commit.expected_allocation.component_spec);
        if current_spec_committed.checked_sub(1)
            != usize::try_from(receipt.remaining_spec_committed_instances).ok()
        {
            return Err(RootComponentAllocationCommitError::ConflictingState);
        }

        let component = commit.expected_partition.binding.component;
        let partition_key = ComponentRegistryEntryKey::partition(component);
        let principal_key =
            ComponentRegistryPrincipalKey::from(commit.expected_partition.binding.canister_id);
        let allocation_key =
            RootComponentAllocationOperationKey::from(commit.expected_allocation.operation_id);
        let draining_key = RootComponentDrainingKey::from(component);

        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            let current_meta = state
                .current
                .as_ref()
                .ok_or(RootComponentAllocationCommitError::Uninitialized)?;
            if current_meta != commit.expected_meta {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let current_partition = COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.get(&partition_key).and_then(|entry| match entry {
                    ComponentRegistryEntryRecord::Partition(partition) => Some(partition),
                    _ => None,
                })
            });
            let current_allocation =
                ROOT_COMPONENT_ALLOCATIONS.with_borrow(|map| map.get(&allocation_key));
            let current_draining =
                ROOT_COMPONENT_DRAINING.with_borrow(|map| map.get(&draining_key));
            let partition_is_exact = current_partition.as_ref() == Some(commit.expected_partition);
            let allocation_is_exact =
                current_allocation.as_ref() == Some(commit.expected_allocation);
            let draining_is_exact = current_draining.as_ref() == Some(commit.expected_draining);
            if ![partition_is_exact, allocation_is_exact, draining_is_exact]
                .into_iter()
                .all(|exact| exact)
            {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }
            let principal_is_exact = COMPONENT_REGISTRY_PRINCIPAL_INDEX.with_borrow(|map| {
                let indexed_component_is_exact = map
                    .get(&principal_key)
                    .is_some_and(|indexed| indexed.component == component);
                let component_principal_count = map
                    .iter()
                    .filter(|entry| entry.value().component == component)
                    .count();
                indexed_component_is_exact && component_principal_count == 1
            });
            if !principal_is_exact || !Self::component_live_inventory_is_empty(component) {
                return Err(RootComponentAllocationCommitError::ConflictingState);
            }

            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.remove(&partition_key);
            });
            COMPONENT_REGISTRY_PRINCIPAL_INDEX.with_borrow_mut(|map| {
                map.remove(&principal_key);
            });
            ROOT_COMPONENT_ALLOCATIONS.with_borrow_mut(|map| {
                map.insert(allocation_key, commit.next_allocation);
            });
            ROOT_COMPONENT_DRAINING.with_borrow_mut(|map| {
                map.insert(draining_key, commit.next_draining);
            });
            state.current = Some(commit.next_meta);
            cell.set(state);
            Ok(())
        })
    }

    fn replace_component_draining(
        expected_record: &RootComponentDrainingRecord,
        next_record: RootComponentDrainingRecord,
    ) -> Result<(), RootComponentAllocationCommitError> {
        let key = RootComponentDrainingKey::from(expected_record.component);
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
