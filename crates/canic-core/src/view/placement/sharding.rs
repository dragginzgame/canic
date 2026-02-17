use crate::{cdk::candid::Principal, ids::CanisterRole};

///
/// ShardPlacement
///

#[derive(Clone, Debug)]
pub struct ShardPlacement {
    pub pool: String,
    pub slot: u32,
    pub capacity: u32,
    pub count: u32,
    pub role: CanisterRole,
    pub created_at: u64,
}

impl ShardPlacement {
    pub const UNASSIGNED_SLOT: u32 = u32::MAX;
}

///
/// ShardPartitionKeyAssignment
///

#[derive(Clone, Debug)]
pub struct ShardPartitionKeyAssignment {
    pub partition_key: String,
    pub pid: Principal,
}

///
/// ShardingPlanState
/// Outcome variants of a shard plan.
///

#[derive(Clone, Debug)]
pub enum ShardingPlanState {
    AlreadyAssigned { pid: Principal },
    UseExisting { pid: Principal },
    CreateAllowed,
    CreateBlocked { reason: CreateBlockedReason },
}

///
/// CreateBlockedReason
///

#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum CreateBlockedReason {
    #[error("pool at capacity")]
    PoolAtCapacity,

    #[error("no free shard slots")]
    NoFreeSlots,

    #[error("{0}")]
    PolicyViolation(String),
}
