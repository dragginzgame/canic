use crate::{
    domain::policy::placement::sharding::ShardingPlanState,
    dto::placement::sharding::{ShardEntryView, ShardingPlanStateView},
    ops::storage::placement::sharding::ShardEntry,
};

///
/// ShardingMapper
///

pub struct ShardingMapper;

impl ShardingMapper {
    #[must_use]
    pub fn shard_entry_to_view(entry: &ShardEntry) -> ShardEntryView {
        ShardEntryView {
            slot: entry.slot,
            capacity: entry.capacity,
            count: entry.count,
            pool: entry.pool.to_string(),
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
