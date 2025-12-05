use crate::{Error, core::types::WasmModule, model::wasm::WasmRegistry, types::CanisterType};

///
/// WasmOps
/// Thin ops-layer wrapper around the embedded WasmRegistry.
///

pub struct WasmOps;

impl WasmOps {
    #[must_use]
    pub fn get(ty: &CanisterType) -> Option<WasmModule> {
        WasmRegistry::get(ty)
    }

    pub fn try_get(ty: &CanisterType) -> Result<WasmModule, Error> {
        WasmRegistry::try_get(ty)
    }

    pub fn import_static(wasms: &'static [(CanisterType, &[u8])]) {
        WasmRegistry::import(wasms);
    }
}
