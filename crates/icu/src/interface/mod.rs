pub mod ck;
pub mod ic;
pub mod icrc;

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
        types::{Account, CanisterType, Cycles, Int, Nat, Principal, Subaccount},
        utils::time::now_secs,
    };
    pub use serde::{Deserialize, Serialize};
}

use crate::cdk::{
    call::{CallFailed, CandidDecodeFailed, Error as CallError},
    candid::Error as CandidError,
};
use thiserror::Error as ThisError;

///
/// InterfaceError
///

#[derive(Debug, ThisError)]
pub enum InterfaceError {
    #[error("cycles overflow")]
    CyclesOverflow,

    #[error("wasm hash matches")]
    WasmHashMatches,

    #[error("call error: {0}")]
    CallError(CallError),

    #[error("call error: {0}")]
    CallFailed(CallFailed),

    #[error("candid error: {0}")]
    CandidDecodeFailed(CandidDecodeFailed),

    #[error("candid error: {0}")]
    CandidError(CandidError),
}

impl From<CallError> for InterfaceError {
    fn from(e: CallError) -> Self {
        Self::CallError(e)
    }
}

impl From<CallFailed> for InterfaceError {
    fn from(e: CallFailed) -> Self {
        Self::CallFailed(e)
    }
}

impl From<CandidError> for InterfaceError {
    fn from(e: CandidError) -> Self {
        Self::CandidError(e)
    }
}

impl From<CandidDecodeFailed> for InterfaceError {
    fn from(e: CandidDecodeFailed) -> Self {
        Self::CandidDecodeFailed(e)
    }
}
