//! Placement-index registry stable state.

use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::{
        structures::{DefaultMemoryImpl, memory::VirtualMemory},
        types::{BoundedString64, BoundedString128},
    },
    eager_static,
    role_contract::allocation::memory::placement::PLACEMENT_INDEX_REGISTRY_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

eager_static! {
    static PLACEMENT_INDEX_REGISTRY: RefCell<
        StableBtreeMap<PlacementIndexKey, PlacementIndexEntryRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.placement_index.v1", ty = PlacementIndexRegistry, id = PLACEMENT_INDEX_REGISTRY_ID)),
    );
}

///
/// PlacementIndexKey
///

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PlacementIndexKey {
    pub pool: BoundedString64,
    pub key_value: BoundedString128,
}

impl PlacementIndexKey {
    pub const STORABLE_MAX_SIZE: u32 = 192;

    pub(crate) fn try_new(pool: &str, key_value: &str) -> Result<Self, String> {
        Ok(Self {
            pool: BoundedString64::try_from(pool).map_err(|err| err.to_string())?,
            key_value: BoundedString128::try_from(key_value).map_err(|err| err.to_string())?,
        })
    }
}

impl_storable_bounded!(
    PlacementIndexKey,
    PlacementIndexKey::STORABLE_MAX_SIZE,
    false
);

///
/// PlacementIndexRegistryEntryRecord
///
/// One logical index-registry snapshot row.
///

#[derive(Clone, Debug)]
pub struct PlacementIndexRegistryEntryRecord {
    pub key: PlacementIndexKey,
    pub entry: PlacementIndexEntryRecord,
}

impl PlacementIndexRegistryEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "PlacementIndexRegistryEntryRecord";
}

///
/// PlacementIndexRegistryData
///
/// Canonical index-registry export snapshot.
///

#[derive(Clone, Debug)]
pub struct PlacementIndexRegistryData {
    pub entries: Vec<PlacementIndexRegistryEntryRecord>,
}

impl PlacementIndexRegistryData {
    pub const STATE_CONTRACT_NAME: &'static str = "PlacementIndexRegistryData";
}

///
/// PlacementIndexEntryRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PlacementIndexEntryRecord {
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

impl PlacementIndexEntryRecord {
    pub const STORABLE_MAX_SIZE: u32 = 192;
}

impl_storable_bounded!(
    PlacementIndexEntryRecord,
    PlacementIndexEntryRecord::STORABLE_MAX_SIZE,
    false
);

///
/// PlacementIndexRegistry
///

pub struct PlacementIndexRegistry;

impl PlacementIndexRegistry {
    #[must_use]
    pub(crate) fn get(key: &PlacementIndexKey) -> Option<PlacementIndexEntryRecord> {
        PLACEMENT_INDEX_REGISTRY.with_borrow(|map| map.get(key))
    }

    pub(crate) fn insert(key: PlacementIndexKey, entry: PlacementIndexEntryRecord) {
        PLACEMENT_INDEX_REGISTRY.with_borrow_mut(|map| {
            map.insert(key, entry);
        });
    }

    #[must_use]
    pub(crate) fn remove(key: &PlacementIndexKey) -> Option<PlacementIndexEntryRecord> {
        PLACEMENT_INDEX_REGISTRY.with_borrow_mut(|map| map.remove(key))
    }

    #[must_use]
    pub(crate) fn export() -> PlacementIndexRegistryData {
        PlacementIndexRegistryData {
            entries: PLACEMENT_INDEX_REGISTRY.with_borrow(|map| {
                map.iter()
                    .map(|entry| PlacementIndexRegistryEntryRecord {
                        key: entry.key().clone(),
                        entry: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn clear() {
        PLACEMENT_INDEX_REGISTRY.with_borrow_mut(StableBtreeMap::clear_new);
    }
}
