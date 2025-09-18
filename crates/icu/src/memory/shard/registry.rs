use crate::{
    Error,
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    memory::{
        MemoryError,
        shard::{SHARD_CORE, ShardCore, ShardEntry, ShardKey, ShardRegistryError},
    },
    types::CanisterType,
    utils::time::now_secs,
};
use candid::Principal;

///
/// ShardRegistry
///

pub type ShardRegistryView = Vec<(Principal, ShardEntry)>;

pub struct ShardRegistry;

impl ShardRegistry {
    // Helpers to access the core
    pub(crate) fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&ShardCore<VirtualMemory<DefaultMemoryImpl>>) -> R,
    {
        SHARD_CORE.with_borrow(f)
    }

    pub(crate) fn with_mut<F, R>(f: F) -> R
    where
        F: FnOnce(&mut ShardCore<VirtualMemory<DefaultMemoryImpl>>) -> R,
    {
        SHARD_CORE.with_borrow_mut(f)
    }

    // ------------------------
    // Lifecycle
    // ------------------------

    pub fn clear() {
        Self::with_mut(|s| s.clear());
    }

    #[must_use]
    pub fn count() -> u64 {
        Self::with(|s| s.all_entries().len() as u64)
    }

    /// Create a shard entry
    pub fn create(shard_pid: Principal, pool: &str, ty: &CanisterType, capacity: u32) {
        let entry = ShardEntry {
            canister_type: ty.clone(),
            capacity,
            count: 0,
            created_at_secs: now_secs(),
            pool: pool.to_string(),
        };

        Self::with_mut(|s| s.insert_entry(shard_pid, entry));
    }

    /// Remove shard (must be empty)
    pub fn remove(shard_pid: Principal) -> Result<(), Error> {
        let entry = Self::with(|s| s.get_entry(&shard_pid));
        if let Some(e) = entry
            && e.count > 0
        {
            Err(MemoryError::from(ShardRegistryError::ShardFull(shard_pid)))?;
        }

        Self::with_mut(|s| s.remove_entry(&shard_pid)).map_err(MemoryError::from)?;

        Ok(())
    }

    // ------------------------
    // Assignments
    // ------------------------

    pub fn assign(pool: &str, tenant: Principal, shard: Principal) -> Result<(), Error> {
        // check shard exists + capacity
        let mut entry = Self::with(|s| s.get_entry(&shard))
            .ok_or(MemoryError::from(ShardRegistryError::ShardNotFound(shard)))?;

        if entry.pool != pool {
            Err(MemoryError::from(ShardRegistryError::ShardNotFound(shard)))?;
        }
        if entry.count >= entry.capacity {
            Err(MemoryError::from(ShardRegistryError::ShardFull(shard)))?;
        }

        // replace existing assignment if different
        if let Some(current) = Self::with(|s| s.get_assignment(&ShardKey::new(pool, tenant))) {
            if current != shard {
                Self::release(pool, tenant)?;
            } else {
                return Ok(()); // already assigned
            }
        }

        Self::with_mut(|s| s.insert_assignment(ShardKey::new(pool, tenant), shard));
        entry.count = entry.count.saturating_add(1);
        Self::with_mut(|s| s.insert_entry(shard, entry));

        Ok(())
    }

    /// Assign tenant directly to a shard (no checks beyond core).
    pub fn assign_direct(pool: &str, tenant: Principal, shard: Principal) -> Result<(), Error> {
        Self::assign(pool, tenant, shard)
    }

    /// Try to assign tenant to any available shard in the pool.
    #[must_use]
    pub fn assign_best_effort(pool: &str, tenant: Principal) -> Option<Principal> {
        // if already assigned and shard is usable, return it
        if let Some(pid) = Self::tenant_shard(pool, tenant)
            && Self::export()
                .iter()
                .any(|(p, e)| *p == pid && e.pool == pool && e.count < e.capacity)
        {
            return Some(pid);
        }

        // find least-loaded candidate shard
        let candidate = Self::peek_best_effort(pool)?;
        Self::assign(pool, tenant, candidate).ok()?;

        Some(candidate)
    }

    pub fn release(pool: &str, tenant: Principal) -> Result<(), Error> {
        let key = ShardKey::new(pool, tenant);
        let shard = Self::with_mut(|s| s.remove_assignment(&key)).map_err(MemoryError::from)?;

        // decrement count if shard still exists
        if let Some(mut entry) = Self::with(|s| s.get_entry(&shard)) {
            entry.count = entry.count.saturating_sub(1);
            Self::with_mut(|s| s.insert_entry(shard, entry));
        }

        Ok(())
    }

    // ------------------------
    // Queries
    // ------------------------

    /// Look at the least loaded shard in a pool (without assignment).
    #[must_use]
    pub fn peek_best_effort(pool: &str) -> Option<Principal> {
        Self::export()
            .into_iter()
            .filter(|(_, e)| e.pool == pool && e.count < e.capacity)
            .min_by_key(|(_, e)| (e.load_bps().unwrap_or(u64::MAX), e.count, e.created_at_secs))
            .map(|(pid, _)| pid)
    }

    #[must_use]
    pub fn tenant_shard(pool: &str, tenant: Principal) -> Option<Principal> {
        Self::with(|s| s.get_assignment(&ShardKey::new(pool, tenant)))
    }

    #[must_use]
    pub fn tenants_in_shard(pool: &str, shard: Principal) -> Vec<Principal> {
        Self::with(|s| {
            s.all_assignments()
                .into_iter()
                .filter(|(k, v)| v == &shard && k.pool == pool)
                .map(|(k, _)| k.tenant_pid)
                .collect()
        })
    }

    #[must_use]
    pub fn export() -> ShardRegistryView {
        Self::with(|s| s.all_entries())
    }
}
