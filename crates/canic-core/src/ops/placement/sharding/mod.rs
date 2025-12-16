pub mod assign;
mod hrw;
mod metrics;
pub mod policy;

pub use crate::ops::storage::sharding::ShardingRegistryOps;
pub use metrics::{PoolMetrics, pool_metrics};
pub use {assign::*, policy::*};

use crate::{Error, ThisError, cdk::types::Principal, model::memory::sharding::ShardEntry};

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

    #[error("shard full: {0}")]
    ShardFull(Principal),

    #[error("shard not found: {0}")]
    ShardNotFound(Principal),

    #[error("sharding disabled")]
    ShardingDisabled,

    #[error("tenant '{0}' not found")]
    TenantNotFound(String),
}

impl From<ShardingOpsError> for Error {
    fn from(err: ShardingOpsError) -> Self {
        Self::OpsError(err.to_string())
    }
}

///
/// ShardingRegistryDto
///

pub type ShardingRegistryDto = Vec<(Principal, ShardEntry)>;
