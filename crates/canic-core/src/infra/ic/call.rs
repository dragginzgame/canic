use crate::{
    cdk::{
        call::Response,
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
use std::borrow::Cow;

const EMPTY_ARGS: &[u8] = b"DIDL\0\0";

///
/// Call
///

pub struct Call;

impl Call {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder::new(WaitMode::Bounded, canister_id.into(), method)
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder::new(WaitMode::Unbounded, canister_id.into(), method)
    }
}

///
/// CallBuilder
///

pub struct CallBuilder<'a> {
    wait: WaitMode,
    canister_id: Principal,
    method: String,
    cycles: u128,
    args: Cow<'a, [u8]>, // always present; defaults to ()
}

impl CallBuilder<'_> {
    fn new(wait: WaitMode, canister_id: Principal, method: &str) -> Self {
        Self {
            wait,
            canister_id,
            method: method.to_string(),
            cycles: 0,
            args: Cow::Borrowed(EMPTY_ARGS),
        }
    }

    /// Use pre-encoded Candid arguments (no validation performed).
    #[must_use]
    pub fn with_raw_args<'b>(self, args: impl Into<Cow<'b, [u8]>>) -> CallBuilder<'b> {
        let Self {
            wait,
            canister_id,
            method,
            cycles,
            ..
        } = self;

        CallBuilder {
            wait,
            canister_id,
            method,
            cycles,
            args: args.into(),
        }
    }

    /// Encode a single argument into Candid bytes (fallible).
    pub fn with_arg<A>(self, arg: A) -> Result<Self, InfraError>
    where
        A: CandidType,
    {
        let mut builder = self;
        builder.args = encode_one(arg).map_err(IcInfraError::from)?.into();
        Ok(builder)
    }

    /// Encode multiple arguments into Candid bytes (fallible).
    pub fn with_args<A>(self, args: A) -> Result<Self, InfraError>
    where
        A: ArgumentEncoder,
    {
        let mut builder = self;
        builder.args = encode_args(args).map_err(IcInfraError::from)?.into();
        Ok(builder)
    }

    #[must_use]
    pub const fn with_cycles(mut self, cycles: u128) -> Self {
        self.cycles = cycles;
        self
    }

    pub async fn execute(self) -> Result<CallResult, InfraError> {
        let mut call = match self.wait {
            WaitMode::Bounded => {
                crate::cdk::call::Call::bounded_wait(self.canister_id, &self.method)
            }
            WaitMode::Unbounded => {
                crate::cdk::call::Call::unbounded_wait(self.canister_id, &self.method)
            }
        };

        call = call.with_cycles(self.cycles);
        call = call.with_raw_args(self.args.as_ref());

        let response = call.await.map_err(IcInfraError::from)?;

        Ok(CallResult { inner: response })
    }
}

///
/// CallResult
///

pub struct CallResult {
    inner: Response,
}

impl CallResult {
    pub fn raw_equals(&self, expected: &[u8]) -> bool {
        self.inner == expected
    }

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

#[derive(Clone, Copy, Debug)]
enum WaitMode {
    Bounded,
    Unbounded,
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
        assert_eq!(builder.args.as_ref(), raw.as_slice());
    }
}
