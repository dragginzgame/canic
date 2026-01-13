pub mod cycles;
pub mod env;
pub mod log;
pub mod placement;
pub mod pool;
pub mod randomness;
pub mod topology;
pub mod upgrade;

use crate::{InternalError, ThisError, domain::DomainError};

///
/// PolicyError
///

#[derive(Debug, ThisError)]
pub enum PolicyError {
    #[error(transparent)]
    EnvPolicy(#[from] env::EnvPolicyError),

    #[error(transparent)]
    PoolPolicy(#[from] pool::PoolPolicyError),

    #[error(transparent)]
    TopologyPolicy(#[from] topology::TopologyPolicyError),

    #[error(transparent)]
    ScalingPolicy(#[from] placement::scaling::ScalingPolicyError),

    #[error(transparent)]
    ShardingPolicy(#[from] placement::sharding::ShardingPolicyError),
}

impl From<PolicyError> for InternalError {
    fn from(err: PolicyError) -> Self {
        DomainError::from(err).into()
    }
}
