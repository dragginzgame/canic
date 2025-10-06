//! Interface Helpers
//! Aggregates thin wrappers around external canisters (IC, ck-tokens, ICRC).

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

use thiserror::Error as ThisError;

///
/// InterfaceError
/// Shared result type for interface helpers.
///

#[derive(Debug, ThisError)]
pub enum InterfaceError {
    #[error("cycles overflow")]
    CyclesOverflow,

    #[error("wasm hash matches")]
    WasmHashMatches,
}
