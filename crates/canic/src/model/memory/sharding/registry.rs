use crate::{
    Error,
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    model::memory::sharding::{SHARDING_CORE, ShardEntry, ShardKey, ShardingCore, ShardingError},
    types::CanisterType,
    utils::time::now_secs,
};
use candid::Principal;
use std::collections::BTreeSet;

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
    pub fn create(shard_pid: Principal, pool: &str, slot: u32, ty: &CanisterType, capacity: u32) {
        let entry = ShardEntry {
            slot,
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

    /// Ensure every shard in the pool has a deterministic slot assignment.
    pub fn ensure_slot_assignments(pool: &str, max_slots: u32) {
        if max_slots == 0 {
            return;
        }

        Self::with_mut(|core| {
            let mut updates = Vec::new();

            // deterministic sorted order for etnries
            let mut entries: Vec<_> = core
                .all_entries()
                .into_iter()
                .filter(|(_, entry)| entry.pool == pool)
                .collect();

            if entries.is_empty() {
                return;
            }
            entries.sort_by_key(|(pid, _)| *pid);

            //
            let mut occupied: BTreeSet<u32> = entries
                .iter()
                .filter_map(|(_, entry)| entry.has_assigned_slot().then_some(entry.slot))
                .collect();
            let available: Vec<u32> = (0..max_slots)
                .filter(|slot| !occupied.contains(slot))
                .collect();

            if available.is_empty() {
                return;
            }

            let mut idx = 0usize;
            for (pid, mut entry) in entries {
                if entry.has_assigned_slot() {
                    continue;
                }

                if idx >= available.len() {
                    break;
                }

                entry.slot = available[idx];
                occupied.insert(entry.slot);
                idx += 1;
                updates.push((pid, entry));
            }

            for (pid, entry) in updates {
                core.insert_entry(pid, entry);
            }
        });
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
    pub fn export() -> ShardingRegistryView {
        Self::with(ShardingCore::all_entries)
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CanisterType;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn ensure_slot_assignments_backfills_unassigned_entries() {
        ShardingRegistry::clear();
        let ty = CanisterType::new("alpha");

        ShardingRegistry::with_mut(|core| {
            core.insert_entry(
                p(1),
                ShardEntry {
                    slot: ShardEntry::UNASSIGNED_SLOT,
                    canister_type: ty.clone(),
                    capacity: 10,
                    count: 0,
                    pool: "poolA".into(),
                    created_at: 0,
                },
            );
            core.insert_entry(
                p(2),
                ShardEntry {
                    slot: ShardEntry::UNASSIGNED_SLOT,
                    canister_type: ty.clone(),
                    capacity: 10,
                    count: 0,
                    pool: "poolA".into(),
                    created_at: 0,
                },
            );
        });

        ShardingRegistry::ensure_slot_assignments("poolA", 4);

        let slot1 = ShardingRegistry::slot_for_shard("poolA", p(1)).unwrap();
        let slot2 = ShardingRegistry::slot_for_shard("poolA", p(2)).unwrap();
        assert_ne!(slot1, slot2);
    }

    #[test]
    fn assign_and_release_updates_count() {
        ShardingRegistry::clear();
        let ty = CanisterType::new("alpha");
        let shard_pid = p(1);

        ShardingRegistry::create(shard_pid, "poolA", 0, &ty, 2);
        assert_eq!(ShardingRegistry::count(), 1);

        ShardingRegistry::assign("poolA", "tenant1", shard_pid).unwrap();
        let count_after = ShardingRegistry::with(|s| s.get_entry(&shard_pid).unwrap().count);
        assert_eq!(count_after, 1);

        ShardingRegistry::release("poolA", "tenant1").unwrap();
        let count_final = ShardingRegistry::with(|s| s.get_entry(&shard_pid).unwrap().count);
        assert_eq!(count_final, 0);
    }
}
