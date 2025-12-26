use crate::{
    Error, ThisError, access::AccessError, model::memory::state::AppMode, ops::state::AppStateOps,
};

///
/// GuardError
///

#[derive(Debug, ThisError)]
pub enum GuardError {
    #[error("application is disabled")]
    AppDisabled,

    #[error("application is in readonly mode")]
    AppReadonly,
}

impl From<GuardError> for Error {
    fn from(err: GuardError) -> Self {
        AccessError::GuardError(err).into()
    }
}

/// Validate access for query calls.
///
/// Rules:
/// - Enabled and Readonly modes permit queries.
/// - Disabled mode rejects queries.
pub fn guard_app_query() -> Result<(), Error> {
    match AppStateOps::get_mode() {
        AppMode::Enabled | AppMode::Readonly => Ok(()),
        AppMode::Disabled => Err(GuardError::AppDisabled.into()),
    }
}

/// Validate access for update calls.
///
/// Rules:
/// - Enabled mode permits updates.
/// - Readonly rejects updates.
/// - Disabled rejects updates.
pub fn guard_app_update() -> Result<(), Error> {
    match AppStateOps::get_mode() {
        AppMode::Enabled => Ok(()),
        AppMode::Readonly => Err(GuardError::AppReadonly.into()),
        AppMode::Disabled => Err(GuardError::AppDisabled.into()),
    }
}
