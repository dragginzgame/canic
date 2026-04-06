mod backfill;
mod hrw;
mod metrics;

pub use crate::view::placement::sharding::{CreateBlockedReason, ShardingPlanState};
pub use hrw::HrwSelector;
pub use metrics::{PoolMetrics, compute_pool_metrics};

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::candid::Principal,
    config::schema::{ShardPool, ShardPoolPolicy},
    view::placement::sharding::{ShardPartitionKeyAssignment, ShardPlacement},
};
use backfill::plan_slot_backfill;

#[derive(Debug, thiserror::Error)]
pub enum ShardingPolicyError {
    #[error(
        "unknown shard pool '{requested}': not found in current canister sharding config; configured pools: {available}"
    )]
    PoolNotFound {
        requested: String,
        available: String,
    },

    #[error(
        "shard creation blocked: partition_key '{partition_key}' assignment blocked: {reason} in pool '{pool}'"
    )]
    ShardCreationBlocked {
        reason: CreateBlockedReason,
        partition_key: String,
        pool: String,
    },

    #[error("sharding disabled")]
    ShardingDisabled,
}

impl From<ShardingPolicyError> for InternalError {
    fn from(err: ShardingPolicyError) -> Self {
        Self::domain(InternalErrorOrigin::Domain, err.to_string())
    }
}

pub struct ShardingState<'a> {
    pub pool: &'a str,
    pub config: ShardPool,
    pub metrics: &'a PoolMetrics,
    pub entries: &'a [(Principal, ShardPlacement)],
    pub assignments: &'a [ShardPartitionKeyAssignment],
}

#[derive(Clone, Debug)]
pub struct ShardingPlan {
    pub state: ShardingPlanState,
    pub target_slot: Option<u32>,
}

pub struct ShardingPolicy;

impl ShardingPolicy {
    #[must_use]
    pub const fn can_create(metrics: PoolMetrics, policy: &ShardPoolPolicy) -> bool {
        metrics.active_count < policy.max_shards
    }

    #[must_use]
    pub(crate) fn lookup_partition_key(
        partition_key: &str,
        assignments: &[ShardPartitionKeyAssignment],
    ) -> Option<Principal> {
        assignments
            .iter()
            .find(|assignment| assignment.partition_key == partition_key)
            .map(|assignment| assignment.pid)
    }

    pub(crate) fn plan_assign(
        state: &ShardingState,
        partition_key: &str,
        exclude_pid: Option<Principal>,
    ) -> ShardingPlan {
        let pool_cfg = state.config.clone();
        let metrics = state.metrics;
        let entries = state.entries;

        let slot_plan = plan_slot_backfill(state.pool, entries, pool_cfg.policy.max_shards);

        if let Some(pid) = Self::lookup_partition_key(partition_key, state.assignments)
            .filter(|pid| exclude_pid != Some(*pid))
        {
            let slot = slot_plan.slots.get(&pid).copied();
            return Self::make_plan(ShardingPlanState::AlreadyAssigned { pid }, *metrics, slot);
        }

        let shards_with_capacity: Vec<_> = entries
            .iter()
            .filter(|(pid, entry)| {
                entry.pool.as_str() == state.pool
                    && entry_has_capacity(entry)
                    && exclude_pid != Some(*pid)
            })
            .map(|(pid, _)| *pid)
            .collect();

        if let Some(target_pid) = HrwSelector::select(partition_key, &shards_with_capacity) {
            let slot = slot_plan.slots.get(&target_pid).copied();
            return Self::make_plan(
                ShardingPlanState::UseExisting { pid: target_pid },
                *metrics,
                slot,
            );
        }

        let max_slots = pool_cfg.policy.max_shards;
        let free_slots: Vec<u32> = (0..max_slots)
            .filter(|slot| !slot_plan.occupied.contains(slot))
            .collect();

        let Some(target_slot) =
            HrwSelector::select_from_slots(state.pool, partition_key, &free_slots)
        else {
            return Self::make_plan(
                ShardingPlanState::CreateBlocked {
                    reason: CreateBlockedReason::NoFreeSlots,
                },
                *metrics,
                None,
            );
        };

        if Self::can_create(*metrics, &pool_cfg.policy) {
            Self::make_plan(
                ShardingPlanState::CreateAllowed,
                *metrics,
                Some(target_slot),
            )
        } else {
            Self::make_plan(
                ShardingPlanState::CreateBlocked {
                    reason: CreateBlockedReason::PoolAtCapacity,
                },
                *metrics,
                Some(target_slot),
            )
        }
    }

    const fn make_plan(
        state: ShardingPlanState,
        _metrics: PoolMetrics,
        slot: Option<u32>,
    ) -> ShardingPlan {
        ShardingPlan {
            state,
            target_slot: slot,
        }
    }
}

const fn entry_has_capacity(entry: &ShardPlacement) -> bool {
    entry.count < entry.capacity
}
