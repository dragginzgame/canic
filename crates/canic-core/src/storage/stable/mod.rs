pub mod auth;
pub mod blob_storage;
pub mod children;
pub mod cycles;
pub mod directory;
pub mod env;
pub mod icp_refill;
pub mod index;
pub mod intent;
pub mod log;
pub mod pool;
pub mod registry;
pub mod replay;
pub mod scaling;
pub mod sharding;
pub mod state;

use crate::{InternalError, storage::prelude::*};
use thiserror::Error as ThisError;

///
/// StableMemoryError
///

#[derive(Debug, ThisError)]
pub enum StableMemoryError {
    #[error("log write failed: current_size={current_size}, delta={delta}")]
    LogWriteFailed { current_size: u64, delta: u64 },
}

impl From<StableMemoryError> for InternalError {
    fn from(err: StableMemoryError) -> Self {
        StorageError::StableMemory(err).into()
    }
}
