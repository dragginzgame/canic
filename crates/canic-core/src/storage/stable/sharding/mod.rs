//! Module: storage::stable::sharding
//!
//! Responsibility: define sharding stable schemas and feature-gated storage cores.
//! Does not own: sharding policy, workflow orchestration, or endpoint DTOs.
//! Boundary: schema names remain available to the unconditional state descriptor registry.

#![cfg_attr(
    not(feature = "sharding"),
    expect(
        dead_code,
        reason = "sharding schema remains available to the unconditional state descriptor registry"
    )
)]

#[cfg(feature = "sharding")]
pub mod lifecycle;
#[cfg(feature = "sharding")]
pub mod registry;

#[cfg(feature = "sharding")]
use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
#[cfg(feature = "sharding")]
use crate::{
    cdk::structures::{DefaultMemoryImpl, Memory, memory::VirtualMemory},
    role_contract::allocation::memory::placement::{SHARDING_ASSIGNMENT_ID, SHARDING_REGISTRY_ID},
    storage::stable::sharding::registry::ShardingRegistry,
};
use crate::{
    cdk::types::{BoundedString64, BoundedString128},
    storage::prelude::*,
};
#[cfg(feature = "sharding")]
use std::cell::RefCell;

//
// SHARDING CORE
//

#[cfg(feature = "sharding")]
eager_static! {
    static SHARDING_CORE: RefCell<ShardingCore<VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        ShardingCore::new(
            StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.sharding_registry.v1", ty = ShardingRegistry, id = SHARDING_REGISTRY_ID)),
            StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.sharding_assignment.v1", ty = ShardingRegistry, id = SHARDING_ASSIGNMENT_ID)),
        )
    );
}

///
/// ShardKey
/// Composite key: (pool, partition_key) → shard
///

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ShardKey {
    pub pool: BoundedString64,
    pub partition_key: BoundedString128,
}

impl ShardKey {
    pub const STORABLE_MAX_SIZE: u32 = 192;

    #[cfg(feature = "sharding")]
    pub(crate) fn try_new(pool: &str, partition_key: &str) -> Result<Self, String> {
        Ok(Self {
            pool: pool.try_into()?,
            partition_key: partition_key.try_into()?,
        })
    }
}

impl_storable_bounded!(ShardKey, ShardKey::STORABLE_MAX_SIZE, false);

///
/// ShardEntryRecord
/// (bare-bones; policy like has_capacity is higher-level)
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShardEntryRecord {
    /// Logical slot index within the pool (assigned deterministically).
    #[serde(default = "ShardEntryRecord::slot_default")]
    pub slot: u32,
    pub capacity: u32,
    pub count: u32,
    pub pool: BoundedString64,
    pub canister_role: CanisterRole,
    pub created_at: u64,
}

impl ShardEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "ShardEntryRecord";
    pub const STORABLE_MAX_SIZE: u32 = 240;
    pub const UNASSIGNED_SLOT: u32 = u32::MAX;

    #[cfg(feature = "sharding")]
    pub(crate) fn try_new(
        pool: &str,
        slot: u32,
        role: CanisterRole,
        capacity: u32,
        created_at: u64,
    ) -> Result<Self, String> {
        let pool = BoundedString64::try_new(pool).map_err(|err| format!("pool name: {err}"))?;

        Ok(Self {
            slot,
            canister_role: role,
            capacity,
            count: 0,
            pool,
            created_at,
        })
    }

    const fn slot_default() -> u32 {
        Self::UNASSIGNED_SLOT
    }

    #[must_use]
    #[cfg(feature = "sharding")]
    pub const fn has_assigned_slot(&self) -> bool {
        self.slot != Self::UNASSIGNED_SLOT
    }
}

impl_storable_bounded!(ShardEntryRecord, ShardEntryRecord::STORABLE_MAX_SIZE, false);

///
/// ShardingRegistryEntryRecord
///
/// One logical sharding-registry snapshot row.
///

#[derive(Clone, Debug)]
pub struct ShardingRegistryEntryRecord {
    pub pid: Principal,
    pub entry: ShardEntryRecord,
}

///
/// ShardingAssignmentRecord
///
/// One logical partition-key assignment snapshot row.
///

#[derive(Clone, Debug)]
pub struct ShardingAssignmentRecord {
    pub key: ShardKey,
    pub shard: Principal,
}

impl ShardingAssignmentRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "ShardingAssignmentRecord";
}

///
/// ShardingRegistryData
///
/// Canonical sharding-registry export snapshot.
///

#[derive(Clone, Debug)]
pub struct ShardingRegistryData {
    pub entries: Vec<ShardingRegistryEntryRecord>,
}

impl ShardingRegistryData {
    pub const STATE_CONTRACT_NAME: &'static str = "ShardingRegistryData";
}

///
/// ShardingAssignmentsData
///
/// Canonical sharding-assignment export snapshot.
///

#[derive(Clone, Debug)]
pub struct ShardingAssignmentsData {
    pub entries: Vec<ShardingAssignmentRecord>,
}

impl ShardingAssignmentsData {
    pub const STATE_CONTRACT_NAME: &'static str = "ShardingAssignmentsData";
}

///
/// ShardingActiveSetRecord
///
/// One logical active-shard snapshot row.
///

#[derive(Clone, Debug)]
pub struct ShardingActiveSetRecord {
    pub pid: Principal,
}

impl ShardingActiveSetRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "ShardingActiveSetRecord";
}

///
/// ShardingActiveSetData
///
/// Canonical active-shard-set export snapshot.
///

#[derive(Clone, Debug)]
pub struct ShardingActiveSetData {
    pub entries: Vec<ShardingActiveSetRecord>,
}

impl ShardingActiveSetData {
    pub const STATE_CONTRACT_NAME: &'static str = "ShardingActiveSetData";
}

///
/// ShardingCore
/// Registry + assignments
///

#[cfg(feature = "sharding")]
pub struct ShardingCore<M: Memory> {
    registry: StableBtreeMap<Principal, ShardEntryRecord, M>,
    assignments: StableBtreeMap<ShardKey, Principal, M>,
}

#[cfg(feature = "sharding")]
impl<M: Memory> ShardingCore<M> {
    pub const fn new(
        registry: StableBtreeMap<Principal, ShardEntryRecord, M>,
        assignments: StableBtreeMap<ShardKey, Principal, M>,
    ) -> Self {
        Self {
            registry,
            assignments,
        }
    }

    // ---------------------------
    // Registry CRUD
    // ---------------------------

    pub fn insert_entry(&mut self, pid: Principal, entry: ShardEntryRecord) {
        self.registry.insert(pid, entry);
    }

    pub fn get_entry(&self, pid: &Principal) -> Option<ShardEntryRecord> {
        self.registry.get(pid)
    }

    pub fn all_entries(&self) -> Vec<ShardingRegistryEntryRecord> {
        self.registry
            .iter()
            .map(|entry| ShardingRegistryEntryRecord {
                pid: *entry.key(),
                entry: entry.value(),
            })
            .collect()
    }

    // ---------------------------
    // Assignments CRUD
    // ---------------------------

    pub fn insert_assignment(&mut self, key: ShardKey, shard: Principal) {
        self.assignments.insert(key, shard);
    }

    pub fn remove_assignment(&mut self, key: &ShardKey) -> Option<Principal> {
        self.assignments.remove(key)
    }

    pub fn get_assignment(&self, key: &ShardKey) -> Option<Principal> {
        self.assignments.get(key)
    }

    pub fn all_assignments(&self) -> Vec<ShardingAssignmentRecord> {
        self.assignments
            .iter()
            .map(|entry| ShardingAssignmentRecord {
                key: entry.key().clone(),
                shard: entry.value(),
            })
            .collect()
    }
}
