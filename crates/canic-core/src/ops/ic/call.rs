//! Module: ops::ic::call
//!
//! Responsibility: wrap inter-canister call construction, execution, and decoding.
//! Does not own: call policy, workflow routing, or raw transport mechanics.
//! Boundary: records metrics and delegates call mechanics to infra.

use crate::{
    InternalError,
    domain::metrics::{
        PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
        PlatformCallMetricSurface,
    },
    infra::ic::call::{
        Call as InfraCall, CallBuilder as InfraCallBuilder, CallResult as InfraCallResult,
    },
    ops::{
        OpsError,
        prelude::*,
        runtime::metrics::{
            inter_canister_call::InterCanisterCallMetrics, platform_call::PlatformCallMetrics,
        },
    },
};
use ic_platform::candid::contract::{ArgumentDecoder, ArgumentEncoder, CandidType};
use serde::de::DeserializeOwned;
use std::borrow::Cow;

///
/// CallOps
///
/// Operations-layer facade for approved platform call construction.
///

pub struct CallOps;

impl CallOps {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        let canister_id: Principal = canister_id.into();
        InterCanisterCallMetrics::record_call(canister_id, method);

        CallBuilder {
            inner: InfraCall::bounded_wait(canister_id, method),
            mode: PlatformCallMetricMode::BoundedWait,
        }
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        let canister_id: Principal = canister_id.into();
        InterCanisterCallMetrics::record_call(canister_id, method);

        CallBuilder {
            inner: InfraCall::unbounded_wait(canister_id, method),
            mode: PlatformCallMetricMode::UnboundedWait,
        }
    }
}
///
/// CallBuilder (ops)
///
/// Operations-layer inter-canister call builder with metric context.
///

pub struct CallBuilder<'a> {
    inner: InfraCallBuilder<'a>,
    mode: PlatformCallMetricMode,
}

impl<'a> CallBuilder<'a> {
    // single-arg convenience
    /// Encode a single argument into Candid bytes (fallible).
    pub fn with_arg<A>(self, arg: A) -> Result<Self, InternalError>
    where
        A: CandidType,
    {
        Self::encoded(self.mode, self.inner.with_arg(arg).map_err(OpsError::from))
    }

    // multi-arg convenience (IMPORTANT FIX)
    /// Encode multiple arguments into Candid bytes (fallible).
    pub fn with_args<A>(self, args: A) -> Result<Self, InternalError>
    where
        A: ArgumentEncoder,
    {
        Self::encoded(
            self.mode,
            self.inner.with_args(args).map_err(OpsError::from),
        )
    }

    fn encoded(
        mode: PlatformCallMetricMode,
        result: Result<InfraCallBuilder<'a>, OpsError>,
    ) -> Result<Self, InternalError> {
        result.map(|inner| Self { inner, mode }).map_err(|err| {
            record_generic_call(
                mode,
                PlatformCallMetricOutcome::Failed,
                PlatformCallMetricReason::CandidEncode,
            );
            err.into()
        })
    }

    /// Use pre-encoded Candid arguments (no validation performed).
    #[must_use]
    pub fn with_raw_args<'b>(self, args: impl Into<Cow<'b, [u8]>>) -> CallBuilder<'b> {
        CallBuilder {
            inner: self.inner.with_raw_args(args),
            mode: self.mode,
        }
    }

    #[must_use]
    pub fn with_cycles(mut self, cycles: u128) -> Self {
        self.inner = self.inner.with_cycles(cycles);
        self
    }

    pub async fn execute(self) -> Result<CallResult, InternalError> {
        record_generic_call(
            self.mode,
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
        let inner = match self.inner.execute().await.map_err(OpsError::from) {
            Ok(inner) => inner,
            Err(err) => {
                record_generic_call(
                    self.mode,
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::Infra,
                );
                return Err(err.into());
            }
        };
        record_generic_call(
            self.mode,
            PlatformCallMetricOutcome::Completed,
            PlatformCallMetricReason::Ok,
        );
        Ok(CallResult {
            inner,
            mode: self.mode,
        })
    }
}

///
/// CallResult
///
/// Operations-layer inter-canister call result with metric context.
///

pub struct CallResult {
    inner: InfraCallResult,
    mode: PlatformCallMetricMode,
}

impl CallResult {
    /// Return the raw reply for an explicitly selected protocol decoder.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.inner.into_bytes()
    }

    fn decoded<R>(&self, result: Result<R, OpsError>) -> Result<R, InternalError> {
        let outcome = if result.is_ok() {
            PlatformCallMetricOutcome::Completed
        } else {
            PlatformCallMetricOutcome::Failed
        };
        record_generic_call(self.mode, outcome, PlatformCallMetricReason::CandidDecode);
        result.map_err(InternalError::from)
    }

    pub fn candid<R>(&self) -> Result<R, InternalError>
    where
        R: CandidType + DeserializeOwned,
    {
        self.decoded(self.inner.candid().map_err(OpsError::from))
    }

    pub fn candid_tuple<R>(&self) -> Result<R, InternalError>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        self.decoded(self.inner.candid_tuple().map_err(OpsError::from))
    }
}

// Record one generic platform call metric with no target or method labels.
fn record_generic_call(
    mode: PlatformCallMetricMode,
    outcome: PlatformCallMetricOutcome,
    reason: PlatformCallMetricReason,
) {
    PlatformCallMetrics::record(PlatformCallMetricSurface::Generic, mode, outcome, reason);
}
