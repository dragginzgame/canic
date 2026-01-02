use super::IcOpsError;
use crate::{
    Error, ThisError,
    cdk::{
        call::{Call as IcCall, CallFailed, CandidDecodeFailed, Response as IcResponse},
        candid::Principal,
    },
    infra::InfraError,
    ops::runtime::metrics::icc::record_icc_call,
};
use candid::{CandidType, encode_one};
use serde::de::DeserializeOwned;

///
/// CallError
///

#[derive(Debug, ThisError)]
pub enum CallError {
    #[error("call failed: {0}")]
    Failed(#[from] CallFailed),

    #[error("candid decode failed: {0}")]
    CandidDecode(#[from] CandidDecodeFailed),
}

impl From<CallError> for InfraError {
    fn from(err: CallError) -> Self {
        match err {
            CallError::Failed(err) => Self::from(err),
            CallError::CandidDecode(err) => Self::from(err),
        }
    }
}

impl From<CallError> for IcOpsError {
    fn from(err: CallError) -> Self {
        match err {
            CallError::Failed(err) => Self::from(err),
            CallError::CandidDecode(err) => Self::from(err),
        }
    }
}

impl From<CallError> for Error {
    fn from(err: CallError) -> Self {
        InfraError::from(err).into()
    }
}

///
/// Call
/// Ops-owned call builder that records metrics.
///

pub struct Call;

impl Call {
    #[must_use]
    #[expect(dead_code)]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        let canister_id: Principal = canister_id.into();
        record_icc_call(canister_id, method);

        CallBuilder {
            wait: WaitMode::Bounded,
            canister_id,
            method: method.to_string(),
            cycles: 0,
            args: None,
        }
    }

    /// Create a call builder that will be awaited without cycle limits.
    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        let canister_id: Principal = canister_id.into();
        record_icc_call(canister_id, method);

        CallBuilder {
            wait: WaitMode::Unbounded,
            canister_id,
            method: method.to_string(),
            cycles: 0,
            args: None,
        }
    }
}

///
/// CallBuilder
///

pub struct CallBuilder {
    wait: WaitMode,
    canister_id: Principal,
    method: String,
    cycles: u128,
    args: Option<Vec<u8>>,
}

impl CallBuilder {
    #[must_use]
    pub fn with_arg<A: CandidType>(mut self, arg: A) -> Self {
        let bytes = encode_one(arg).expect("call arg encoding failed");
        self.args = Some(bytes);
        self
    }

    #[must_use]
    pub const fn with_cycles(mut self, cycles: u128) -> Self {
        self.cycles = cycles;
        self
    }

    pub async fn execute(self) -> Result<CallResult, CallError> {
        let mut call = match self.wait {
            WaitMode::Bounded => IcCall::bounded_wait(self.canister_id, &self.method),
            WaitMode::Unbounded => IcCall::unbounded_wait(self.canister_id, &self.method),
        };

        call = call.with_cycles(self.cycles);
        if let Some(ref args) = self.args {
            call = call.with_raw_args(args);
        }

        let response = call.await.map_err(CallError::from)?;
        Ok(CallResult { inner: response })
    }
}

///
/// CallResult
///

pub struct CallResult {
    inner: IcResponse,
}

#[derive(Clone, Copy, Debug)]
enum WaitMode {
    Bounded,
    Unbounded,
}

impl CallResult {
    pub fn candid<R>(&self) -> Result<R, CallError>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner.candid().map_err(CallError::from)
    }
}
