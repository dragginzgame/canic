pub mod network;
pub mod provision;
pub mod xrc;

use crate::{Error, ThisError, workflow::WorkflowError};

///
/// IcError
///

#[derive(Debug, ThisError)]
pub enum IcError {
    #[error(transparent)]
    ProvisionOps(#[from] provision::ProvisionError),

    #[error(transparent)]
    Xrc(#[from] xrc::XrcWorkflowError),
}

impl From<IcError> for Error {
    fn from(err: IcError) -> Self {
        WorkflowError::from(err).into()
    }
}
