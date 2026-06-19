//! Module: view::placement::scaling
//!
//! Responsibility: define scaling placement read-only projections.
//! Does not own: scaling policy, worker records, or endpoint DTOs.
//! Boundary: ops mappers produce these views for scaling workflows.

use crate::{cdk::types::BoundedString64, ids::CanisterRole};

///
/// ScalingWorkerPlanEntry
///
/// Read-only projection of one scaling worker plan entry.
/// Owned by view and produced by scaling placement ops mappers.
///

#[derive(Clone, Debug)]
pub struct ScalingWorkerPlanEntry {
    pub pool: BoundedString64,
    pub canister_role: CanisterRole,
}
