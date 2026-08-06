//! Module: view::canister_pool
//!
//! Responsibility: expose read-only projections of durable Canister pool work.
//! Does not own: stable records, endpoint DTOs, or workflow mutation.
//! Boundary: ops projects storage records here before workflow orchestration.

use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::model::replay::ReplayCostGuardSettlement,
};

/// Read-only reason one autonomous refill is blocked without a principal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanisterPoolCreationFailureView {
    UnresolvedAfterLedgerWindow,
    LedgerCreationFailed,
    LedgerRejected,
}

/// Read-only progress of one exact autonomous pool refill.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanisterPoolCreationProgressView {
    Intent {
        uncertain_result: bool,
    },
    Created {
        block_index: u64,
        canister_id: Principal,
    },
    Blocked {
        failure: CanisterPoolCreationFailureView,
    },
}

/// Read-only authority for one Cycles Ledger pool-refill operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanisterPoolCreationView {
    pub operation_id: [u8; 32],
    pub cycles_ledger: Principal,
    pub placement_subnet: Principal,
    pub root: Principal,
    pub ledger_amount: Cycles,
    pub created_at_time_ns: u64,
    pub cost_guard_settlement: Option<ReplayCostGuardSettlement>,
    pub progress: CanisterPoolCreationProgressView,
}

/// Read-only authority for one in-progress draining-root asset handoff.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanisterPoolHandoffView {
    pub canister_id: Principal,
    pub recipient: Principal,
}
