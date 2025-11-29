pub mod assign;
pub mod hrw;
pub mod policy;
pub mod registry;

pub use {assign::*, policy::*, registry::*};

use crate::{
    Error, ThisError,
    ops::{OpsError, model::ModelOpsError, model::memory::MemoryOpsError},
};

///
/// ShardingOpsError
/// Logical or configuration errors that occur during sharding planning.
///

#[derive(Debug, ThisError)]
pub enum ShardingOpsError {
    #[error("shard pool not found: {0}")]
    PoolNotFound(String),

    #[error("shard cap reached")]
    ShardCapReached,

    #[error("shard creation blocked: {0}")]
    ShardCreationBlocked(String),

    #[error("sharding disabled")]
    ShardingDisabled,

    #[error("tenant '{0}' not found")]
    TenantNotFound(String),
}

impl From<ShardingOpsError> for Error {
    fn from(err: ShardingOpsError) -> Self {
        OpsError::ModelOpsError(ModelOpsError::MemoryOpsError(
            MemoryOpsError::ShardingOpsError(err),
        ))
        .into()
    }
}
