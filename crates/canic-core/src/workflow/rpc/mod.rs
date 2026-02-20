pub mod adapter;
pub mod request;

use crate::{InternalError, InternalErrorOrigin, cdk::types::Principal, ids::CanisterRole};
use thiserror::Error as ThisError;

///
/// RpcWorkflowError
///

#[derive(Debug, ThisError)]
pub enum RpcWorkflowError {
    #[error("canister role {0} not found")]
    CanisterRoleNotFound(CanisterRole),

    #[error("child canister {0} not found")]
    ChildNotFound(Principal),

    #[error("create_canister: missing new pid")]
    MissingNewCanisterPid,

    #[error("canister {0} is not a child of caller {1}")]
    NotChildOfCaller(Principal, Principal),

    #[error("canister {0}'s parent was not found")]
    ParentNotFound(Principal),

    #[error("insufficient root cycles: requested={requested}, available={available}")]
    InsufficientRootCycles { requested: u128, available: u128 },
}

impl From<RpcWorkflowError> for InternalError {
    fn from(err: RpcWorkflowError) -> Self {
        Self::workflow(InternalErrorOrigin::Workflow, err.to_string())
    }
}
