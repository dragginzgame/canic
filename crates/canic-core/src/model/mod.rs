pub mod icrc;
pub mod memory;
pub mod metrics;

use crate::model::memory::MemoryError;
use thiserror::Error as ThisError;

///
/// ModelError
///

#[derive(Debug, ThisError)]
pub enum ModelError {
    #[error(transparent)]
    Memory(#[from] MemoryError),
}
