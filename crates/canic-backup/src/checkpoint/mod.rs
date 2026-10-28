//! Module: checkpoint
//!
//! Responsibility: bind one complete Root checkpoint unit to original IC Backup ingress accounting.
//! Does not own: transport authentication, Root-paid effects, application settlement or live-create admission.
//! Boundary: checkpoint commands never grant snapshot, restore or release authority.

mod driver;
mod model;
#[cfg(test)]
mod tests;

use crate::persistence::PersistenceError;
use thiserror::Error;

pub use driver::{
    CheckpointDriver, CheckpointExecutionError, CheckpointProvider, CheckpointReplyError,
};
pub use model::{CheckpointIngress, CheckpointUnitRecord, FrozenCheckpoint};

/// Local checkpoint binding/admission failure, without repeat-dispatch authority.
#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("invalid or changed complete checkpoint unit")]
    UnitMismatch,
    #[error("original checkpoint request or stage differs")]
    OriginalMismatch,
    #[error("Root checkpoint reply does not match the exact original transition")]
    ReplyMismatch,
    #[error(transparent)]
    Wire(#[from] ic_wire::Error),
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
    #[error("checkpoint identity encoding failed: {0}")]
    Identity(#[from] canic_contracts::serialization::SerializeError),
    #[error("original checkpoint accounting failed: {0}")]
    Accounting(#[source] Box<dyn std::error::Error>),
}

impl CheckpointError {
    pub(crate) fn accounting(error: impl std::error::Error + 'static) -> Self {
        Self::Accounting(Box::new(error))
    }
}
