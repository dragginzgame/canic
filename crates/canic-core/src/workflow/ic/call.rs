use crate::{
    Error,
    ops::ic::call::{CallBuilder as OpsCallBuilder, CallOps},
    workflow::prelude::*,
};
use serde::de::DeserializeOwned;

///
/// CallWorkflow
///

pub struct CallWorkflow;

impl CallWorkflow {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        CallBuilder {
            inner: CallOps::bounded_wait(canister_id, method),
        }
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        CallBuilder {
            inner: CallOps::unbounded_wait(canister_id, method),
        }
    }
}

///
/// CallBuilder (workflow)
///

pub struct CallBuilder {
    inner: OpsCallBuilder,
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
        let inner = self.inner.try_with_arg(arg)?;

        Ok(Self { inner })
    }

    pub fn try_with_args<A>(self, args: A) -> Result<Self, Error>
    where
        A: CandidType,
    {
        let inner = self.inner.try_with_args(args)?;

        Ok(Self { inner })
    }

    #[must_use]
    pub fn with_cycles(self, cycles: u128) -> Self {
        Self {
            inner: self.inner.with_cycles(cycles),
        }
    }

    pub async fn execute(self) -> Result<CallResult, Error> {
        let inner = self.inner.execute().await?;

        Ok(CallResult { inner })
    }
}

///
/// CallResult (workflow)
///

pub struct CallResult {
    inner: crate::ops::ic::call::CallResult,
}

impl CallResult {
    pub fn candid<R>(&self) -> Result<R, Error>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner.candid()
    }
}
