pub mod cascade;
pub mod orchestrator;

use crate::{Error, ThisError, ops::OpsError};

///
/// OrchestrationOpsError
///

#[derive(Debug, ThisError)]
pub enum OrchestrationOpsError {
    #[error(transparent)]
    CascadeOpsError(#[from] cascade::CascadeOpsError),

    #[error(transparent)]
    OrchestrationOpsError(#[from] orchestrator::OrchestratorOpsError),
}

impl From<OrchestrationOpsError> for Error {
    fn from(err: OrchestrationOpsError) -> Self {
        OpsError::OrchestrationOpsError(err).into()
    }
}
