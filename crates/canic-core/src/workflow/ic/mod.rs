pub mod http;
pub mod payment;
pub mod provision;
pub mod timer;
pub mod xrc;

use crate::{Error, ThisError, workflow::WorkflowError};

///
/// IcError
///

#[derive(Debug, ThisError)]
pub enum IcError {
    #[error(transparent)]
    ProvisionOpsError(#[from] provision::ProvisionError),
}

impl From<IcError> for Error {
    fn from(err: IcError) -> Self {
        WorkflowError::from(err).into()
    }
}
