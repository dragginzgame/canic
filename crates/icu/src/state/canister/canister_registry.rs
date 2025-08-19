use crate::{
    Error, Log, log,
    state::{
        StateError,
        canister::{Canister, CanisterView},
    },
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
/// CanisterRegistryView
///

pub type CanisterRegistryView = Vec<(String, CanisterView)>;

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
            Err(Error::from(StateError::CanisterRegistryError(
                CanisterRegistryError::CanisterNotFound(path.to_string()),
            )))?
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn insert(canister: &Canister) {
        CANISTER_REGISTRY.with_borrow_mut(|reg| {
            reg.insert(canister.kind.to_string(), canister.clone());
        });

        log!(
            Log::Info,
            "📄 canister_registry.insert: {} ({:.2} KB)",
            canister.kind,
            canister.wasm.len() as f64 / 1000.0
        );
    }

    //
    // IMPORT & EXPORT
    //

    pub fn import(canisters: &[Canister]) {
        for canister in canisters {
            Self::insert(canister);
        }
    }

    #[must_use]
    pub fn export() -> CanisterRegistryView {
        CANISTER_REGISTRY
            .with_borrow(|reg| reg.iter().map(|(k, v)| (k.clone(), v.into())).collect())
    }
}
