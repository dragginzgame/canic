use crate::{Error, ids::CanisterRole, model::wasm::WasmRegistry, types::WasmModule};

///
/// WasmOps
/// Thin ops-layer wrapper around the embedded WasmRegistry.
///

pub struct WasmOps;

impl WasmOps {
    /// Fetch a WASM module for the given canister type if registered.
    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<WasmModule> {
        WasmRegistry::get(role)
    }

    /// Fetch a WASM module or return an error when missing.
    pub fn try_get(role: &CanisterRole) -> Result<WasmModule, Error> {
        WasmRegistry::try_get(role)
    }

    /// Import a static slice of (role, wasm bytes) at startup.
    pub fn import_static(wasms: &'static [(CanisterRole, &[u8])]) {
        WasmRegistry::import(wasms);
    }
}
