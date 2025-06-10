use crate::{
    ic::structures::{BTreeMap, DefaultMemory},
    impl_storable_unbounded,
};
use candid::CandidType;
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// RegistryError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum RegistryError {
    #[error("ID {0} is already registered with type {1}, tried to register type {2}")]
    AlreadyRegistered(u8, String, String),

    #[error("memory id {0} is reserved")]
    Reserved(u8),
}

///
/// Registry
///

#[derive(Deref, DerefMut)]
pub struct Registry(BTreeMap<u8, RegistryEntry>);

impl Registry {
    #[must_use]
    pub fn init(memory: DefaultMemory) -> Self {
        Self(BTreeMap::init(memory))
    }

    #[must_use]
    pub fn get_data(&self) -> RegistryData {
        self.iter().collect()
    }

    pub fn register(&mut self, id: u8, entry: RegistryEntry) -> Result<(), RegistryError> {
        if id == 0 {
            Err(RegistryError::Reserved(id))?;
        }

        if let Some(existing) = self.get(&id) {
            if existing.path != entry.path {
                return Err(RegistryError::AlreadyRegistered(
                    id,
                    existing.path,
                    entry.path,
                ));
            }

            return Ok(());
        }

        self.insert(id, entry);

        Ok(())
    }
}

///
/// RegistryEntry
///

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub path: String,
}

impl_storable_unbounded!(RegistryEntry);

///
/// RegistryData
///

pub type RegistryData = Vec<(u8, RegistryEntry)>;

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MEMORY_REGISTRY;

    #[test]
    fn cannot_register_zero() {
        MEMORY_REGISTRY.with_borrow_mut(|registry| {
            let result = registry.register(
                0,
                RegistryEntry {
                    path: "crate::Foo".to_string(),
                },
            );
            assert!(matches!(result, Err(RegistryError::Reserved(0))));
        });
    }

    #[test]
    fn can_register_valid_id() {
        MEMORY_REGISTRY.with_borrow_mut(|registry| {
            let result = registry.register(
                1,
                RegistryEntry {
                    path: "crate::Foo".to_string(),
                },
            );
            assert!(result.is_ok());
        });
    }

    #[test]
    fn duplicate_same_path_is_ok() {
        MEMORY_REGISTRY.with_borrow_mut(|registry| {
            registry
                .register(
                    1,
                    RegistryEntry {
                        path: "crate::Foo".to_string(),
                    },
                )
                .unwrap();

            let result = registry.register(
                1,
                RegistryEntry {
                    path: "crate::Foo".to_string(),
                },
            );
            assert!(result.is_ok());
        });
    }

    #[test]
    fn duplicate_different_path_fails() {
        MEMORY_REGISTRY.with_borrow_mut(|registry| {
            registry
                .register(
                    1,
                    RegistryEntry {
                        path: "crate::Foo".to_string(),
                    },
                )
                .unwrap();

            let result = registry.register(
                1,
                RegistryEntry {
                    path: "crate::Bar".to_string(),
                },
            );

            match result {
                Err(RegistryError::AlreadyRegistered(id, old, new)) => {
                    assert_eq!(id, 1);
                    assert_eq!(old, "crate::Foo");
                    assert_eq!(new, "crate::Bar");
                }
                other => panic!("Unexpected result: {other:?}"),
            }
        });
    }

    #[test]
    fn registry_data_is_correct() {
        MEMORY_REGISTRY.with_borrow_mut(|registry| {
            registry
                .register(
                    1,
                    RegistryEntry {
                        path: "crate::Foo".to_string(),
                    },
                )
                .unwrap();
            registry
                .register(
                    2,
                    RegistryEntry {
                        path: "crate::Bar".to_string(),
                    },
                )
                .unwrap();

            let data = registry.get_data();
            assert_eq!(data.len(), 2);
            assert!(
                data.iter()
                    .any(|(id, e)| *id == 1 && e.path == "crate::Foo")
            );
            assert!(
                data.iter()
                    .any(|(id, e)| *id == 2 && e.path == "crate::Bar")
            );
        });
    }
}
