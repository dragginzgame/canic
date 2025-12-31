pub mod registry;

use crate::{Error, ThisError, infra::ic::IcInfraError};

///
/// NnsInfraError
///

#[derive(Debug, ThisError)]
pub enum NnsInfraError {
    #[error(transparent)]
    NnsRegistryInfra(#[from] registry::NnsRegistryInfraError),
}

impl From<NnsInfraError> for Error {
    fn from(err: NnsInfraError) -> Self {
        IcInfraError::from(err).into()
    }
}
