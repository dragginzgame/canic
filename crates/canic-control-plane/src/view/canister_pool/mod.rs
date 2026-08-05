//! Module: view::canister_pool
//!
//! Responsibility: expose read-only projections of durable Canister pool work.
//! Does not own: stable records, endpoint DTOs, or workflow mutation.
//! Boundary: ops projects storage records here before workflow orchestration.

use canic_core::cdk::types::Principal;

/// Read-only authority for one in-progress draining-root asset handoff.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanisterPoolHandoffView {
    pub canister_id: Principal,
    pub recipient: Principal,
}
