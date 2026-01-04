pub mod ic;
pub mod network;

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
        ThisError,
        cdk::types::{Account, Cycles, Principal},
        infra::{InfraError, ic::call::Call},
        log,
        log::Topic,
    };
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
