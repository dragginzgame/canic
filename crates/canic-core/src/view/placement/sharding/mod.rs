//! Module: view::placement::sharding
//!
//! Responsibility: define sharding placement read-only projections.
//! Does not own: sharding policy, shard records, or endpoint DTOs.
//! Boundary: ops mappers produce these views for sharding workflows.

use crate::cdk::candid::Principal;

///
/// ShardPlacement
///
/// Read-only projection of one shard placement slot.
/// Owned by view and produced by sharding placement ops mappers.
///

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

///
/// ShardPartitionKeyAssignment
///
/// Read-only projection of one partition key assignment.
/// Owned by view and produced by sharding placement ops mappers.
///

#[derive(Clone, Debug)]
pub struct ShardPartitionKeyAssignment {
    pub partition_key: String,
    pub pid: Principal,
}

///
/// ShardingPlanState
///
/// Outcome variants of a shard plan.
/// Owned by view and consumed by sharding workflow orchestration.
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
/// Reason a shard creation plan cannot proceed.
/// Owned by view and surfaced through sharding plan state.
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
