use crate::{
    Error, PublicError, ThisError, access::AccessError, dto::state::AppModeView,
    ops::storage::state::app::AppStateOps,
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
        AccessError::Guard(err).into()
    }
}

impl GuardError {
    #[must_use]
    pub fn public(&self) -> PublicError {
        PublicError::unauthorized(self.to_string())
    }
}

/// Validate access for query calls.
///
/// Rules:
/// - Enabled and Readonly modes permit queries.
/// - Disabled mode rejects queries.
pub fn guard_app_query() -> Result<(), PublicError> {
    match AppStateOps::export_view().mode {
        AppModeView::Enabled | AppModeView::Readonly => Ok(()),
        AppModeView::Disabled => Err(GuardError::AppDisabled.public()),
    }
}

/// Validate access for update calls.
///
/// Rules:
/// - Enabled mode permits updates.
/// - Readonly rejects updates.
/// - Disabled rejects updates.
pub fn guard_app_update() -> Result<(), PublicError> {
    match AppStateOps::export_view().mode {
        AppModeView::Enabled => Ok(()),
        AppModeView::Readonly => Err(GuardError::AppReadonly.public()),
        AppModeView::Disabled => Err(GuardError::AppDisabled.public()),
    }
}
