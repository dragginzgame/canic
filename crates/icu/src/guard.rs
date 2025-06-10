use crate::{
    ic::api::{is_controller, msg_caller},
    interface::memory::app::state::{AppMode, get_mode},
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// GuardError
///
/// The guard functions require a String to be returned, not an Error
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
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

    match get_mode() {
        AppMode::Enabled | AppMode::Readonly => Ok(()),
        AppMode::Disabled => Err(GuardError::AppDisabled.to_string()),
    }
}

// guard_update
pub fn guard_update() -> Result<(), String> {
    if is_controller(&msg_caller()) {
        return Ok(());
    }

    match get_mode() {
        AppMode::Enabled => Ok(()),
        AppMode::Readonly => Err(GuardError::AppReadonly.to_string()),
        AppMode::Disabled => Err(GuardError::AppDisabled.to_string()),
    }
}
