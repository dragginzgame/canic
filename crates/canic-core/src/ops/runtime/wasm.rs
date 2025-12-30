use crate::{Error, cdk::types::WasmModule, ids::CanisterRole, log, log::Topic};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// Runtime WASM registry
//
// Application-owned, in-memory registry mapping canister roles
// to their embedded WASM modules. This is runtime state, not domain
// state and not infrastructure plumbing.
//

thread_local! {
    static WASM_REGISTRY: RefCell<HashMap<CanisterRole, WasmModule>> =
        RefCell::new(HashMap::new());
}

///
/// WasmError
///

#[derive(Debug, ThisError)]
pub enum WasmError {
    #[error("wasm '{0}' not found")]
    WasmNotFound(CanisterRole),
}

impl From<WasmError> for Error {
    fn from(err: WasmError) -> Self {
        err.into()
    }
}

///
/// Wasm
/// Runtime API for accessing embedded WASM modules.
///

pub struct Wasm;

impl Wasm {
    /// Fetch a WASM module for the given canister role, if registered.
    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<WasmModule> {
        WASM_REGISTRY.with_borrow(|reg| reg.get(role).cloned())
    }

    /// Fetch a WASM module or return an error if missing.
    pub fn try_get(role: &CanisterRole) -> Result<WasmModule, Error> {
        Self::get(role).ok_or_else(|| WasmError::WasmNotFound(role.clone()).into())
    }

    /// Import a static slice of (role, wasm bytes) at startup.
    ///
    /// Intended to be called during canister initialization.
    #[allow(clippy::cast_precision_loss)]
    pub fn import_static(wasms: &'static [(CanisterRole, &[u8])]) {
        for (role, bytes) in wasms {
            let wasm = WasmModule::new(bytes);
            let size = wasm.len();

            WASM_REGISTRY.with_borrow_mut(|reg| {
                reg.insert(role.clone(), wasm);
            });

            log!(
                Topic::Wasm,
                Info,
                "📄 wasm.import: {} ({:.2} KB)",
                role,
                size as f64 / 1000.0
            );
        }
    }

    /// Import a static slice of (role, wasm bytes) without logging.
    pub fn import_static_quiet(wasms: &'static [(CanisterRole, &[u8])]) {
        for (role, bytes) in wasms {
            let wasm = WasmModule::new(bytes);
            WASM_REGISTRY.with_borrow_mut(|reg| {
                reg.insert(role.clone(), wasm);
            });
        }
    }

    /// Clear the registry (tests only).
    #[cfg(test)]
    pub(crate) fn clear_for_test() {
        WASM_REGISTRY.with_borrow_mut(|reg| reg.clear());
    }
}
