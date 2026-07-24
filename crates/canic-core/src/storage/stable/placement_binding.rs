//! Placement-binding registry stable state.

use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::{
        structures::{DefaultMemoryImpl, memory::VirtualMemory},
        types::{BoundedString64, BoundedString128},
    },
    eager_static,
    role_contract::allocation::memory::placement::PLACEMENT_BINDING_REGISTRY_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

eager_static! {
    static PLACEMENT_BINDING_REGISTRY: RefCell<
        StableBtreeMap<PlacementBindingKey, PlacementBindingEntryRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.directory_registry.v1", ty = PlacementBindingRegistry, id = PLACEMENT_BINDING_REGISTRY_ID)),
    );
}

///
/// PlacementBindingKey
///

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PlacementBindingKey {
    pub pool: BoundedString64,
    pub key_value: BoundedString128,
}

impl PlacementBindingKey {
    pub const STORABLE_MAX_SIZE: u32 = 192;

    pub(crate) fn try_new(pool: &str, key_value: &str) -> Result<Self, String> {
        Ok(Self {
            pool: BoundedString64::try_from(pool).map_err(|err| err.to_string())?,
            key_value: BoundedString128::try_from(key_value).map_err(|err| err.to_string())?,
        })
    }
}

impl_storable_bounded!(
    PlacementBindingKey,
    PlacementBindingKey::STORABLE_MAX_SIZE,
    false
);

///
/// PlacementBindingRegistryEntryRecord
///
/// One logical binding-registry snapshot row.
///

#[derive(Clone, Debug)]
pub struct PlacementBindingRegistryEntryRecord {
    pub key: PlacementBindingKey,
    pub entry: PlacementBindingEntryRecord,
}

impl PlacementBindingRegistryEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "PlacementBindingRegistryEntryRecord";
}

///
/// PlacementBindingRegistryData
///
/// Canonical binding-registry export snapshot.
///

#[derive(Clone, Debug)]
pub struct PlacementBindingRegistryData {
    pub entries: Vec<PlacementBindingRegistryEntryRecord>,
}

impl PlacementBindingRegistryData {
    pub const STATE_CONTRACT_NAME: &'static str = "PlacementBindingRegistryData";
}

///
/// PlacementBindingEntryRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PlacementBindingEntryRecord {
    Pending {
        claim_id: u64,
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
}

impl PlacementBindingEntryRecord {
    pub const STORABLE_MAX_SIZE: u32 = 192;
}

impl_storable_bounded!(
    PlacementBindingEntryRecord,
    PlacementBindingEntryRecord::STORABLE_MAX_SIZE,
    false
);

///
/// PlacementBindingRegistry
///

pub struct PlacementBindingRegistry;

impl PlacementBindingRegistry {
    #[must_use]
    pub(crate) fn get(key: &PlacementBindingKey) -> Option<PlacementBindingEntryRecord> {
        PLACEMENT_BINDING_REGISTRY.with_borrow(|map| map.get(key))
    }

    pub(crate) fn insert(key: PlacementBindingKey, entry: PlacementBindingEntryRecord) {
        PLACEMENT_BINDING_REGISTRY.with_borrow_mut(|map| {
            map.insert(key, entry);
        });
    }

    #[must_use]
    pub(crate) fn remove(key: &PlacementBindingKey) -> Option<PlacementBindingEntryRecord> {
        PLACEMENT_BINDING_REGISTRY.with_borrow_mut(|map| map.remove(key))
    }

    #[must_use]
    pub(crate) fn export() -> PlacementBindingRegistryData {
        PlacementBindingRegistryData {
            entries: PLACEMENT_BINDING_REGISTRY.with_borrow(|map| {
                map.iter()
                    .map(|entry| PlacementBindingRegistryEntryRecord {
                        key: entry.key().clone(),
                        entry: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn clear() {
        PLACEMENT_BINDING_REGISTRY.with_borrow_mut(StableBtreeMap::clear_new);
    }
}
