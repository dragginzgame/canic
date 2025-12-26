pub mod directory;
pub mod placement;
pub mod pool;

use crate::ThisError;

///
/// PolicyError
///

#[derive(Debug, ThisError)]
pub enum PolicyError {
    #[error(transparent)]
    PoolPolicyError(#[from] pool::PoolPolicyError),

    #[error(transparent)]
    ShardingPolicyError(#[from] placement::sharding::ShardingPolicyError),
}
