pub mod canister;
pub mod env;
pub mod log;
pub mod memory;
pub mod metrics;
pub mod network;
pub mod timer;
pub mod wasm;

use crate::{Error, ThisError, ops::OpsError};

///
/// RuntimeOpsError
///

#[derive(Debug, ThisError)]
pub enum RuntimeOpsError {
    #[error(transparent)]
    EnvOps(#[from] env::EnvOpsError),

    #[error(transparent)]
    MemoryOps(#[from] memory::MemoryOpsError),

    #[error(transparent)]
    WasmOps(#[from] wasm::WasmOpsError),
}

impl From<RuntimeOpsError> for Error {
    fn from(err: RuntimeOpsError) -> Self {
        OpsError::from(err).into()
    }
}
