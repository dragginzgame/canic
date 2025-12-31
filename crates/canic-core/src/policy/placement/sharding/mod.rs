pub mod hrw;
pub mod metrics;
pub mod policy;

use crate::{Error, ThisError, policy::PolicyError};

///
/// ShardingPolicyError
///

#[derive(Debug, ThisError)]
pub enum ShardingPolicyError {
    #[error("shard pool not found: {0}")]
    PoolNotFound(String),

    #[error("shard creation blocked: {0}")]
    ShardCreationBlocked(String),

    #[error("sharding disabled")]
    ShardingDisabled,
}

impl From<ShardingPolicyError> for Error {
    fn from(err: ShardingPolicyError) -> Self {
        PolicyError::ShardingPolicy(err).into()
    }
}
