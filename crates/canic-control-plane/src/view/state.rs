//! Module: view::state
//!
//! Responsibility: model read-only control-plane state projections.
//! Does not own: persisted state, endpoint responses, or state transitions.
//! Boundary: storage ops construct these values for workflow consumption.

use crate::ids::{WasmStoreBinding, WasmStoreCreationPurpose, WasmStoreGcMode};
use canic_core::{
    cdk::types::Principal, control_plane_support::model::replay::ReplayCostGuardSettlement,
};

///
/// PublicationStoreStateView
///
/// Read-only publication-store binding lifecycle state projected by storage ops.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PublicationStoreStateView {
    pub active_binding: Option<WasmStoreBinding>,
    pub detached_binding: Option<WasmStoreBinding>,
    pub retired_binding: Option<WasmStoreBinding>,
    pub generation: u64,
    pub changed_at: u64,
    pub retired_at: u64,
}

///
/// WasmStoreGcView
///
/// Read-only wasm-store garbage-collection state projected by storage ops.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WasmStoreGcView {
    pub mode: WasmStoreGcMode,
    pub changed_at: u64,
    pub prepared_at: Option<u64>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub runs_completed: u32,
}

///
/// WasmStoreView
///
/// Read-only runtime-managed wasm-store state projected by storage ops.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmStoreView {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub created_at: u64,
    pub gc: WasmStoreGcView,
}

///
/// WasmStoreCreationProgressView
///
/// Internal read-only phase of one root-owned Store creation operation.
/// Constructed by storage ops and consumed by Store creation recovery.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WasmStoreCreationProgressView {
    CreationIntent,
    Created {
        pid: Principal,
        created_at: u64,
    },
    InstallIntent {
        pid: Principal,
        created_at: u64,
        cost_guard_settlement: ReplayCostGuardSettlement,
    },
    Installed {
        pid: Principal,
        created_at: u64,
        cost_guard_settlement: ReplayCostGuardSettlement,
    },
}

///
/// WasmStoreCreationView
///
/// Internal read-only projection of one durable root-owned Store creation operation.
/// Constructed by storage ops and consumed by Store creation recovery.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmStoreCreationView {
    pub sequence: u64,
    pub purpose: WasmStoreCreationPurpose,
    pub expected_module_hash: [u8; 32],
    pub payload_size_bytes: u64,
    pub controllers: Vec<Principal>,
    pub initial_cycles: u128,
    pub creation_cost_guard_settlement: ReplayCostGuardSettlement,
    pub prepared_at: u64,
    pub progress: WasmStoreCreationProgressView,
}
