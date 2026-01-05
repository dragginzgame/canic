pub mod http;
pub mod network;
pub mod signature;

use crate::{
    Error, PublicError,
    cdk::{candid::CandidType, types::Principal},
    dto::canister::CanisterStatusView,
    log::Topic,
    ops::ic::{call::CallOps, mgmt::MgmtOps},
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
            .map_err(|e| map_internal_error(Error::from(e)))?
            .execute()
            .await
            .map_err(|e| map_internal_error(Error::from(e)))?;

        response
            .candid::<R>()
            .map_err(|e| map_internal_error(Error::from(e)))
    }
}

pub async fn canister_status(pid: Principal) -> Result<CanisterStatusView, PublicError> {
    MgmtOps::canister_status(pid)
        .await
        .map_err(map_internal_error)
}

fn map_internal_error(err: Error) -> PublicError {
    // Log the internal error for operators, but return a stable PublicError contract.
    crate::log!(Topic::Rpc, Error, "api.ic failed: {err:?}");
    PublicError::from(err)
}
