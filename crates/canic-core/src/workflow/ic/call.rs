//! Module: workflow::ic::call
//!
//! Responsibility: expose typed IC call construction and response decoding.
//! Does not own: low-level call execution, reservation state, or endpoint auth.
//! Boundary: thin workflow wrapper over IC call ops.

use crate::{
    InternalError,
    cdk::{candid::CandidType, types::Principal},
    ops::ic::call::{CallBuilder as OpsCallBuilder, CallOps, CallResult as OpsCallResult},
};
use candid::utils::{ArgumentDecoder, ArgumentEncoder};
use serde::de::DeserializeOwned;
use std::borrow::Cow;

///
/// CallWorkflow
///
/// Workflow facade for constructing IC calls.
///

pub struct CallWorkflow;

impl CallWorkflow {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder {
            inner: CallOps::bounded_wait(canister_id, method),
        }
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder {
            inner: CallOps::unbounded_wait(canister_id, method),
        }
    }
}

///
/// CallBuilder (workflow)
///
/// Builder that carries call arguments and attached cycles.
///

pub struct CallBuilder<'a> {
    inner: OpsCallBuilder<'a>,
}

impl CallBuilder<'_> {
    // ---------- arguments ----------

    /// Encode a single argument into Candid bytes (fallible).
    pub fn with_arg<A>(self, arg: A) -> Result<Self, InternalError>
    where
        A: CandidType,
    {
        let Self { inner } = self;
        let inner = inner.with_arg(arg)?;

        Ok(Self { inner })
    }

    /// Encode multiple arguments into Candid bytes (fallible).
    pub fn with_args<A>(self, args: A) -> Result<Self, InternalError>
    where
        A: ArgumentEncoder,
    {
        let Self { inner } = self;
        let inner = inner.with_args(args)?;

        Ok(Self { inner })
    }

    /// Use pre-encoded Candid arguments (no validation performed).
    #[must_use]
    pub fn with_raw_args<'b>(self, args: impl Into<Cow<'b, [u8]>>) -> CallBuilder<'b> {
        let Self { inner } = self;
        let inner = inner.with_raw_args(args);

        CallBuilder { inner }
    }

    // ---------- cycles ----------

    #[must_use]
    pub fn with_cycles(self, cycles: u128) -> Self {
        Self {
            inner: self.inner.with_cycles(cycles),
        }
    }

    // ---------- execution ----------

    pub async fn execute(self) -> Result<CallResult, InternalError> {
        Ok(CallResult {
            inner: self.inner.execute().await?,
        })
    }
}

///
/// CallResult (workflow)
///
/// Workflow wrapper around decoded IC call results.
///

pub struct CallResult {
    inner: OpsCallResult,
}

impl CallResult {
    pub fn candid<R>(&self) -> Result<R, InternalError>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner.candid()
    }

    pub fn candid_tuple<R>(&self) -> Result<R, InternalError>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        self.inner.candid_tuple()
    }
}
