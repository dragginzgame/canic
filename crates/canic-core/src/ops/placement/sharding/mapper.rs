//! Module: ops::placement::sharding::mapper
//!
//! Responsibility: convert sharding records and plan states into policy/view shapes.
//! Does not own: sharding policy, registry mutation, or endpoint DTO schemas.
//! Boundary: ops mapper used by sharding workflows and storage facades.

use crate::{
    cdk::types::Principal,
    dto::placement::sharding::{ShardEntry, ShardingPlanStateResponse},
    storage::stable::sharding::{ShardEntryRecord, ShardKey},
    view::placement::sharding::{ShardPartitionKeyAssignment, ShardPlacement, ShardingPlanState},
};

///
/// ShardPlacementPolicyInputMapper
///
/// Operations-layer mapper for shard entries and placement policy inputs.
///

pub struct ShardPlacementPolicyInputMapper;

impl ShardPlacementPolicyInputMapper {
    #[must_use]
    pub fn record_to_policy_input(
        pid: Principal,
        entry: &ShardEntryRecord,
    ) -> (Principal, ShardPlacement) {
        (
            pid,
            ShardPlacement {
                pool: entry.pool.to_string(),
                slot: entry.slot,
                capacity: entry.capacity,
                count: entry.count,
            },
        )
    }
}

///
/// ShardPartitionKeyAssignmentPolicyInputMapper
///
/// Operations-layer mapper for shard assignment records and policy inputs.
///

pub struct ShardPartitionKeyAssignmentPolicyInputMapper;

impl ShardPartitionKeyAssignmentPolicyInputMapper {
    #[must_use]
    pub fn record_to_policy_input(key: &ShardKey, pid: Principal) -> ShardPartitionKeyAssignment {
        ShardPartitionKeyAssignment {
            partition_key: key.partition_key.to_string(),
            pid,
        }
    }
}

///
/// ShardEntryMapper
///
/// Operations-layer mapper for shard records and public views.
///

pub struct ShardEntryMapper;

impl ShardEntryMapper {
    #[must_use]
    pub fn record_to_view(entry: &ShardEntryRecord) -> ShardEntry {
        ShardEntry {
            slot: entry.slot,
            capacity: entry.capacity,
            count: entry.count,
            pool: entry.pool.to_string(),
            canister_role: entry.canister_role.clone(),
            created_at: entry.created_at,
        }
    }
}

///
/// ShardingPlanStateResponseMapper
///
/// Operations-layer mapper for sharding plan states and response views.
///

pub struct ShardingPlanStateResponseMapper;

impl ShardingPlanStateResponseMapper {
    #[must_use]
    pub fn record_to_view(state: ShardingPlanState) -> ShardingPlanStateResponse {
        match state {
            ShardingPlanState::AlreadyAssigned { pid } => {
                ShardingPlanStateResponse::AlreadyAssigned { pid }
            }
            ShardingPlanState::UseExisting { pid } => {
                ShardingPlanStateResponse::UseExisting { pid }
            }
            ShardingPlanState::CreateAllowed => ShardingPlanStateResponse::CreateAllowed,
            ShardingPlanState::CreateBlocked { reason } => {
                ShardingPlanStateResponse::CreateBlocked {
                    reason: reason.to_string(),
                }
            }
        }
    }
}
