pub mod call;
pub mod http;
pub mod ledger;
pub mod mgmt;
pub mod network;
pub mod signature;

// re-exports
pub use crate::ops::ic::{now_micros, now_millis, now_nanos, now_secs};

use crate::{api::prelude::*, cdk::candid::CandidType, ops::ic::call::CallOps};
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
/// CallBuilder (api)
///

pub struct CallBuilder {
    inner: crate::ops::ic::call::CallBuilder,
}

impl CallBuilder {
    pub fn try_with_arg<A: CandidType>(self, arg: A) -> Result<Self, PublicError> {
        let inner = self.inner.try_with_arg(arg)?;

        Ok(Self { inner })
    }

    #[must_use]
    pub fn with_cycles(self, cycles: u128) -> Self {
        Self {
            inner: self.inner.with_cycles(cycles),
        }
    }

    pub async fn execute(self) -> Result<CallResult, PublicError> {
        let inner = self.inner.execute().await?;

        Ok(CallResult { inner })
    }
}

///
/// CallResult (api)
///
/// Public, stable result wrapper.
///

pub struct CallResult {
    inner: crate::ops::ic::call::CallResult,
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
