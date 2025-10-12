pub mod cycles;
pub mod scaling;
pub mod sharding;

use thiserror::Error as ThisError;

///
/// ExtensionError
///

#[derive(Debug, ThisError)]
pub enum ExtensionError {
    #[error(transparent)]
    ScalingError(#[from] scaling::ScalingError),

    #[error(transparent)]
    ShardingError(#[from] sharding::ShardingError),
}
