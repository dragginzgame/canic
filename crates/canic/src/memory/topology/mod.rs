mod app;
mod subnet;

pub use app::*;
pub use subnet::*;

use crate::{Error, ThisError, memory::MemoryError, types::CanisterType};
use candid::Principal;

///
/// TopologyError
///

#[derive(Debug, ThisError)]
pub enum TopologyError {
    #[error("canister already installed: {0}")]
    CanisterAlreadyInstalled(Principal),

    #[error("canister not found: {0}")]
    PrincipalNotFound(Principal),

    #[error("subnet not found: {0}")]
    SubnetNotFound(Principal),

    #[error("canister not found: {0}")]
    TypeNotFound(CanisterType),
}

impl From<TopologyError> for Error {
    fn from(err: TopologyError) -> Self {
        MemoryError::from(err).into()
    }
}
