pub mod ic;
pub mod icrc;
pub mod request;
pub mod response;
pub mod root;
pub mod state;

use thiserror::Error as ThisError;

///
/// InterfaceError
///

#[derive(Debug, ThisError)]
pub enum InterfaceError {
    #[error(transparent)]
    IcError(#[from] ic::IcError),

    #[error(transparent)]
    RequestError(#[from] request::RequestError),
}
