use crate::{
    InternalError,
    infra::{
        InfraError,
        ic::call::{
            Call as InfraCall, CallBuilder as InfraCallBuilder, CallResult as InfraCallResult,
        },
    },
    ops::{ic::IcOpsError, prelude::*, runtime::metrics::icc::IccMetrics},
};
use candid::{
    CandidType,
    utils::{ArgumentDecoder, ArgumentEncoder},
};
use serde::de::DeserializeOwned;
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
/// Ops-level IC call façade.
///
/// This type:
/// - records call metrics
/// - delegates all mechanics to infra
/// - imposes no policy
/// - exposes the approved IC call surface
///

pub struct CallOps;

impl CallOps {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        let canister_id: Principal = canister_id.into();
        IccMetrics::record_call(canister_id, method);

        CallBuilder {
            inner: InfraCall::bounded_wait(canister_id, method),
        }
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        let canister_id: Principal = canister_id.into();
        IccMetrics::record_call(canister_id, method);

        CallBuilder {
            inner: InfraCall::unbounded_wait(canister_id, method),
        }
    }
}
///
/// CallBuilder (ops)
///

pub struct CallBuilder {
    inner: InfraCallBuilder,
}

impl CallBuilder {
    // single-arg convenience
    #[must_use]
    pub fn with_arg<A>(self, arg: A) -> Self
    where
        A: CandidType,
    {
        Self {
            inner: self.inner.with_arg(arg),
        }
    }

    // multi-arg convenience (IMPORTANT FIX)
    #[must_use]
    pub fn with_args<A>(self, args: A) -> Self
    where
        A: ArgumentEncoder,
    {
        Self {
            inner: self.inner.with_args(args),
        }
    }

    pub fn try_with_arg<A>(self, arg: A) -> Result<Self, InternalError>
    where
        A: CandidType,
    {
        let inner = self.inner.try_with_arg(arg).map_err(CallError::from)?;
        Ok(Self { inner })
    }

    pub fn try_with_args<A>(self, args: A) -> Result<Self, InternalError>
    where
        A: ArgumentEncoder,
    {
        let inner = self.inner.try_with_args(args).map_err(CallError::from)?;
        Ok(Self { inner })
    }

    #[must_use]
    pub fn with_cycles(mut self, cycles: u128) -> Self {
        self.inner = self.inner.with_cycles(cycles);
        self
    }

    pub async fn execute(self) -> Result<CallResult, InternalError> {
        let inner = self.inner.execute().await.map_err(CallError::from)?;
        Ok(CallResult { inner })
    }
}

///
/// CallResult
///

pub struct CallResult {
    inner: InfraCallResult,
}

impl CallResult {
    pub fn candid<R>(&self) -> Result<R, InternalError>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner
            .candid()
            .map_err(CallError::from)
            .map_err(InternalError::from)
    }

    pub fn candid_tuple<R>(&self) -> Result<R, InternalError>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        self.inner
            .candid_tuple()
            .map_err(CallError::from)
            .map_err(InternalError::from)
    }
}
