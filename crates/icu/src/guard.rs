use crate::{
    ic::api::{is_controller, msg_caller},
    memory::{self, cells::AppMode},
};
use thiserror::Error as ThisError;

///
/// GuardError
///
/// The guard functions require a String to be returned, not an Error
///

#[derive(Debug, ThisError)]
pub enum GuardError {
    #[error("app is disabled")]
    AppDisabled,

    #[error("app is readonly")]
    AppReadonly,
}

// guard_query
pub fn guard_query() -> Result<(), String> {
    if is_controller(&msg_caller()) {
        return Ok(());
    }

    match memory::AppState::get_mode() {
        AppMode::Enabled | AppMode::Readonly => Ok(()),
        AppMode::Disabled => Err(GuardError::AppDisabled.to_string()),
    }
}

// guard_update
pub fn guard_update() -> Result<(), String> {
    if is_controller(&msg_caller()) {
        return Ok(());
    }

    match memory::AppState::get_mode() {
        AppMode::Enabled => Ok(()),
        AppMode::Readonly => Err(GuardError::AppReadonly.to_string()),
        AppMode::Disabled => Err(GuardError::AppDisabled.to_string()),
    }
}
