pub mod auth;
pub mod cascade;
pub mod guard;
pub mod ic;
//pub mod request;
//pub mod response;
pub mod wasm;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// InterfaceError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum InterfaceError {
    #[error(transparent)]
    AuthError(#[from] auth::AuthError),

    #[error(transparent)]
    CascadeError(#[from] cascade::CascadeError),

    #[error(transparent)]
    GuardError(#[from] guard::GuardError),

    //  #[error(transparent)]
    //  RequestError(#[from] request::RequestError),
    #[error(transparent)]
    WasmError(#[from] wasm::WasmError),
}
