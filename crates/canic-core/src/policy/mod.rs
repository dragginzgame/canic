pub mod directory;
pub mod placement;

use crate::{Error, ThisError, ops::OpsError};

///
/// StorageOpsError
/// Error envelope shared across operations submodules
///

#[derive(Debug, ThisError)]
pub enum StorageOpsError {
    #[error(transparent)]
    EnvOpsError(#[from] env::EnvOpsError),

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
