use super::ShardingWorkflow;
use crate::{
    cdk::types::Principal,
    ops::{
        placement::sharding::mapper::ShardPlacementPolicyInputMapper,
        storage::{children::CanisterChildrenOps, placement::sharding::ShardingRegistryOps},
    },
    view::placement::sharding::ShardPlacement,
};
use std::collections::BTreeSet;

impl ShardingWorkflow {
    pub(super) fn pool_entry_views(pool: &str) -> Vec<(Principal, ShardPlacement)> {
        let direct_children = Self::direct_child_pid_set();
        ShardingRegistryOps::entries_for_pool(pool)
            .iter()
            .filter(|(pid, _)| direct_children.is_empty() || direct_children.contains(pid))
            .map(|(pid, entry)| {
                ShardPlacementPolicyInputMapper::record_to_policy_input(*pid, entry)
            })
            .collect()
    }

    pub(super) fn routable_active_set(active: &BTreeSet<Principal>) -> BTreeSet<Principal> {
        let direct_children = Self::direct_child_pid_set();
        if direct_children.is_empty() {
            return active.clone();
        }

        active.intersection(&direct_children).copied().collect()
    }

    fn direct_child_pid_set() -> BTreeSet<Principal> {
        CanisterChildrenOps::data()
            .entries
            .into_iter()
            .map(|(pid, _)| pid)
            .collect()
    }

    pub(super) fn free_slots(max_shards: u32, entries: &[(Principal, ShardPlacement)]) -> Vec<u32> {
        let mut occupied = BTreeSet::new();
        for (_, entry) in entries {
            if entry.slot != ShardPlacement::UNASSIGNED_SLOT {
                occupied.insert(entry.slot);
            }
        }

        (0..max_shards)
            .filter(|slot| !occupied.contains(slot))
            .collect()
    }
}
