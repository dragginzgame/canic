pub mod serialize;

pub use serialize::{SerializeError, deserialize, serialize};
use thiserror::Error as ThisError;

///
/// CoreError
///

#[derive(Debug, ThisError)]
pub enum CoreError {
    #[error("{0}")]
    SerializeError(#[from] SerializeError),
}
