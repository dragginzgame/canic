pub mod canister;
pub mod delegation;
pub mod pool;
pub mod request;
pub mod response;
pub mod root;
pub mod shard;
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
        types::{CanisterType, Cycles, Int, Nat, Principal, Subaccount},
    };
    pub use serde::{Deserialize, Serialize};
}

use thiserror::Error as ThisError;

///
/// OpsError
///

#[derive(Debug, ThisError)]
pub enum OpsError {
    #[error("this function can only be called from the root canister")]
    NotRoot,

    #[error(transparent)]
    DelegationError(#[from] delegation::DelegationError),

    #[error(transparent)]
    RequestError(#[from] request::RequestError),

    #[error(transparent)]
    ShardError(#[from] shard::ShardError),
}
