//! Interface Helpers
//! Aggregates thin wrappers around external canisters (IC, ck-tokens, ICRC).

pub mod ck;
pub mod ic;
pub mod icrc;

pub mod prelude {
    pub use crate::{
        cdk::{
            api::{canister_self, msg_caller},
            candid::CandidType,
            types::{Account, Int, Nat, Principal, Subaccount},
            utils::time::now_secs,
        },
        ids::CanisterRole,
        interface::{InterfaceError, ic::call::Call},
        log,
        log::Level,
        types::Cycles,
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
    #[error("wasm hash matches")]
    WasmHashMatches,

    #[error(transparent)]
    CyclesInterfaceError(#[from] ic::cycles::CyclesInterfaceError),
}
