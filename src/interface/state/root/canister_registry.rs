use crate::{Error, state::StateError};

pub use crate::state::root::canister_registry::{
    Canister, CanisterDef, CanisterRegistry, CanisterRegistryInfo,
};

// add_canister
pub fn add_canister(path: &str, def: &CanisterDef, wasm: &'static [u8]) -> Result<(), Error> {
    CanisterRegistry::add_canister(path, def, wasm).map_err(StateError::CanisterRegistryError)?;

    Ok(())
}

// get_canister
pub fn get_canister(path: &str) -> Result<Canister, Error> {
    let canister =
        CanisterRegistry::get_canister(path).map_err(StateError::CanisterRegistryError)?;

    Ok(canister)
}

// get_info
pub fn get_info() -> Result<CanisterRegistryInfo, Error> {
    let info = CanisterRegistry::get_info().map_err(StateError::CanisterRegistryError)?;

    Ok(info)
}
