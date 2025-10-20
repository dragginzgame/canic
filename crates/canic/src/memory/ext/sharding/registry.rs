use crate::{
    Error,
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    memory::ext::sharding::{SHARDING_CORE, ShardEntry, ShardKey, ShardingCore, ShardingError},
    types::CanisterType,
    utils::time::now_secs,
};
use candid::Principal;

/// ---------------------------------------------------------------------------
/// Sharding Registry
///
/// Persistent memory interface for tracking shard entries and tenant → shard
/// assignments. This layer is purely responsible for durable state and
/// consistency enforcement — not for selection, policy, or orchestration.
/// ---------------------------------------------------------------------------
pub struct ShardingRegistry;

/// Public snapshot view type.
pub type ShardingRegistryView = Vec<(Principal, ShardEntry)>;

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

    /// Creates a new shard entry with the specified capacity and type.
    pub fn create(shard_pid: Principal, pool: &str, ty: &CanisterType, capacity: u32) {
        let entry = ShardEntry {
            canister_type: ty.clone(),
            capacity,
            count: 0,
            pool: pool.to_string(),
            created_at: now_secs(),
        };

        Self::with_mut(|s| s.insert_entry(shard_pid, entry));
    }

    /// Removes a shard entry from the registry. The shard must be empty.
    pub fn remove(shard_pid: Principal) -> Result<(), Error> {
        let entry = Self::with(|s| s.get_entry(&shard_pid));

        if let Some(e) = entry
            && e.count > 0
        {
            Err(ShardingError::ShardFull(shard_pid))?;
        }

        Self::with_mut(|s| s.remove_entry(&shard_pid))?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Assignments
    // -----------------------------------------------------------------------

    /// Assigns a tenant to a specific shard.
    ///
    /// This is a low-level operation; capacity and pool validity are checked,
    /// but no balancing or policy logic is applied here.
    pub fn assign<P, T>(pool: P, tenant: T, shard: Principal) -> Result<(), Error>
    where
        P: AsRef<str>,
        T: AsRef<str>,
    {
        let pool = pool.as_ref();
        let tenant = tenant.as_ref();

        // Ensure shard exists and has available capacity
        let mut entry =
            Self::with(|s| s.get_entry(&shard)).ok_or(ShardingError::ShardNotFound(shard))?;

        if entry.pool != pool {
            Err(ShardingError::ShardNotFound(shard))?;
        }

        if entry.count >= entry.capacity {
            Err(ShardingError::ShardFull(shard))?;
        }

        // If tenant is already assigned, replace only if different
        if let Some(current) = Self::with(|s| s.get_assignment(&ShardKey::new(pool, tenant))) {
            if current == shard {
                return Ok(()); // no-op
            }
            Self::release(pool, tenant)?; // clean old assignment
        }

        // Insert assignment and update shard load
        Self::with_mut(|s| s.insert_assignment(ShardKey::new(pool, tenant), shard));
        entry.count = entry.count.saturating_add(1);
        Self::with_mut(|s| s.insert_entry(shard, entry));

        Ok(())
    }

    /// Releases a tenant from its assigned shard and decrements the count.
    pub fn release(pool: &str, tenant: &str) -> Result<(), Error> {
        let key = ShardKey::new(pool, tenant);
        let shard = Self::with_mut(|s| s.remove_assignment(&key))?;

        // Decrement shard count if still present
        if let Some(mut entry) = Self::with(|s| s.get_entry(&shard)) {
            entry.count = entry.count.saturating_sub(1);
            Self::with_mut(|s| s.insert_entry(shard, entry));
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Lists all active shard Principals for the specified pool.
    ///
    /// A shard is considered *active* if it exists in the registry and has
    /// a nonzero capacity (even if currently empty). This does not filter
    /// by load, only by membership.
    #[must_use]
    pub fn list_active_shards(pool: &str) -> Vec<Principal> {
        Self::with(|s| {
            s.all_entries()
                .into_iter()
                .filter(|(_, entry)| entry.pool == pool)
                .map(|(pid, _)| pid)
                .collect()
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
    pub fn export() -> ShardingRegistryView {
        Self::with(ShardingCore::all_entries)
    }
}
