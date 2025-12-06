pub mod memory;

use crate::{
    Error, ThisError,
    ops::{OpsError, model::memory::MemoryOpsError},
};

///
/// ModelOpsError
/// Logical or configuration errors that occur during sharding planning.
///

#[derive(Debug, ThisError)]
pub enum ModelOpsError {
    #[error(transparent)]
    MemoryOpsError(#[from] MemoryOpsError),
}

impl From<ModelOpsError> for Error {
    fn from(err: ModelOpsError) -> Self {
        OpsError::ModelOpsError(err).into()
    }
}
