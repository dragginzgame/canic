use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    storage::{
        prelude::*,
        stable::sharding::{SHARDING_CORE, ShardEntryRecord, ShardKey, ShardingCore},
    },
};

///
/// ShardingRegistryRecord
///

#[derive(Clone, Debug)]
pub struct ShardingRegistryRecord {
    pub entries: Vec<(Principal, ShardEntryRecord)>,
}

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
            core.registry.clear();
            core.assignments.clear();
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
        Self::with(|s| {
            s.all_assignments()
                .into_iter()
                .filter(|(k, v)| v == &shard && k.pool.as_ref() == pool)
                .map(|(k, _)| k.partition_key.to_string())
                .collect()
        })
    }

    /// Exports all shard entries (structural data only).
    ///
    /// NOTE:
    /// - Assignments are intentionally excluded.
    /// - Partition key → shard mappings are unbounded and must be queried explicitly.
    #[must_use]
    pub(crate) fn export() -> ShardingRegistryRecord {
        ShardingRegistryRecord {
            entries: Self::with(ShardingCore::all_entries),
        }
    }
}
