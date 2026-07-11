use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    storage::{
        prelude::*,
        stable::sharding::{
            SHARDING_CORE, ShardKey, ShardingAssignmentRecord, ShardingAssignmentsData,
            ShardingCore, ShardingRegistryData, ShardingRegistryEntryRecord,
        },
    },
};

///
/// ShardingRegistry
///
/// Persistent memory interface for tracking shard entries and partition_key → shard
/// assignments. This layer is purely responsible for durable state and
/// consistency enforcement — not for selection, policy, or orchestration.
///

pub struct ShardingRegistry;

impl ShardingRegistry {
    // -----------------------------------------------------------------------
    // Core Access Helpers
    // -----------------------------------------------------------------------

    pub(crate) fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&ShardingCore<VirtualMemory<DefaultMemoryImpl>>) -> R,
    {
        SHARDING_CORE.with_borrow(f)
    }

    pub(crate) fn with_mut<F, R>(f: F) -> R
    where
        F: FnOnce(&mut ShardingCore<VirtualMemory<DefaultMemoryImpl>>) -> R,
    {
        SHARDING_CORE.with_borrow_mut(f)
    }

    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    #[cfg(test)]
    pub(crate) fn clear() {
        Self::with_mut(|core| {
            core.registry.clear_new();
            core.assignments.clear_new();
        });
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Lookup the slot index for a given shard principal.
    #[must_use]
    pub(crate) fn slot_for_shard(pool: &str, shard: Principal) -> Option<u32> {
        Self::with(|s| s.get_entry(&shard)).and_then(|entry| {
            if entry.pool.as_ref() == pool && entry.has_assigned_slot() {
                Some(entry.slot)
            } else {
                None
            }
        })
    }

    /// Returns the shard assigned to the given partition_key (if any).
    #[must_use]
    pub(crate) fn partition_key_shard(pool: &str, partition_key: &str) -> Option<Principal> {
        let key = ShardKey::try_new(pool, partition_key).ok()?;
        Self::with(|s| s.get_assignment(&key))
    }

    /// Lists all partition_keys currently assigned to the specified shard.
    #[must_use]
    pub(crate) fn partition_keys_in_shard(pool: &str, shard: Principal) -> Vec<String> {
        Self::export_assignments()
            .entries
            .into_iter()
            .filter(|record| record.shard == shard && record.key.pool.as_ref() == pool)
            .map(|record| record.key.partition_key.to_string())
            .collect()
    }

    /// Returns all shard entries registered for one pool.
    #[must_use]
    pub(crate) fn entries_for_pool(pool: &str) -> Vec<ShardingRegistryEntryRecord> {
        Self::export_registry()
            .entries
            .into_iter()
            .filter(|record| record.entry.pool.as_ref() == pool)
            .collect()
    }

    /// Returns all assignments registered for one pool.
    #[must_use]
    pub(crate) fn assignments_for_pool(pool: &str) -> Vec<ShardingAssignmentRecord> {
        Self::export_assignments()
            .entries
            .into_iter()
            .filter(|record| record.key.pool.as_ref() == pool)
            .collect()
    }

    /// Exports all shard entries (structural data only).
    ///
    /// NOTE:
    /// - Assignments are intentionally excluded.
    /// - Partition key → shard mappings are unbounded and must be queried explicitly.
    #[must_use]
    pub(crate) fn export_registry() -> ShardingRegistryData {
        ShardingRegistryData {
            entries: Self::with(ShardingCore::all_entries),
        }
    }

    /// Export all partition-key assignments.
    #[must_use]
    pub(crate) fn export_assignments() -> ShardingAssignmentsData {
        ShardingAssignmentsData {
            entries: Self::with(ShardingCore::all_assignments),
        }
    }
}
