use crate::{
    Error, Log,
    canister::CanisterType,
    log,
    state::{
        StateError,
        canister::{CanisterConfig, CanisterConfigView},
    },
};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// CANISTER_CATALOG
//

thread_local! {
    pub static CANISTER_CATALOG: RefCell<HashMap<CanisterType, CanisterConfig>> = RefCell::new(HashMap::new());
}

///
/// CanisterCatalogError
///

#[derive(Debug, ThisError)]
pub enum CanisterCatalogError {
    #[error("canister '{0}' not found")]
    CanisterNotFound(CanisterType),
}

///
/// CanisterCatalogView
///

pub type CanisterCatalogView = Vec<(CanisterType, CanisterConfigView)>;

///
/// CanisterCatalog
///

#[derive(Debug, Default)]
pub struct CanisterCatalog {}

impl CanisterCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(ty: &CanisterType) -> Option<CanisterConfig> {
        CANISTER_CATALOG.with_borrow(|reg| reg.get(ty).cloned())
    }

    pub fn try_get(ty: &CanisterType) -> Result<CanisterConfig, Error> {
        Self::get(ty).ok_or_else(|| {
            Error::from(StateError::CanisterCatalogError(
                CanisterCatalogError::CanisterNotFound(ty.clone()),
            ))
        })
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn insert(canister_type: &CanisterType, canister: CanisterConfig) {
        let wasm_size = canister.wasm.len();

        CANISTER_CATALOG.with_borrow_mut(|reg| {
            reg.insert(canister_type.clone(), canister);
        });

        log!(
            Log::Info,
            "📄 CANISTER_CATALOG.insert: {} ({:.2} KB)",
            canister_type,
            wasm_size as f64 / 1000.0
        );
    }

    //
    // IMPORT & EXPORT
    //

    pub fn import(canisters: &'static [(&'static CanisterType, CanisterConfig)]) {
        for (ty, canister) in canisters {
            Self::insert(ty, canister.clone());
        }
    }

    #[must_use]
    pub fn export() -> CanisterCatalogView {
        CANISTER_CATALOG.with_borrow(|reg| reg.iter().map(|(k, v)| (k.clone(), v.into())).collect())
    }
}
