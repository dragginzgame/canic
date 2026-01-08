use crate::{
    api::prelude::*,
    cdk::candid::CandidType,
    workflow::ic::call::{CallBuilder as WorkflowCallBuilder, CallWorkflow},
};
use serde::de::DeserializeOwned;

///
/// Call
///
/// Public IC call façade.
///

pub struct Call;

impl Call {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        CallBuilder {
            inner: CallWorkflow::bounded_wait(canister_id, method),
        }
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        CallBuilder {
            inner: CallWorkflow::unbounded_wait(canister_id, method),
        }
    }
}

///
/// CallBuilder (api)
///

pub struct CallBuilder {
    inner: WorkflowCallBuilder,
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

    pub fn try_with_arg<A: CandidType>(self, arg: A) -> Result<Self, PublicError> {
        let inner = self.inner.try_with_arg(arg).map_err(PublicError::from)?;

        Ok(Self { inner })
    }

    pub fn try_with_args<A>(self, args: A) -> Result<Self, PublicError>
    where
        A: CandidType,
    {
        let inner = self.inner.try_with_args(args).map_err(PublicError::from)?;

        Ok(Self { inner })
    }

    #[must_use]
    pub fn with_cycles(self, cycles: u128) -> Self {
        Self {
            inner: self.inner.with_cycles(cycles),
        }
    }

    pub async fn execute(self) -> Result<CallResult, PublicError> {
        let inner = self.inner.execute().await.map_err(PublicError::from)?;

        Ok(CallResult { inner })
    }
}

///
/// CallResult (api)
///
/// Public, stable result wrapper.
///

pub struct CallResult {
    inner: crate::workflow::ic::call::CallResult,
}

impl CallResult {
    /// Decode the candid response.
    pub fn candid<R>(&self) -> Result<R, PublicError>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner.candid().map_err(PublicError::from)
    }
}
