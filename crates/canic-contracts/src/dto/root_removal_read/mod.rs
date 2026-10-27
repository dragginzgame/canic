//! Complete Root-removal observation, including original cycle-conservation authority.
use crate::dto::retirement_control::{
    Draining, FinalInventory, LedgerTransferReceipt, ReadinessIntentRequest, ReadinessRequest,
    RemovalPublication,
};
use ic_platform::Principal;

/// Complete RootRemovalOperationStatus evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Status {
    pub operation_id: [u8; 32],
    pub draining: Draining,
    pub final_inventory: Option<FinalInventory>,
    pub removal: Option<RemovalPublication>,
    pub store_reclamation: Option<StoreReclamation>,
    pub store_binding_finalization: Option<StoreFinalization>,
    pub store_deletion: Option<StoreDeletion>,
    pub deletion_readiness_intent: Option<ReadinessIntentRequest>,
    pub deletion_readiness: Option<ReadinessRequest>,
    pub deletion_preparation: Option<Preparation>,
}

/// Complete FleetSubnetRootStoreReclamationResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct StoreReclamation {
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

/// Complete FleetSubnetRootStoreBindingFinalizationResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct StoreFinalization {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub final_inventory_hash: [u8; 32],
    pub reclamation_hash: [u8; 32],
    pub source_generation: u64,
    pub finalized_generation: u64,
    pub finalized_at_secs: u64,
    pub completed_at_ns: u64,
    pub finalization_hash: [u8; 32],
}

/// Complete FleetSubnetRootStoreDeletionResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct StoreDeletion {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
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

/// Complete FleetSubnetRootDeletionPreparationResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Preparation {
    pub ledger_receipt: LedgerTransferReceipt,
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
