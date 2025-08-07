pub mod cascade;
pub mod ic;
pub mod icrc;
pub mod request;
pub mod response;
pub mod root;

use thiserror::Error as ThisError;

///
/// InterfaceError
///

#[derive(Debug, ThisError)]
pub enum InterfaceError {
    #[error("indexable canisters can only be created on root")]
    CannotCreateIndexable,

    #[error("this can only be called from the root canister")]
    NotRoot,

    #[error(transparent)]
    IcError(#[from] ic::IcError),

    #[error(transparent)]
    RequestError(#[from] request::RequestError),
}
