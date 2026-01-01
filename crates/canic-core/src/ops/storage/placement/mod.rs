pub mod adapter;
pub mod scaling;
pub mod sharding;

use crate::{Error, ThisError, ops::storage::StorageOpsError};

///
/// PlacementOpsError
///

#[derive(Debug, ThisError)]
pub enum PlacementOpsError {
    #[error(transparent)]
    ShardingRegistryOps(#[from] sharding::ShardingRegistryOpsError),
}

impl From<PlacementOpsError> for Error {
    fn from(err: PlacementOpsError) -> Self {
        StorageOpsError::PlacementOps(err).into()
    }
}
