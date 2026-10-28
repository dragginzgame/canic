//! Module: infra::ic::call
//!
//! Responsibility: adapt prepared IC Call requests with external Candid helpers.
//! Does not own: retry policy, workflow decisions, or endpoint error mapping.
//! Boundary: infra adapters use this before returning raw decoded results to ops.

use crate::{cdk::types::Principal, infra::ic::IcInfraError};
use ic_call::{Call as PreparedCall, WaitPolicy};
use ic_platform::candid::contract::{
    ArgumentDecoder, ArgumentEncoder, CandidType, DecoderConfig, decode_args_with_config,
    encode_args, encode_one,
};
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
        CallBuilder::new(
            WaitPolicy::Bounded {
                timeout_seconds: 300,
            },
            canister_id.into(),
            method,
        )
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder::new(WaitPolicy::Unbounded, canister_id.into(), method)
    }
}

///
/// CallBuilder
///
/// Builder for one IC call request, including wait mode, cycles, and Candid bytes.
/// Owned by IC infra and consumed by `execute`.
///

pub struct CallBuilder<'a> {
    inner: PreparedCall<'static, 'a>,
}

impl CallBuilder<'_> {
    fn new(wait: WaitPolicy, canister_id: Principal, method: &str) -> Self {
        Self {
            inner: PreparedCall::new(canister_id.into(), method.to_owned(), wait)
                .with_raw_args(EMPTY_ARGS),
        }
    }

    /// Use caller-selected protocol bytes without validation or encoding.
    #[must_use]
    pub fn with_raw_args<'b>(self, args: impl Into<Cow<'b, [u8]>>) -> CallBuilder<'b> {
        let inner = match args.into() {
            Cow::Borrowed(bytes) => self.inner.with_raw_args(bytes),
            Cow::Owned(bytes) => self.inner.take_raw_args(bytes),
        };
        CallBuilder { inner }
    }

    /// Encode a single argument into Candid bytes (fallible).
    pub fn with_arg<A>(self, arg: A) -> Result<Self, IcInfraError>
    where
        A: CandidType,
    {
        let mut builder = self;
        builder.inner = builder
            .inner
            .take_raw_args(encode_one(arg).map_err(IcInfraError::from)?);
        Ok(builder)
    }

    /// Encode multiple arguments into Candid bytes (fallible).
    pub fn with_args<A>(self, args: A) -> Result<Self, IcInfraError>
    where
        A: ArgumentEncoder,
    {
        let mut builder = self;
        builder.inner = builder
            .inner
            .take_raw_args(encode_args(args).map_err(IcInfraError::from)?);
        Ok(builder)
    }

    /// Attach cycles to the call request.
    #[must_use]
    pub fn with_cycles(self, cycles: u128) -> Self {
        let Self { inner } = self;
        Self {
            inner: inner.with_cycles(cycles),
        }
    }

    /// Return the exact current-Subnet cost of scheduling this call.
    #[must_use]
    pub fn cost(&self) -> u128 {
        self.inner.get_cost()
    }

    /// Execute the configured IC call and return the raw response wrapper.
    pub async fn execute(self) -> Result<CallResult, IcInfraError> {
        let response = self.inner.execute().await.map_err(IcInfraError::from)?;

        Ok(CallResult {
            inner: response.into_bytes(),
        })
    }
}

///
/// CallResult
///
/// Raw IC call response wrapper with Candid decoding helpers.
/// Owned by IC infra and returned by `CallBuilder::execute`.
///

pub struct CallResult {
    inner: Vec<u8>,
}

impl CallResult {
    /// Preserve the upstream wire response for a caller-owned projection.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.inner
    }

    /// Decode the response as a single Candid value.
    pub fn candid<R>(&self) -> Result<R, IcInfraError>
    where
        R: CandidType + DeserializeOwned,
    {
        self.candid_tuple::<(R,)>().map(|(value,)| value)
    }

    /// Decode the response as a Candid tuple.
    pub fn candid_tuple<R>(&self) -> Result<R, IcInfraError>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        decode_args_with_config(self.inner.as_ref(), &DecoderConfig::new())
            .map_err(IcInfraError::CandidDecode)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{Call, CallResult, EMPTY_ARGS, IcInfraError, Principal, encode_args};
    use ic_call::WaitPolicy;

    #[test]
    fn prepared_call_owns_method_and_preserves_external_defaults() {
        let method = String::from("dynamic_method");
        let builder = Call::bounded_wait(Principal::anonymous(), &method);
        drop(method);
        assert_eq!(builder.inner.method(), "dynamic_method");
        assert_eq!(builder.inner.target(), Principal::anonymous().into());
        assert_eq!(builder.inner.raw_args(), EMPTY_ARGS);
        assert_eq!(builder.inner.attached_cycles(), 0);
        assert_eq!(
            builder.inner.wait_policy(),
            WaitPolicy::Bounded {
                timeout_seconds: 300
            }
        );
        // Construction and an unpolled execution future must remain effect-free.
        drop(builder.execute());
    }

    #[test]
    fn raw_argument_custody_preserves_wait_and_attachment() {
        let bytes = vec![7, 8, 9];
        let borrowed = Call::unbounded_wait(Principal::anonymous(), "wire")
            .with_cycles(42)
            .with_raw_args(bytes.as_slice());
        assert_eq!(borrowed.inner.raw_args().as_ptr(), bytes.as_ptr());
        let owned = borrowed.with_raw_args(bytes.clone());
        drop(bytes);
        assert_eq!(owned.inner.raw_args(), &[7, 8, 9]);
        assert_eq!(owned.inner.wait_policy(), WaitPolicy::Unbounded);
        assert_eq!(owned.inner.attached_cycles(), 42);
        let replaced = owned.with_cycles(19).with_args((42_u64, "value")).unwrap();
        assert_eq!(
            replaced.inner.raw_args(),
            encode_args((42_u64, "value")).unwrap()
        );
        assert_eq!(replaced.inner.attached_cycles(), 19);
        assert_eq!(replaced.inner.wait_policy(), WaitPolicy::Unbounded);
    }

    #[test]
    fn empty_args_match_candid_encoding() {
        let encoded = encode_args(()).expect("encode empty tuple");
        assert_eq!(EMPTY_ARGS, encoded.as_slice());
    }

    #[test]
    fn with_raw_args_overrides_default() {
        let raw = vec![1_u8, 2, 3, 4];
        let builder = Call::bounded_wait(Principal::anonymous(), "noop").with_raw_args(raw.clone());
        assert_eq!(builder.inner.raw_args(), raw.as_slice());
    }
    #[test]
    fn response_decoding_preserves_single_tuple_and_skipped_arguments() {
        let response = CallResult {
            inner: encode_args((42_u64, "extra")).unwrap(),
        };
        assert_eq!(response.candid::<u64>().unwrap(), 42);
        assert_eq!(
            response.candid_tuple::<(u64, String)>().unwrap(),
            (42, "extra".into())
        );
        let empty = CallResult {
            inner: EMPTY_ARGS.to_vec(),
        };
        empty.candid_tuple::<()>().unwrap();
        assert!(matches!(
            empty.candid::<u64>(),
            Err(IcInfraError::CandidDecode(_))
        ));
    }

    #[test]
    fn invalid_response_is_a_typed_codec_failure() {
        let response = CallResult {
            inner: b"invalid".to_vec(),
        };
        assert!(matches!(
            response.candid::<u64>(),
            Err(IcInfraError::CandidDecode(_))
        ));
    }
}
