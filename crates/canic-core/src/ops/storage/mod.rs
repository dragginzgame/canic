pub mod children;
pub mod cycles;
pub mod directory;
pub mod placement;
pub mod pool;
pub mod registry;
pub mod state;

use crate::{Error, ThisError, ops::OpsError};

///
/// StorageOpsError
/// Error envelope shared across operations submodules
///

#[derive(Debug, ThisError)]
pub enum StorageOpsError {
    #[error(transparent)]
    PlacementOps(#[from] placement::PlacementOpsError),

    #[error(transparent)]
    RegistryOps(#[from] registry::RegistryOpsError),

    #[error(transparent)]
    StateOps(#[from] state::StateOpsError),
}

impl From<StorageOpsError> for Error {
    fn from(err: StorageOpsError) -> Self {
        OpsError::StorageOps(err).into()
    }
}
