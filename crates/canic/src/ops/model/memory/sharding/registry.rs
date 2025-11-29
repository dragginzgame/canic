use crate::{
    Error,
    model::memory::sharding::{ShardKey, ShardingError, ShardingRegistry},
};
use candid::Principal;
use std::collections::BTreeSet;

///
/// ShardingRegistryOps
///

pub struct ShardingRegistryOps;

impl ShardingRegistryOps {
    /// Assign a tenant to a shard with basic capacity/pool validation.
    pub fn assign(pool: &str, tenant: &str, shard: Principal) -> Result<(), Error> {
        let mut entry = ShardingRegistry::with(|s| s.get_entry(&shard))
            .ok_or(ShardingError::ShardNotFound(shard))?;

        if entry.pool != pool {
            Err(ShardingError::ShardNotFound(shard))?;
        }

        if entry.count >= entry.capacity {
            Err(ShardingError::ShardFull(shard))?;
        }

        // If tenant is already assigned, replace only if different
        if let Some(current) =
            ShardingRegistry::with(|s| s.get_assignment(&ShardKey::new(pool, tenant)))
        {
            if current == shard {
                return Ok(()); // no-op
            }
            Self::release(pool, tenant)?; // clean old assignment
        }

        // Insert assignment and update shard load
        ShardingRegistry::with_mut(|s| s.insert_assignment(ShardKey::new(pool, tenant), shard));
        entry.count = entry.count.saturating_add(1);
        ShardingRegistry::with_mut(|s| s.insert_entry(shard, entry));

        Ok(())
    }

    /// Release a tenant from its shard and decrement the shard's load.
    pub fn release(pool: &str, tenant: &str) -> Result<(), Error> {
        let key = ShardKey::new(pool, tenant);
        let shard = ShardingRegistry::with_mut(|s| s.remove_assignment(&key))?;

        if let Some(mut entry) = ShardingRegistry::with(|s| s.get_entry(&shard)) {
            entry.count = entry.count.saturating_sub(1);
            ShardingRegistry::with_mut(|s| s.insert_entry(shard, entry));
        }

        Ok(())
    }

    /// Backfill unassigned shard slots deterministically within a pool.
    pub fn ensure_slot_assignments(pool: &str, max_slots: u32) {
        if max_slots == 0 {
            return;
        }

        ShardingRegistry::with_mut(|core| {
            let mut updates = Vec::new();

            let mut entries: Vec<_> = core
                .all_entries()
                .into_iter()
                .filter(|(_, entry)| entry.pool == pool)
                .collect();

            if entries.is_empty() {
                return;
            }
            entries.sort_by_key(|(pid, _)| *pid);

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::memory::sharding::ShardEntry, types::CanisterType};

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

        ShardingRegistryOps::ensure_slot_assignments("poolA", 4);

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
        ShardingRegistryOps::assign("poolA", "tenant1", shard_pid).unwrap();
        let count_after = ShardingRegistry::with(|s| s.get_entry(&shard_pid).unwrap().count);
        assert_eq!(count_after, 1);

        ShardingRegistryOps::release("poolA", "tenant1").unwrap();
        let count_final = ShardingRegistry::with(|s| s.get_entry(&shard_pid).unwrap().count);
        assert_eq!(count_final, 0);
    }
}
