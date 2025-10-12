mod app;
mod subnet;

pub use app::*;
pub use subnet::*;

use crate::{Error, ThisError, memory::MemoryError};

///
/// DirectoryError
///

#[derive(Debug, ThisError)]
pub enum DirectoryError {
    #[error(transparent)]
    AppDirectoryError(#[from] AppDirectoryError),

    #[error(transparent)]
    SubnetDirectoryError(#[from] SubnetDirectoryError),
}

impl From<DirectoryError> for Error {
    fn from(err: DirectoryError) -> Self {
        MemoryError::from(err).into()
    }
}
