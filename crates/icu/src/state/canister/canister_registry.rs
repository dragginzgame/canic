use crate::{
    Error, Log, log,
    state::{
        StateError,
        canister::{Canister, CanisterType, CanisterView},
    },
};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// CANISTER_REGISTRY
//

thread_local! {
    pub static CANISTER_REGISTRY: RefCell<HashMap<CanisterType, Canister>> = RefCell::new(HashMap::new());
}

///
/// CanisterRegistryError
///

#[derive(Debug, ThisError)]
pub enum CanisterRegistryError {
    #[error("canister '{0}' not found")]
    CanisterNotFound(CanisterType),
}

///
/// CanisterRegistryView
///

pub type CanisterRegistryView = Vec<(CanisterType, CanisterView)>;

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

    #[must_use]
    pub fn get(ty: &CanisterType) -> Option<Canister> {
        CANISTER_REGISTRY.with_borrow(|reg| reg.get(ty).cloned())
    }

    pub fn try_get(ty: &CanisterType) -> Result<Canister, Error> {
        Self::get(ty).ok_or_else(|| {
            Error::from(StateError::CanisterRegistryError(
                CanisterRegistryError::CanisterNotFound(ty.clone()),
            ))
        })
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn insert(canister_type: &CanisterType, canister: Canister) {
        let wasm_size = canister.wasm.len();

        CANISTER_REGISTRY.with_borrow_mut(|reg| {
            reg.insert(canister_type.clone(), canister);
        });

        log!(
            Log::Info,
            "📄 canister_registry.insert: {} ({:.2} KB)",
            canister_type,
            wasm_size as f64 / 1000.0
        );
    }

    //
    // IMPORT & EXPORT
    //

    pub fn import(canisters: &'static [(&'static CanisterType, Canister)]) {
        for (ty, canister) in canisters {
            Self::insert(ty, canister.clone());
        }
    }

    #[must_use]
    pub fn export() -> CanisterRegistryView {
        CANISTER_REGISTRY
            .with_borrow(|reg| reg.iter().map(|(k, v)| (k.clone(), v.into())).collect())
    }
}
