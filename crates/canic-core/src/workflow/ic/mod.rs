pub mod network;
pub mod provision;
pub mod xrc;

use crate::{Error, ThisError, workflow::WorkflowError};

///
/// IcWorkflowError
///

#[derive(Debug, ThisError)]
pub enum IcWorkflowError {
    #[error(transparent)]
    ProvisionWorkflow(#[from] provision::ProvisionWorkflowError),

    #[error(transparent)]
    XrcWorkflow(#[from] xrc::XrcWorkflowError),
}

impl From<IcWorkflowError> for Error {
    fn from(err: IcWorkflowError) -> Self {
        WorkflowError::from(err).into()
    }
}
