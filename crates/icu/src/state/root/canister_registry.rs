use crate::{Error, ic::println, state::StateError};
use candid::CandidType;
use derive_more::Deref;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// CANISTER_REGISTRY
//

thread_local! {
    pub static CANISTER_REGISTRY: RefCell<CanisterRegistry> = RefCell::new(CanisterRegistry::new());
}

///
/// CanisterRegistryError
///

#[derive(Debug, ThisError)]
pub enum CanisterRegistryError {
    #[error("canister '{0}' not found")]
    CanisterNotFound(String),
}

///
/// Canister
///

#[derive(Clone, Debug)]
pub struct Canister {
    pub attributes: CanisterAttributes,
    pub wasm: &'static [u8],
}

///
/// CanisterData
/// the front-facing version
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterData {
    pub attributes: CanisterAttributes,
    pub wasm_size: usize,
}

impl From<&Canister> for CanisterData {
    fn from(canister: &Canister) -> Self {
        Self {
            attributes: canister.attributes.clone(),
            wasm_size: canister.wasm.len(),
        }
    }
}

///
/// CanisterAttributes
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterAttributes {
    pub auto_create: bool,
    pub is_sharded: bool,
}

///
/// CanisterRegistry
///

#[derive(Debug, Default, Deref)]
pub struct CanisterRegistry(HashMap<String, Canister>);

impl CanisterRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    //
    // METHODS
    //

    #[must_use]
    pub fn get(path: &str) -> Option<Canister> {
        CANISTER_REGISTRY.with_borrow(|reg| reg.get(path).cloned())
    }

    pub fn try_get(path: &str) -> Result<Canister, Error> {
        if let Some(canister) = Self::get(path) {
            Ok(canister)
        } else {
            Err(StateError::from(CanisterRegistryError::CanisterNotFound(
                path.to_string(),
            )))?
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn insert(
        path: &str,
        attributes: &CanisterAttributes,
        wasm: &'static [u8],
    ) -> Result<(), CanisterRegistryError> {
        CANISTER_REGISTRY.with_borrow_mut(|reg| {
            reg.0.insert(
                path.to_string(),
                Canister {
                    attributes: attributes.clone(),
                    wasm,
                },
            );
        });

        println!("add_wasm: {} ({:.2} KB)", path, wasm.len() as f64 / 1000.0);

        Ok(())
    }

    //
    // EXPORT
    //

    #[must_use]
    pub fn export() -> CanisterRegistryData {
        let data = CANISTER_REGISTRY.with(|registry| {
            registry
                .borrow()
                .iter()
                .map(|(k, v)| (k.clone(), v.into()))
                .collect()
        });

        CanisterRegistryData(data)
    }
}

///
/// CanisterRegistryData
///

#[derive(CandidType, Clone, Debug, Deref, Deserialize, Serialize)]
pub struct CanisterRegistryData(HashMap<String, CanisterData>);

impl IntoIterator for CanisterRegistryData {
    type Item = (String, CanisterData);
    type IntoIter = std::collections::hash_map::IntoIter<String, CanisterData>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
