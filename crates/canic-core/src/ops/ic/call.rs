use crate::{
    InternalError,
    infra::{
        InfraError,
        ic::call::{
            Call as InfraCall, CallBuilder as InfraCallBuilder, CallResult as InfraCallResult,
        },
    },
    ops::{
        ic::IcOpsError,
        prelude::*,
        runtime::metrics::{
            inter_canister_call::InterCanisterCallMetrics,
            platform_call::{
                PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
                PlatformCallMetricSurface, PlatformCallMetrics,
            },
        },
    },
};
use candid::{
    CandidType,
    utils::{ArgumentDecoder, ArgumentEncoder},
};
use serde::de::DeserializeOwned;
use std::borrow::Cow;
use thiserror::Error as ThisError;

///
/// CallError
///

#[derive(Debug, ThisError)]
#[error(transparent)]
pub struct CallError(#[from] InfraError);

impl From<CallError> for InternalError {
    fn from(err: CallError) -> Self {
        IcOpsError::from(err).into()
    }
}

///
/// CallOps
///
/// Ops-level platform call façade.
///
/// This type:
/// - records call metrics
/// - delegates all mechanics to infra
/// - imposes no policy
/// - exposes the approved platform call surface
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

pub struct CallBuilder<'a> {
    inner: InfraCallBuilder<'a>,
    mode: PlatformCallMetricMode,
}

impl CallBuilder<'_> {
    // single-arg convenience
    /// Encode a single argument into Candid bytes (fallible).
    pub fn with_arg<A>(self, arg: A) -> Result<Self, InternalError>
    where
        A: CandidType,
    {
        let mode = self.mode;
        let inner = match self.inner.with_arg(arg).map_err(CallError::from) {
            Ok(inner) => inner,
            Err(err) => {
                record_generic_call(
                    mode,
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::CandidEncode,
                );
                return Err(err.into());
            }
        };
        Ok(Self { inner, mode })
    }

    // multi-arg convenience (IMPORTANT FIX)
    /// Encode multiple arguments into Candid bytes (fallible).
    pub fn with_args<A>(self, args: A) -> Result<Self, InternalError>
    where
        A: ArgumentEncoder,
    {
        let mode = self.mode;
        let inner = match self.inner.with_args(args).map_err(CallError::from) {
            Ok(inner) => inner,
            Err(err) => {
                record_generic_call(
                    mode,
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::CandidEncode,
                );
                return Err(err.into());
            }
        };
        Ok(Self { inner, mode })
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
        let inner = match self.inner.execute().await.map_err(CallError::from) {
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

pub struct CallResult {
    inner: InfraCallResult,
    mode: PlatformCallMetricMode,
}

impl CallResult {
    pub fn raw_equals(&self, expected: &[u8]) -> bool {
        self.inner.raw_equals(expected)
    }

    pub fn candid<R>(&self) -> Result<R, InternalError>
    where
        R: CandidType + DeserializeOwned,
    {
        match self.inner.candid().map_err(CallError::from) {
            Ok(value) => {
                record_generic_call(
                    self.mode,
                    PlatformCallMetricOutcome::Completed,
                    PlatformCallMetricReason::CandidDecode,
                );
                Ok(value)
            }
            Err(err) => {
                record_generic_call(
                    self.mode,
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::CandidDecode,
                );
                Err(err.into())
            }
        }
    }

    pub fn candid_tuple<R>(&self) -> Result<R, InternalError>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        match self.inner.candid_tuple().map_err(CallError::from) {
            Ok(value) => {
                record_generic_call(
                    self.mode,
                    PlatformCallMetricOutcome::Completed,
                    PlatformCallMetricReason::CandidDecode,
                );
                Ok(value)
            }
            Err(err) => {
                record_generic_call(
                    self.mode,
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::CandidDecode,
                );
                Err(err.into())
            }
        }
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
