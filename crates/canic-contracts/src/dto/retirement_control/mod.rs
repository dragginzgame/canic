//! Native removal and retirement intents; canonical cycle conservation and receipt ownership stay unchanged.
use crate::dto::{fleet_snapshot::FleetSubnetRootEntry, registry_discovery::RegistryVersion};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Native FleetSubnetRootDrainingReservationRequest boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RemovalRequest {
    pub asset_recipient: Principal,
    pub operation_id: [u8; 32],
    pub expected_registry: RegistryVersion,
    pub expected_root: FleetSubnetRootEntry,
}

/// Native FleetSubnetRootDrainingReservationResponse boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RemovalReservation {
    pub request: RemovalRequest,
    pub coordinator: Principal,
    pub prepared_at_ns: u64,
    pub reservation_hash: [u8; 32],
}

/// Native FleetRetirementRequest boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RetirementRequest {
    pub operation_id: [u8; 32],
    pub expected_registry: RegistryVersion,
    pub destination: Principal,
    pub maximum_ledger_fee: u128,
}

/// Native FleetRetirementStatus boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RetirementStatus {
    pub request: RetirementRequest,
    pub prepared_at_ns: u64,
    pub ledger_transfer: Option<LedgerTransferIntent>,
    pub ledger_receipt: Option<LedgerTransferReceipt>,
}

/// Native FleetLedgerTransferIntent boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct LedgerTransferIntent {
    pub source: Principal,
    pub destination: Principal,
    pub balance_before: u128,
    pub fee: u128,
    pub created_at_time: u64,
    pub memo: [u8; 32],
}

/// Native FleetLedgerTransferReceipt boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct LedgerTransferReceipt {
    pub intent: LedgerTransferIntent,
    pub block_index: Option<u128>,
}

/// Exact Coordinator reservation delivered to the Root before its local drain fence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootRemovalRequest {
    pub reservation: RemovalReservation,
}

/// Complete native FleetSubnetRootDrainingResponse evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Draining {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: ic_platform::Principal,
    pub placement_subnet: ic_platform::Principal,
    pub active_registry: crate::dto::registry_discovery::RegistryVersion,
    pub reservation_hash: [u8; 32],
    pub component_topology_digest: crate::ids::ComponentTopologyDigest,
    pub active_release_set: crate::ids::FleetSubnetRootReleaseSet,
    pub next_allocation_sequence: u64,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    pub managed_descendants: u32,
    pub known_created_component_canisters: u32,
    pub root_registry_encoded_bytes: u64,
    pub started_at_ns: u64,
}

/// Complete native FleetSubnetRootFinalInventoryResponse evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FinalInventory {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: ic_platform::Principal,
    pub placement_subnet: ic_platform::Principal,
    pub registry: crate::dto::registry_discovery::RegistryVersion,
    pub component_topology_digest: crate::ids::ComponentTopologyDigest,
    pub active_release_set: crate::ids::FleetSubnetRootReleaseSet,
    pub next_allocation_sequence: u64,
    pub removed_component_instances: u32,
    pub terminal_component_history_hash: [u8; 32],
    pub root_registry_encoded_bytes: u64,
    pub wasm_store: ic_platform::Principal,
    pub wasm_store_catalog_hash: [u8; 32],
    pub wasm_store_catalog_entries: u32,
    pub wasm_store_occupied_bytes: u64,
    pub wasm_store_template_count: u32,
    pub wasm_store_release_count: u32,
    pub wasm_store_gc_prepared_at_secs: u64,
    pub finalized_at_ns: u64,
    pub inventory_hash: [u8; 32],
}

/// Complete native FleetSubnetRootDrainingPublicationResponse evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DrainingPublication {
    pub root_draining: Draining,
    pub previous_version: crate::dto::registry_discovery::RegistryVersion,
    pub version: crate::dto::registry_discovery::RegistryVersion,
}

/// Complete native FleetSubnetRootRemovalPublicationResponse evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RemovalPublication {
    pub final_inventory: FinalInventory,
    pub previous_version: crate::dto::registry_discovery::RegistryVersion,
    pub version: crate::dto::registry_discovery::RegistryVersion,
}

/// Complete native FleetSubnetRootDeletionReadinessIntentRequest evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ReadinessIntentRequest {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: ic_platform::Principal,
    pub final_inventory_hash: [u8; 32],
    pub store_deletion_hash: [u8; 32],
    pub observed_cycles_before_reclamation: u128,
    pub retained_cycles_target: u128,
    pub observed_reserved_cycles: u128,
    pub observed_idle_cycles_burned_per_day: u128,
    pub observed_freezing_threshold_seconds: u128,
    pub prepared_at_ns: u64,
}

/// Complete native FleetSubnetRootDeletionReadinessIntentResponse evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ReadinessIntent {
    pub request: ReadinessIntentRequest,
    pub coordinator: ic_platform::Principal,
    pub recorded_at_ns: u64,
    pub intent_hash: [u8; 32],
}

/// Complete native FleetSubnetRootDeletionReadinessRequest evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ReadinessRequest {
    pub ledger_receipt: LedgerTransferReceipt,
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: ic_platform::Principal,
    pub expected_intent_hash: [u8; 32],
    pub observed_cycles_after_reclamation: u128,
    pub cycles_reclaimed_at_ns: u64,
}

/// Complete native FleetSubnetRootDeletionReadinessResponse evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Readiness {
    pub request: ReadinessRequest,
    pub coordinator: ic_platform::Principal,
    pub final_inventory_hash: [u8; 32],
    pub store_deletion_hash: [u8; 32],
    pub observed_cycles_before_reclamation: u128,
    pub retained_cycles_target: u128,
    pub observed_reserved_cycles: u128,
    pub observed_idle_cycles_burned_per_day: u128,
    pub observed_freezing_threshold_seconds: u128,
    pub prepared_at_ns: u64,
    pub recorded_at_ns: u64,
    pub readiness_hash: [u8; 32],
}

/// Complete native CoordinatorRootRemovalOperationStatus evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RemovalStatus {
    pub operation_id: [u8; 32],
    pub reservation: RemovalReservation,
    pub draining: Option<DrainingPublication>,
    pub removal: Option<RemovalPublication>,
    pub readiness_intent: Option<ReadinessIntent>,
    pub readiness: Option<Readiness>,
    pub execution: Option<crate::dto::root_deletion::Execution>,
    pub completion: Option<crate::dto::root_deletion::Receipt>,
}
