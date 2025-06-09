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

// create_canisters
pub async fn create_canisters() -> Result<(), Error> {
    pub use crate::interface::{memory::subnet::index, request::canister_create};

    // iterate canisters
    // TODO - this won't work if they have arguments
    for (path, info) in get_info()? {
        if info.def.auto_create && index::get_canister(&path).is_none() {
            canister_create(&path, ()).await.unwrap();
        }
    }

    Ok(())
}
