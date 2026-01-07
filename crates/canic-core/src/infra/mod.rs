pub mod ic;

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

use crate::{
    ThisError,
    cdk::{
        call::{CallFailed, CandidDecodeFailed, Error as CallError},
        candid::Error as CandidError,
    },
};

///
/// InfraError
///

#[derive(Debug, ThisError)]
pub enum InfraError {
    #[error(transparent)]
    IcInfra(#[from] ic::IcInfraError),
}

impl From<CallFailed> for InfraError {
    fn from(err: CallFailed) -> Self {
        ic::IcInfraError::from(err).into()
    }
}

impl From<CandidDecodeFailed> for InfraError {
    fn from(err: CandidDecodeFailed) -> Self {
        ic::IcInfraError::from(err).into()
    }
}

impl From<CandidError> for InfraError {
    fn from(err: CandidError) -> Self {
        ic::IcInfraError::from(err).into()
    }
}

/// Normalize call-layer errors back into IC mechanical failures.
///
/// This conversion must remain lossless and mechanical only.
impl From<CallError> for InfraError {
    fn from(err: CallError) -> Self {
        match err {
            CallError::CandidDecodeFailed(err) => Self::from(err),
            CallError::InsufficientLiquidCycleBalance(err) => {
                Self::from(CallFailed::InsufficientLiquidCycleBalance(err))
            }
            CallError::CallPerformFailed(err) => Self::from(CallFailed::CallPerformFailed(err)),
            CallError::CallRejected(err) => Self::from(CallFailed::CallRejected(err)),
        }
    }
}
