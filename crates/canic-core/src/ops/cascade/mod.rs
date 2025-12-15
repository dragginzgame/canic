//! Cascade propagation.
//!
//! Pushes environment/topology/state changes from root to child canisters.
//! This is orchestration logic (fanout), not storage and not placement strategy.

pub mod state;
pub mod topology;

use crate::{Error, ThisError, log, log::Topic, ops::OpsError};
use candid::Principal;

const SYNC_CALL_WARN_THRESHOLD: usize = 10;

///
/// CascadeOpsError
/// Errors raised during synchronization
///

#[derive(Debug, ThisError)]
pub enum CascadeOpsError {
    #[error("canister not found")]
    CanisterNotFound(Principal),

    #[error("root canister not found")]
    RootNotFound,

    #[error("invalid parent chain: empty")]
    InvalidParentChain,

    #[error("parent chain does not start with self ({0})")]
    ParentChainMissingSelf(Principal),

    #[error("cycle detected in parent chain at {0}")]
    ParentChainCycle(Principal),

    #[error("parent chain length {0} exceeds registry size")]
    ParentChainTooLong(usize),

    #[error("parent chain did not terminate at root (stopped at {0})")]
    ParentChainNotRootTerminated(Principal),

    #[error("next hop {0} not found in parent chain")]
    NextHopNotFound(Principal),
}

impl From<CascadeOpsError> for Error {
    fn from(err: CascadeOpsError) -> Self {
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
