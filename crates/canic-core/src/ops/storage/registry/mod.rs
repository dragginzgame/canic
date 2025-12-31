pub mod app;
pub mod subnet;

use crate::{Error, ThisError, ops::storage::StorageOpsError};

///
/// RegistryOpsError
///

#[derive(Debug, ThisError)]
pub enum RegistryOpsError {
    #[error(transparent)]
    SubnetRegistryOps(#[from] subnet::SubnetRegistryOpsError),
}

impl From<RegistryOpsError> for Error {
    fn from(err: RegistryOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}
