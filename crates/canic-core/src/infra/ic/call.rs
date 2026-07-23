//! Module: infra::ic::call
//!
//! Responsibility: wrap low-level IC calls with Candid argument/result helpers.
//! Does not own: retry policy, workflow decisions, or endpoint error mapping.
//! Boundary: infra adapters use this before returning raw decoded results to ops.

use crate::{
    cdk::{
        candid::{
            CandidType,
            utils::{ArgumentDecoder, ArgumentEncoder},
        },
        types::Principal,
    },
    infra::ic::IcInfraError,
};
use candid::{encode_args, encode_one};
use ic_cdk::call::Response;
use serde::de::DeserializeOwned;
use std::borrow::Cow;

const EMPTY_ARGS: &[u8] = b"DIDL\0\0";

///
/// Call
///
/// Factory for bounded and unbounded IC call builders.
/// Owned by IC infra and used by low-level adapter modules.
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
/// Builder for one IC call request, including wait mode, cycles, and Candid bytes.
/// Owned by IC infra and consumed by `execute`.
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
    pub fn with_arg<A>(self, arg: A) -> Result<Self, IcInfraError>
    where
        A: CandidType,
    {
        let mut builder = self;
        builder.args = encode_one(arg).map_err(IcInfraError::from)?.into();
        Ok(builder)
    }

    /// Encode multiple arguments into Candid bytes (fallible).
    pub fn with_args<A>(self, args: A) -> Result<Self, IcInfraError>
    where
        A: ArgumentEncoder,
    {
        let mut builder = self;
        builder.args = encode_args(args).map_err(IcInfraError::from)?.into();
        Ok(builder)
    }

    /// Attach cycles to the call request.
    #[must_use]
    pub const fn with_cycles(mut self, cycles: u128) -> Self {
        self.cycles = cycles;
        self
    }

    /// Execute the configured IC call and return the raw response wrapper.
    pub async fn execute(self) -> Result<CallResult, IcInfraError> {
        let mut call = match self.wait {
            WaitMode::Bounded => ic_cdk::call::Call::bounded_wait(self.canister_id, &self.method),
            WaitMode::Unbounded => {
                ic_cdk::call::Call::unbounded_wait(self.canister_id, &self.method)
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
/// Raw IC call response wrapper with Candid decoding helpers.
/// Owned by IC infra and returned by `CallBuilder::execute`.
///

pub struct CallResult {
    inner: Response,
}

impl CallResult {
    /// Decode the response as a single Candid value.
    pub fn candid<R>(&self) -> Result<R, IcInfraError>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner.candid().map_err(IcInfraError::from)
    }

    /// Decode the response as a Candid tuple.
    pub fn candid_tuple<R>(&self) -> Result<R, IcInfraError>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        self.inner.candid_tuple().map_err(IcInfraError::from)
    }
}

#[derive(Clone, Copy, Debug)]
enum WaitMode {
    Bounded,
    Unbounded,
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

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
