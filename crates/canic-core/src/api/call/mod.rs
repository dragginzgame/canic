//! Module: api::call
//!
//! Responsibility: expose Canic's instrumented inter-canister call builder.
//! Does not own: call policy, transport mechanics, or protected Canic RPC.
//! Boundary: maps the IC call workflow and its typed failures into the public API.

use crate::{
    dto::error::Error,
    workflow::ic::call::{
        CallBuilder as WorkflowCallBuilder, CallResult as WorkflowCallResult, CallWorkflow,
    },
};
use candid::{
    CandidType, Principal,
    utils::{ArgumentDecoder, ArgumentEncoder},
};
use serde::de::DeserializeOwned;
use std::borrow::Cow;

/// Entry point for constructing an instrumented inter-canister call.
pub struct Call;

impl Call {
    /// Construct a call that uses the IC's bounded-wait behavior.
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder {
            inner: CallWorkflow::bounded_wait(canister_id, method),
        }
    }

    /// Construct a call that uses the IC's unbounded-wait behavior.
    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder {
            inner: CallWorkflow::unbounded_wait(canister_id, method),
        }
    }
}

/// Public builder for one instrumented inter-canister call.
pub struct CallBuilder<'a> {
    inner: WorkflowCallBuilder<'a>,
}

impl CallBuilder<'_> {
    /// Encode one Candid argument.
    pub fn with_arg<A>(self, arg: A) -> Result<Self, Error>
    where
        A: CandidType,
    {
        Ok(Self {
            inner: self.inner.with_arg(arg).map_err(Error::from)?,
        })
    }

    /// Encode a Candid argument tuple.
    pub fn with_args<A>(self, args: A) -> Result<Self, Error>
    where
        A: ArgumentEncoder,
    {
        Ok(Self {
            inner: self.inner.with_args(args).map_err(Error::from)?,
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
    pub async fn execute(self) -> Result<CallResult, Error> {
        Ok(CallResult {
            inner: self.inner.execute().await.map_err(Error::from)?,
        })
    }

    /// Execute the configured call and decode one Candid response value.
    pub async fn execute_candid<R>(self) -> Result<R, Error>
    where
        R: CandidType + DeserializeOwned,
    {
        self.execute().await?.candid()
    }

    /// Execute the configured call and decode a Candid response tuple.
    pub async fn execute_candid_tuple<R>(self) -> Result<R, Error>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        self.execute().await?.candid_tuple()
    }
}

/// Public response wrapper with typed Candid decoding.
pub struct CallResult {
    inner: WorkflowCallResult,
}

impl CallResult {
    /// Decode the response as one Candid value.
    pub fn candid<R>(&self) -> Result<R, Error>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner.candid().map_err(Error::from)
    }

    /// Decode the response as a Candid tuple.
    pub fn candid_tuple<R>(&self) -> Result<R, Error>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        self.inner.candid_tuple().map_err(Error::from)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::runtime::metrics::inter_canister_call::{
        InterCanisterCallMetricKey, InterCanisterCallMetrics,
    };
    use std::collections::HashMap;

    #[test]
    fn public_builder_routes_through_instrumented_call_authority() {
        InterCanisterCallMetrics::reset();
        let target = Principal::from_slice(&[7; 29]);

        let _builder = Call::bounded_wait(target, "example")
            .with_arg(42_u64)
            .expect("encode one argument")
            .with_cycles(1_000);

        let counts: HashMap<_, _> = InterCanisterCallMetrics::snapshot()
            .entries
            .into_iter()
            .collect();
        assert_eq!(
            counts.get(&InterCanisterCallMetricKey {
                target,
                method: "example".to_string(),
            }),
            Some(&1)
        );
    }
}
