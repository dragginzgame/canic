//! Helpers that build requests routed through the root canister.
//!
//! Non-root canisters submit orchestration requests to root using the
//! `canic_response` endpoint. This module owns the request envelope and
//! high-level helpers for creating new canisters, triggering upgrades, or
//! moving cycles between principals.

mod request;
mod response;

pub use request::*;
pub use response::*;

use crate::{Error, ThisError, cdk::types::Principal, ids::CanisterRole, ops::OpsError};

///
/// RequestOpsError
/// Errors produced during request dispatch or response handling
///

#[derive(Debug, ThisError)]
pub enum RequestOpsError {
    #[error("canister type {0} not found")]
    CanisterRoleNotFound(CanisterRole),

    #[error("child canister {0} not found")]
    ChildNotFound(Principal),

    #[error("canister {0} is not a child of caller {1}")]
    NotChildOfCaller(Principal, Principal),

    #[error("canister {0}'s parent was not found")]
    ParentNotFound(Principal),

    #[error("invalid response type")]
    InvalidResponseType,

    #[error("cannot find the root canister")]
    RootNotFound,
}

impl From<RequestOpsError> for Error {
    fn from(err: RequestOpsError) -> Self {
        OpsError::from(err).into()
    }
}
