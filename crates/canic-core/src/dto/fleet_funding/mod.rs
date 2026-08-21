//! Module: dto::fleet_funding
//!
//! Responsibility: carry bounded Root/Coordinator funding requests and exact results.
//! Does not own: caller identity, policy decisions, persistence, accounting, or effects.
//! Boundary: transport callers supply no recipient; the Coordinator derives it from the caller.

use crate::{cdk::types::Cycles, dto::fleet_registry::FleetRegistryVersion};
use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Root-authored request for one exact Coordinator operating-cycle decision.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRootFundingRequest {
    pub operation_id: [u8; 32],
    pub operation_sequence: u64,
    pub expected_registry: FleetRegistryVersion,
    pub observed_balance: Cycles,
    pub requested_cycles: Cycles,
    pub policy_hash: [u8; 32],
}

/// Coordinator-authored exact same-Root acceptance request carrying cycles.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRootFundingAcceptanceRequest {
    pub operation_id: [u8; 32],
    pub operation_sequence: u64,
    pub expected_registry: FleetRegistryVersion,
    pub observed_balance: Cycles,
    pub granted_cycles: Cycles,
    pub policy_hash: [u8; 32],
}

/// Root-authored exact receipt for one fresh or replayed grant acceptance.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRootFundingAcceptanceReceipt {
    pub request: FleetRootFundingAcceptanceRequest,
    pub fleet_subnet_root: candid::Principal,
    pub coordinator: candid::Principal,
    pub accepted_at_ns: u64,
}

/// Terminal reason that one authenticated request transferred no cycles.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetRootFundingNoGrantReason {
    CooldownActive,
    CoordinatorReserveUnavailable,
    FleetWindowExhausted,
    FundingDisabled,
    InvalidRequest,
    PolicyMismatch,
    RegistryStale,
    RootIneligible,
    RootRejected,
    RootWindowExhausted,
}

/// Durable exact zero-transfer result for one Root operation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRootFundingNoGrantReceipt {
    pub request: FleetRootFundingRequest,
    pub reason: FleetRootFundingNoGrantReason,
    pub decided_at_ns: u64,
}

/// Exact terminal outcome returned to the authenticated requesting Root.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetRootFundingResponse {
    Granted(FleetRootFundingAcceptanceReceipt),
    NoGrant(FleetRootFundingNoGrantReceipt),
}
