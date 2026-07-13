//! Public IC call façade.
//!
//! This module defines the stable, public API used by application code to make
//! inter-canister calls. It deliberately exposes a *thin* surface:
//!
//! - argument encoding
//! - cycle attachment
//!
//! It does NOT:
//! - perform orchestration itself
//! - leak workflow or storage details
//!
//! This separation keeps application code simple while ensuring correctness
//! under concurrency.
use crate::{
    cdk::{candid::CandidType, types::Principal},
    dto::error::Error,
    workflow::ic::call::{
        CallBuilder as WorkflowCallBuilder, CallResult as WorkflowCallResult, CallWorkflow,
    },
};
use candid::utils::{ArgumentDecoder, ArgumentEncoder};
use serde::de::DeserializeOwned;
use std::borrow::Cow;

///
/// Call
///
/// Entry point for constructing inter-canister calls.
///
/// `Call` itself has no state; it simply selects the wait semantics
/// (bounded vs unbounded) and produces a `CallBuilder`.
///
/// Think of this as the *verb* (“make a call”), not the call itself.
///

pub struct Call;

impl Call {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder {
            inner: CallWorkflow::bounded_wait(canister_id, method),
        }
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder {
            inner: CallWorkflow::unbounded_wait(canister_id, method),
        }
    }
}

///
/// CallBuilder (api)
///

pub struct CallBuilder<'a> {
    inner: WorkflowCallBuilder<'a>,
}

impl CallBuilder<'_> {
    // ---------- arguments ----------

    /// Encode a single argument into Candid bytes (fallible).
    pub fn with_arg<A>(self, arg: A) -> Result<Self, Error>
    where
        A: CandidType,
    {
        Ok(Self {
            inner: self.inner.with_arg(arg).map_err(Error::from)?,
        })
    }

    /// Encode multiple arguments into Candid bytes (fallible).
    pub fn with_args<A>(self, args: A) -> Result<Self, Error>
    where
        A: ArgumentEncoder,
    {
        Ok(Self {
            inner: self.inner.with_args(args).map_err(Error::from)?,
        })
    }

    /// Use pre-encoded Candid arguments (no validation performed).
    #[must_use]
    pub fn with_raw_args<'b>(self, args: impl Into<Cow<'b, [u8]>>) -> CallBuilder<'b> {
        CallBuilder {
            inner: self.inner.with_raw_args(args),
        }
    }

    // ---------- cycles ----------

    #[must_use]
    pub fn with_cycles(self, cycles: u128) -> Self {
        Self {
            inner: self.inner.with_cycles(cycles),
        }
    }

    // ---------- execution ----------

    pub async fn execute(self) -> Result<CallResult, Error> {
        Ok(CallResult {
            inner: self.inner.execute().await.map_err(Error::from)?,
        })
    }
}

///
/// CallResult
///
/// Stable wrapper around an inter-canister call response.
///
/// This type exists to:
/// - decouple API consumers from infra response types
/// - provide uniform decoding helpers
/// - allow future extension without breaking callers
///

pub struct CallResult {
    inner: WorkflowCallResult,
}

impl CallResult {
    pub fn candid<R>(&self) -> Result<R, Error>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner.candid().map_err(Error::from)
    }

    pub fn candid_tuple<R>(&self) -> Result<R, Error>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        self.inner.candid_tuple().map_err(Error::from)
    }
}
