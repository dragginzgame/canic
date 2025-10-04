pub mod cycles;
pub mod scaling;
pub mod sharding;

use thiserror::Error as ThisError;

///
/// CapabilityError
///

#[derive(Debug, ThisError)]
pub enum CapabilityError {
    #[error(transparent)]
    ScalingError(#[from] scaling::ScalingError),

    #[error(transparent)]
    ShardingError(#[from] sharding::ShardingError),
}
