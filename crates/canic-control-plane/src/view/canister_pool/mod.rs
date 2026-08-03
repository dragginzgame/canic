//! Module: view::canister_pool
//!
//! Responsibility: expose read-only projections of durable Canister pool work.
//! Does not own: stable records, endpoint DTOs, or workflow mutation.
//! Boundary: ops projects storage records here before workflow orchestration.

use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::model::replay::ReplayCostGuardSettlement,
};

/// Read-only progress for one paid empty-Canister creation effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanisterPoolCreationView {
    pub operation_id: [u8; 32],
    pub canister_cycles: Cycles,
    pub cost_guard_settlement: ReplayCostGuardSettlement,
    pub canister_id: Option<Principal>,
}

/// Read-only authority for one in-progress draining-root asset handoff.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanisterPoolHandoffView {
    pub canister_id: Principal,
    pub recipient: Principal,
}
