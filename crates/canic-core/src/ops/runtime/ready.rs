//! Module: ops::runtime::ready
//!
//! Responsibility: own the runtime readiness barrier.
//! Does not own: bootstrap orchestration, lifecycle adapters, or public status DTOs.
//! Boundary: records the single transition from not-ready to ready.

use super::bootstrap::BootstrapStatusOps;
use std::cell::Cell;

thread_local! {
    // Readiness is an internal synchronization barrier.
    // It must only be set once by bootstrap completion.
    // It must never be reset or inferred from public state.
    static READY: Cell<bool> = const { Cell::new(false) };
}

///
/// ReadyOps
///
/// Operations-layer facade for the bootstrap readiness barrier.
///

pub struct ReadyOps;

impl ReadyOps {
    #[must_use]
    pub fn is_ready() -> bool {
        READY.with(Cell::get)
    }

    pub fn mark_ready() {
        READY.with(|ready| {
            if !ready.get() {
                ready.set(true);
                BootstrapStatusOps::mark_ready();
                crate::log!(crate::log::Topic::Init, Info, "canister marked READY");
            }
        });
    }
}
