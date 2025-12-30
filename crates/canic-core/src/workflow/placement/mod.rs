pub mod scaling;
pub mod sharding;

use crate::{Error, ThisError, workflow::WorkflowError};

///
/// PlacementError
///

#[derive(Debug, ThisError)]
pub enum PlacementError {
    #[error("parent {0} not found in registry")]
    Scaling(scaling::ScalingWorkflowError),
}

impl From<PlacementError> for Error {
    fn from(err: PlacementError) -> Self {
        WorkflowError::Placement(err).into()
    }
}
