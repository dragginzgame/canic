//! Module: model::placement::scaling
//!
//! Responsibility: own canonical scaling plan values shared across layers.
//! Does not own: scaling policy evaluation, registry storage, or worker creation.

use crate::{domain::value::BoundedString64, ids::CanisterRole};

/// Worker registry values admitted by a scaling or bootstrap plan.
#[derive(Clone, Debug)]
pub struct ScalingWorkerEntry {
    pub pool: BoundedString64,
    pub canister_role: CanisterRole,
}

/// Stable reason categories emitted by scaling plans.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalingPlanReason {
    AtMaxWorkers,
    BelowMinWorkers,
    WithinBounds,
}
