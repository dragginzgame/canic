use crate::{
    cdk::{
        call::{Call as IcCall, Response as IcResponse},
        candid::{
            CandidType,
            utils::{ArgumentDecoder, ArgumentEncoder},
        },
        types::Principal,
    },
    infra::{InfraError, ic::IcInfraError},
};
use candid::{encode_args, encode_one};
use serde::de::DeserializeOwned;

///
/// Call
///

pub struct Call;

impl Call {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        CallBuilder::new(WaitMode::Bounded, canister_id.into(), method)
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        CallBuilder::new(WaitMode::Unbounded, canister_id.into(), method)
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
    args: Vec<u8>, // always present; defaults to ()
}

impl CallBuilder {
    fn new(wait: WaitMode, canister_id: Principal, method: &str) -> Self {
        // Spec: default args = candid empty tuple ()
        let args = encode_args(()).expect("failed to encode default candid args ()");

        Self {
            wait,
            canister_id,
            method: method.to_string(),
            cycles: 0,
            args,
        }
    }

    // Infallible convenience (panic on encoding failure, same as your current pattern)
    #[must_use]
    pub fn with_arg<A>(self, arg: A) -> Self
    where
        A: CandidType,
    {
        self.try_with_arg(arg).expect("failed to encode call arg")
    }

    #[must_use]
    pub fn with_args<A>(self, args: A) -> Self
    where
        A: ArgumentEncoder,
    {
        self.try_with_args(args)
            .expect("failed to encode call args")
    }

    pub fn try_with_arg<A>(mut self, arg: A) -> Result<Self, InfraError>
    where
        A: CandidType,
    {
        self.args = encode_one(arg).map_err(IcInfraError::from)?;

        Ok(self)
    }

    // Critical: multi-arg encoding
    pub fn try_with_args<A>(mut self, args: A) -> Result<Self, InfraError>
    where
        A: ArgumentEncoder,
    {
        self.args = encode_args(args).map_err(IcInfraError::from)?;

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
        call = call.with_raw_args(&self.args);

        let response = call.await.map_err(IcInfraError::from)?;

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
        self.inner
            .candid()
            .map_err(IcInfraError::from)
            .map_err(InfraError::from)
    }

    // Optional: parity with IC Response::candid_tuple
    pub fn candid_tuple<R>(&self) -> Result<R, InfraError>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        self.inner
            .candid_tuple()
            .map_err(IcInfraError::from)
            .map_err(InfraError::from)
    }
}
