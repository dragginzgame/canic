pub mod reserve;
pub mod service;

use crate::{Error, ThisError, ops::OpsError};

///
/// SubsystemOpsError
///

#[derive(Debug, ThisError)]
pub enum SubsystemOpsError {
    #[error(transparent)]
    ReserveOpsError(#[from] reserve::ReserveOpsError),
}

impl From<SubsystemOpsError> for Error {
    fn from(err: SubsystemOpsError) -> Self {
        OpsError::from(err).into()
    }
}
