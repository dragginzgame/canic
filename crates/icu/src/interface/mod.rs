pub mod ic;
pub mod icrc;
pub mod request;
pub mod root;
pub mod state;

use thiserror::Error as ThisError;

///
/// InterfaceError
///

#[derive(Debug, ThisError)]
pub enum InterfaceError {
    #[error("indexable canisters can only be created on root")]
    CannotCreateIndexable,

    #[error(transparent)]
    IcError(#[from] ic::IcError),

    #[error(transparent)]
    RequestError(#[from] request::RequestError),
}
