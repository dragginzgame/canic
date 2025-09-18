use crate::{
    Error,
    cdk::structures::{
        BTreeMap, DefaultMemoryImpl,
        memory::{MemoryId, VirtualMemory},
    },
    impl_storable_unbounded,
    memory::{MEMORY_MANAGER, MEMORY_REGISTRY_MEMORY_ID, MemoryError},
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// MEMORY_REGISTRY
//

thread_local! {
    static MEMORY_REGISTRY: RefCell<MemoryRegistryCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(MemoryRegistryCore::new(BTreeMap::init(
            MEMORY_MANAGER.with_borrow(|this| {
                this.get(MemoryId::new(MEMORY_REGISTRY_MEMORY_ID))
            }),
        )));

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
        MEMORY_REGISTRY.with_borrow(MemoryRegistryCore::is_empty)
    }

    pub fn register(id: u8, entry: MemoryRegistryEntry) -> Result<(), Error> {
        MEMORY_REGISTRY.with_borrow_mut(|core| core.register(id, entry))
    }

    #[must_use]
    pub fn export() -> MemoryRegistryView {
        MEMORY_REGISTRY.with_borrow(MemoryRegistryCore::export)
    }
}

///
/// MemoryRegistryCore
///

pub struct MemoryRegistryCore<M: crate::cdk::structures::Memory> {
    map: BTreeMap<u8, MemoryRegistryEntry, M>,
}

impl<M: crate::cdk::structures::Memory> MemoryRegistryCore<M> {
    pub const fn new(map: BTreeMap<u8, MemoryRegistryEntry, M>) -> Self {
        Self { map }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn register(&mut self, id: u8, entry: MemoryRegistryEntry) -> Result<(), Error> {
        if id == MEMORY_REGISTRY_MEMORY_ID {
            Err(MemoryError::from(MemoryRegistryError::Reserved(id)))?;
        }

        if let Some(existing) = self.map.get(&id) {
            if existing.path != entry.path {
                Err(MemoryError::from(MemoryRegistryError::AlreadyRegistered(
                    id,
                    existing.path,
                    entry.path,
                )))?;
            }
            return Ok(()); // same path = idempotent
        }

        self.map.insert(id, entry);
        Ok(())
    }

    pub fn export(&self) -> MemoryRegistryView {
        self.map
            .iter()
            .map(|entry| (*entry.key(), entry.value()))
            .collect()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::structures::DefaultMemoryImpl;

    fn make_core() -> MemoryRegistryCore<DefaultMemoryImpl> {
        let map = BTreeMap::init(DefaultMemoryImpl::default());
        MemoryRegistryCore::new(map)
    }

    #[test]
    fn cannot_register_reserved_id() {
        let mut core = make_core();
        let result = core.register(
            MEMORY_REGISTRY_MEMORY_ID,
            MemoryRegistryEntry {
                path: "crate::Foo".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn can_register_valid_id() {
        let mut core = make_core();
        let result = core.register(
            1,
            MemoryRegistryEntry {
                path: "crate::Foo".to_string(),
            },
        );
        assert!(result.is_ok());
        assert!(!core.is_empty());
    }

    #[test]
    fn duplicate_same_path_is_ok() {
        let mut core = make_core();
        core.register(
            1,
            MemoryRegistryEntry {
                path: "crate::Foo".to_string(),
            },
        )
        .unwrap();

        // re-register with same path → ok
        let result = core.register(
            1,
            MemoryRegistryEntry {
                path: "crate::Foo".to_string(),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn duplicate_different_path_fails() {
        let mut core = make_core();
        core.register(
            1,
            MemoryRegistryEntry {
                path: "crate::Foo".to_string(),
            },
        )
        .unwrap();

        let result = core.register(
            1,
            MemoryRegistryEntry {
                path: "crate::Bar".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn export_contains_all_entries() {
        let mut core = make_core();
        core.register(
            1,
            MemoryRegistryEntry {
                path: "crate::Foo".to_string(),
            },
        )
        .unwrap();
        core.register(
            2,
            MemoryRegistryEntry {
                path: "crate::Bar".to_string(),
            },
        )
        .unwrap();

        let data = core.export();
        assert_eq!(data.len(), 2);
        assert!(
            data.iter()
                .any(|(id, e)| *id == 1 && e.path == "crate::Foo")
        );
        assert!(
            data.iter()
                .any(|(id, e)| *id == 2 && e.path == "crate::Bar")
        );
    }
}
