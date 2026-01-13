pub mod ic;

use crate::{InternalError, InternalErrorOrigin};
use thiserror::Error as ThisError;

///
/// InfraError
///

#[derive(Debug, ThisError)]
pub enum InfraError {
    #[error(transparent)]
    IcInfra(#[from] ic::IcInfraError),
}

impl From<InfraError> for InternalError {
    fn from(err: InfraError) -> Self {
        Self::infra(InternalErrorOrigin::Infra, err.to_string())
    }
}
