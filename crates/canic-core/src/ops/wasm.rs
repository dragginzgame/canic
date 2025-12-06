use crate::{Error, ids::CanisterRole, model::wasm::WasmRegistry, types::WasmModule};

///
/// WasmOps
/// Thin ops-layer wrapper around the embedded WasmRegistry.
///

pub struct WasmOps;

impl WasmOps {
    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<WasmModule> {
        WasmRegistry::get(role)
    }

    pub fn try_get(role: &CanisterRole) -> Result<WasmModule, Error> {
        WasmRegistry::try_get(role)
    }

    pub fn import_static(wasms: &'static [(CanisterRole, &[u8])]) {
        WasmRegistry::import(wasms);
    }
}
