use crate::{
    access::AccessError, ops::storage::state::app::AppStateOps,
    storage::stable::state::app::AppMode,
};
use thiserror::Error as ThisError;

///
/// GuardAccessError
/// Access errors raised by application state guards.
///

#[derive(Debug, ThisError)]
pub enum GuardAccessError {
    #[error("application is disabled")]
    AppDisabled,

    #[error("application is in readonly mode")]
    AppReadonly,
}

/// Validate access for query calls.
///
/// Rules:
/// - Enabled and Readonly modes permit queries.
/// - Disabled mode rejects queries.
pub fn guard_app_query() -> Result<(), AccessError> {
    let mode = AppStateOps::get_mode();

    match mode {
        AppMode::Enabled | AppMode::Readonly => Ok(()),
        AppMode::Disabled => Err(GuardAccessError::AppDisabled.into()),
    }
}

/// Validate access for update calls.
///
/// Rules:
/// - Enabled mode permits updates.
/// - Readonly rejects updates.
/// - Disabled rejects updates.
pub fn guard_app_update() -> Result<(), AccessError> {
    let mode = AppStateOps::get_mode();

    match mode {
        AppMode::Enabled => Ok(()),
        AppMode::Readonly => Err(GuardAccessError::AppReadonly.into()),
        AppMode::Disabled => Err(GuardAccessError::AppDisabled.into()),
    }
}
