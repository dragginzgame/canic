//! Module: storage::transient::canister_pool
//!
//! Responsibility: exclude overlapping reset execution for each physical asset.
//! Does not own: durable reset authority, allocation claims or recovery decisions.
//! Boundary: ops admits a reset; its guard survives every workflow await.

use canic_core::cdk::types::Principal;
use std::{cell::RefCell, collections::BTreeSet};

thread_local! {
    static RESETTING: RefCell<BTreeSet<Principal>> = const { RefCell::new(BTreeSet::new()) };
}

///
/// PoolResetExecutionGuard
///
/// Transient storage ownership for one running reset. Stable pending state retains
/// recovery authority when CDK task cancellation drops the guard.
///

#[derive(Debug)]
pub struct PoolResetExecutionGuard {
    canister_id: Principal,
}

impl PoolResetExecutionGuard {
    /// Exclude a competing call without expiring an outstanding management effect.
    pub(crate) fn try_claim(canister_id: Principal) -> Option<Self> {
        RESETTING.with_borrow_mut(|resetting| {
            if resetting.insert(canister_id) {
                Some(Self { canister_id })
            } else {
                None
            }
        })
    }
}

impl Drop for PoolResetExecutionGuard {
    fn drop(&mut self) {
        RESETTING.with_borrow_mut(|resetting| {
            resetting.remove(&self.canister_id);
        });
    }
}
