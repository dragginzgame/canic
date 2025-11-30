use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    model::memory::sharding::{SHARDING_CORE, ShardEntry, ShardKey, ShardingCore},
};
use candid::Principal;

///
/// Sharding Registry
///
/// Persistent memory interface for tracking shard entries and tenant → shard
/// assignments. This layer is purely responsible for durable state and
/// consistency enforcement — not for selection, policy, or orchestration.
///

pub struct ShardingRegistry;

impl ShardingRegistry {
    // -----------------------------------------------------------------------
    // Core Access Helpers
    // -----------------------------------------------------------------------

    #[inline]
    pub(crate) fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&ShardingCore<VirtualMemory<DefaultMemoryImpl>>) -> R,
    {
        SHARDING_CORE.with_borrow(f)
    }

    #[inline]
    pub(crate) fn with_mut<F, R>(f: F) -> R
    where
        F: FnOnce(&mut ShardingCore<VirtualMemory<DefaultMemoryImpl>>) -> R,
    {
        SHARDING_CORE.with_borrow_mut(f)
    }

    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    /// Clears all shard and tenant assignments (for tests or full reset).
    pub fn clear() {
        Self::with_mut(ShardingCore::clear);
    }

    /// Returns the total number of shard entries.
    #[must_use]
    pub fn count() -> u64 {
        Self::with(|s| s.all_entries().len() as u64)
    }

    /// Removes a shard entry from the registry. The shard must be empty.
    pub fn remove(shard_pid: Principal) -> Option<ShardEntry> {
        Self::with_mut(|s| s.remove_entry(&shard_pid))
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Lookup the shard principal that backs a specific slot (if any).
    #[must_use]
    pub fn shard_for_slot(pool: &str, slot: u32) -> Option<Principal> {
        Self::with(|s| {
            s.all_entries()
                .into_iter()
                .find(|(_, entry)| {
                    entry.pool == pool && entry.has_assigned_slot() && entry.slot == slot
                })
                .map(|(pid, _)| pid)
        })
    }

    /// Lookup the slot index for a given shard principal.
    #[must_use]
    pub fn slot_for_shard(pool: &str, shard: Principal) -> Option<u32> {
        Self::with(|s| s.get_entry(&shard)).and_then(|entry| {
            if entry.pool == pool && entry.has_assigned_slot() {
                Some(entry.slot)
            } else {
                None
            }
        })
    }

    /// Returns the shard assigned to the given tenant (if any).
    #[must_use]
    pub fn tenant_shard(pool: &str, tenant: &str) -> Option<Principal> {
        Self::with(|s| s.get_assignment(&ShardKey::new(pool, tenant)))
    }

    /// Lists all tenants currently assigned to the specified shard.
    #[must_use]
    pub fn tenants_in_shard(pool: &str, shard: Principal) -> Vec<String> {
        Self::with(|s| {
            s.all_assignments()
                .into_iter()
                .filter(|(k, v)| v == &shard && k.pool.as_ref() == pool)
                .map(|(k, _)| k.tenant.to_string())
                .collect()
        })
    }

    /// Exports all shard entries (for inspection or snapshot purposes).
    #[must_use]
    pub fn export() -> Vec<(Principal, ShardEntry)> {
        Self::with(ShardingCore::all_entries)
    }
}
