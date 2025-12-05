use crate::{
    Error, core::types::WasmModule, log, log::Topic, model::ModelError, types::CanisterType,
};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// WASM_REGISTRY
//

thread_local! {
    pub static WASM_REGISTRY: RefCell<HashMap<CanisterType, WasmModule>> = RefCell::new(HashMap::new());
}

///
/// WasmRegistryError
///

#[derive(Debug, ThisError)]
pub enum WasmRegistryError {
    #[error("wasm '{0}' not found")]
    WasmNotFound(CanisterType),
}

impl From<WasmRegistryError> for Error {
    fn from(err: WasmRegistryError) -> Self {
        ModelError::from(err).into()
    }
}

///
/// WasmRegistry
///

#[derive(Debug, Default)]
pub struct WasmRegistry {}

impl WasmRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(ty: &CanisterType) -> Option<WasmModule> {
        WASM_REGISTRY.with_borrow(|reg| reg.get(ty).cloned())
    }

    pub fn try_get(ty: &CanisterType) -> Result<WasmModule, Error> {
        Self::get(ty).ok_or_else(|| WasmRegistryError::WasmNotFound(ty.clone()).into())
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn insert(canister_type: &CanisterType, wasm: WasmModule) {
        let wasm_size = wasm.len();

        WASM_REGISTRY.with_borrow_mut(|reg| {
            reg.insert(canister_type.clone(), wasm);
        });

        log!(
            Topic::Wasm,
            Info,
            "📄 registry.insert: {} ({:.2} KB)",
            canister_type,
            wasm_size as f64 / 1000.0
        );
    }

    pub fn import(wasms: &'static [(CanisterType, &[u8])]) {
        for (ty, bytes) in wasms {
            Self::insert(ty, WasmModule::new(bytes));
        }
    }
}
