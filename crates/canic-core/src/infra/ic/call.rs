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

const EMPTY_ARGS: &[u8] = b"DIDL\0\0";

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
        Self {
            wait,
            canister_id,
            method: method.to_string(),
            cycles: 0,
            args: EMPTY_ARGS.to_vec(),
        }
    }

    /// Use pre-encoded Candid arguments (no validation performed).
    #[must_use]
    pub fn with_raw_args(mut self, args: Vec<u8>) -> Self {
        self.args = args;
        self
    }

    /// Encode a single argument into Candid bytes (fallible).
    pub fn with_arg<A>(self, arg: A) -> Result<Self, InfraError>
    where
        A: CandidType,
    {
        let mut builder = self;
        builder.args = encode_one(arg).map_err(IcInfraError::from)?;
        Ok(builder)
    }

    /// Encode multiple arguments into Candid bytes (fallible).
    pub fn with_args<A>(self, args: A) -> Result<Self, InfraError>
    where
        A: ArgumentEncoder,
    {
        let mut builder = self;
        builder.args = encode_args(args).map_err(IcInfraError::from)?;
        Ok(builder)
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

#[cfg(test)]
mod tests {
    use super::{Call, EMPTY_ARGS, Principal, encode_args};

    #[test]
    fn empty_args_match_candid_encoding() {
        let encoded = encode_args(()).expect("encode empty tuple");
        assert_eq!(EMPTY_ARGS, encoded.as_slice());
    }

    #[test]
    fn with_raw_args_overrides_default() {
        let raw = vec![1_u8, 2, 3, 4];
        let builder = Call::bounded_wait(Principal::anonymous(), "noop").with_raw_args(raw.clone());
        assert_eq!(builder.args, raw);
    }
}
