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

    /// Returns the shard assigned to the given tenant (if any).
    #[must_use]
    pub(crate) fn tenant_shard(pool: &str, tenant: &str) -> Option<Principal> {
        let key = ShardKey::try_new(pool, tenant).ok()?;
        Self::with(|s| s.get_assignment(&key))
    }

    /// Lists all tenants currently assigned to the specified shard.
    #[must_use]
    pub(crate) fn tenants_in_shard(pool: &str, shard: Principal) -> Vec<String> {
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
    pub(crate) fn export() -> Vec<(Principal, ShardEntry)> {
        Self::with(ShardingCore::all_entries)
    }
}
