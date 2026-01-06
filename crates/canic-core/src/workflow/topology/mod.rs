pub mod children;
pub mod directory;
pub mod guard;
pub mod registry;

use crate::{Error, ThisError, workflow::WorkflowError};

///
/// TopologyWorkflowError
/// Errors raised during synchronization
///

#[derive(Debug, ThisError)]
pub enum TopologyWorkflowError {
    #[error(transparent)]
    TopologyGuard(#[from] guard::TopologyGuardError),
}

impl From<TopologyWorkflowError> for Error {
    fn from(err: TopologyWorkflowError) -> Self {
        WorkflowError::from(err).into()
    }
}
