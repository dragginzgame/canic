//! Module: ops::storage::placement::sharding
//!
//! Responsibility: provide deterministic sharding registry CRUD and queries.
//! Does not own: shard placement policy, workflow orchestration, or endpoint DTOs.
//! Boundary: storage ops facade over stable sharding registry records.

use crate::{
    InternalError,
    ops::{prelude::*, storage::StorageOpsError},
    storage::stable::sharding::{
        ShardEntryRecord, ShardKey, ShardingAssignmentRecord, ShardingRegistryData,
        ShardingRegistryEntryRecord, registry::ShardingRegistry,
    },
};
use thiserror::Error as ThisError;

///
/// ShardingRegistryOpsError
///
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

    #[error("shard {pid} conflicts with its existing registry entry")]
    ShardConflict { pid: Principal },

    #[error("shard {pid} assignment count is already zero")]
    AssignmentCountUnderflow { pid: Principal },

    #[error("shard {pid} assignment count is already at its maximum")]
    AssignmentCountOverflow { pid: Principal },
}

impl From<ShardingRegistryOpsError> for InternalError {
    fn from(err: ShardingRegistryOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

///
/// ShardingRegistryOps
///
/// Storage-ops facade for sharding registry CRUD and queries.
///

pub struct ShardingRegistryOps;

impl ShardingRegistryOps {
    /// Validate a partition assignment key before workflow performs external effects.
    pub fn validate_assignment_key(pool: &str, partition_key: &str) -> Result<(), InternalError> {
        ShardKey::try_new(pool, partition_key)
            .map(|_| ())
            .map_err(|err| ShardingRegistryOpsError::InvalidKey(err).into())
    }

    /// Validate a prospective shard record before workflow performs external effects.
    pub fn validate_new_shard(
        pool: &str,
        slot: u32,
        canister_role: &CanisterRole,
        capacity: u32,
    ) -> Result<(), InternalError> {
        ShardEntryRecord::try_new(pool, slot, canister_role.clone(), capacity, 0)
            .map(|_| ())
            .map_err(|err| ShardingRegistryOpsError::InvalidKey(err).into())
    }

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
            if let Some(existing) = core.get_entry(&pid) {
                if existing.pool.as_ref() == pool
                    && existing.slot == slot
                    && existing.canister_role == *canister_role
                    && existing.capacity == capacity
                {
                    return Ok(());
                }

                return Err(ShardingRegistryOpsError::ShardConflict { pid }.into());
            }

            if slot != ShardEntryRecord::UNASSIGNED_SLOT {
                for record in core.all_entries() {
                    if record.pid != pid
                        && record.entry.pool.as_ref() == pool
                        && record.entry.slot == slot
                    {
                        return Err(ShardingRegistryOpsError::SlotOccupied {
                            pool: pool.to_string(),
                            slot,
                            pid: record.pid,
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
            let mut target_entry = core
                .get_entry(&shard)
                .ok_or(ShardingRegistryOpsError::ShardNotFound(shard))?;

            if target_entry.pool.as_ref() != pool {
                return Err(ShardingRegistryOpsError::PoolMismatch {
                    pid: shard,
                    expected: pool.to_string(),
                    actual: target_entry.pool.to_string(),
                }
                .into());
            }

            let key = ShardKey::try_new(pool, partition_key)
                .map_err(ShardingRegistryOpsError::InvalidKey)?;

            let previous_entry = if let Some(current) = core.get_assignment(&key) {
                if current == shard {
                    return Ok(());
                }

                let mut old_entry = core
                    .get_entry(&current)
                    .ok_or(ShardingRegistryOpsError::ShardNotFound(current))?;
                if old_entry.pool.as_ref() != pool {
                    return Err(ShardingRegistryOpsError::PoolMismatch {
                        pid: current,
                        expected: pool.to_string(),
                        actual: old_entry.pool.to_string(),
                    }
                    .into());
                }
                old_entry.count = old_entry
                    .count
                    .checked_sub(1)
                    .ok_or(ShardingRegistryOpsError::AssignmentCountUnderflow { pid: current })?;
                Some((current, old_entry))
            } else {
                None
            };

            target_entry.count = target_entry
                .count
                .checked_add(1)
                .ok_or(ShardingRegistryOpsError::AssignmentCountOverflow { pid: shard })?;

            if let Some((previous, entry)) = previous_entry {
                core.insert_entry(previous, entry);
            }
            core.insert_assignment(key, shard);
            core.insert_entry(shard, target_entry);

            Ok(())
        })
    }

    /// Release (unassign) a partition_key from its shard, decrementing that
    /// shard's derived load counter. Returns the shard the key was assigned to,
    /// or `None` if the key had no assignment. Inverse of [`Self::assign`]; used
    /// by eviction / reclamation workflows to free shard capacity.
    pub fn release(pool: &str, partition_key: &str) -> Result<Option<Principal>, InternalError> {
        ShardingRegistry::with_mut(|core| {
            let key = ShardKey::try_new(pool, partition_key)
                .map_err(ShardingRegistryOpsError::InvalidKey)?;

            let Some(shard) = core.get_assignment(&key) else {
                return Ok(None);
            };

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
            entry.count = entry
                .count
                .checked_sub(1)
                .ok_or(ShardingRegistryOpsError::AssignmentCountUnderflow { pid: shard })?;

            let _ = core.remove_assignment(&key);
            core.insert_entry(shard, entry);

            Ok(Some(shard))
        })
    }

    /// NOTE:
    /// Returns canonical assignment keys. Callers should not stringify unless required
    /// at an API or DTO boundary.
    #[must_use]
    pub fn assignments_for_pool(pool: &str) -> Vec<ShardingAssignmentRecord> {
        ShardingRegistry::assignments_for_pool(pool)
    }

    /// Return all shard entries registered for one pool.
    #[must_use]
    pub fn entries_for_pool(pool: &str) -> Vec<ShardingRegistryEntryRecord> {
        ShardingRegistry::entries_for_pool(pool)
    }

    /// Export all shard entries.
    #[must_use]
    pub fn registry_data() -> ShardingRegistryData {
        ShardingRegistry::export_registry()
    }

    #[cfg(test)]
    pub(crate) fn clear_for_test() {
        ShardingRegistry::clear();
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn set_count(pid: Principal, count: u32) {
        ShardingRegistry::with_mut(|core| {
            let mut entry = core.get_entry(&pid).expect("test shard entry");
            entry.count = count;
            core.insert_entry(pid, entry);
        });
    }

    fn insert_assignment(pool: &str, partition_key: &str, shard: Principal) {
        let key = ShardKey::try_new(pool, partition_key).expect("test assignment key");
        ShardingRegistry::with_mut(|core| core.insert_assignment(key, shard));
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

    #[test]
    fn release_frees_slot_and_decrements_count() {
        ShardingRegistryOps::clear_for_test();
        let role = CanisterRole::new("alpha");
        let shard_pid = p(1);

        ShardingRegistryOps::create(shard_pid, "poolA", 0, &role, 2, 0).unwrap();
        ShardingRegistryOps::assign("poolA", "pk1", shard_pid).unwrap();
        assert_eq!(ShardingRegistryOps::get(shard_pid).unwrap().count, 1);

        // Releasing the key returns its shard, drops the assignment, and frees
        // shard capacity (count back to 0).
        let released = ShardingRegistryOps::release("poolA", "pk1").unwrap();
        assert_eq!(released, Some(shard_pid));
        assert_eq!(ShardingRegistryOps::get(shard_pid).unwrap().count, 0);
        assert!(ShardingRegistryOps::partition_key_shard("poolA", "pk1").is_none());

        // Releasing an unknown key is a no-op returning None.
        assert_eq!(ShardingRegistryOps::release("poolA", "pk1").unwrap(), None);
        assert_eq!(ShardingRegistryOps::get(shard_pid).unwrap().count, 0);
    }

    #[test]
    fn repeated_create_preserves_existing_shard_state() {
        ShardingRegistryOps::clear_for_test();
        let role = CanisterRole::new("alpha");
        let shard_pid = p(1);

        ShardingRegistryOps::create(shard_pid, "poolA", 0, &role, 2, 10).unwrap();
        ShardingRegistryOps::assign("poolA", "pk1", shard_pid).unwrap();
        ShardingRegistryOps::create(shard_pid, "poolA", 0, &role, 2, 20).unwrap();

        let entry = ShardingRegistryOps::get(shard_pid).unwrap();
        assert_eq!(entry.count, 1);
        assert_eq!(entry.created_at, 10);
    }

    #[test]
    fn repeated_create_rejects_conflicting_shard_identity() {
        ShardingRegistryOps::clear_for_test();
        let role = CanisterRole::new("alpha");
        let shard_pid = p(1);

        ShardingRegistryOps::create(shard_pid, "poolA", 0, &role, 2, 10).unwrap();
        let err = ShardingRegistryOps::create(shard_pid, "poolA", 1, &role, 2, 20)
            .expect_err("same shard principal with a different slot must reject");

        assert_eq!(err.class(), crate::InternalErrorClass::Ops);
        assert_eq!(err.origin(), crate::InternalErrorOrigin::Ops);
        assert_eq!(ShardingRegistryOps::get(shard_pid).unwrap().slot, 0);
    }

    #[test]
    fn assignment_key_validation_rejects_oversized_partition_key() {
        let err = ShardingRegistryOps::validate_assignment_key("poolA", &"x".repeat(129))
            .expect_err("oversized partition keys must reject before assignment");

        assert_eq!(err.class(), crate::InternalErrorClass::Ops);
        assert_eq!(err.origin(), crate::InternalErrorOrigin::Ops);
    }

    #[test]
    fn reassignment_rejects_counter_underflow_without_mutation() {
        ShardingRegistryOps::clear_for_test();
        let role = CanisterRole::new("alpha");
        let old_shard = p(1);
        let new_shard = p(2);

        ShardingRegistryOps::create(old_shard, "poolA", 0, &role, 2, 0).unwrap();
        ShardingRegistryOps::create(new_shard, "poolA", 1, &role, 2, 0).unwrap();
        ShardingRegistryOps::assign("poolA", "pk1", old_shard).unwrap();
        set_count(old_shard, 0);

        let err = ShardingRegistryOps::assign("poolA", "pk1", new_shard)
            .expect_err("a corrupt old counter must reject reassignment");

        assert_eq!(err.class(), crate::InternalErrorClass::Ops);
        assert_eq!(err.origin(), crate::InternalErrorOrigin::Ops);
        assert_eq!(
            ShardingRegistryOps::partition_key_shard("poolA", "pk1"),
            Some(old_shard)
        );
        assert_eq!(ShardingRegistryOps::get(old_shard).unwrap().count, 0);
        assert_eq!(ShardingRegistryOps::get(new_shard).unwrap().count, 0);
    }

    #[test]
    fn assignment_rejects_counter_overflow_without_mutation() {
        ShardingRegistryOps::clear_for_test();
        let role = CanisterRole::new("alpha");
        let shard = p(1);

        ShardingRegistryOps::create(shard, "poolA", 0, &role, u32::MAX, 0).unwrap();
        set_count(shard, u32::MAX);

        let err = ShardingRegistryOps::assign("poolA", "pk1", shard)
            .expect_err("a full-width counter must reject assignment");

        assert_eq!(err.class(), crate::InternalErrorClass::Ops);
        assert_eq!(err.origin(), crate::InternalErrorOrigin::Ops);
        assert!(ShardingRegistryOps::partition_key_shard("poolA", "pk1").is_none());
        assert_eq!(ShardingRegistryOps::get(shard).unwrap().count, u32::MAX);
    }

    #[test]
    fn release_rejects_missing_shard_without_losing_assignment() {
        ShardingRegistryOps::clear_for_test();
        let missing_shard = p(1);
        insert_assignment("poolA", "pk1", missing_shard);

        let err = ShardingRegistryOps::release("poolA", "pk1")
            .expect_err("a dangling assignment must fail closed");

        assert_eq!(err.class(), crate::InternalErrorClass::Ops);
        assert_eq!(err.origin(), crate::InternalErrorOrigin::Ops);
        assert_eq!(
            ShardingRegistryOps::partition_key_shard("poolA", "pk1"),
            Some(missing_shard)
        );
    }

    #[test]
    fn reassignment_rejects_missing_old_shard_without_mutation() {
        ShardingRegistryOps::clear_for_test();
        let role = CanisterRole::new("alpha");
        let missing_shard = p(1);
        let target_shard = p(2);

        ShardingRegistryOps::create(target_shard, "poolA", 0, &role, 2, 0).unwrap();
        insert_assignment("poolA", "pk1", missing_shard);

        let err = ShardingRegistryOps::assign("poolA", "pk1", target_shard)
            .expect_err("a dangling old assignment must fail closed");

        assert_eq!(err.class(), crate::InternalErrorClass::Ops);
        assert_eq!(err.origin(), crate::InternalErrorOrigin::Ops);
        assert_eq!(
            ShardingRegistryOps::partition_key_shard("poolA", "pk1"),
            Some(missing_shard)
        );
        assert_eq!(ShardingRegistryOps::get(target_shard).unwrap().count, 0);
    }

    #[test]
    fn release_rejects_counter_underflow_without_losing_assignment() {
        ShardingRegistryOps::clear_for_test();
        let role = CanisterRole::new("alpha");
        let shard = p(1);

        ShardingRegistryOps::create(shard, "poolA", 0, &role, 2, 0).unwrap();
        insert_assignment("poolA", "pk1", shard);

        let err = ShardingRegistryOps::release("poolA", "pk1")
            .expect_err("a zero counter must fail closed");

        assert_eq!(err.class(), crate::InternalErrorClass::Ops);
        assert_eq!(err.origin(), crate::InternalErrorOrigin::Ops);
        assert_eq!(
            ShardingRegistryOps::partition_key_shard("poolA", "pk1"),
            Some(shard)
        );
        assert_eq!(ShardingRegistryOps::get(shard).unwrap().count, 0);
    }
}
