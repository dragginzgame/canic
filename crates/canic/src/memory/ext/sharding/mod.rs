mod metrics;
mod registry;

pub use metrics::*;
pub use registry::*;

use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    eager_static, ic_memory, impl_storable_bounded,
    memory::{
        MemoryError,
        ext::ExtensionError,
        id::ext::sharding::{SHARDING_ASSIGNMENT_ID, SHARDING_REGISTRY_ID},
    },
    types::{BoundedString32, BoundedString128, CanisterType, Principal},
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// SHARDING CORE
//

eager_static! {
    static SHARDING_CORE: RefCell<ShardingCore<VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        ShardingCore::new(
            BTreeMap::init(ic_memory!(ShardingRegistry, SHARDING_REGISTRY_ID)),
            BTreeMap::init(ic_memory!(ShardingRegistry, SHARDING_ASSIGNMENT_ID)),
        )
    );
}

///
/// ShardingError
///

#[derive(Debug, ThisError)]
pub enum ShardingError {
    #[error("shard not found: {0}")]
    ShardNotFound(Principal),

    #[error("shard full: {0}")]
    ShardFull(Principal),

    #[error("tenant not found: {0}")]
    TenantNotFound(String),
}

impl From<ShardingError> for Error {
    fn from(err: ShardingError) -> Self {
        MemoryError::from(ExtensionError::from(err)).into()
    }
}

///
/// ShardKey
/// Composite key: (pool, tenant) → shard
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ShardKey {
    pub pool: BoundedString32,
    pub tenant: BoundedString128,
}

impl ShardKey {
    pub const STORABLE_MAX_SIZE: u32 = 160;

    #[must_use]
    pub fn new(pool: &str, tenant: &str) -> Self {
        Self {
            pool: pool.try_into().unwrap(),
            tenant: tenant.try_into().unwrap(),
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
    /// Logical slot index within the pool (assigned deterministically).
    #[serde(default = "ShardEntry::slot_default")]
    pub slot: u32,
    pub capacity: u32,
    pub count: u32,
    pub pool: String,
    pub canister_type: CanisterType,
    pub created_at: u64,
}

impl ShardEntry {
    pub const STORABLE_MAX_SIZE: u32 = 208;
    pub const UNASSIGNED_SLOT: u32 = u32::MAX;

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

    #[inline]
    const fn slot_default() -> u32 {
        Self::UNASSIGNED_SLOT
    }

    #[must_use]
    pub const fn has_assigned_slot(&self) -> bool {
        self.slot != Self::UNASSIGNED_SLOT
    }
}

impl_storable_bounded!(ShardEntry, ShardEntry::STORABLE_MAX_SIZE, false);

///
/// ShardingCore
/// Registry + assignments
///

pub struct ShardingCore<M: Memory> {
    registry: BTreeMap<Principal, ShardEntry, M>,
    assignments: BTreeMap<ShardKey, Principal, M>,
}

impl<M: Memory> ShardingCore<M> {
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

    pub fn remove_entry(&mut self, pid: &Principal) -> Result<(), Error> {
        if self.registry.remove(pid).is_none() {
            return Err(ShardingError::ShardNotFound(*pid).into());
        }

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

    pub fn remove_assignment(&mut self, key: &ShardKey) -> Result<Principal, Error> {
        self.assignments
            .remove(key)
            .ok_or_else(|| ShardingError::TenantNotFound(key.tenant.to_string()).into())
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

    pub fn increment_count(&mut self, pid: Principal) -> Result<(), Error> {
        let mut entry = self
            .registry
            .get(&pid)
            .ok_or(ShardingError::ShardNotFound(pid))?;

        entry.count = entry.count.saturating_add(1);
        self.registry.insert(pid, entry);

        Ok(())
    }

    pub fn decrement_count(&mut self, pid: Principal) -> Result<(), Error> {
        let mut entry = self
            .registry
            .get(&pid)
            .ok_or(ShardingError::ShardNotFound(pid))?;

        entry.count = entry.count.saturating_sub(1);
        self.registry.insert(pid, entry);

        Ok(())
    }

    pub fn clear(&mut self) {
        self.registry.clear();
        self.assignments.clear();
    }
}
