use crate::{
    Error, ThisError,
    cdk::utils::time::now_secs,
    ops::{prelude::*, storage::StorageOpsError},
    storage::stable::sharding::{
        ShardEntry as ModelShardEntry, ShardKey,
        registry::{ShardingRegistry, ShardingRegistryData as ModelShardingRegistryData},
    },
};

///
/// ShardEntry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShardEntry {
    pub slot: u32,
    pub capacity: u32,
    pub count: u32,
    pub pool: crate::cdk::types::BoundedString64,
    pub canister_role: CanisterRole,
    pub created_at: u64,
}

impl ShardEntry {
    pub const UNASSIGNED_SLOT: u32 = u32::MAX;
}

///
/// ShardingRegistrySnapshor
///

#[derive(Clone, Debug)]
pub struct ShardingRegistrySnapshot {
    pub entries: Vec<(Principal, ShardEntry)>,
}

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

    #[error("invalid sharding key: {0}")]
    InvalidKey(String),

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
        canister_role: &CanisterRole,
        capacity: u32,
    ) -> Result<(), Error> {
        ShardingRegistry::with_mut(|core| {
            if slot != ShardEntry::UNASSIGNED_SLOT {
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
                ModelShardEntry::try_new(pool, slot, canister_role.clone(), capacity, now_secs())
                    .map_err(ShardingRegistryOpsError::InvalidKey)?;
            core.insert_entry(pid, entry);

            Ok(())
        })
    }

    /// Fetch a shard entry by principal (tests only).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn get(pid: Principal) -> Option<ShardEntry> {
        ShardingRegistry::with(|core| core.get_entry(&pid).map(Into::into))
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

            if entry.pool.as_ref() != pool {
                return Err(ShardingRegistryOpsError::PoolMismatch {
                    pid: shard,
                    expected: pool.to_string(),
                    actual: entry.pool.to_string(),
                }
                .into());
            }

            let key =
                ShardKey::try_new(pool, tenant).map_err(ShardingRegistryOpsError::InvalidKey)?;

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

    /// Export all shard entries
    #[must_use]
    pub fn export() -> ShardingRegistrySnapshot {
        ShardingRegistry::export().into()
    }

    #[cfg(test)]
    pub(crate) fn clear_for_test() {
        ShardingRegistry::clear();
    }
}

impl From<ModelShardingRegistryData> for ShardingRegistrySnapshot {
    fn from(data: ModelShardingRegistryData) -> Self {
        Self {
            entries: data
                .entries
                .into_iter()
                .map(|(pid, entry)| (pid, entry.into()))
                .collect(),
        }
    }
}

impl From<ModelShardEntry> for ShardEntry {
    fn from(entry: ModelShardEntry) -> Self {
        Self {
            slot: entry.slot,
            capacity: entry.capacity,
            count: entry.count,
            pool: entry.pool,
            canister_role: entry.canister_role,
            created_at: entry.created_at,
        }
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

        ShardingRegistryOps::create(shard_pid, "poolA", 0, &role, 2).unwrap();
        ShardingRegistryOps::assign("poolA", "tenant1", shard_pid).unwrap();
        let count_after = ShardingRegistryOps::get(shard_pid).unwrap().count;
        assert_eq!(count_after, 1);
    }
}
