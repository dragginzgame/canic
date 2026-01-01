//! Stable-memory state adapters.
pub mod app;
pub mod subnet;

use crate::{Error, ThisError, ops::storage::StorageOpsError};

///
/// StateOpsError
///

#[derive(Debug, ThisError)]
pub enum StateOpsError {
    #[error(transparent)]
    AppStateOps(#[from] app::AppStateOpsError),
}

impl From<StateOpsError> for Error {
    fn from(err: StateOpsError) -> Self {
        StorageOpsError::StateOps(err).into()
    }
}
