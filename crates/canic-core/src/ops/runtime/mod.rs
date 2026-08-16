//! Module: ops::runtime
//!
//! Responsibility: group runtime operations used by workflow.
//! Does not own: domain policy, endpoint authorization, or stable schemas.
//! Boundary: exposes ops-layer runtime facades and their typed error surface.

pub mod bootstrap;
pub mod cycles_funding;
pub mod env;
pub mod fleet_activation;
pub mod init_payload;
pub mod install_source;
pub mod log;
pub mod memory;
pub mod metrics;
pub mod ready;
pub mod recent_failure;

use crate::InternalError;
use thiserror::Error as ThisError;

///
/// RuntimeOpsError
///
/// Typed failure surface for runtime operation facades.
///

#[derive(Debug, ThisError)]
pub enum RuntimeOpsError {
    #[error(transparent)]
    EnvOps(#[from] env::EnvOpsError),

    #[error(transparent)]
    LogStorage(#[from] crate::storage::StorageError),

    #[error(transparent)]
    MemoryRegistryOps(#[from] memory::MemoryRegistryOpsError),
}

impl From<RuntimeOpsError> for InternalError {
    fn from(err: RuntimeOpsError) -> Self {
        match err {
            RuntimeOpsError::EnvOps(err) => err.into(),
            RuntimeOpsError::LogStorage(err) => err.into(),
            RuntimeOpsError::MemoryRegistryOps(err) => err.into(),
        }
    }
}
