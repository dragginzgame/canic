mod app;
mod subnet;

pub use app::*;
pub use subnet::*;

use crate::{
    Error, ThisError,
    model::{ModelError, memory::MemoryError},
};

///
/// StateError
///

#[derive(Debug, ThisError)]
pub enum StateError {
    #[error(transparent)]
    AppStateError(#[from] AppStateError),

    #[error(transparent)]
    SubnetStateError(#[from] SubnetStateError),
}

impl From<StateError> for Error {
    fn from(err: StateError) -> Self {
        ModelError::MemoryError(MemoryError::from(err)).into()
    }
}
