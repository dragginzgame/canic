use crate::{
    Error,
    state::{StateError, wasm::WasmManager},
};

// get_wasm
pub fn get_wasm<S: ToString>(path: S) -> Result<&'static [u8], Error> {
    let wasm = WasmManager::get_wasm(path).map_err(StateError::WasmError)?;

    Ok(wasm)
}

// info
pub fn info() -> Result<Vec<(String, usize)>, Error> {
    let info = WasmManager::info().map_err(StateError::WasmError)?;

    Ok(info)
}
