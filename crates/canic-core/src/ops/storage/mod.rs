pub mod children;
pub mod cycles;
pub mod directory;
pub mod placement;
pub mod pool;
pub mod registry;
pub mod state;

// re-export from storage
pub use crate::storage::canister::CanisterRecord;

use crate::{InternalError, ops::OpsError};
use thiserror::Error as ThisError;

///
/// StorageOpsError
/// InternalError envelope shared across operations submodules
///

#[derive(Debug, ThisError)]
pub enum StorageOpsError {
    #[error(transparent)]
    DirectoryOps(#[from] directory::DirectoryOpsError),

    #[error(transparent)]
    ShardingRegistryOps(#[from] placement::sharding::ShardingRegistryOpsError),

    #[error(transparent)]
    SubnetRegistryOps(#[from] registry::subnet::SubnetRegistryOpsError),

    #[error(transparent)]
    AppStateOps(#[from] state::app::AppStateOpsError),
}

impl From<StorageOpsError> for InternalError {
    fn from(err: StorageOpsError) -> Self {
        OpsError::StorageOps(err).into()
    }
}
