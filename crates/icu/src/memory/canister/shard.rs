use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_bounded,
    memory::{CanisterChildren, MemoryError, SHARD_REGISTRY_MEMORY_ID, SHARD_TENANT_MAP_MEMORY_ID},
    types::CanisterType,
    utils::time::now_secs,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// Shard Registry (State Layer)
//
// Purpose:
// --------
// Provides the raw, stable-memory mapping of shards and tenant assignments.
//
// - Registry: shard_id (Principal) → ShardEntry
// - Assignments: (pool, tenant) → shard_id
//
// Responsibilities:
// -----------------
// * Store shard metadata (capacity, count, pool, created_at)
// * Track tenant-to-shard assignments
// * Provide low-level CRUD for registry and assignments
// * Compute pool metrics and repair counts
//
// Non-responsibilities:
// ---------------------
// * Policy decisions (growth thresholds, balancing)
// * Logging or monitoring
// * Canister creation
//
// These belong in the ops layer (`ops::shard`).
//

thread_local! {
    static SHARD_CORE: RefCell<ShardCore<VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        ShardCore::new(
            BTreeMap::init(icu_register_memory!(SHARD_REGISTRY_MEMORY_ID)),
            BTreeMap::init(icu_register_memory!(SHARD_TENANT_MAP_MEMORY_ID)),
        )
    );
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

    #[error("tenant not found: {0}")]
    TenantNotFound(Principal),
}

impl From<ShardRegistryError> for Error {
    fn from(e: ShardRegistryError) -> Self {
        MemoryError::from(e).into()
    }
}

///
/// ShardRegistryKey
/// Composite key: (pool, tenant) → shard
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ShardRegistryKey {
    pub pool: String,
    pub tenant_pid: Principal,
}

impl ShardRegistryKey {
    pub const STORABLE_MAX_SIZE: u32 = 128;

    #[must_use]
    pub fn new(pool: &str, tenant_pid: Principal) -> Self {
        Self {
            pool: pool.to_string(),
            tenant_pid,
        }
    }
}

impl_storable_bounded!(ShardRegistryKey, ShardRegistryKey::STORABLE_MAX_SIZE, false);

///
/// PoolMetrics
///

#[derive(Clone, Copy, Debug)]
pub struct PoolMetrics {
    pub utilization_pct: u32,
    pub active_count: u32,
    pub total_capacity: u64,
    pub total_used: u64,
}

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
    const fn load_bps(self) -> Option<u64> {
        if self.capacity == 0 {
            None
        } else {
            Some((self.count as u64).saturating_mul(10_000) / self.capacity as u64)
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
    pub pool: String,
}

impl ShardEntry {
    pub const STORABLE_MAX_SIZE: u32 = 192;

    const fn metrics(&self) -> ShardMetrics {
        ShardMetrics {
            capacity: self.capacity,
            count: self.count,
        }
    }

    #[must_use]
    pub const fn has_capacity(&self) -> bool {
        self.metrics().has_capacity()
    }

    #[must_use]
    pub const fn load_bps(&self) -> Option<u64> {
        self.metrics().load_bps()
    }
}

impl_storable_bounded!(ShardEntry, ShardEntry::STORABLE_MAX_SIZE, false);

///
/// ShardRegistry
///

pub type ShardRegistryView = Vec<(Principal, ShardEntry)>;

pub struct ShardRegistry;

impl ShardRegistry {
    fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&ShardCore<VirtualMemory<DefaultMemoryImpl>>) -> R,
    {
        SHARD_CORE.with_borrow(f)
    }

    fn with_mut<F, R>(f: F) -> R
    where
        F: FnOnce(&mut ShardCore<VirtualMemory<DefaultMemoryImpl>>) -> R,
    {
        SHARD_CORE.with_borrow_mut(f)
    }

    /// Clears registry and assignments (test-only).
    pub fn clear() {
        Self::with_mut(|s| {
            s.registry.clear();
            s.assignments.clear();
        });
    }

    /// Returns the number of registered shards.
    #[must_use]
    pub fn count() -> u64 {
        Self::with(|s| s.registry.len())
    }

    /// Register or update a shard.
    pub fn register(shard_pid: Principal, pool: &str, capacity: u32) {
        Self::with_mut(|s| s.upsert(shard_pid, pool, capacity));
    }

    /// Remove a shard (must be empty).
    pub fn remove_shard(shard_pid: Principal) -> Result<(), Error> {
        Self::with_mut(|s| s.remove_shard(shard_pid))
    }

    /// Assign tenant explicitly to a shard.
    pub fn assign_tenant_to_shard(
        pool: &str,
        tenant: Principal,
        shard: Principal,
    ) -> Result<(), Error> {
        Self::with_mut(|s| s.assign_tenant_to_shard(pool, tenant, shard))
    }

    /// Assign tenant to any available shard (best effort).
    #[must_use]
    pub fn assign_tenant_best_effort(
        pool: &str,
        tenant: Principal,
        exclude: Option<Principal>,
    ) -> Option<Principal> {
        Self::with_mut(|s| s.assign_tenant_best_effort(pool, tenant, exclude))
    }

    /// Release tenant from current shard.
    pub fn release_tenant(pool: &str, tenant: Principal) -> Result<(), Error> {
        Self::with_mut(|s| s.release_tenant(pool, tenant))
    }

    /// Lookup shard for a tenant.
    #[must_use]
    pub fn get_tenant_shard(pool: &str, tenant: Principal) -> Option<Principal> {
        Self::with(|s| s.get_tenant_shard(pool, tenant))
    }

    /// Ensure tenant is in a valid shard (migrate if needed).
    #[must_use]
    pub fn ensure_tenant_migrated(pool: &str, tenant: Principal) -> Option<Principal> {
        Self::with_mut(|s| s.ensure_tenant_migrated(pool, tenant))
    }

    /// List all tenants in a shard.
    #[must_use]
    pub fn tenants_for_shard(pool: &str, shard: Principal) -> Vec<Principal> {
        Self::with(|s| s.tenants_for_shard(pool, shard))
    }

    /// Return latest created_at for shards of given type.
    #[must_use]
    pub fn last_created_at_for_type(ty: &CanisterType) -> u64 {
        Self::with(|s| s.last_created_at_for_type(ty))
    }

    /// Recompute counts from assignments.
    pub fn repair_counts() {
        Self::with_mut(ShardCore::repair_counts);
    }

    /// Peek candidate shard for a pool.
    #[must_use]
    pub fn peek_best_effort_for_pool(pool: &str) -> Option<Principal> {
        Self::with(|s| s.peek_best_effort_for_pool(pool))
    }

    /// Compute pool metrics.
    #[must_use]
    pub fn metrics_for_pool(pool: &str) -> PoolMetrics {
        let view = Self::export();
        let mut active = 0;
        let mut cap = 0;
        let mut used = 0;
        for (_, e) in &view {
            if e.capacity > 0 && e.pool == pool {
                active += 1;
                cap += u64::from(e.capacity);
                used += u64::from(e.count);
            }
        }
        let utilization = if cap == 0 {
            0
        } else {
            ((used * 100) / cap).min(100) as u32
        };

        PoolMetrics {
            utilization_pct: utilization,
            active_count: active,
            total_capacity: cap,
            total_used: used,
        }
    }

    /// Export view of all shards.
    pub fn export() -> ShardRegistryView {
        Self::with(ShardCore::export)
    }
}

///
/// ShardCore
///

struct ShardCore<M: Memory> {
    registry: BTreeMap<Principal, ShardEntry, M>,
    assignments: BTreeMap<ShardRegistryKey, Principal, M>,
}

impl<M: Memory> ShardCore<M> {
    const fn new(
        registry: BTreeMap<Principal, ShardEntry, M>,
        assignments: BTreeMap<ShardRegistryKey, Principal, M>,
    ) -> Self {
        Self {
            registry,
            assignments,
        }
    }

    fn get(&self, pid: &Principal) -> Option<ShardEntry> {
        self.registry.get(pid)
    }

    fn put(&mut self, pid: Principal, entry: ShardEntry) {
        self.registry.insert(pid, entry);
    }

    fn upsert(&mut self, pid: Principal, pool: &str, capacity: u32) {
        let mut entry = self.get(&pid).unwrap_or_default();
        let is_new = entry.created_at_secs == 0;
        entry.capacity = capacity;
        pool.clone_into(&mut entry.pool);
        if is_new && capacity > 0 {
            entry.created_at_secs = now_secs();
        }
        self.put(pid, entry);
    }

    fn remove_shard(&mut self, pid: Principal) -> Result<(), Error> {
        if let Some(entry) = self.get(&pid)
            && entry.count != 0
        {
            return Err(ShardRegistryError::ShardFull(pid).into());
        }

        self.registry.remove(&pid);

        Ok(())
    }

    fn get_tenant_shard(&self, pool: &str, tenant: Principal) -> Option<Principal> {
        self.assignments.get(&ShardRegistryKey::new(pool, tenant))
    }

    fn assign_tenant_to_shard(
        &mut self,
        pool: &str,
        tenant: Principal,
        shard: Principal,
    ) -> Result<(), Error> {
        if self.get_tenant_shard(pool, tenant) == Some(shard) {
            return Ok(());
        }
        let mut entry = self
            .get(&shard)
            .ok_or(ShardRegistryError::ShardNotFound(shard))?;
        if entry.pool != pool {
            return Err(ShardRegistryError::ShardNotFound(shard).into());
        }
        if !entry.has_capacity() {
            return Err(ShardRegistryError::ShardFull(shard).into());
        }
        if let Some(prev) = self.get_tenant_shard(pool, tenant)
            && prev != shard
        {
            self.release_tenant(pool, tenant)?;
        }

        self.assignments
            .insert(ShardRegistryKey::new(pool, tenant), shard);
        entry.count = entry.count.saturating_add(1);
        self.put(shard, entry);

        Ok(())
    }

    fn release_tenant(&mut self, pool: &str, tenant: Principal) -> Result<(), Error> {
        let key = ShardRegistryKey::new(pool, tenant);
        let shard = self
            .assignments
            .get(&key)
            .ok_or(ShardRegistryError::TenantNotFound(tenant))?;
        self.assignments.remove(&key);

        if let Some(mut entry) = self.get(&shard) {
            entry.count = entry.count.saturating_sub(1);
            self.put(shard, entry);
        }

        Ok(())
    }

    fn assign_tenant_best_effort(
        &mut self,
        pool: &str,
        tenant: Principal,
        exclude: Option<Principal>,
    ) -> Option<Principal> {
        if let Some(current) = self.get_tenant_shard(pool, tenant)
            && self.get(&current).is_some_and(|e| e.has_capacity())
            && exclude != Some(current)
        {
            return Some(current);
        }

        let candidate = self.pick_candidate(pool, exclude)?;
        self.assign_tenant_to_shard(pool, tenant, candidate).ok()?;

        Some(candidate)
    }

    fn ensure_tenant_migrated(&mut self, pool: &str, tenant: Principal) -> Option<Principal> {
        let current = self.get_tenant_shard(pool, tenant)?;
        let valid = self.get(&current).is_some_and(|e| e.has_capacity());
        if valid {
            Some(current)
        } else {
            self.assign_tenant_best_effort(pool, tenant, None)
        }
    }

    fn tenants_for_shard(&self, pool: &str, shard: Principal) -> Vec<Principal> {
        self.assignments
            .view()
            .filter(|(k, v)| v == &shard && k.pool == pool)
            .map(|(k, _)| k.tenant_pid)
            .collect()
    }

    fn last_created_at_for_type(&self, ty: &CanisterType) -> u64 {
        self.registry.iter().fold(0, |last, e| {
            let (pid, entry) = (*e.key(), e.value());
            if CanisterChildren::get(&pid).as_ref() == Some(ty) {
                last.max(entry.created_at_secs)
            } else {
                last
            }
        })
    }

    fn repair_counts(&mut self) {
        let mut counts = std::collections::BTreeMap::<Principal, u32>::new();
        for (_, shard) in self.assignments.view() {
            *counts.entry(shard).or_insert(0) += 1;
        }
        let mut all: Vec<Principal> = self.registry.iter().map(|e| *e.key()).collect();
        all.extend(counts.keys().copied());
        for pid in &all {
            let mut entry = self.get(pid).unwrap_or_default();
            entry.count = *counts.get(pid).unwrap_or(&0);
            self.put(*pid, entry);
        }
    }

    fn peek_best_effort_for_pool(&self, pool: &str) -> Option<Principal> {
        self.pick_candidate(pool, None)
    }

    fn pick_candidate(&self, pool: &str, exclude: Option<Principal>) -> Option<Principal> {
        self.registry
            .iter()
            .filter(|e| {
                e.value().has_capacity() && e.value().pool == pool && exclude != Some(*e.key())
            })
            .min_by_key(|e| {
                let m = e.value();
                (m.load_bps().unwrap_or(u64::MAX), m.count, m.created_at_secs)
            })
            .map(|e| *e.key())
    }

    fn export(&self) -> ShardRegistryView {
        self.registry
            .iter()
            .map(|e| (*e.key(), e.value()))
            .collect()
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
    fn register_and_export() {
        ShardRegistry::clear();
        let shard = p(1);
        ShardRegistry::register(shard, "pool1", 2);

        let view = ShardRegistry::export();
        assert_eq!(view.len(), 1);
        let (pid, entry) = &view[0];
        assert_eq!(*pid, shard);
        assert_eq!(entry.capacity, 2);
        assert_eq!(entry.pool, "pool1");
    }

    #[test]
    fn remove_shard_succeeds_if_empty() {
        ShardRegistry::clear();
        let shard = p(2);
        ShardRegistry::register(shard, "pool1", 1);
        assert!(ShardRegistry::remove_shard(shard).is_ok());
        assert_eq!(ShardRegistry::count(), 0);
    }

    #[test]
    fn peek_best_effort_returns_lowest_load() {
        ShardRegistry::clear();
        let shard1 = p(4);
        let shard2 = p(5);

        ShardRegistry::register(shard1, "poolX", 2);
        ShardRegistry::register(shard2, "poolX", 2);

        ShardRegistry::assign_tenant_to_shard("poolX", p(10), shard1).unwrap();
        // shard2 has lower load
        let candidate = ShardRegistry::peek_best_effort_for_pool("poolX");
        assert_eq!(candidate, Some(shard2));
    }
}
