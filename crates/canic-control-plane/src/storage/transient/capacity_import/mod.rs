//! Exclusive execution of the current Root capacity import's paid transitions.
//!
//! This gate prevents overlapping management callbacks and terminal accounting.
//! It carries no effect authority: stable import records survive its cancellation.

use std::cell::Cell;

thread_local! {
    static EXECUTING: Cell<bool> = const { Cell::new(false) };
}

/// Held across every await in an import advance or terminal balance observation.
/// CDK task cancellation drops the guard; issued stable intent stays unresolved.
#[derive(Debug)]
pub struct PoolImportExecutionGuard;

impl PoolImportExecutionGuard {
    /// Claim without waiting or expiring another invocation's outstanding call.
    pub(crate) fn try_claim() -> Option<Self> {
        EXECUTING.with(|executing| {
            if executing.replace(true) {
                None
            } else {
                Some(Self)
            }
        })
    }
}

impl Drop for PoolImportExecutionGuard {
    fn drop(&mut self) {
        EXECUTING.set(false);
    }
}
