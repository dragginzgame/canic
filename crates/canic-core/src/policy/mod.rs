pub mod directory;
pub mod placement;
pub mod pool;
pub mod upgrade;

use crate::ThisError;

///
/// PolicyError
///

#[derive(Debug, ThisError)]
pub enum PolicyError {
    #[error(transparent)]
    PoolPolicy(#[from] pool::PoolPolicyError),

    #[error(transparent)]
    ScalingPolicy(#[from] placement::scaling::ScalingPolicyError),

    #[error(transparent)]
    ShardingPolicy(#[from] placement::sharding::ShardingPolicyError),
}
