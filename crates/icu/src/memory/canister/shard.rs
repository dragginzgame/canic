use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_bounded,
    memory::{CanisterChildren, MemoryError, SHARD_ITEM_MAP_MEMORY_ID, SHARD_REGISTRY_MEMORY_ID},
    types::CanisterType,
    utils::time::now_secs,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// SHARD REGISTRY
//
// Generic registry: assigns items (Principals) to shard canisters (Principals)
// with capacity accounting.
//

thread_local! {
    static SHARD_REGISTRY: RefCell<CanisterShardRegistryCore<VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        CanisterShardRegistryCore::new(BTreeMap::init(icu_register_memory!(SHARD_REGISTRY_MEMORY_ID))));

    static SHARD_ITEM_MAP: RefCell<BTreeMap<ItemKey, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(icu_register_memory!(SHARD_ITEM_MAP_MEMORY_ID)));
}

///
/// ShardRegistryError
///

#[derive(Debug, ThisError)]
pub enum ShardRegistryError {
    #[error("shard not found: {0}")]
    ShardNotFound(Principal),

    #[error("shard full: {0}")]
    ShardFull(Principal),

    #[error("item not found: {0}")]
    ItemNotFound(Principal),
}

///
/// PoolName
/// Identifier for a shard pool (scoped under a parent hub).

#[derive(
    CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Ord, PartialOrd, Hash,
)]
pub struct PoolName(pub String);

impl PoolName {
    pub const STORABLE_MAX_SIZE: u32 = 64;
}

impl From<&str> for PoolName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl_storable_bounded!(PoolName, PoolName::STORABLE_MAX_SIZE, false);

///
/// ItemKey
/// Composite key to store per-pool item assignments.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ItemKey {
    pub item: Principal,
    pub pool: PoolName,
}

impl ItemKey {
    pub const STORABLE_MAX_SIZE: u32 = 128;
}

// Allow sufficient headroom for serde encoding overhead
impl_storable_bounded!(ItemKey, ItemKey::STORABLE_MAX_SIZE, false);

///
/// ShardMetrics
///

#[derive(Clone, Copy, Debug)]
struct ShardMetrics {
    capacity: u32,
    count: u32,
}

impl ShardMetrics {
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
/// ShardEntry
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShardEntry {
    pub capacity: u32,
    pub count: u32,
    pub created_at_secs: u64,
    pub pool: Option<PoolName>,
}

impl ShardEntry {
    const STORABLE_MAX_SIZE: u32 = 192;

    const fn metrics(&self) -> ShardMetrics {
        ShardMetrics {
            capacity: self.capacity,
            count: self.count,
        }
    }

    /// Return true if this shard has remaining capacity.
    #[must_use]
    pub const fn has_capacity(&self) -> bool {
        self.metrics().has_capacity()
    }

    /// Load as basis points (0..=10000). Zero capacity is treated as max load.
    #[must_use]
    pub const fn load_bps(&self) -> u64 {
        self.metrics().load_bps()
    }
}

// Allow sufficient headroom for serde encoding overhead
impl_storable_bounded!(ShardEntry, ShardEntry::STORABLE_MAX_SIZE, false);

///
/// CanisterShardRegistry
///

pub type CanisterShardRegistryView = Vec<(Principal, ShardEntry)>;

pub struct CanisterShardRegistry;

impl CanisterShardRegistry {
    #[inline]
    fn item_key(item: Principal, pool: &PoolName) -> ItemKey {
        ItemKey {
            item,
            pool: pool.clone(),
        }
    }
    /// List items currently assigned to a specific shard within a pool.
    #[must_use]
    pub fn items_for_shard(pool: &PoolName, shard_pid: Principal) -> Vec<Principal> {
        let mut items = Vec::new();
        SHARD_ITEM_MAP.with_borrow(|map| {
            for (key, part) in map.view() {
                if part == shard_pid && key.pool == *pool {
                    items.push(key.item);
                }
            }
        });
        items
    }

    /// Remove a shard from the registry. Requires it to be empty (count == 0).
    pub fn remove_shard(shard_pid: Principal) -> Result<(), Error> {
        SHARD_REGISTRY.with_borrow_mut(|core| {
            let Some(entry) = core.get(&shard_pid) else {
                return Ok(()); // already gone
            };
            if entry.count != 0 {
                return Err(MemoryError::from(ShardRegistryError::ShardFull(shard_pid)).into());
            }
            core.remove(&shard_pid);
            Ok(())
        })
    }

    /// Best-effort assignment excluding a specific shard (e.g., when draining it).
    #[must_use]
    pub fn assign_item_best_effort_excluding(
        item: Principal,
        pool: &PoolName,
        exclude: Principal,
    ) -> Option<Principal> {
        // Already assigned to a valid shard different from exclude? keep it
        if let Some(current) = Self::get_item_partition(&item, pool)
            && current != exclude
        {
            let valid = SHARD_REGISTRY
                .with_borrow(|core| core.get(&current).is_some_and(|e| e.has_capacity()));
            if valid {
                return Some(current);
            }
        }

        // Pick a candidate excluding the provided shard
        let pid = SHARD_REGISTRY
            .with_borrow(|core| core.pick_least_loaded_of_pool_excluding(pool, &exclude))?;
        Self::assign_item_to_partition(item, pool, pid).ok()?;
        Some(pid)
    }
    pub fn register(shard_pid: Principal, pool: PoolName, capacity: u32) {
        SHARD_REGISTRY.with_borrow_mut(|core| {
            core.upsert(shard_pid, pool, capacity);
        });
    }

    /// Assign item to any shard with available capacity for the given type (least-loaded).
    #[must_use]
    pub fn assign_item_best_effort(item: Principal, pool: &PoolName) -> Option<Principal> {
        // Already assigned? Keep if still valid
        if let Some(current) = Self::get_item_partition(&item, pool) {
            let valid = SHARD_REGISTRY.with_borrow(|core| {
                core.get(&current)
                    .is_some_and(|e| e.metrics().has_capacity())
            });
            if valid {
                return Some(current);
            }
        }

        let pid = SHARD_REGISTRY.with_borrow(|core| core.pick_least_loaded_of_pool(pool))?;
        Self::assign_item_to_partition(item, pool, pid).ok()?;

        Some(pid)
    }

    /// Assign item to a specific shard for the given type (idempotent).
    pub fn assign_item_to_partition(
        item: Principal,
        pool: &PoolName,
        shard_pid: Principal,
    ) -> Result<(), Error> {
        // No-op if already assigned
        if Self::get_item_partition(&item, pool) == Some(shard_pid) {
            return Ok(());
        }

        let mut entry = Self::try_get_entry(shard_pid)?;
        if entry.pool.as_ref() != Some(pool) {
            return Err(MemoryError::from(ShardRegistryError::ShardNotFound(shard_pid)).into());
        }
        if !entry.has_capacity() {
            return Err(MemoryError::from(ShardRegistryError::ShardFull(shard_pid)).into());
        }

        // If assigned elsewhere (for this pool), release first
        if let Some(prev) = Self::get_item_partition(&item, pool)
            && prev != shard_pid
        {
            Self::release_item(&item, pool)?;
        }

        // Insert mapping and increment count
        let key = Self::item_key(item, pool);
        SHARD_ITEM_MAP.with_borrow_mut(|map| {
            map.insert(key, shard_pid);
        });
        entry.count = entry.count.saturating_add(1);
        SHARD_REGISTRY.with_borrow_mut(|core| core.put(shard_pid, entry));

        Ok(())
    }

    /// Release an item from its shard (decrement count) for the given type.
    pub fn release_item(item: &Principal, pool: &PoolName) -> Result<(), Error> {
        let key = Self::item_key(*item, pool);
        let pid = SHARD_ITEM_MAP
            .with_borrow(|map| map.get(&key))
            .ok_or_else(|| MemoryError::from(ShardRegistryError::ItemNotFound(*item)))?;

        SHARD_ITEM_MAP.with_borrow_mut(|map| {
            map.remove(&key);
        });

        SHARD_REGISTRY.with_borrow_mut(|core| {
            if let Some(mut e) = core.get(&pid) {
                e.count = e.count.saturating_sub(1);
                core.put(pid, e);
            }
        });

        Ok(())
    }

    #[must_use]
    pub fn get_item_partition(item: &Principal, pool: &PoolName) -> Option<Principal> {
        let key = Self::item_key(*item, pool);
        SHARD_ITEM_MAP.with_borrow(|map| map.get(&key))
    }

    fn try_get_entry(pid: Principal) -> Result<ShardEntry, Error> {
        SHARD_REGISTRY
            .with_borrow(|reg| reg.get(&pid))
            .ok_or_else(|| MemoryError::from(ShardRegistryError::ShardNotFound(pid)).into())
    }

    /// Ensure current shard is still valid; migrate if necessary.
    #[must_use]
    pub fn ensure_item_migrated(item: Principal, pool: &PoolName) -> Option<Principal> {
        let current = Self::get_item_partition(&item, pool)?;
        let valid =
            SHARD_REGISTRY.with_borrow(|core| core.get(&current).is_some_and(|e| e.has_capacity()));
        if !valid {
            return Self::assign_item_best_effort(item, pool);
        }

        Some(current)
    }

    /// Most recent creation timestamp across shards of a given type.
    #[must_use]
    pub fn last_created_at_for_type(ty: &CanisterType) -> u64 {
        SHARD_REGISTRY.with_borrow(|core| {
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

    /// Auditor: recompute counts by scanning ITEM_TO_SHARD and writing back.
    pub fn audit_and_fix_counts() {
        let mut counts = std::collections::BTreeMap::<Principal, u32>::new();
        SHARD_ITEM_MAP.with_borrow(|map| {
            for (_key, part) in map.view() {
                *counts.entry(part).or_insert(0) += 1;
            }
        });

        SHARD_REGISTRY.with_borrow_mut(|core| {
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
    pub fn peek_best_effort_for_pool(pool: &PoolName) -> Option<Principal> {
        SHARD_REGISTRY.with_borrow(|core| core.pick_least_loaded_of_pool(pool))
    }

    #[must_use]
    pub fn export() -> CanisterShardRegistryView {
        SHARD_REGISTRY.with_borrow(CanisterShardRegistryCore::export)
    }
}

///
/// CanisterShardRegistryCore
///

struct CanisterShardRegistryCore<M: Memory> {
    partitions: BTreeMap<Principal, ShardEntry, M>,
}

impl<M: Memory> CanisterShardRegistryCore<M> {
    pub const fn new(map: BTreeMap<Principal, ShardEntry, M>) -> Self {
        Self { partitions: map }
    }

    fn get(&self, pid: &Principal) -> Option<ShardEntry> {
        self.partitions.get(pid)
    }

    fn put(&mut self, pid: Principal, entry: ShardEntry) {
        self.partitions.insert(pid, entry);
    }

    fn remove(&mut self, pid: &Principal) -> Option<ShardEntry> {
        self.partitions.remove(pid)
    }

    fn upsert(&mut self, pid: Principal, pool: PoolName, capacity: u32) {
        let mut entry = self.get(&pid).unwrap_or_default();
        let is_new = entry.capacity == 0 && entry.count == 0 && entry.created_at_secs == 0;
        entry.capacity = capacity;
        entry.pool = Some(pool);
        if is_new && capacity > 0 {
            entry.created_at_secs = now_secs();
        }
        self.put(pid, entry);
    }

    fn export(&self) -> CanisterShardRegistryView {
        self.partitions
            .iter()
            .map(|e| (*e.key(), e.value()))
            .collect()
    }

    fn pick_least_loaded_of_pool(&self, pool: &PoolName) -> Option<Principal> {
        self.partitions
            .iter()
            .filter(|e| e.value().has_capacity() && e.value().pool.as_ref() == Some(pool))
            .min_by_key(|e| (e.value().load_bps(), e.value().count))
            .map(|e| *e.key())
    }

    fn pick_least_loaded_of_pool_excluding(
        &self,
        pool: &PoolName,
        exclude: &Principal,
    ) -> Option<Principal> {
        self.partitions
            .iter()
            .filter(|e| {
                e.value().has_capacity()
                    && e.value().pool.as_ref() == Some(pool)
                    && e.key() != exclude
            })
            .min_by_key(|e| (e.value().load_bps(), e.value().count))
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

        let index_key = ShardEntry::STORABLE_MAX_SIZE;
        let size = Storable::to_bytes(&index_key).len();

        assert!(size <= ShardEntry::STORABLE_MAX_SIZE as usize);
    }

    fn clear_all() {
        SHARD_ITEM_MAP.with_borrow_mut(BTreeMap::clear);
        SHARD_REGISTRY.with_borrow_mut(|c| c.partitions.clear());
    }

    #[test]
    fn best_effort_distributes_to_least_loaded() {
        clear_all();
        let p1 = Principal::from_slice(&[1]);
        let p2 = Principal::from_slice(&[2]);
        // Register pool for selection
        let pool = PoolName::from("alpha");

        // Preload p2 with 2 items
        let i0 = Principal::from_slice(&[10]);
        let i1 = Principal::from_slice(&[11]);
        CanisterShardRegistry::register(p1, pool.clone(), 3);
        CanisterShardRegistry::register(p2, pool.clone(), 3);
        CanisterShardRegistry::assign_item_to_partition(i0, &pool, p2).unwrap();
        CanisterShardRegistry::assign_item_to_partition(i1, &pool, p2).unwrap();

        // Now best-effort should prefer p1 until balanced
        let i2 = Principal::from_slice(&[12]);
        let i3 = Principal::from_slice(&[13]);
        let a2 = CanisterShardRegistry::assign_item_best_effort(i2, &pool).unwrap();
        let a3 = CanisterShardRegistry::assign_item_best_effort(i3, &pool).unwrap();
        assert_eq!(a2, p1);
        assert_eq!(a3, p1);

        // After balancing: counts should be equal (2 and 2)
        let view = CanisterShardRegistry::export();
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
        let pool = PoolName::from("alpha");
        CanisterShardRegistry::register(p, pool.clone(), 1);
        let item = Principal::from_slice(&[200]);
        assert!(CanisterShardRegistry::assign_item_to_partition(item, &pool, p).is_ok());
        assert!(CanisterShardRegistry::release_item(&item, &pool).is_ok());
        // extra release should error but not underflow
        assert!(CanisterShardRegistry::release_item(&item, &pool).is_err());
        let view = CanisterShardRegistry::export();
        let e = view.into_iter().find(|(pid, _)| *pid == p).unwrap().1;
        assert_eq!(e.count, 0);
    }

    #[test]
    fn auditor_resets_counts_from_backpointers() {
        clear_all();
        let p = Principal::from_slice(&[42]);
        let pool = PoolName::from("alpha");
        CanisterShardRegistry::register(p, pool.clone(), 10);
        let i1 = Principal::from_slice(&[101]);
        let i2 = Principal::from_slice(&[102]);
        CanisterShardRegistry::assign_item_to_partition(i1, &pool, p).unwrap();
        CanisterShardRegistry::assign_item_to_partition(i2, &pool, p).unwrap();

        // Tamper: set count to wrong value by re-inserting entry directly
        SHARD_REGISTRY.with_borrow_mut(|core| {
            let mut e = core.get(&p).unwrap();
            e.count = 0;
            core.put(p, e);
        });

        CanisterShardRegistry::audit_and_fix_counts();
        let e = CanisterShardRegistry::export()
            .into_iter()
            .find(|(pid, _)| *pid == p)
            .unwrap()
            .1;
        assert_eq!(e.count, 2);
    }

    #[test]
    fn list_items_for_shard_and_remove() {
        clear_all();
        let pool = PoolName::from("bravo");
        let shard = Principal::from_slice(&[5]);
        CanisterShardRegistry::register(shard, pool.clone(), 3);

        let a = Principal::from_slice(&[100]);
        let b = Principal::from_slice(&[101]);
        CanisterShardRegistry::assign_item_to_partition(a, &pool, shard).unwrap();
        CanisterShardRegistry::assign_item_to_partition(b, &pool, shard).unwrap();

        let items = CanisterShardRegistry::items_for_shard(&pool, shard);
        assert_eq!(items.len(), 2);
        assert!(items.contains(&a) && items.contains(&b));

        // cannot remove while non-empty
        assert!(CanisterShardRegistry::remove_shard(shard).is_err());

        // release items then remove
        CanisterShardRegistry::release_item(&a, &pool).unwrap();
        CanisterShardRegistry::release_item(&b, &pool).unwrap();
        assert!(CanisterShardRegistry::remove_shard(shard).is_ok());
    }

    #[test]
    fn best_effort_excluding_picks_other() {
        clear_all();
        let pool = PoolName::from("charlie");
        let a = Principal::from_slice(&[11]);
        let b = Principal::from_slice(&[12]);
        CanisterShardRegistry::register(a, pool.clone(), 2);
        CanisterShardRegistry::register(b, pool.clone(), 2);

        let it = Principal::from_slice(&[200]);
        // Exclude a → should pick b
        let picked = CanisterShardRegistry::assign_item_best_effort_excluding(it, &pool, a)
            .expect("should assign");
        assert_eq!(picked, b);
    }
}
