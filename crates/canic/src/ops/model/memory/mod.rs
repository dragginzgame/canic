pub mod cycles;
pub mod directory;
pub mod log;
pub mod reserve;
pub mod scaling;
pub mod sharding;
pub mod topology;

use crate::{
    Error, ThisError,
    ops::{
        OpsError,
        model::{
            ModelOpsError,
            memory::{scaling::ScalingOpsError, sharding::ShardingOpsError},
        },
    },
};

///
/// MemoryOpsError
///

#[derive(Debug, ThisError)]
pub enum MemoryOpsError {
    #[error(transparent)]
    ScalingOpsError(#[from] ScalingOpsError),

    #[error(transparent)]
    ShardingOpsError(#[from] ShardingOpsError),
}

impl From<MemoryOpsError> for Error {
    fn from(err: MemoryOpsError) -> Self {
        OpsError::ModelOpsError(ModelOpsError::MemoryOpsError(err)).into()
    }
}
