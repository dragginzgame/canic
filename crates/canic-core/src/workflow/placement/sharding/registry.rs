//! Module: workflow::placement::sharding::registry
//!
//! Responsibility: project sharding registry state into policy input views.
//! Does not own: storage mutation, policy evaluation, or endpoint DTOs.
//! Boundary: filters storage records to routable child shard views.

use crate::{
    InternalError,
    cdk::types::Principal,
    model::placement::sharding::{ShardPartitionKeyAssignment, ShardPlacement},
    ops::{
        placement::sharding::mapper::{ShardPartitionKeyAssignmentMapper, ShardPlacementMapper},
        storage::placement::sharding::ShardingRegistryOps,
    },
    workflow::placement::sharding::ShardingWorkflow,
};
use std::collections::BTreeSet;

impl ShardingWorkflow {
    pub(super) fn pool_entry_views(pool: &str) -> Vec<(Principal, ShardPlacement)> {
        ShardingRegistryOps::entries_for_pool(pool)
            .iter()
            .filter(|record| ShardingRegistryOps::entry_is_current(record))
            .map(|record| ShardPlacementMapper::record_to_observation(record.pid, &record.entry))
            .collect()
    }

    pub(super) fn routable_active_set(active: &BTreeSet<Principal>) -> BTreeSet<Principal> {
        ShardingRegistryOps::registry_data()
            .entries
            .into_iter()
            .filter(|record| {
                active.contains(&record.pid) && ShardingRegistryOps::entry_is_current(record)
            })
            .map(|record| record.pid)
            .collect()
    }

    // Existing ownership survives temporary unavailability and must never look like a free key.
    pub(super) fn assignment_view(
        pool: &str,
        partition_key: &str,
        routable_active: &BTreeSet<Principal>,
    ) -> Result<Option<ShardPartitionKeyAssignment>, InternalError> {
        ShardingRegistryOps::assignment_for_key(pool, partition_key)
            .map(|record| {
                if !ShardingRegistryOps::assignment_is_current(&record) {
                    return Err(InternalError::public(
                        crate::diagnostics::codes::POSITION_UNAVAILABLE,
                    ));
                }
                Self::require_routable_shard(record.shard, routable_active)?;
                Ok(ShardPartitionKeyAssignmentMapper::record_to_assignment(
                    &record.key,
                    record.shard,
                ))
            })
            .transpose()
    }

    // Recheck after an allocation await before assigning or returning another caller's mapping.
    pub(super) fn assign_available_key(
        pool: &str,
        partition_key: &str,
        candidate: Principal,
    ) -> Result<Principal, InternalError> {
        let active = ShardingRegistryOps::active_shards().into_iter().collect();
        let routable = Self::routable_active_set(&active);
        if let Some(assignment) = Self::assignment_view(pool, partition_key, &routable)? {
            return Ok(assignment.pid);
        }
        Self::require_routable_shard(candidate, &routable)?;
        ShardingRegistryOps::assign(pool, partition_key, candidate)
    }

    fn require_routable_shard(
        shard: Principal,
        routable_active: &BTreeSet<Principal>,
    ) -> Result<(), InternalError> {
        if routable_active.contains(&shard) {
            Ok(())
        } else {
            Err(InternalError::public(
                crate::diagnostics::codes::POSITION_UNAVAILABLE,
            ))
        }
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
