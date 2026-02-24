use crate::{
    InternalError,
    cdk::types::WasmModule,
    ops::{prelude::*, runtime::RuntimeOpsError},
};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

thread_local! {
    ///
    /// Runtime WASM registry
    ///
    /// Application-owned, in-memory registry mapping canister roles
    /// to their embedded WASM modules. This is runtime state, not domain
    /// state and not infrastructure plumbing.
    ///
    static WASM_REGISTRY: RefCell<HashMap<CanisterRole, WasmModule>> =
        RefCell::new(HashMap::new());
}

///
/// WasmOpsError
///

#[derive(Debug, ThisError)]
pub enum WasmOpsError {
    #[error("wasm '{0}' not found")]
    WasmNotFound(CanisterRole),

    #[error("wasm registry not initialized before root bootstrap")]
    RegistryUninitialized,
}

impl From<WasmOpsError> for InternalError {
    fn from(err: WasmOpsError) -> Self {
        RuntimeOpsError::WasmOps(err).into()
    }
}

///
/// WasmOps
/// Runtime API for accessing embedded WASM modules.
///

pub struct WasmOps;

impl WasmOps {
    /// Returns true if the WASM registry has been populated.
    #[must_use]
    pub fn is_initialized() -> bool {
        WASM_REGISTRY.with_borrow(|reg| !reg.is_empty())
    }

    /// Ensures embedded WASMs were registered before root bootstrap.
    pub fn require_initialized() -> Result<(), InternalError> {
        if Self::is_initialized() {
            Ok(())
        } else {
            Err(WasmOpsError::RegistryUninitialized.into())
        }
    }

    /// Fetch a WASM module for the given canister role, if registered.
    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<WasmModule> {
        WASM_REGISTRY.with_borrow(|reg| reg.get(role).cloned())
    }

    /// Fetch a WASM module or return an error if missing.
    pub fn try_get(role: &CanisterRole) -> Result<WasmModule, InternalError> {
        Self::get(role).ok_or_else(|| WasmOpsError::WasmNotFound(role.clone()).into())
    }

    /// Import a static slice of (role, wasm bytes) at startup.
    ///
    /// Intended to be called during canister initialization.
    #[expect(clippy::cast_precision_loss)]
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

    /// Clear the registry (tests only).
    #[cfg(test)]
    #[expect(dead_code)]
    pub(crate) fn clear_for_test() {
        WASM_REGISTRY.with_borrow_mut(HashMap::clear);
    }
}
