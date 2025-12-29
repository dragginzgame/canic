use crate::{Error, cdk::types::WasmModule, ids::CanisterRole, model::ModelError};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// WASM_REGISTRY
//

thread_local! {
    pub static WASM_REGISTRY: RefCell<HashMap<CanisterRole, WasmModule>> = RefCell::new(HashMap::new());
}

///
/// WasmRegistryError
///

#[derive(Debug, ThisError)]
pub enum WasmRegistryError {
    #[error("wasm '{0}' not found")]
    WasmNotFound(CanisterRole),
}

impl From<WasmRegistryError> for Error {
    fn from(err: WasmRegistryError) -> Self {
        ModelError::from(err).into()
    }
}

///
/// WasmRegistry
///

pub struct WasmRegistry {}

impl WasmRegistry {
    #[must_use]
    pub(crate) fn get(role: &CanisterRole) -> Option<WasmModule> {
        WASM_REGISTRY.with_borrow(|reg| reg.get(role).cloned())
    }

    pub(crate) fn try_get(role: &CanisterRole) -> Result<WasmModule, Error> {
        Self::get(role).ok_or_else(|| WasmRegistryError::WasmNotFound(role.clone()).into())
    }

    pub(crate) fn insert(canister_role: &CanisterRole, wasm: WasmModule) {
        WASM_REGISTRY.with_borrow_mut(|reg| {
            reg.insert(canister_role.clone(), wasm);
        });
    }
}
