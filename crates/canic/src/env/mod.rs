#[macro_use]
mod macros;

pub mod ck;
pub mod nns;
pub mod sns;

use crate::env::sns::SnsError;
use thiserror::Error as ThisError;

///
/// EnvError
///

#[derive(Debug, ThisError)]
pub enum EnvError {
    #[error(transparent)]
    SnsError(#[from] SnsError),
}
