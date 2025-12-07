pub mod memory;

use crate::{
    Error, ThisError,
    ops::{OpsError, model::memory::MemoryOpsError},
};
use std::time::Duration;

/// Shared initial delay for ops timers to allow init work to settle.
pub const OPS_INIT_DELAY: Duration = Duration::from_secs(10);

/// Shared cadence for cycle tracking (10 minutes).
pub const OPS_CYCLE_TRACK_INTERVAL: Duration = Duration::from_secs(60 * 10);

/// Shared cadence for log retention (10 minutes).
pub const OPS_LOG_RETENTION_INTERVAL: Duration = Duration::from_secs(60 * 10);

/// Reserve timer initial delay (30 seconds) before first check.
pub const OPS_RESERVE_INIT_DELAY: Duration = Duration::from_secs(30);

/// Reserve check cadence (30 minutes).
pub const OPS_RESERVE_CHECK_INTERVAL: Duration = Duration::from_secs(30 * 60);

///
/// ModelOpsError
/// Logical or configuration errors that occur during sharding planning.
///

#[derive(Debug, ThisError)]
pub enum ModelOpsError {
    #[error(transparent)]
    MemoryOpsError(#[from] MemoryOpsError),
}

impl From<ModelOpsError> for Error {
    fn from(err: ModelOpsError) -> Self {
        OpsError::ModelOpsError(err).into()
    }
}
