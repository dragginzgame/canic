use crate::{Error, state::StateError};
use candid::Principal;

// get_controllers
pub fn get_controllers() -> Result<Vec<Principal>, Error> {
    let config = crate::state::config::get_config().map_err(StateError::ConfigError)?;

    Ok(config.controllers.to_vec())
}
