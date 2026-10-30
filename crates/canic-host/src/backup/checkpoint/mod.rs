//! Module: checkpoint
//!
//! Responsibility: bind one complete Root checkpoint unit to original IC Backup ingress accounting.
//! Does not own: transport authentication, Root-paid effects, application settlement or live-create admission.
//! Boundary: checkpoint commands never grant snapshot, restore or release authority.

mod agent;
mod driver;
mod model;
mod recovery;
#[cfg(test)]
mod tests;

use thiserror::Error;

pub use agent::{AgentCheckpointError, AgentCheckpointProvider, CheckpointAdmission};
pub use driver::{
    CheckpointDriver, CheckpointExecutionError, CheckpointProvider, CheckpointReplyError,
};
pub use model::{CheckpointIngress, CheckpointUnitRecord, FrozenCheckpoint};
pub use recovery::{
    CheckpointObservation, CheckpointRecoveryError, CheckpointRecoveryProvider,
    reconcile_checkpoint,
};

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
    Persistence(#[from] ic_host_fs::durable::NamedWriteError<std::io::Error>),
    #[error("original checkpoint accounting failed: {0}")]
    Accounting(#[source] Box<dyn std::error::Error>),
}

impl CheckpointError {
    pub(crate) fn accounting(error: impl std::error::Error + 'static) -> Self {
        Self::Accounting(Box::new(error))
    }
}
