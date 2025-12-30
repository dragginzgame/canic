pub mod canister;
pub mod env;
pub mod log;
pub mod metrics;
pub mod timer;
pub mod wasm;

use crate::ThisError;

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
    MemoryInfraError(#[from] memory::MemoryInfraError),

    #[error(transparent)]
    RpcInfraError(#[from] rpc::RpcInfraError),
}
