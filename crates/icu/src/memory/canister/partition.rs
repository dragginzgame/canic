use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_bounded,
    memory::{
        CanisterChildren, MemoryError, PARTITION_ITEM_MAP_MEMORY_ID, PARTITION_REGISTRY_MEMORY_ID,
    },
    types::CanisterType,
    utils::time::now_secs,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// PARTITION REGISTRY
//
// Generic registry: assigns items (Principals) to partition canisters (Principals)
// with capacity accounting.
//

thread_local! {
    static PARTITION_REGISTRY: RefCell<PartitionRegistryCore<VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        PartitionRegistryCore::new(BTreeMap::init(icu_register_memory!(PARTITION_REGISTRY_MEMORY_ID))));

    static PARTITION_ITEM_MAP: RefCell<BTreeMap<Principal, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(icu_register_memory!(PARTITION_ITEM_MAP_MEMORY_ID)));
}

///
/// PartitionRegistryError
///

#[derive(Debug, ThisError)]
pub enum PartitionRegistryError {
    #[error("partition not found: {0}")]
    PartitionNotFound(Principal),

    #[error("partition full: {0}")]
    PartitionFull(Principal),

    #[error("item not found: {0}")]
    ItemNotFound(Principal),
}

///
/// PartitionMetrics
///

#[derive(Clone, Copy, Debug)]
struct PartitionMetrics {
    capacity: u32,
    count: u32,
}

impl PartitionMetrics {
    const fn has_capacity(self) -> bool {
        self.count < self.capacity
    }

    const fn load_bps(self) -> u64 {
        if self.capacity == 0 {
            u64::MAX // treat zero capacity as unusable
        } else {
            (self.count as u64)
                .saturating_mul(10_000)
                .saturating_div(self.capacity as u64)
        }
    }
}

///
/// PartitionEntry
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PartitionEntry {
    pub capacity: u32,
    pub count: u32,
    pub created_at_secs: u64,
}

impl PartitionEntry {
    const STORABLE_MAX_SIZE: u32 = 64;

    const fn metrics(&self) -> PartitionMetrics {
        PartitionMetrics {
            capacity: self.capacity,
            count: self.count,
        }
    }
}

// Allow sufficient headroom for serde encoding overhead
impl_storable_bounded!(PartitionEntry, PartitionEntry::STORABLE_MAX_SIZE, false);

///
/// PartitionRegistry
///

pub type PartitionRegistryView = Vec<(Principal, PartitionEntry)>;

pub struct PartitionRegistry;

impl PartitionRegistry {
    pub fn register(partition_pid: Principal, capacity: u32) {
        PARTITION_REGISTRY.with_borrow_mut(|core| {
            core.upsert(partition_pid, capacity);
        });
    }

    /// Assign item to any partition with available capacity (least-loaded).
    #[must_use]
    pub fn assign_item_best_effort(item: Principal) -> Option<Principal> {
        // Already assigned? Keep if still valid
        if let Some(current) = Self::get_item_partition(&item) {
            let valid = PARTITION_REGISTRY.with_borrow(|core| {
                core.get(&current)
                    .is_some_and(|e| e.metrics().has_capacity())
            });
            if valid {
                return Some(current);
            }
        }

        let pid = PARTITION_REGISTRY.with_borrow(PartitionRegistryCore::pick_least_loaded)?;
        Self::assign_item_to_partition(item, pid).ok()?;

        Some(pid)
    }

    /// Assign item to a specific partition (idempotent).
    pub fn assign_item_to_partition(
        item: Principal,
        partition_pid: Principal,
    ) -> Result<(), Error> {
        // No-op if already assigned
        if Self::get_item_partition(&item) == Some(partition_pid) {
            return Ok(());
        }

        let mut entry = Self::try_get_entry(partition_pid)?;
        if !entry.metrics().has_capacity() {
            return Err(
                MemoryError::from(PartitionRegistryError::PartitionFull(partition_pid)).into(),
            );
        }

        // If assigned elsewhere, release first
        if let Some(prev) = Self::get_item_partition(&item)
            && prev != partition_pid
        {
            Self::release_item(&item)?;
        }

        // Insert mapping and increment count
        PARTITION_ITEM_MAP.with_borrow_mut(|map| {
            map.insert(item, partition_pid);
        });
        entry.count = entry.count.saturating_add(1);
        PARTITION_REGISTRY.with_borrow_mut(|core| core.put(partition_pid, entry));

        Ok(())
    }

    /// Release an item from its partition (decrement count).
    pub fn release_item(item: &Principal) -> Result<(), Error> {
        let pid = Self::get_item_partition(item)
            .ok_or_else(|| MemoryError::from(PartitionRegistryError::ItemNotFound(*item)))?;

        PARTITION_ITEM_MAP.with_borrow_mut(|map| {
            map.remove(item);
        });

        PARTITION_REGISTRY.with_borrow_mut(|core| {
            if let Some(mut e) = core.get(&pid) {
                e.count = e.count.saturating_sub(1);
                core.put(pid, e);
            }
        });

        Ok(())
    }

    #[must_use]
    pub fn get_item_partition(item: &Principal) -> Option<Principal> {
        PARTITION_ITEM_MAP.with_borrow(|map| map.get(item))
    }

    fn try_get_entry(pid: Principal) -> Result<PartitionEntry, Error> {
        PARTITION_REGISTRY
            .with_borrow(|reg| reg.get(&pid))
            .ok_or_else(|| MemoryError::from(PartitionRegistryError::PartitionNotFound(pid)).into())
    }

    /// Ensure current partition is still valid; migrate if necessary.
    #[must_use]
    pub fn ensure_item_migrated(item: Principal) -> Option<Principal> {
        let current = Self::get_item_partition(&item)?;
        let valid = PARTITION_REGISTRY.with_borrow(|core| {
            core.get(&current)
                .is_some_and(|e| e.metrics().has_capacity())
        });
        if !valid {
            return Self::assign_item_best_effort(item);
        }

        Some(current)
    }

    /// Most recent creation timestamp across partitions of a given type.
    #[must_use]
    pub fn last_created_at_for_type(ty: &CanisterType) -> u64 {
        PARTITION_REGISTRY.with_borrow(|core| {
            core.partitions.iter().fold(0, |last, e| {
                let pid = *e.key();
                let entry = e.value();
                if CanisterChildren::get(&pid).as_ref() == Some(ty) {
                    last.max(entry.created_at_secs)
                } else {
                    last
                }
            })
        })
    }

    /// Auditor: recompute counts by scanning ITEM_TO_PARTITION and writing back.
    pub fn audit_and_fix_counts() {
        let mut counts = std::collections::BTreeMap::<Principal, u32>::new();
        PARTITION_ITEM_MAP.with_borrow(|map| {
            for (_item, part) in map.view() {
                *counts.entry(part).or_insert(0) += 1;
            }
        });

        PARTITION_REGISTRY.with_borrow_mut(|core| {
            let keys: Vec<Principal> = core.partitions.iter().map(|e| *e.key()).collect();
            for pid in keys {
                let mut entry = core.get(&pid).unwrap_or_default();
                entry.count = counts.get(&pid).copied().unwrap_or(0);
                core.put(pid, entry);
            }
            for (pid, c) in counts {
                let mut entry = core.get(&pid).unwrap_or_default();
                entry.count = c;
                core.put(pid, entry);
            }
        });
    }

    /// Non-mutating candidate selection using least-loaded.
    #[must_use]
    pub fn peek_best_effort() -> Option<Principal> {
        PARTITION_REGISTRY.with_borrow(PartitionRegistryCore::pick_least_loaded)
    }

    #[must_use]
    pub fn export() -> PartitionRegistryView {
        PARTITION_REGISTRY.with_borrow(PartitionRegistryCore::export)
    }
}

///
/// PartitionRegistryCore
///

struct PartitionRegistryCore<M: Memory> {
    partitions: BTreeMap<Principal, PartitionEntry, M>,
}

impl<M: Memory> PartitionRegistryCore<M> {
    pub const fn new(map: BTreeMap<Principal, PartitionEntry, M>) -> Self {
        Self { partitions: map }
    }

    fn get(&self, pid: &Principal) -> Option<PartitionEntry> {
        self.partitions.get(pid)
    }

    fn put(&mut self, pid: Principal, entry: PartitionEntry) {
        self.partitions.insert(pid, entry);
    }

    fn upsert(&mut self, pid: Principal, capacity: u32) {
        let mut entry = self.get(&pid).unwrap_or_default();
        let is_new = entry.capacity == 0 && entry.count == 0 && entry.created_at_secs == 0;
        entry.capacity = capacity;
        if is_new && capacity > 0 {
            entry.created_at_secs = now_secs();
        }
        self.put(pid, entry);
    }

    fn export(&self) -> PartitionRegistryView {
        self.partitions
            .iter()
            .map(|e| (*e.key(), e.value()))
            .collect()
    }

    fn pick_least_loaded(&self) -> Option<Principal> {
        self.partitions
            .iter()
            .filter(|e| {
                let m = e.value().metrics();
                m.has_capacity()
            })
            .min_by_key(|e| {
                let m = e.value().metrics();
                (m.load_bps(), m.count)
            })
            .map(|e| *e.key())
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_key_max_size_is_bounded() {
        use crate::cdk::structures::Storable;

        let index_key = PartitionEntry::STORABLE_MAX_SIZE;
        let size = Storable::to_bytes(&index_key).len();

        assert!(size <= PartitionEntry::STORABLE_MAX_SIZE as usize);
    }

    fn clear_all() {
        PARTITION_ITEM_MAP.with_borrow_mut(BTreeMap::clear);
        PARTITION_REGISTRY.with_borrow_mut(|c| c.partitions.clear());
    }

    #[test]
    fn best_effort_distributes_to_least_loaded() {
        clear_all();
        let p1 = Principal::from_slice(&[1]);
        let p2 = Principal::from_slice(&[2]);
        PartitionRegistry::register(p1, 3);
        PartitionRegistry::register(p2, 3);

        // Preload p2 with 2 items
        let i0 = Principal::from_slice(&[10]);
        let i1 = Principal::from_slice(&[11]);
        PartitionRegistry::assign_item_to_partition(i0, p2).unwrap();
        PartitionRegistry::assign_item_to_partition(i1, p2).unwrap();

        // Now best-effort should prefer p1 until balanced
        let i2 = Principal::from_slice(&[12]);
        let i3 = Principal::from_slice(&[13]);
        let a2 = PartitionRegistry::assign_item_best_effort(i2).unwrap();
        let a3 = PartitionRegistry::assign_item_best_effort(i3).unwrap();
        assert_eq!(a2, p1);
        assert_eq!(a3, p1);

        // After balancing: counts should be equal (2 and 2)
        let view = PartitionRegistry::export();
        let mut c1 = 0u32;
        let mut c2 = 0u32;
        for (pid, e) in view {
            if pid == p1 {
                c1 = e.count;
            }
            if pid == p2 {
                c2 = e.count;
            }
        }
        assert_eq!(c1, 2);
        assert_eq!(c2, 2);
    }

    #[test]
    fn assign_release_roundtrip_counts_non_negative() {
        clear_all();
        let p = Principal::from_slice(&[77]);
        PartitionRegistry::register(p, 1);
        let item = Principal::from_slice(&[200]);
        assert!(PartitionRegistry::assign_item_to_partition(item, p).is_ok());
        assert!(PartitionRegistry::release_item(&item).is_ok());
        // extra release should error but not underflow
        assert!(PartitionRegistry::release_item(&item).is_err());
        let view = PartitionRegistry::export();
        let e = view.into_iter().find(|(pid, _)| *pid == p).unwrap().1;
        assert_eq!(e.count, 0);
    }

    #[test]
    fn auditor_resets_counts_from_backpointers() {
        clear_all();
        let p = Principal::from_slice(&[42]);
        PartitionRegistry::register(p, 10);
        let i1 = Principal::from_slice(&[101]);
        let i2 = Principal::from_slice(&[102]);
        PartitionRegistry::assign_item_to_partition(i1, p).unwrap();
        PartitionRegistry::assign_item_to_partition(i2, p).unwrap();

        // Tamper: set count to wrong value by re-inserting entry directly
        PARTITION_REGISTRY.with_borrow_mut(|core| {
            let mut e = core.get(&p).unwrap();
            e.count = 0;
            core.put(p, e);
        });

        PartitionRegistry::audit_and_fix_counts();
        let e = PartitionRegistry::export()
            .into_iter()
            .find(|(pid, _)| *pid == p)
            .unwrap()
            .1;
        assert_eq!(e.count, 2);
    }
}
