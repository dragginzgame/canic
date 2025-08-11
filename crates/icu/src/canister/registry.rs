use crate::{
    Error,
    canister::{Attributes, Canister, CanisterError, CanisterView},
};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// CANISTER_REGISTRY
//

thread_local! {
    pub static CANISTER_REGISTRY: RefCell<HashMap<String, Canister>> = RefCell::new(HashMap::new());
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
/// CanisterRegistryData
///

pub type CanisterRegistryData = HashMap<String, CanisterView>;

///
/// CanisterRegistry
///

#[derive(Debug, Default)]
pub struct CanisterRegistry {}

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
            Err(Error::from(CanisterError::CanisterRegistryError(
                CanisterRegistryError::CanisterNotFound(path.to_string()),
            )))?
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn insert(kind: &'static str, attributes: &Attributes, wasm: &'static [u8]) {
        CANISTER_REGISTRY.with_borrow_mut(|reg| {
            reg.insert(
                kind.to_string(),
                Canister {
                    kind,
                    attributes: attributes.clone(),
                    wasm,
                },
            );
        });

        //   println!("add_wasm: {} ({:.2} KB)", path, wasm.len() as f64 / 1000.0);
    }

    //
    // EXPORT
    //

    #[must_use]
    pub fn export() -> CanisterRegistryData {
        CANISTER_REGISTRY
            .with_borrow(|reg| reg.iter().map(|(k, v)| (k.clone(), v.into())).collect())
    }
}
