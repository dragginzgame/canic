use crate::Error;
use candid::Principal;

// get_controllers
pub fn get_controllers() -> Result<Vec<Principal>, Error> {
    let config = crate::config::get_config()?;

    Ok(config.controllers.to_vec())
}
