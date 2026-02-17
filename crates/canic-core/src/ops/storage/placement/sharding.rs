use crate::{
    InternalError,
    ops::{prelude::*, storage::StorageOpsError},
    storage::stable::sharding::{
        ShardEntryRecord, ShardKey,
        registry::{ShardingRegistry, ShardingRegistryRecord},
    },
};
use thiserror::Error as ThisError;

///
/// ShardingRegistryOpsError
/// Storage-layer errors for sharding registry CRUD and consistency checks.
///

#[derive(Debug, ThisError)]
pub enum ShardingRegistryOpsError {
    #[error("invalid sharding key: {0}")]
    InvalidKey(String),

    #[error("shard {pid} belongs to pool '{actual}', not '{expected}'")]
    PoolMismatch {
        pid: Principal,
        expected: String,
        actual: String,
    },

    #[error("shard not found: {0}")]
    ShardNotFound(Principal),

    #[error("slot {slot} in pool '{pool}' already assigned to shard {pid}")]
    SlotOccupied {
        pool: String,
        slot: u32,
        pid: Principal,
    },

    #[error("partition_key '{partition_key}' is not assigned to any shard in pool '{pool}'")]
    PartitionKeyNotAssigned { pool: String, partition_key: String },
}

impl From<ShardingRegistryOpsError> for InternalError {
    fn from(err: ShardingRegistryOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

///
/// ShardingRegistryOps
///

pub struct ShardingRegistryOps;

impl ShardingRegistryOps {
    /// Create a new shard entry in the registry.
    pub fn create(
        pid: Principal,
        pool: &str,
        slot: u32,
        canister_role: &CanisterRole,
        capacity: u32,
        created_at: u64,
    ) -> Result<(), InternalError> {
        // NOTE: Slot uniqueness is enforced by linear scan.
        // Shard counts are expected to be small and bounded.
        ShardingRegistry::with_mut(|core| {
            if slot != ShardEntryRecord::UNASSIGNED_SLOT {
                for (other_pid, other_entry) in core.all_entries() {
                    if other_pid != pid
                        && other_entry.pool.as_ref() == pool
                        && other_entry.slot == slot
                    {
                        return Err(ShardingRegistryOpsError::SlotOccupied {
                            pool: pool.to_string(),
                            slot,
                            pid: other_pid,
                        }
                        .into());
                    }
                }
            }

            let entry =
                ShardEntryRecord::try_new(pool, slot, canister_role.clone(), capacity, created_at)
                    .map_err(ShardingRegistryOpsError::InvalidKey)?;
            core.insert_entry(pid, entry);

            Ok(())
        })
    }

    /// Fetch a shard entry by principal (tests only).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn get(pid: Principal) -> Option<ShardEntryRecord> {
        ShardingRegistry::with(|core| core.get_entry(&pid))
    }

    /// Returns the shard assigned to the given partition_key (if any).
    #[must_use]
    pub fn partition_key_shard(pool: &str, partition_key: &str) -> Option<Principal> {
        ShardingRegistry::partition_key_shard(pool, partition_key)
    }

    pub fn partition_key_shard_required(
        pool: &str,
        partition_key: &str,
    ) -> Result<Principal, InternalError> {
        Self::partition_key_shard(pool, partition_key).ok_or_else(|| {
            ShardingRegistryOpsError::PartitionKeyNotAssigned {
                pool: pool.to_string(),
                partition_key: partition_key.to_string(),
            }
            .into()
        })
    }

    /// Lookup the slot index for a given shard principal.
    #[must_use]
    pub fn slot_for_shard(pool: &str, shard: Principal) -> Option<u32> {
        ShardingRegistry::slot_for_shard(pool, shard)
    }

    /// Lists all partition_keys currently assigned to the specified shard.
    #[must_use]
    pub fn partition_keys_in_shard(pool: &str, shard: Principal) -> Vec<String> {
        ShardingRegistry::partition_keys_in_shard(pool, shard)
    }

    /// Assign (or reassign) a partition_key to a shard.
    ///
    /// Storage responsibilities:
    /// - enforce referential integrity (target shard must exist)
    /// - enforce pool consistency (assignment pool must match shard entry pool)
    /// - maintain derived counters (`ShardEntryRecord.count`)
    pub fn assign(pool: &str, partition_key: &str, shard: Principal) -> Result<(), InternalError> {
        ShardingRegistry::with_mut(|core| {
            let mut entry = core
                .get_entry(&shard)
                .ok_or(ShardingRegistryOpsError::ShardNotFound(shard))?;

            if entry.pool.as_ref() != pool {
                return Err(ShardingRegistryOpsError::PoolMismatch {
                    pid: shard,
                    expected: pool.to_string(),
                    actual: entry.pool.to_string(),
                }
                .into());
            }

            let key = ShardKey::try_new(pool, partition_key)
                .map_err(ShardingRegistryOpsError::InvalidKey)?;

            // If partition_key is already assigned, decrement the old shard count.
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

    /// NOTE:
    /// Returns canonical assignment keys. Callers should not stringify unless required
    /// at an API or DTO boundary.
    #[must_use]
    pub fn assignments_for_pool(pool: &str) -> Vec<(ShardKey, Principal)> {
        ShardingRegistry::with(|core| {
            core.all_assignments()
                .into_iter()
                .filter(|(k, _)| k.pool.as_ref() == pool)
                .collect()
        })
    }

    /// Export all shard entries
    #[must_use]
    pub fn export() -> ShardingRegistryRecord {
        ShardingRegistry::export()
    }

    #[cfg(test)]
    pub(crate) fn clear_for_test() {
        ShardingRegistry::clear();
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn assign_updates_count() {
        ShardingRegistryOps::clear_for_test();
        let role = CanisterRole::new("alpha");
        let shard_pid = p(1);
        let created_at = 0;

        ShardingRegistryOps::create(shard_pid, "poolA", 0, &role, 2, created_at).unwrap();
        ShardingRegistryOps::assign("poolA", "partition_key1", shard_pid).unwrap();
        let count_after = ShardingRegistryOps::get(shard_pid).unwrap().count;
        assert_eq!(count_after, 1);
    }
}
