pub mod ic;

use crate::ThisError;

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
    IcInfraError(#[from] ic::IcInfraError),
}
