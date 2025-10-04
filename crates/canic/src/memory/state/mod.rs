mod app;
mod canister;
mod subnet;

pub use app::*;
pub use canister::*;
pub use subnet::*;

use thiserror::Error as ThisError;

///
/// StateError
///

#[derive(Debug, ThisError)]
pub enum StateError {
    #[error(transparent)]
    AppStateError(#[from] AppStateError),

    #[error(transparent)]
    CanisterStateError(#[from] CanisterStateError),

    #[error(transparent)]
    SubnetStateError(#[from] SubnetStateError),
}
