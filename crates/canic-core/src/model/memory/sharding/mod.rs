mod registry;

pub use registry::*;

use crate::{
    cdk::{
        structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
        types::Principal,
    },
    eager_static, ic_memory,
    ids::CanisterRole,
    memory::impl_storable_bounded,
    model::memory::id::sharding::{SHARDING_ASSIGNMENT_ID, SHARDING_REGISTRY_ID},
    types::{BoundedString32, BoundedString128},
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

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

    pub(crate) fn try_new(pool: &str, tenant: &str) -> Result<Self, String> {
        Ok(Self {
            pool: pool.try_into()?,
            tenant: tenant.try_into()?,
        })
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
    pub pool: BoundedString32,
    pub canister_type: CanisterRole,
    pub created_at: u64,
}

impl ShardEntry {
    pub const STORABLE_MAX_SIZE: u32 = 208;
    pub const UNASSIGNED_SLOT: u32 = u32::MAX;

    pub(crate) fn try_new(
        pool: &str,
        slot: u32,
        ty: CanisterRole,
        capacity: u32,
        created_at: u64,
    ) -> Result<Self, String> {
        let pool = BoundedString32::try_new(pool).map_err(|err| format!("pool name: {err}"))?;

        Ok(Self {
            slot,
            canister_type: ty,
            capacity,
            count: 0,
            pool,
            created_at,
        })
    }

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

    pub(crate) fn insert_entry(&mut self, pid: Principal, entry: ShardEntry) {
        self.registry.insert(pid, entry);
    }

    pub(crate) fn get_entry(&self, pid: &Principal) -> Option<ShardEntry> {
        self.registry.get(pid)
    }

    pub(crate) fn all_entries(&self) -> Vec<(Principal, ShardEntry)> {
        self.registry
            .iter()
            .map(|e| (*e.key(), e.value()))
            .collect()
    }

    // ---------------------------
    // Assignments CRUD
    // ---------------------------

    pub(crate) fn insert_assignment(&mut self, key: ShardKey, shard: Principal) {
        self.assignments.insert(key, shard);
    }

    pub(crate) fn remove_assignment(&mut self, key: &ShardKey) -> Option<Principal> {
        self.assignments.remove(key)
    }

    pub(crate) fn get_assignment(&self, key: &ShardKey) -> Option<Principal> {
        self.assignments.get(key)
    }

    pub(crate) fn all_assignments(&self) -> Vec<(ShardKey, Principal)> {
        self.assignments
            .iter()
            .map(|e| (e.key().clone(), e.value()))
            .collect()
    }
}
