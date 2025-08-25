pub mod canister;
pub mod pool;
pub mod request;
pub mod response;
pub mod root;
pub mod state;

pub mod prelude {
    pub use crate::{
        Log,
        cdk::{
            api::{canister_self, msg_caller},
            call::Call,
            candid::CandidType,
        },
        interface::InterfaceError,
        log,
        ops::OpsError,
        types::{Account, CanisterType, Cycles, Int, Nat, Principal, Subaccount},
    };
    pub use serde::{Deserialize, Serialize};
}

use crate::interface::InterfaceError;
use thiserror::Error as ThisError;

///
/// OpsError
///

#[derive(Debug, ThisError)]
pub enum OpsError {
    #[error("this function can only be called from the root canister")]
    NotRoot,

    #[error(transparent)]
    InterfaceError(#[from] InterfaceError),

    #[error(transparent)]
    RequestError(#[from] request::RequestError),
}
