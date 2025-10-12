use crate::{
    Error,
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    memory::ext::sharding::{SHARDING_CORE, ShardEntry, ShardKey, ShardingCore, ShardingError},
    types::CanisterType,
    utils::time::now_secs,
};
use candid::Principal;

///
/// ShardingRegistry
///

pub struct ShardingRegistry;

pub type ShardingRegistryView = Vec<(Principal, ShardEntry)>;

impl ShardingRegistry {
    // Helpers to access the core
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

    // ------------------------
    // Lifecycle
    // ------------------------

    pub fn clear() {
        Self::with_mut(ShardingCore::clear);
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
            Err(ShardingError::ShardFull(shard_pid))?;
        }

        Self::with_mut(|s| s.remove_entry(&shard_pid))?;

        Ok(())
    }

    // ------------------------
    // Assignments
    // ------------------------

    pub fn assign(pool: &str, tenant: Principal, shard: Principal) -> Result<(), Error> {
        // check shard exists + capacity
        let mut entry =
            Self::with(|s| s.get_entry(&shard)).ok_or(ShardingError::ShardNotFound(shard))?;

        if entry.pool != pool {
            Err(ShardingError::ShardNotFound(shard))?;
        }
        if entry.count >= entry.capacity {
            Err(ShardingError::ShardFull(shard))?;
        }

        // replace existing assignment if different
        if let Some(current) = Self::with(|s| s.get_assignment(&ShardKey::new(pool, tenant))) {
            if current == shard {
                return Ok(()); // already assigned
            }

            Self::release(pool, tenant)?;
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
        Self::assign_best_effort_internal(pool, tenant, None)
    }

    /// Try to assign tenant to any available shard, excluding a specific shard.
    #[must_use]
    pub fn assign_best_effort_excluding(
        pool: &str,
        tenant: Principal,
        exclude: Principal,
    ) -> Option<Principal> {
        Self::assign_best_effort_internal(pool, tenant, Some(exclude))
    }

    pub fn release(pool: &str, tenant: Principal) -> Result<(), Error> {
        let key = ShardKey::new(pool, tenant);
        let shard = Self::with_mut(|s| s.remove_assignment(&key))?;

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
        let snapshot = Self::export();
        Self::candidate_order(&snapshot, pool, None)
            .into_iter()
            .next()
    }

    #[must_use]
    pub fn peek_best_effort_excluding(pool: &str, exclude: Principal) -> Option<Principal> {
        let snapshot = Self::export();
        Self::candidate_order(&snapshot, pool, Some(&exclude))
            .into_iter()
            .next()
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
    pub fn export() -> ShardingRegistryView {
        Self::with(ShardingCore::all_entries)
    }

    fn assign_best_effort_internal(
        pool: &str,
        tenant: Principal,
        exclude: Option<Principal>,
    ) -> Option<Principal> {
        let snapshot = Self::export();
        let exclude_ref = exclude.as_ref();

        if let Some(pid) = Self::tenant_shard(pool, tenant)
            && exclude_ref.is_none_or(|ex| *ex != pid)
            && snapshot
                .iter()
                .any(|(p, e)| *p == pid && e.pool == pool && e.count < e.capacity)
        {
            return Some(pid);
        }

        let candidates = Self::candidate_order(&snapshot, pool, exclude_ref);

        candidates
            .into_iter()
            .find(|&candidate| Self::assign(pool, tenant, candidate).is_ok())
    }

    fn candidate_order(
        snapshot: &[(Principal, ShardEntry)],
        pool: &str,
        exclude: Option<&Principal>,
    ) -> Vec<Principal> {
        let mut candidates: Vec<(Principal, (u64, u32, u64))> = snapshot
            .iter()
            .filter_map(|(pid, entry)| {
                if entry.pool == pool && entry.count < entry.capacity && (exclude != Some(pid)) {
                    Some((
                        *pid,
                        (
                            entry.load_bps().unwrap_or(u64::MAX),
                            entry.count,
                            entry.created_at_secs,
                        ),
                    ))
                } else {
                    None
                }
            })
            .collect();

        candidates.sort_by_key(|(_, key)| *key);

        candidates.into_iter().map(|(pid, _)| pid).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    const POOL: &str = "pool";

    #[test]
    fn assign_best_effort_excluding_skips_donor() {
        ShardingRegistry::clear();

        let donor = p(1);
        let other = p(2);
        let tenant = p(99);

        ShardingRegistry::create(donor, POOL, &CanisterType::new("donor"), 2);
        ShardingRegistry::create(other, POOL, &CanisterType::new("other"), 2);
        ShardingRegistry::assign(POOL, tenant, donor).unwrap();

        let reassigned = ShardingRegistry::assign_best_effort_excluding(POOL, tenant, donor);
        assert_eq!(reassigned, Some(other));
        assert_eq!(ShardingRegistry::tenant_shard(POOL, tenant), Some(other));

        let donor_entry = ShardingRegistry::with(|s| s.get_entry(&donor)).unwrap();
        assert_eq!(donor_entry.count, 0);
        let other_entry = ShardingRegistry::with(|s| s.get_entry(&other)).unwrap();
        assert_eq!(other_entry.count, 1);
    }

    #[test]
    fn assign_best_effort_excluding_returns_none_when_no_alternative() {
        ShardingRegistry::clear();

        let donor = p(3);
        let tenant = p(4);

        ShardingRegistry::create(donor, POOL, &CanisterType::new("solo"), 2);
        ShardingRegistry::assign(POOL, tenant, donor).unwrap();

        let reassigned = ShardingRegistry::assign_best_effort_excluding(POOL, tenant, donor);
        assert!(reassigned.is_none());
        assert_eq!(ShardingRegistry::tenant_shard(POOL, tenant), Some(donor));
    }
}
