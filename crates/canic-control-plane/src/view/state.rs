//! Module: view::state
//!
//! Responsibility: model read-only control-plane state projections.
//! Does not own: persisted state, endpoint responses, or state transitions.
//! Boundary: storage ops construct these values for workflow consumption.

use crate::ids::{WasmStoreBinding, WasmStoreGcMode};
use canic_core::cdk::types::Principal;

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
