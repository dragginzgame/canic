pub mod scaling;
pub mod sharding;

use crate::{InternalError, ThisError, workflow::WorkflowError};

///
/// PlacementWorkflowError
///

#[derive(Debug, ThisError)]
pub enum PlacementWorkflowError {
    #[error(transparent)]
    Scaling(#[from] scaling::ScalingWorkflowError),

    #[error(transparent)]
    Sharding(#[from] sharding::ShardingWorkflowError),
}

impl From<PlacementWorkflowError> for InternalError {
    fn from(err: PlacementWorkflowError) -> Self {
        WorkflowError::Placement(err).into()
    }
}
