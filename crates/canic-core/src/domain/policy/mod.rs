pub mod directory;
pub mod placement;
pub mod pool;
pub mod upgrade;

use crate::{Error, ThisError, domain::DomainError};

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

impl From<PolicyError> for Error {
    fn from(err: PolicyError) -> Self {
        DomainError::from(err).into()
    }
}
