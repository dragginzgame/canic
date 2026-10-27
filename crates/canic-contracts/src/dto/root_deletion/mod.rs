//! Exact observed authority and receipts for controller-owned Root deletion.
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// FleetSubnetRootDeletionExecutionRequest with native principal identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PrepareRequest {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub expected_readiness_hash: [u8; 32],
    pub observed_module_hash: [u8; 32],
    pub observed_controllers: Vec<Principal>,
    pub observed_cycles_after_reclamation: u128,
    pub observed_reserved_cycles: u128,
    pub observed_idle_cycles_burned_per_day: u128,
    pub observed_freezing_threshold_seconds: u128,
}

/// FleetSubnetRootDeletionExecutionResponse with native principal identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Execution {
    pub request: PrepareRequest,
    pub executor: Principal,
    pub prepared_at_ns: u64,
    pub execution_hash: [u8; 32],
}

/// FleetSubnetRootDeletionCompletionRequest with native principal identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CompleteRequest {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub expected_execution_hash: [u8; 32],
    pub observed_absent_at_ns: u64,
}

/// FleetSubnetRootDeletionResponse with native principal identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Receipt {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub coordinator: Principal,
    pub executor: Principal,
    pub readiness_hash: [u8; 32],
    pub execution_hash: [u8; 32],
    pub observed_module_hash: [u8; 32],
    pub observed_controllers: Vec<Principal>,
    pub observed_cycles_after_reclamation: u128,
    pub observed_absent_at_ns: u64,
    pub completed_at_ns: u64,
    pub deletion_hash: [u8; 32],
}
