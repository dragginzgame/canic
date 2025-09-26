mod metrics;
mod registry;

pub use metrics::*;
pub use registry::*;

use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_eager_static, icu_memory, impl_storable_bounded,
    memory::id::capability::{SHARD_REGISTRY_ID, SHARD_TENANTS_ID},
    types::CanisterType,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

// (this i) SHARD_CORE
icu_eager_static! {
    static SHARD_CORE: RefCell<ShardCore<VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        ShardCore::new(
            BTreeMap::init(icu_memory!(ShardRegistry, SHARD_REGISTRY_ID)),
            BTreeMap::init(icu_memory!(ShardRegistry, SHARD_TENANTS_ID)),
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

///
/// ShardKey
/// Composite key: (pool, tenant) → shard
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ShardKey {
    pub pool: String,
    pub tenant_pid: Principal,
}

impl ShardKey {
    pub const STORABLE_MAX_SIZE: u32 = 128;

    #[must_use]
    pub fn new(pool: &str, tenant_pid: Principal) -> Self {
        Self {
            pool: pool.to_string(),
            tenant_pid,
        }
    }
}

impl_storable_bounded!(ShardKey, ShardKey::STORABLE_MAX_SIZE, false);

///
/// ShardEntry
/// (bare-bones; policy like has_capacity is higher-level)
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShardEntry {
    pub capacity: u32,
    pub count: u32,
    pub created_at_secs: u64,
    pub pool: String,
    pub canister_type: CanisterType,
}

impl ShardEntry {
    pub const STORABLE_MAX_SIZE: u32 = 192;

    /// Whether this shard has room for more tenants.
    #[must_use]
    pub const fn has_capacity(&self) -> bool {
        self.count < self.capacity
    }

    /// Returns load in basis points (0–10_000), or `None` if capacity is 0.
    #[must_use]
    pub const fn load_bps(&self) -> Option<u64> {
        if self.capacity == 0 {
            None
        } else {
            Some((self.count as u64).saturating_mul(10_000) / self.capacity as u64)
        }
    }
}

impl_storable_bounded!(ShardEntry, ShardEntry::STORABLE_MAX_SIZE, false);

///
/// ShardCore
/// Registry + assignments
///

pub struct ShardCore<M: Memory> {
    registry: BTreeMap<Principal, ShardEntry, M>,
    assignments: BTreeMap<ShardKey, Principal, M>,
}

impl<M: Memory> ShardCore<M> {
    pub const fn new(
        registry: BTreeMap<Principal, ShardEntry, M>,
        assignments: BTreeMap<ShardKey, Principal, M>,
    ) -> Self {
        Self {
            registry,
            assignments,
        }
    }

    // ---------------------------
    // Registry CRUD
    // ---------------------------
    pub fn insert_entry(&mut self, pid: Principal, entry: ShardEntry) {
        self.registry.insert(pid, entry);
    }

    pub fn remove_entry(&mut self, pid: &Principal) -> Result<(), ShardRegistryError> {
        self.registry
            .remove(pid)
            .ok_or(ShardRegistryError::ShardNotFound(*pid))?;

        Ok(())
    }

    pub fn get_entry(&self, pid: &Principal) -> Option<ShardEntry> {
        self.registry.get(pid)
    }

    pub fn all_entries(&self) -> Vec<(Principal, ShardEntry)> {
        self.registry
            .iter()
            .map(|e| (*e.key(), e.value()))
            .collect()
    }

    // ---------------------------
    // Assignments CRUD
    // ---------------------------
    pub fn insert_assignment(&mut self, key: ShardKey, shard: Principal) {
        self.assignments.insert(key, shard);
    }

    pub fn remove_assignment(&mut self, key: &ShardKey) -> Result<Principal, ShardRegistryError> {
        self.assignments
            .remove(key)
            .ok_or(ShardRegistryError::TenantNotFound(key.tenant_pid))
    }

    pub fn get_assignment(&self, key: &ShardKey) -> Option<Principal> {
        self.assignments.get(key)
    }

    pub fn all_assignments(&self) -> Vec<(ShardKey, Principal)> {
        self.assignments
            .iter()
            .map(|e| (e.key().clone(), e.value()))
            .collect()
    }

    pub fn increment_count(&mut self, pid: Principal) -> Result<(), ShardRegistryError> {
        let mut entry = self
            .registry
            .get(&pid)
            .ok_or(ShardRegistryError::ShardNotFound(pid))?;

        entry.count = entry.count.saturating_add(1);
        self.registry.insert(pid, entry);

        Ok(())
    }

    pub fn decrement_count(&mut self, pid: Principal) -> Result<(), ShardRegistryError> {
        let mut entry = self
            .registry
            .get(&pid)
            .ok_or(ShardRegistryError::ShardNotFound(pid))?;

        entry.count = entry.count.saturating_sub(1);
        self.registry.insert(pid, entry);

        Ok(())
    }

    pub fn clear(&mut self) {
        self.registry.clear();
        self.assignments.clear();
    }
}
