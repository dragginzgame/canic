use crate::{
    Error,
    state::{StateError, wasm::WasmManager},
};

// add_wasm
pub fn add_wasm<S: ToString>(path: S, wasm: &'static [u8]) -> Result<(), Error> {
    WasmManager::add_wasm(path, wasm).map_err(StateError::WasmError)?;

    Ok(())
}

// add_wasms
pub fn add_wasms<S: ToString>(wasms: &[(S, &'static [u8])]) -> Result<(), Error> {
    WasmManager::add_wasms(wasms).map_err(StateError::WasmError)?;

    Ok(())
}

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
