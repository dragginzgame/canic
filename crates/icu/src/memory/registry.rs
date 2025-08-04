use crate::{
    Error,
    ic::structures::{BTreeMap, memory::MemoryId},
    impl_storable_unbounded,
    memory::{MEMORY_MANAGER, MEMORY_REGISTRY_MEMORY_ID, MemoryError},
};
use candid::CandidType;
use derive_more::Deref;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// MEMORY_REGISTRY
//

thread_local! {
    pub static MEMORY_REGISTRY: RefCell<BTreeMap<u8, MemoryRegistryEntry>> =
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
/// MemoryRegistry
///

pub struct MemoryRegistry {}

impl MemoryRegistry {
    pub fn with<R>(f: impl FnOnce(&BTreeMap<u8, MemoryRegistryEntry>) -> R) -> R {
        MEMORY_REGISTRY.with(|cell| f(&cell.borrow()))
    }

    pub fn with_mut<R>(f: impl FnOnce(&mut BTreeMap<u8, MemoryRegistryEntry>) -> R) -> R {
        MEMORY_REGISTRY.with(|cell| f(&mut cell.borrow_mut()))
    }

    pub fn register(id: u8, entry: MemoryRegistryEntry) -> Result<(), Error> {
        Self::with_mut(|map| {
            if id == MEMORY_REGISTRY_MEMORY_ID {
                Err(MemoryError::from(MemoryRegistryError::Reserved(id)))?;
            }

            if let Some(existing) = map.get(&id) {
                if existing.path != entry.path {
                    Err(MemoryError::from(MemoryRegistryError::AlreadyRegistered(
                        id,
                        existing.path,
                        entry.path,
                    )))?;
                }

                return Ok(());
            }

            map.insert(id, entry);

            Ok(())
        })
    }

    #[must_use]
    pub fn get_data() -> MemoryRegistryData {
        Self::with(|map| {
            let data = map
                .iter()
                .map(|entry| (*entry.key(), entry.value()))
                .collect();

            MemoryRegistryData(data)
        })
    }
}

///
/// MemoryRegistryEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MemoryRegistryEntry {
    pub path: String,
}

impl_storable_unbounded!(MemoryRegistryEntry);

///
/// MemoryRegistryData
///

#[derive(CandidType, Clone, Debug, Deref, Deserialize, Serialize)]
pub struct MemoryRegistryData(Vec<(u8, MemoryRegistryEntry)>);

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_register_zero() {
        let result = MemoryRegistry::register(
            0,
            MemoryRegistryEntry {
                path: "crate::Foo".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn can_register_valid_id() {
        let result = MemoryRegistry::register(
            1,
            MemoryRegistryEntry {
                path: "crate::Foo".to_string(),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn duplicate_same_path_is_ok() {
        MemoryRegistry::register(
            1,
            MemoryRegistryEntry {
                path: "crate::Foo".to_string(),
            },
        )
        .unwrap();

        let result = MemoryRegistry::register(
            1,
            MemoryRegistryEntry {
                path: "crate::Foo".to_string(),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn duplicate_different_path_fails() {
        MemoryRegistry::register(
            1,
            MemoryRegistryEntry {
                path: "crate::Foo".to_string(),
            },
        )
        .unwrap();

        let result = MemoryRegistry::register(
            1,
            MemoryRegistryEntry {
                path: "crate::Bar".to_string(),
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn registry_data_is_correct() {
        MemoryRegistry::register(
            1,
            MemoryRegistryEntry {
                path: "crate::Foo".to_string(),
            },
        )
        .unwrap();
        MemoryRegistry::register(
            2,
            MemoryRegistryEntry {
                path: "crate::Bar".to_string(),
            },
        )
        .unwrap();

        let data = MemoryRegistry::get_data();
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
