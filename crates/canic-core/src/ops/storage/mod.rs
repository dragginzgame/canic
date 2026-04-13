pub mod auth;
pub mod children;
pub mod cycles;
pub mod index;
pub mod intent;
pub mod placement;
pub mod pool;
pub mod registry;
pub mod replay;
pub mod state;

use crate::{InternalError, ops::OpsError};
use thiserror::Error as ThisError;

///
/// StorageOpsError
/// InternalError envelope shared across operations submodules
///

#[derive(Debug, ThisError)]
pub enum StorageOpsError {
    #[error(transparent)]
    AppStateOps(#[from] state::app::AppStateOpsError),

    #[error(transparent)]
    IndexOps(#[from] index::IndexOpsError),

    #[error(transparent)]
    IntentStoreOps(#[from] intent::IntentStoreOpsError),

    #[error(transparent)]
    DirectoryRegistryOps(#[from] placement::directory::DirectoryRegistryOpsError),

    #[cfg(feature = "sharding")]
    #[error(transparent)]
    ShardingRegistryOps(#[from] placement::sharding::ShardingRegistryOpsError),

    #[error(transparent)]
    SubnetRegistryOps(#[from] registry::subnet::SubnetRegistryOpsError),
}

impl From<StorageOpsError> for InternalError {
    fn from(err: StorageOpsError) -> Self {
        OpsError::StorageOps(err).into()
    }
}
