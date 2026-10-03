//! Module: view::icp_refill
//!
//! Responsibility: define read-only ICP refill operation projections.
//! Does not own: stable storage records, workflow decisions, or DTO responses.
//! Boundary: internal view used between storage ops and ICP refill workflows.

use crate::domain::icp_refill::{IcpRefillErrorCode, IcpRefillStatus, IcpRefillTrigger};
use candid::{Nat, Principal};

///
/// IcpRefillOperation
///
/// Read-only projection of one ICP refill operation.
/// Owned by view and consumed by workflow orchestration.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpRefillOperation {
    pub id: u64,
    pub operation_id: [u8; 32],
    pub trigger: IcpRefillTrigger,
    pub policy_hash: [u8; 32],
    pub source_canister: Principal,
    pub source_subaccount: Option<[u8; 32]>,
    pub target_canister: Principal,
    pub ledger_canister_id: Principal,
    pub cmc_canister_id: Principal,
    pub cmc_to_account_owner: Principal,
    pub cmc_to_account_subaccount: Option<[u8; 32]>,
    pub amount_e8s: u64,
    pub fee_e8s: u64,
    pub budget_window_start_secs: u64,
    pub budget_reserved: bool,
    pub memo: Vec<u8>,
    pub created_at_time_ns: u64,
    pub ledger_block_index: Option<u64>,
    pub transfer_uncertain: bool,
    pub notify_attempts: u32,
    pub cycles_sent: Option<Nat>,
    pub status: IcpRefillStatus,
    pub error_code: Option<IcpRefillErrorCode>,
    pub error_message: Option<String>,
    pub refund_block_index: Option<u64>,
    pub transaction_too_old_min_block_index: Option<u64>,
}

/// A bounded, key-ordered census of retained refill evidence for Fleet release.
///
/// Includes terminal and non-resumable records: neither retry exhaustion nor a
/// later completed operation proves an earlier paid effect settled. Source
/// accounts remain relevant even after completion. This is a time-local read,
/// not a producer fence, settlement decision or authority to repeat an effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpRefillReleasePage {
    pub operations: Vec<IcpRefillOperation>,
    pub next_after: Option<u64>,
}
