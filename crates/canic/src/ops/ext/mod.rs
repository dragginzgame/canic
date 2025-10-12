pub mod cycles;
pub mod scaling;
pub mod sharding;

use crate::{Error, ThisError, ops::OpsError};

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

impl From<ExtensionError> for Error {
    fn from(err: ExtensionError) -> Self {
        OpsError::from(err).into()
    }
}
