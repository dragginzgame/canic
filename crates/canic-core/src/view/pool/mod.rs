//! Module: view::pool
//!
//! Responsibility: define read-only canister-pool scan projections.
//! Does not own: stable storage, mutation authority, or scheduler decisions.
//! Boundary: storage ops return bounded pages to the pool recovery workflow.

use crate::cdk::types::Principal;

/// Stable ordering position for a pending-reset pool scan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PoolPendingResetCursor {
    pub created_at: u64,
    pub pid: Principal,
}

/// Bounded pending-reset page used by the pool recovery scheduler.
pub struct PoolPendingResetPage {
    pub pids: Vec<Principal>,
    pub next_cursor: Option<PoolPendingResetCursor>,
}
