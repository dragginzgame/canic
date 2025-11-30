pub mod assign;
pub mod hrw;
pub mod metrics;
pub mod policy;
pub mod registry;

pub use {assign::*, metrics::*, policy::*, registry::*};

use crate::{
    Error, ThisError, model::memory::sharding::ShardEntry, ops::model::memory::MemoryOpsError,
    types::Principal,
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

    #[error("shard not found: {0}")]
    ShardNotFound(Principal),

    #[error("shard full: {0}")]
    ShardFull(Principal),

    #[error("sharding disabled")]
    ShardingDisabled,

    #[error("tenant '{0}' not found")]
    TenantNotFound(String),
}

impl From<ShardingOpsError> for Error {
    fn from(err: ShardingOpsError) -> Self {
        MemoryOpsError::ShardingOpsError(err).into()
    }
}

///
/// ShardingRegistryDto
///

pub type ShardingRegistryDto = Vec<(Principal, ShardEntry)>;
