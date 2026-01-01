use crate::{
    domain::policy::placement::{
        scaling::ScalingWorkerPlanEntry, sharding::policy::ShardingPlanState,
    },
    dto::placement::{ShardEntryView, ShardingPlanStateView, WorkerEntryView},
    storage::memory::{scaling::WorkerEntry, sharding::ShardEntry},
};

pub struct PlacementMapper;

impl PlacementMapper {
    #[must_use]
    pub fn worker_entry_from_view(view: WorkerEntryView) -> WorkerEntry {
        WorkerEntry {
            pool: view.pool,
            canister_role: view.canister_role,
            created_at_secs: view.created_at_secs,
        }
    }

    #[must_use]
    pub fn worker_entry_to_view(entry: &WorkerEntry) -> WorkerEntryView {
        WorkerEntryView {
            pool: entry.pool.clone(),
            canister_role: entry.canister_role.clone(),
            created_at_secs: entry.created_at_secs,
        }
    }

    #[must_use]
    pub fn worker_plan_entry_to_view(entry: ScalingWorkerPlanEntry) -> WorkerEntryView {
        WorkerEntryView {
            pool: entry.pool,
            canister_role: entry.canister_role,
            created_at_secs: entry.created_at_secs,
        }
    }

    #[must_use]
    pub fn shard_entry_to_view(entry: &ShardEntry) -> ShardEntryView {
        ShardEntryView {
            slot: entry.slot,
            capacity: entry.capacity,
            count: entry.count,
            pool: entry.pool.clone(),
            canister_role: entry.canister_role.clone(),
            created_at: entry.created_at,
        }
    }

    #[must_use]
    pub fn sharding_plan_state_to_view(state: ShardingPlanState) -> ShardingPlanStateView {
        match state {
            ShardingPlanState::AlreadyAssigned { pid } => {
                ShardingPlanStateView::AlreadyAssigned { pid }
            }
            ShardingPlanState::UseExisting { pid } => ShardingPlanStateView::UseExisting { pid },
            ShardingPlanState::CreateAllowed => ShardingPlanStateView::CreateAllowed,
            ShardingPlanState::CreateBlocked { reason } => ShardingPlanStateView::CreateBlocked {
                reason: reason.to_string(),
            },
        }
    }
}
