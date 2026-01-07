pub mod request;

use crate::{Error, ThisError, cdk::types::Principal, ids::CanisterRole, workflow::WorkflowError};

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
}

impl From<RpcWorkflowError> for Error {
    fn from(err: RpcWorkflowError) -> Self {
        WorkflowError::Rpc(err).into()
    }
}
