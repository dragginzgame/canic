pub mod cycles;
pub mod directory;
pub mod env;
pub mod log;
pub mod registry;
pub mod reserve;
pub mod scaling;
pub mod sharding;
pub mod state;
pub mod topology;

pub use env::EnvOps;

use crate::{
    Error, ThisError,
    ops::model::{
        ModelOpsError,
        memory::{
            env::EnvOpsError, registry::MemoryRegistryOpsError, scaling::ScalingOpsError,
            sharding::ShardingOpsError, state::AppStateOpsError,
        },
    },
};

///
/// MemoryOpsError
///

#[derive(Debug, ThisError)]
pub enum MemoryOpsError {
    #[error(transparent)]
    AppStateOpsError(#[from] AppStateOpsError),

    #[error(transparent)]
    EnvOpsError(#[from] EnvOpsError),

    #[error(transparent)]
    MemoryRegistryOpsError(#[from] MemoryRegistryOpsError),

    #[error(transparent)]
    ScalingOpsError(#[from] ScalingOpsError),

    #[error(transparent)]
    ShardingOpsError(#[from] ShardingOpsError),
}

impl From<MemoryOpsError> for Error {
    fn from(err: MemoryOpsError) -> Self {
        ModelOpsError::MemoryOpsError(err).into()
    }
}
