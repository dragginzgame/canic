use crate::{
    Error, ThisError,
    infra::{
        InfraError,
        ic::call::{
            Call as InfraCall, CallBuilder as InfraCallBuilder, CallResult as InfraCallResult,
        },
    },
    ops::{ic::IcOpsError, prelude::*, runtime::metrics::icc::IccMetrics},
};
use serde::de::DeserializeOwned;

///
/// CallError
///

#[derive(Debug, ThisError)]
#[error(transparent)]
pub struct CallError(#[from] InfraError);

impl From<CallError> for Error {
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

    /// Create a call builder that will be awaited without cycle limits.
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
    #[must_use]
    pub fn with_args<A>(self, args: A) -> Self
    where
        A: CandidType,
    {
        Self {
            inner: self.inner.with_args(args),
        }
    }

    pub fn try_with_arg<A: CandidType>(self, arg: A) -> Result<Self, Error> {
        let inner = self.inner.try_with_arg(arg).map_err(CallError::from)?;

        Ok(Self { inner })
    }

    pub fn try_with_args<A>(self, args: A) -> Result<Self, Error>
    where
        A: CandidType,
    {
        let inner = self.inner.try_with_args(args).map_err(CallError::from)?;

        Ok(Self { inner })
    }

    #[must_use]
    pub fn with_cycles(mut self, cycles: u128) -> Self {
        self.inner = self.inner.with_cycles(cycles);
        self
    }

    pub async fn execute(self) -> Result<CallResult, Error> {
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
    pub fn candid<R>(&self) -> Result<R, Error>
    where
        R: CandidType + DeserializeOwned,
    {
        let res = self.inner.candid().map_err(CallError::from)?;

        Ok(res)
    }
}
