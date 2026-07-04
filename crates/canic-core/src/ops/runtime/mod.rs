//! Module: ops::runtime
//!
//! Responsibility: group runtime operations used by workflow and API layers.
//! Does not own: domain policy, endpoint authorization, or stable schemas.
//! Boundary: exposes ops-layer runtime facades and their typed error surface.

pub mod bootstrap;
pub mod cycles_funding;
pub mod env;
pub mod install_source;
pub mod log;
pub mod memory;
pub mod metrics;
pub mod ready;
pub mod recent_failure;
pub mod timer;

use crate::{InternalError, ops::OpsError};
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
    LogOps(#[from] log::LogOpsError),

    #[error(transparent)]
    MemoryRegistryOps(#[from] memory::MemoryRegistryOpsError),
}

impl From<RuntimeOpsError> for InternalError {
    fn from(err: RuntimeOpsError) -> Self {
        OpsError::from(err).into()
    }
}
