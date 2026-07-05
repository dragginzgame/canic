//! Module: view::icp_refill
//!
//! Responsibility: define read-only ICP refill operation projections.
//! Does not own: stable storage records, workflow decisions, or DTO responses.
//! Boundary: internal view used between storage ops and ICP refill workflows.

use crate::{
    cdk::{
        candid::Nat,
        types::{Principal, Subaccount},
    },
    domain::icp_refill::{IcpRefillErrorCode, IcpRefillStatus},
};

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
    pub source_canister: Principal,
    pub source_subaccount: Option<Subaccount>,
    pub target_canister: Principal,
    pub ledger_canister_id: Principal,
    pub cmc_canister_id: Principal,
    pub amount_e8s: u64,
    pub fee_e8s: u64,
    pub memo: Vec<u8>,
    pub created_at_time_ns: u64,
    pub ledger_block_index: Option<u64>,
    pub notify_attempts: u32,
    pub cycles_sent: Option<Nat>,
    pub status: IcpRefillStatus,
    pub error_code: Option<IcpRefillErrorCode>,
    pub error_message: Option<String>,
}
