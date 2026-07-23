//! Module: workflow::ic::call
//!
//! Responsibility: expose typed inter-canister call construction and response decoding.
//! Does not own: low-level execution, call policy, or endpoint authorization.
//! Boundary: delegates one call to the instrumented IC call operations authority.

use crate::{
    InternalError,
    ops::ic::call::{CallBuilder as OpsCallBuilder, CallOps, CallResult as OpsCallResult},
};
use candid::{
    CandidType, Principal,
    utils::{ArgumentDecoder, ArgumentEncoder},
};
use serde::de::DeserializeOwned;
use std::borrow::Cow;

/// Workflow entry point for constructing an inter-canister call.
pub struct CallWorkflow;

impl CallWorkflow {
    /// Construct a call using bounded-wait behavior.
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder {
            inner: CallOps::bounded_wait(canister_id, method),
        }
    }

    /// Construct a call using unbounded-wait behavior.
    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder {
            inner: CallOps::unbounded_wait(canister_id, method),
        }
    }
}

/// Workflow builder carrying one operations-layer call.
pub struct CallBuilder<'a> {
    inner: OpsCallBuilder<'a>,
}

impl CallBuilder<'_> {
    /// Encode one Candid argument.
    pub fn with_arg<A>(self, arg: A) -> Result<Self, InternalError>
    where
        A: CandidType,
    {
        Ok(Self {
            inner: self.inner.with_arg(arg)?,
        })
    }

    /// Encode a Candid argument tuple.
    pub fn with_args<A>(self, args: A) -> Result<Self, InternalError>
    where
        A: ArgumentEncoder,
    {
        Ok(Self {
            inner: self.inner.with_args(args)?,
        })
    }

    /// Supply pre-encoded Candid arguments without validating them.
    #[must_use]
    pub fn with_raw_args<'b>(self, args: impl Into<Cow<'b, [u8]>>) -> CallBuilder<'b> {
        CallBuilder {
            inner: self.inner.with_raw_args(args),
        }
    }

    /// Attach cycles to the call.
    #[must_use]
    pub fn with_cycles(self, cycles: u128) -> Self {
        Self {
            inner: self.inner.with_cycles(cycles),
        }
    }

    /// Execute the configured call.
    pub async fn execute(self) -> Result<CallResult, InternalError> {
        Ok(CallResult {
            inner: self.inner.execute().await?,
        })
    }
}

/// Workflow response wrapper around the operations-layer result.
pub struct CallResult {
    inner: OpsCallResult,
}

impl CallResult {
    /// Decode the response as one Candid value.
    pub fn candid<R>(&self) -> Result<R, InternalError>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner.candid()
    }

    /// Decode the response as a Candid tuple.
    pub fn candid_tuple<R>(&self) -> Result<R, InternalError>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        self.inner.candid_tuple()
    }
}
