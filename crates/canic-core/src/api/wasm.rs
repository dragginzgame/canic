use crate::{ids::CanisterRole, workflow::runtime::wasm::WasmWorkflow};

///
/// WasmApi
///
/// Runtime WASM registration API.
///
/// Public, user-callable helpers for registering embedded WASM modules
/// during canister initialization.
///
/// This module exists to expose a stable surface to downstream canisters
/// without making `WasmOps` public.
///
/// Layering:
///     user canister → api → ops → runtime state
///

pub struct WasmApi;

impl WasmApi {
    pub fn import_static(wasms: &'static [(CanisterRole, &[u8])]) {
        WasmWorkflow::import_static(wasms);
    }
}
