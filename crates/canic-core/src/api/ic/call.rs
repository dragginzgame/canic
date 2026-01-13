use crate::{
    cdk::{candid::CandidType, types::Principal},
    dto::error::Error,
    workflow::ic::call::{
        CallBuilder as WorkflowCallBuilder, CallResult as WorkflowCallResult, CallWorkflow,
    },
};
use candid::utils::{ArgumentDecoder, ArgumentEncoder};
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

    pub fn try_with_arg<A>(self, arg: A) -> Result<Self, Error>
    where
        A: CandidType,
    {
        Ok(Self {
            inner: self.inner.try_with_arg(arg).map_err(Error::from)?,
        })
    }

    pub fn try_with_args<A>(self, args: A) -> Result<Self, Error>
    where
        A: ArgumentEncoder,
    {
        Ok(Self {
            inner: self.inner.try_with_args(args).map_err(Error::from)?,
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

    pub async fn execute(self) -> Result<CallResult, Error> {
        Ok(CallResult {
            inner: self.inner.execute().await.map_err(Error::from)?,
        })
    }
}

///
/// CallResult (api)
///
/// Public, stable result wrapper.
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
