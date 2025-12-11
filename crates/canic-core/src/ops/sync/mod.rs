//! Synchronization helpers for propagating state and topology snapshots.

pub mod state;
pub mod topology;

use crate::{Error, ThisError, log, log::Topic, ops::OpsError};
use candid::Principal;

const SYNC_CALL_WARN_THRESHOLD: usize = 10;

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

///
/// Helpers
///

fn warn_if_large(label: &str, count: usize) {
    if count > SYNC_CALL_WARN_THRESHOLD {
        log!(
            Topic::Sync,
            Warn,
            "sync: large {}: {} entries",
            label,
            count
        );
    }
}
