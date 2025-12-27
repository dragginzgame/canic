pub mod app;
pub mod subnet;

pub use app::*;
pub use subnet::*;

pub use crate::model::memory::registry::SubnetIdentity;

use crate::{Error, ThisError, ops::storage::StorageOpsError};

///
/// RegistryOpsError
///

#[derive(Debug, ThisError)]
pub enum RegistryOpsError {
    #[error(transparent)]
    SubnetRegistryOpsError(#[from] SubnetRegistryOpsError),
}

impl From<RegistryOpsError> for Error {
    fn from(err: RegistryOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}
