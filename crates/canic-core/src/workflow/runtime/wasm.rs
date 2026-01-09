use crate::{ids::CanisterRole, ops::runtime::wasm::WasmOps};

///
/// WasmWorkflow
///

pub struct WasmWorkflow;

impl WasmWorkflow {
    pub fn import_static(wasms: &'static [(CanisterRole, &[u8])]) {
        WasmOps::import_static(wasms);
    }
}
