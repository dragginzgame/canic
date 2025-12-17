use crate::{
    Error, ThisError,
    cdk::{types::Principal, utils::time::now_secs},
    ids::CanisterRole,
    model::memory::sharding::{ShardEntry, ShardKey, ShardingRegistry},
    ops::storage::StorageOpsError,
};

///
/// ShardingRegistryOps
///

pub struct ShardingRegistryOps;

///
/// ShardingRegistryOpsError
/// Storage-layer errors for sharding registry CRUD and consistency checks.
///

#[derive(Debug, ThisError)]
pub enum ShardingRegistryOpsError {
    #[error("shard not found: {0}")]
    ShardNotFound(Principal),

    #[error("shard {pid} belongs to pool '{actual}', not '{expected}'")]
    PoolMismatch {
        pid: Principal,
        expected: String,
        actual: String,
    },

    #[error("slot {slot} in pool '{pool}' already assigned to shard {pid}")]
    SlotOccupied {
        pool: String,
        slot: u32,
        pid: Principal,
    },
}

impl From<ShardingRegistryOpsError> for Error {
    fn from(err: ShardingRegistryOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

impl ShardingRegistryOps {
    /// Create a new shard entry in the registry.
    pub fn create(
        pid: Principal,
        pool: &str,
        slot: u32,
        canister_type: &CanisterRole,
        capacity: u32,
    ) -> Result<(), Error> {
        ShardingRegistry::with_mut(|core| {
            if slot != ShardEntry::UNASSIGNED_SLOT {
                for (other_pid, other_entry) in core.all_entries() {
                    if other_pid != pid && other_entry.pool == pool && other_entry.slot == slot {
                        return Err(ShardingRegistryOpsError::SlotOccupied {
                            pool: pool.to_string(),
                            slot,
                            pid: other_pid,
                        }
                        .into());
                    }
                }
            }

            let entry = ShardEntry::new(pool, slot, canister_type.clone(), capacity, now_secs());
            core.insert_entry(pid, entry);

            Ok(())
        })
    }

    /// Fetch a shard entry by principal.
    #[must_use]
    pub fn get(pid: Principal) -> Option<ShardEntry> {
        ShardingRegistry::with(|core| core.get_entry(&pid))
    }

    /// Export all shard entries.
    #[must_use]
    pub fn export() -> Vec<(Principal, ShardEntry)> {
        ShardingRegistry::export()
    }

    /// Returns the shard assigned to the given tenant (if any).
    #[must_use]
    pub fn tenant_shard(pool: &str, tenant: &str) -> Option<Principal> {
        ShardingRegistry::tenant_shard(pool, tenant)
    }

    /// Lookup the slot index for a given shard principal.
    #[must_use]
    pub fn slot_for_shard(pool: &str, shard: Principal) -> Option<u32> {
        ShardingRegistry::slot_for_shard(pool, shard)
    }

    /// Lists all tenants currently assigned to the specified shard.
    #[must_use]
    pub fn tenants_in_shard(pool: &str, shard: Principal) -> Vec<String> {
        ShardingRegistry::tenants_in_shard(pool, shard)
    }

    /// Assign (or reassign) a tenant to a shard.
    ///
    /// Storage responsibilities:
    /// - enforce referential integrity (target shard must exist)
    /// - enforce pool consistency (assignment pool must match shard entry pool)
    /// - maintain derived counters (`ShardEntry.count`)
    pub fn assign(pool: &str, tenant: &str, shard: Principal) -> Result<(), Error> {
        ShardingRegistry::with_mut(|core| {
            let mut entry = core
                .get_entry(&shard)
                .ok_or(ShardingRegistryOpsError::ShardNotFound(shard))?;

            if entry.pool != pool {
                return Err(ShardingRegistryOpsError::PoolMismatch {
                    pid: shard,
                    expected: pool.to_string(),
                    actual: entry.pool,
                }
                .into());
            }

            let key = ShardKey::new(pool, tenant);

            // If tenant is already assigned, decrement the old shard count.
            if let Some(current) = core.get_assignment(&key) {
                if current == shard {
                    return Ok(());
                }

                if let Some(mut old_entry) = core.get_entry(&current) {
                    old_entry.count = old_entry.count.saturating_sub(1);
                    core.insert_entry(current, old_entry);
                }
            }

            // Overwrite the assignment and increment the new shard count.
            core.insert_assignment(key, shard);
            entry.count = entry.count.saturating_add(1);
            core.insert_entry(shard, entry);

            Ok(())
        })
    }

    /// Remove a tenant assignment, if present.
    ///
    /// Returns the shard principal that previously held the assignment.
    pub fn unassign(pool: &str, tenant: &str) -> Result<Option<Principal>, Error> {
        ShardingRegistry::with_mut(|core| {
            let key = ShardKey::new(pool, tenant);
            let Some(shard) = core.remove_assignment(&key) else {
                return Ok(None);
            };

            if let Some(mut entry) = core.get_entry(&shard) {
                entry.count = entry.count.saturating_sub(1);
                core.insert_entry(shard, entry);
            }

            Ok(Some(shard))
        })
    }

    /// Update the logical slot index for a shard entry.
    pub fn set_slot(pid: Principal, slot: u32) -> Result<(), Error> {
        ShardingRegistry::with_mut(|core| {
            let mut entry = core
                .get_entry(&pid)
                .ok_or(ShardingRegistryOpsError::ShardNotFound(pid))?;

            if slot != ShardEntry::UNASSIGNED_SLOT {
                for (other_pid, other_entry) in core.all_entries() {
                    if other_pid != pid
                        && other_entry.pool == entry.pool
                        && other_entry.slot == slot
                    {
                        return Err(ShardingRegistryOpsError::SlotOccupied {
                            pool: entry.pool,
                            slot,
                            pid: other_pid,
                        }
                        .into());
                    }
                }
            }

            entry.slot = slot;
            core.insert_entry(pid, entry);

            Ok(())
        })
    }

    #[cfg(test)]
    pub(crate) fn clear_for_test() {
        ShardingRegistry::clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn assign_and_unassign_updates_count() {
        ShardingRegistryOps::clear_for_test();
        let ty = CanisterRole::new("alpha");
        let shard_pid = p(1);

        ShardingRegistryOps::create(shard_pid, "poolA", 0, &ty, 2).unwrap();
        ShardingRegistryOps::assign("poolA", "tenant1", shard_pid).unwrap();
        let count_after = ShardingRegistryOps::get(shard_pid).unwrap().count;
        assert_eq!(count_after, 1);

        assert_eq!(
            ShardingRegistryOps::unassign("poolA", "tenant1").unwrap(),
            Some(shard_pid)
        );
        let count_final = ShardingRegistryOps::get(shard_pid).unwrap().count;
        assert_eq!(count_final, 0);
    }
}
