pub mod app;
pub mod subnet;

pub use app::*;
pub use subnet::*;

use crate::{Error, ThisError, ops::model::memory::MemoryOpsError};

///
/// TopologyOpsError
///

#[derive(Debug, ThisError)]
pub enum TopologyOpsError {
    #[error(transparent)]
    SubnetCanisterRegistryOpsError(#[from] SubnetCanisterRegistryOpsError),
}

impl From<TopologyOpsError> for Error {
    fn from(err: TopologyOpsError) -> Self {
        MemoryOpsError::TopologyOpsError(err).into()
    }
}
