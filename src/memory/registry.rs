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

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError, Clone)]
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

    /// Returns the next available memory ID
    ///
    /// This function finds the smallest unused ID starting from 1,
    /// as 0 is reserved for the registry itself.
    #[must_use]
    pub fn next_available_id(&self) -> u8 {
        // Get a sorted list of keys already in use
        let mut used_ids = Vec::new();
        let iter = self.0.iter();
        for entry in iter {
            // Extract the key directly
            used_ids.push(entry.0);
        }
        used_ids.sort();

        // Find the first gap starting from 1
        let mut next_id = 1; // Start at 1 since 0 is reserved
        for id in used_ids {
            if id == next_id {
                next_id += 1;
            } else if id > next_id {
                break;
            }
        }

        next_id
    }

    /// Register a memory area with a specific ID
    pub fn register(&mut self, id: u8, entry: RegistryEntry) -> Result<u8, RegistryError> {
        if id == 0 {
            return Err(RegistryError::Reserved(id));
        }

        if let Some(existing) = self.get(&id) {
            if existing.path != entry.path {
                return Err(RegistryError::AlreadyRegistered(
                    id,
                    existing.path.clone(),
                    entry.path,
                ));
            }

            return Ok(id);
        }

        self.insert(id, entry);

        Ok(id)
    }

    /// Register a memory area with an auto-assigned ID
    ///
    /// This function automatically finds the next available ID and uses it
    /// for registration. It returns the assigned ID on success.
    pub fn register_auto(&mut self, entry: RegistryEntry) -> Result<u8, RegistryError> {
        let id = self.next_available_id();
        self.register(id, entry)
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
            assert_eq!(result.unwrap(), 1);
        });
    }

    #[test]
    fn duplicate_same_path_is_ok() {
        MEMORY_REGISTRY.with_borrow_mut(|registry| {
            let result1 = registry.register(
                1,
                RegistryEntry {
                    path: "crate::Foo".to_string(),
                },
            );
            assert!(result1.is_ok());

            let result2 = registry.register(
                1,
                RegistryEntry {
                    path: "crate::Foo".to_string(),
                },
            );
            assert!(result2.is_ok());
            assert_eq!(result2.unwrap(), 1);
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
                other => panic!("Unexpected result: {:?}", other),
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

    #[test]
    fn next_available_id_works() {
        MEMORY_REGISTRY.with_borrow_mut(|registry| {
            // Determine starting ID
            let start_id = registry.next_available_id();
            
            // Register first ID
            registry
                .register(
                    start_id,
                    RegistryEntry {
                        path: "crate::Foo".to_string(),
                    },
                )
                .unwrap();
            assert_eq!(registry.next_available_id(), start_id + 1);

            // Register second ID
            registry
                .register(
                    start_id + 1,
                    RegistryEntry {
                        path: "crate::Bar".to_string(),
                    },
                )
                .unwrap();
            assert_eq!(registry.next_available_id(), start_id + 2);

            // Register ID with a gap (skipping one)
            registry
                .register(
                    start_id + 3,
                    RegistryEntry {
                        path: "crate::Baz".to_string(),
                    },
                )
                .unwrap();
            assert_eq!(registry.next_available_id(), start_id + 2);
        });
    }

    #[test]
    fn register_auto_works() {
        MEMORY_REGISTRY.with_borrow_mut(|registry| {
            // Get the next available ID before we start
            let start_id = registry.next_available_id();
            
            // First auto registration should use start_id
            let result1 = registry.register_auto(RegistryEntry {
                path: "crate::Auto1".to_string(),
            });
            assert!(result1.is_ok());
            assert_eq!(result1.unwrap(), start_id);

            // Second auto registration should use start_id + 1
            let result2 = registry.register_auto(RegistryEntry {
                path: "crate::Auto2".to_string(),
            });
            assert!(result2.is_ok());
            assert_eq!(result2.unwrap(), start_id + 1);

            // Manual registration with a gap
            registry
                .register(
                    start_id + 3, // Skip start_id + 2
                    RegistryEntry {
                        path: "crate::Manual4".to_string(),
                    },
                )
                .unwrap();

            // Next auto registration should use the gap (start_id + 2)
            let result3 = registry.register_auto(RegistryEntry {
                path: "crate::Auto3".to_string(),
            });
            assert!(result3.is_ok());
            assert_eq!(result3.unwrap(), start_id + 2);
        });
    }
}
