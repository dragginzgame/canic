pub mod ic;

use crate::{
    ThisError,
    cdk::{
        call::{CallFailed, CandidDecodeFailed, Error as CallError},
        candid::Error as CandidError,
    },
};

///
/// Prelude
///

pub mod prelude {
    pub use crate::{
        cdk::{
            api::{canister_self, msg_caller},
            candid::CandidType,
            types::{Account, Cycles, Int, Nat, Principal, Subaccount},
        },
        ids::CanisterRole,
        log,
        log::{Level, Topic},
    };
    pub use serde::{Deserialize, Serialize};
}

///
/// InfraError
///

#[derive(Debug, ThisError)]
pub enum InfraError {
    #[error(transparent)]
    IcInfra(#[from] ic::IcInfraError),

    #[error(transparent)]
    Call(#[from] CallError),

    #[error(transparent)]
    CallFailed(#[from] CallFailed),

    #[error(transparent)]
    Candid(#[from] CandidError),

    #[error(transparent)]
    CandidDecode(#[from] CandidDecodeFailed),
}
