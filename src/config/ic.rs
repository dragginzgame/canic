use crate::interface::{InterfaceError, config::ConfigError};
use candid::Principal;

// get_admins
pub fn get_admins() -> Result<Vec<Principal>, ConfigError> {
    let config = config::ic::get_config()?;

    Ok(config.admins.to_vec())
}

// get_admins_api
pub fn get_admins_api() -> Result<Vec<Principal>, InterfaceError> {
    get_admins().map_err(InterfaceError::ConfigError)
}

// get_controllers
pub fn get_controllers() -> Result<Vec<Principal>, ConfigError> {
    let config = config::ic::get_config()?;

    Ok(config.controllers.to_vec())
}

// get_controllers_api
pub fn get_controllers_api() -> Result<Vec<Principal>, InterfaceError> {
    get_controllers().map_err(InterfaceError::ConfigError)
}
