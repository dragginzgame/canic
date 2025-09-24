use crate::{
    Error,
    cdk::structures::{
        BTreeMap, DefaultMemoryImpl,
        memory::{MemoryId, VirtualMemory},
    },
    impl_storable_unbounded,
    memory::{MEMORY_MANAGER, MEMORY_REGISTRY_MEMORY_ID, MemoryError},
    utils::time::now_secs,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

// thread local
thread_local! {
    static MEMORY_REGISTRY: RefCell<BTreeMap<u8, MemoryRegistryEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(
            MEMORY_MANAGER.with_borrow(|this| {
                this.get(MemoryId::new(MEMORY_REGISTRY_MEMORY_ID))
            }),
        ));
}

///
/// MemoryRegistryError
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryError {
    #[error("ID {0} is already registered with type {1}, tried to register type {2}")]
    AlreadyRegistered(u8, String, String),

    #[error("memory id {0} is reserved")]
    Reserved(u8),
}

///
/// MemoryRegistryEntry
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryRegistryEntry {
    pub path: String,
    pub created_at: u64,
}

impl MemoryRegistryEntry {
    #[must_use]
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            created_at: now_secs(),
        }
    }
}

impl_storable_unbounded!(MemoryRegistryEntry);

///
/// MemoryRegistry
///

pub struct MemoryRegistry;

pub type MemoryRegistryView = Vec<(u8, MemoryRegistryEntry)>;

impl MemoryRegistry {
    #[must_use]
    pub fn is_empty() -> bool {
        MEMORY_REGISTRY.with_borrow(|map| map.is_empty())
    }

    #[must_use]
    pub const fn is_reserved(id: u8) -> bool {
        id == MEMORY_REGISTRY_MEMORY_ID
    }

    pub fn register(id: u8, path: &str) -> Result<(), Error> {
        MEMORY_REGISTRY.with_borrow_mut(|map| {
            if Self::is_reserved(id) {
                Err(MemoryError::from(MemoryRegistryError::Reserved(id)))?;
            }

            if let Some(existing) = map.get(&id) {
                if existing.path != path {
                    Err(MemoryError::from(MemoryRegistryError::AlreadyRegistered(
                        id,
                        existing.path,
                        path.to_string(),
                    )))?;
                }

                return Ok(()); // same path = idempotent
            }

            map.insert(id, MemoryRegistryEntry::new(path));

            Ok(())
        })
    }

    #[must_use]
    pub fn get(id: u8) -> Option<MemoryRegistryEntry> {
        MEMORY_REGISTRY.with_borrow(|map| map.get(&id))
    }

    #[must_use]
    pub fn export() -> MemoryRegistryView {
        MEMORY_REGISTRY.with_borrow(|map| {
            map.iter()
                .map(|entry| (*entry.key(), entry.value()))
                .collect()
        })
    }

    pub fn clear() {
        MEMORY_REGISTRY.with_borrow_mut(BTreeMap::clear);
    }
}
