use crate::{
    cdk::api::{is_controller, msg_caller},
    ops::model::memory::state::{AppMode, AppStateOps},
};
use thiserror::Error as ThisError;

///
/// GuardError
///
/// The IC guard functions require a String to be returned, not an Error
///

#[derive(Debug, ThisError)]
pub enum GuardError {
    #[error("app is disabled")]
    AppDisabled,

    #[error("app is readonly")]
    AppReadonly,
}

/// Guard for query calls; allows controllers and rejects when the app is disabled.
pub fn guard_query() -> Result<(), String> {
    if is_controller(&msg_caller()) {
        return Ok(());
    }

    match AppStateOps::get_mode() {
        AppMode::Enabled | AppMode::Readonly => Ok(()),
        AppMode::Disabled => Err(GuardError::AppDisabled.to_string()),
    }
}

/// Guard for update calls; allows controllers and blocks readonly/disabled states.
pub fn guard_update() -> Result<(), String> {
    if is_controller(&msg_caller()) {
        return Ok(());
    }

    match AppStateOps::get_mode() {
        AppMode::Enabled => Ok(()),
        AppMode::Readonly => Err(GuardError::AppReadonly.to_string()),
        AppMode::Disabled => Err(GuardError::AppDisabled.to_string()),
    }
}
