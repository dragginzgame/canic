pub mod assign;
mod hrw;
mod metrics;
pub mod policy;

pub use crate::ops::storage::sharding::ShardingRegistryOps;
pub use metrics::{PoolMetrics, pool_metrics};
pub use {assign::*, policy::*};

use crate::{Error, ThisError, cdk::types::Principal, ops::storage::sharding::ShardEntry};

///
/// ShardingOpsError
/// Logical or configuration errors that occur during sharding planning.
///

#[derive(Debug, ThisError)]
pub enum ShardingOpsError {
    #[error("shard pool not found: {0}")]
    PoolNotFound(String),

    #[error("shard creation blocked: {0}")]
    ShardCreationBlocked(String),

    #[error("sharding disabled")]
    ShardingDisabled,
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
