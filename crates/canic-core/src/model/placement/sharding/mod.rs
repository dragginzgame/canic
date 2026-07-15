//! Module: model::placement::sharding
//!
//! Responsibility: own canonical sharding observations and plan values shared across layers.
//! Does not own: sharding policy evaluation, registry storage, or shard creation.

use crate::domain::value::Principal;

/// One observed shard placement.
#[derive(Clone, Debug)]
pub struct ShardPlacement {
    pub pool: String,
    pub slot: u32,
    pub capacity: u32,
    pub count: u32,
}

impl ShardPlacement {
    pub const UNASSIGNED_SLOT: u32 = u32::MAX;
}

/// One observed partition-key assignment.
#[derive(Clone, Debug)]
pub struct ShardPartitionKeyAssignment {
    pub partition_key: String,
    pub pid: Principal,
}

/// Result of planning one sharding assignment.
#[derive(Clone, Debug)]
pub enum ShardingPlanState {
    AlreadyAssigned { pid: Principal },
    UseExisting { pid: Principal },
    CreateAllowed,
    CreateBlocked { reason: CreateBlockedReason },
}

/// Typed reason that shard creation was denied.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum CreateBlockedReason {
    #[error("pool at capacity")]
    PoolAtCapacity,

    #[error("no free shard slots")]
    NoFreeSlots,

    #[error("{0}")]
    PolicyViolation(String),
}
