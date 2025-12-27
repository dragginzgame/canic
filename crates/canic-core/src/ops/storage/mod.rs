pub mod children;
pub mod cycles;
pub mod directory;
pub mod pool;
pub mod registry;
pub mod scaling;
pub mod sharding;
pub mod state;

pub use crate::model::memory::CanisterSummary;

use crate::{Error, ThisError, ops::OpsError};

///
/// StorageOpsError
/// Error envelope shared across operations submodules
///

#[derive(Debug, ThisError)]
pub enum StorageOpsError {
    #[error(transparent)]
    RegistryOpsError(#[from] registry::RegistryOpsError),

    #[error(transparent)]
    ShardingRegistryOpsError(#[from] sharding::ShardingRegistryOpsError),

    #[error(transparent)]
    StateOpsError(#[from] state::StateOpsError),
}

impl From<StorageOpsError> for Error {
    fn from(err: StorageOpsError) -> Self {
        OpsError::StorageOpsError(err).into()
    }
}
