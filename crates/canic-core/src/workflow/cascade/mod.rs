//! Cascade propagation.
//!
//! Pushes environment/topology/state changes from root to child canisters.
//! This is orchestration logic (fanout), not storage and not placement strategy.

pub mod snapshot;
pub mod state;
pub mod topology;

use crate::{
    Error, ThisError,
    workflow::{WorkflowError, prelude::*},
};

const SYNC_CALL_WARN_THRESHOLD: usize = 10;

///
/// CascadeError
/// Errors raised during synchronization
///

#[derive(Debug, ThisError)]
pub enum CascadeError {
    #[error("child rejected cascade: {0:?}")]
    ChildRejected(Principal),

    #[error("invalid parent chain: empty")]
    InvalidParentChain,

    #[error("parent chain does not start with self ({0})")]
    ParentChainMissingSelf(Principal),

    #[error("next hop {0} not found in parent chain")]
    NextHopNotFound(Principal),
}

impl From<CascadeError> for Error {
    fn from(err: CascadeError) -> Self {
        WorkflowError::from(err).into()
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
