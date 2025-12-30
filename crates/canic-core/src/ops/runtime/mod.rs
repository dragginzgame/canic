pub mod canister;
pub mod env;
pub mod log;
pub mod memory;
pub mod metrics;
pub mod timer;
pub mod wasm;

use crate::{Error, ThisError, ops::OpsError};

///
/// RuntimeOpsError
///

#[derive(Debug, ThisError)]
pub enum RuntimeOpsError {
    /// Raised when a function requires root context, but was called from a child.
    #[error("operation must be called from the root canister")]
    NotRoot,

    /// Raised when a function must not be called from root.
    #[error("operation cannot be called from the root canister")]
    IsRoot,

    #[error(transparent)]
    EnvOpsError(#[from] env::EnvOpsError),

    #[error(transparent)]
    MemoryRegistryOpsError(#[from] memory::MemoryRegistryOpsError),
}

impl From<RuntimeOpsError> for Error {
    fn from(err: RuntimeOpsError) -> Self {
        OpsError::from(err).into()
    }
}
