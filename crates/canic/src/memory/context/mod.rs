pub mod canister;
pub mod subnet;

pub use canister::*;
pub use subnet::*;

use thiserror::Error as ThisError;

///
/// ContextError
///

#[derive(Debug, ThisError)]
pub enum ContextError {
    #[error(transparent)]
    CanisterContextError(#[from] CanisterContextError),

    #[error(transparent)]
    SubnetContextError(#[from] SubnetContextError),
}
