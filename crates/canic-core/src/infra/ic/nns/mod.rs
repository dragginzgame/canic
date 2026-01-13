pub mod registry;

use crate::infra::{InfraError, ic::IcInfraError};
use thiserror::Error as ThisError;

///
/// NnsInfraError
///

#[derive(Debug, ThisError)]
pub enum NnsInfraError {
    #[error(transparent)]
    NnsRegistryInfra(#[from] registry::NnsRegistryInfraError),
}

impl From<NnsInfraError> for InfraError {
    fn from(err: NnsInfraError) -> Self {
        IcInfraError::from(err).into()
    }
}
