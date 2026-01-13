use crate::{
    InternalError,
    ops::ic::call::{CallBuilder as OpsCallBuilder, CallOps, CallResult as OpsCallResult},
    workflow::prelude::*,
};
use candid::utils::{ArgumentDecoder, ArgumentEncoder};
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
    // ---------- arguments ----------

    #[must_use]
    pub fn with_arg<A>(self, arg: A) -> Self
    where
        A: CandidType,
    {
        Self {
            inner: self.inner.with_arg(arg),
        }
    }

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
        Ok(Self {
            inner: self.inner.try_with_arg(arg)?,
        })
    }

    pub fn try_with_args<A>(self, args: A) -> Result<Self, InternalError>
    where
        A: ArgumentEncoder,
    {
        Ok(Self {
            inner: self.inner.try_with_args(args)?,
        })
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
