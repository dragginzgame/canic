pub mod cycles;
pub mod directory;
pub mod pool;
pub mod scaling;
pub mod sharding;
pub mod state;
pub mod topology;

use crate::{Error, ThisError, ops::OpsError};

///
/// StorageOpsError
/// Error envelope shared across operations submodules
///

#[derive(Debug, ThisError)]
pub enum StorageOpsError {
    #[error(transparent)]
    ShardingRegistryOpsError(#[from] sharding::ShardingRegistryOpsError),

    #[error(transparent)]
    StateOpsError(#[from] state::StateOpsError),

    #[error(transparent)]
    TopologyOpsError(#[from] topology::TopologyOpsError),
}

impl From<StorageOpsError> for Error {
    fn from(err: StorageOpsError) -> Self {
        OpsError::StorageOpsError(err).into()
    }
}
