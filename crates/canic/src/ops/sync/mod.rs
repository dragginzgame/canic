//! Synchronization helpers for propagating state and topology snapshots.

pub mod state;
pub mod topology;

use crate::{Error, ThisError, ops::OpsError};
use candid::Principal;

/// Errors raised during synchronization.
#[derive(Debug, ThisError)]
pub enum SyncError {
    #[error("canister not found")]
    CanisterNotFound(Principal),
}

impl From<SyncError> for Error {
    fn from(err: SyncError) -> Self {
        OpsError::from(err).into()
    }
}
