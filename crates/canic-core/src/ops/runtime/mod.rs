pub mod env;
pub mod log;
pub mod memory;
pub mod metrics;
pub mod timer;
pub mod wasm;

use crate::{InternalError, ops::OpsError};
use thiserror::Error as ThisError;

///
/// RuntimeOpsError
///

#[derive(Debug, ThisError)]
pub enum RuntimeOpsError {
    #[error(transparent)]
    EnvOps(#[from] env::EnvOpsError),

    #[error(transparent)]
    LogOps(#[from] log::LogOpsError),

    #[error(transparent)]
    MemoryRegistryOps(#[from] memory::MemoryRegistryOpsError),

    #[error(transparent)]
    WasmOps(#[from] wasm::WasmOpsError),
}

impl From<RuntimeOpsError> for InternalError {
    fn from(err: RuntimeOpsError) -> Self {
        OpsError::from(err).into()
    }
}
