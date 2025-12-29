use crate::{
    Error, cdk::types::WasmModule, ids::CanisterRole, log, log::Topic, model::wasm::WasmRegistry,
};

///
/// WasmOps
/// Thin ops-layer wrapper around the embedded WasmRegistry.
///

pub struct WasmOps;

impl WasmOps {
    /// Fetch a WASM module for the given canister role if registered.
    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<WasmModule> {
        WasmRegistry::get(role)
    }

    /// Fetch a WASM module or return an error when missing.
    pub fn try_get(role: &CanisterRole) -> Result<WasmModule, Error> {
        WasmRegistry::try_get(role)
    }

    /// Import a static slice of (role, wasm bytes) at startup.
    #[allow(clippy::cast_precision_loss)]
    pub fn import_static(wasms: &'static [(CanisterRole, &[u8])]) {
        for (role, bytes) in wasms {
            let wasm = WasmModule::new(bytes);
            let wasm_size = wasm.len();

            WasmRegistry::insert(role, wasm);

            log!(
                Topic::Wasm,
                Info,
                "📄 registry.insert: {} ({:.2} KB)",
                role,
                wasm_size as f64 / 1000.0
            );
        }
    }
}
