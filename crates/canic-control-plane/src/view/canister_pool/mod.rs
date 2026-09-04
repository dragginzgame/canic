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
    WaitingForFunding {
        available_cycles: u128,
        observed_at_ns: u64,
        retry_at_ns: u64,
    },
    Blocked {
        failure: CanisterPoolCreationFailureView,
    },
}

/// Read-only authority for one Cycles Ledger pool-refill operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanisterPoolCreationView {
    pub attempt_count: u32,
    pub operation_id: [u8; 32],
    pub cycles_ledger: Principal,
    pub placement_subnet: Principal,
    pub root: Principal,
    pub ledger_amount: Cycles,
    pub ledger_fee: Cycles,
    pub readiness_floor: Cycles,
    pub creation_execution_margin: Cycles,
    pub management_creation_fee: Cycles,
    pub created_at_time_ns: u64,
    pub last_attempt_at_ns: Option<u64>,
    pub cost_guard_settlement: Option<ReplayCostGuardSettlement>,
    pub progress: CanisterPoolCreationProgressView,
}

/// Read-only authority for one in-progress draining-root asset handoff.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanisterPoolHandoffView {
    pub canister_id: Principal,
    pub recipient: Principal,
}
