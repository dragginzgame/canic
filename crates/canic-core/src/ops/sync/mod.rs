//! Synchronization helpers for propagating state and topology snapshots.

pub mod state;
pub mod topology;

use crate::{Error, ThisError, ops::OpsError};
use candid::Principal;

///
/// SyncOpsError
/// Errors raised during synchronization
///

#[derive(Debug, ThisError)]
pub enum SyncOpsError {
    #[error("canister not found")]
    CanisterNotFound(Principal),

    #[error("root canister not found")]
    RootNotFound,
}

impl From<SyncOpsError> for Error {
    fn from(err: SyncOpsError) -> Self {
        OpsError::from(err).into()
    }
}
