use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::{
        structures::{DefaultMemoryImpl, memory::VirtualMemory},
        types::{BoundedString64, BoundedString128},
    },
    eager_static,
    role_contract::allocation::memory::placement::DIRECTORY_REGISTRY_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

eager_static! {
    static DIRECTORY_REGISTRY: RefCell<
        StableBtreeMap<DirectoryKey, DirectoryEntryRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.directory_registry.v1", ty = DirectoryRegistry, id = DIRECTORY_REGISTRY_ID)),
    );
}

///
/// DirectoryKey
///

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct DirectoryKey {
    pub pool: BoundedString64,
    pub key_value: BoundedString128,
}

impl DirectoryKey {
    pub const STORABLE_MAX_SIZE: u32 = 192;

    pub(crate) fn try_new(pool: &str, key_value: &str) -> Result<Self, String> {
        Ok(Self {
            pool: pool.try_into()?,
            key_value: key_value.try_into()?,
        })
    }
}

impl_storable_bounded!(DirectoryKey, DirectoryKey::STORABLE_MAX_SIZE, false);

///
/// DirectoryRegistryEntryRecord
///
/// One logical directory-registry snapshot row.
///

#[derive(Clone, Debug)]
pub struct DirectoryRegistryEntryRecord {
    pub key: DirectoryKey,
    pub entry: DirectoryEntryRecord,
}

impl DirectoryRegistryEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "DirectoryRegistryEntryRecord";
}

///
/// DirectoryRegistryData
///
/// Canonical directory-registry export snapshot.
///

#[derive(Clone, Debug)]
pub struct DirectoryRegistryData {
    pub entries: Vec<DirectoryRegistryEntryRecord>,
}

impl DirectoryRegistryData {
    pub const STATE_CONTRACT_NAME: &'static str = "DirectoryRegistryData";
}

///
/// DirectoryEntryRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DirectoryEntryRecord {
    Pending {
        #[serde(default)]
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

impl DirectoryEntryRecord {
    pub const STORABLE_MAX_SIZE: u32 = 192;
}

impl_storable_bounded!(
    DirectoryEntryRecord,
    DirectoryEntryRecord::STORABLE_MAX_SIZE,
    false
);

///
/// DirectoryRegistry
///

pub struct DirectoryRegistry;

impl DirectoryRegistry {
    #[must_use]
    pub(crate) fn get(key: &DirectoryKey) -> Option<DirectoryEntryRecord> {
        DIRECTORY_REGISTRY.with_borrow(|map| map.get(key))
    }

    pub(crate) fn insert(key: DirectoryKey, entry: DirectoryEntryRecord) {
        DIRECTORY_REGISTRY.with_borrow_mut(|map| {
            map.insert(key, entry);
        });
    }

    #[must_use]
    pub(crate) fn remove(key: &DirectoryKey) -> Option<DirectoryEntryRecord> {
        DIRECTORY_REGISTRY.with_borrow_mut(|map| map.remove(key))
    }

    #[must_use]
    pub(crate) fn export() -> DirectoryRegistryData {
        DirectoryRegistryData {
            entries: DIRECTORY_REGISTRY.with_borrow(|map| {
                map.iter()
                    .map(|entry| DirectoryRegistryEntryRecord {
                        key: entry.key().clone(),
                        entry: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn clear() {
        DIRECTORY_REGISTRY.with_borrow_mut(StableBtreeMap::clear_new);
    }
}
