use crate::{
    cdk::{
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
        types::{BoundedString64, BoundedString128},
    },
    eager_static, ic_memory,
    storage::{prelude::*, stable::memory::placement::DIRECTORY_REGISTRY_ID},
};
use std::cell::RefCell;

eager_static! {
    static DIRECTORY_REGISTRY: RefCell<
        BTreeMap<DirectoryKey, DirectoryEntryRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(DirectoryRegistry, DIRECTORY_REGISTRY_ID)),
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
/// DirectoryRegistryRecord
///

#[derive(Clone, Debug)]
pub struct DirectoryRegistryRecord {
    pub entries: Vec<(DirectoryKey, DirectoryEntryRecord)>,
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
    pub(crate) fn export() -> DirectoryRegistryRecord {
        DirectoryRegistryRecord {
            entries: DIRECTORY_REGISTRY.with_borrow(|map| {
                map.iter()
                    .map(|entry| (entry.key().clone(), entry.value()))
                    .collect()
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn clear() {
        DIRECTORY_REGISTRY.with_borrow_mut(BTreeMap::clear);
    }
}
