use crate::{
    cdk::{
        call::{Call as IcCall, Response as IcResponse},
        candid::CandidType,
        types::Principal,
    },
    infra::InfraError,
};
use candid::encode_one;
use serde::de::DeserializeOwned;

///
/// Call
/// Raw IC call builder with no metrics or policy.
///

pub struct Call;

impl Call {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        let canister_id: Principal = canister_id.into();

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
    pub fn try_with_arg<A: CandidType>(mut self, arg: A) -> Result<Self, InfraError> {
        let bytes = encode_one(arg).map_err(InfraError::from)?;
        self.args = Some(bytes);

        Ok(self)
    }

    #[must_use]
    pub const fn with_cycles(mut self, cycles: u128) -> Self {
        self.cycles = cycles;
        self
    }

    pub async fn execute(self) -> Result<CallResult, InfraError> {
        let mut call = match self.wait {
            WaitMode::Bounded => IcCall::bounded_wait(self.canister_id, &self.method),
            WaitMode::Unbounded => IcCall::unbounded_wait(self.canister_id, &self.method),
        };

        call = call.with_cycles(self.cycles);
        if let Some(ref args) = self.args {
            call = call.with_raw_args(args);
        }

        let response = call.await.map_err(InfraError::from)?;

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
    pub fn candid<R>(&self) -> Result<R, InfraError>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner.candid().map_err(InfraError::from)
    }
}
