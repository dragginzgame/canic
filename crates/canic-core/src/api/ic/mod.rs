pub mod http;
pub mod mgmt;
pub mod network;
pub mod signature;

use crate::{
    Error, PublicError,
    cdk::{candid::CandidType, types::Principal},
    ops::ic::call::CallOps,
};
use serde::de::DeserializeOwned;

///
/// CallWait
///

pub enum CallWait {
    Bounded,
    Unbounded,
}

///
/// CallBuilder
///

pub struct CallBuilder {
    pid: Principal,
    method: String,
    wait: CallWait,
}

impl CallBuilder {
    #[must_use]
    pub const fn bounded(mut self) -> Self {
        self.wait = CallWait::Bounded;
        self
    }

    #[must_use]
    pub const fn unbounded(mut self) -> Self {
        self.wait = CallWait::Unbounded;
        self
    }

    pub async fn with_arg<R, A>(self, arg: A) -> Result<R, PublicError>
    where
        R: CandidType + DeserializeOwned,
        A: CandidType,
    {
        let call = match self.wait {
            CallWait::Bounded => CallOps::bounded_wait(self.pid, &self.method),
            CallWait::Unbounded => CallOps::unbounded_wait(self.pid, &self.method),
        };

        let response = call
            .try_with_arg(arg)
            .map_err(|e| PublicError::from(Error::from(e)))?
            .execute()
            .await
            .map_err(|e| PublicError::from(Error::from(e)))?;

        response
            .candid::<R>()
            .map_err(|e| PublicError::from(Error::from(e)))
    }
}
