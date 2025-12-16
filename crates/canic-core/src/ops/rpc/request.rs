use crate::{
    Error, ThisError,
    ids::CanisterRole,
    ops::{
        prelude::*,
        rpc::{
            CreateCanisterResponse, CyclesResponse, Response, Rpc, RpcOpsError,
            UpgradeCanisterResponse, execute_rpc,
        },
    },
};
use candid::encode_one;

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

    #[error("create_canister: missing new pid")]
    MissingNewCanisterPid,
}

impl From<RequestOpsError> for Error {
    fn from(err: RequestOpsError) -> Self {
        RpcOpsError::from(err).into()
    }
}

///
/// Request
/// Root-directed orchestration commands.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Request {
    CreateCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    Cycles(CyclesRequest),
}

///
/// CreateCanisterRequest
/// Payload for [`Request::CreateCanister`]
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CreateCanisterRequest {
    pub canister_role: CanisterRole,
    pub parent: CreateCanisterParent,
    pub extra_arg: Option<Vec<u8>>,
}

///
/// CreateCanisterParent
/// Parent-location choices for a new canister
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum CreateCanisterParent {
    Root,
    /// Use the requesting canister as parent.
    ThisCanister,
    /// Use the requesting canister's parent (creates a sibling).
    Parent,
    Canister(Principal),
    Directory(CanisterRole),
}

///
/// UpgradeCanisterRequest
/// Payload for [`Request::UpgradeCanister`]
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct UpgradeCanisterRequest {
    pub canister_pid: Principal,
}

///
/// CyclesRequest
/// Payload for [`Request::Cycles`]
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CyclesRequest {
    pub cycles: u128,
}

///
/// CreateCanister
///

pub async fn create_canister_request<A>(
    canister_role: &CanisterRole,
    parent: CreateCanisterParent,
    extra: Option<A>,
) -> Result<CreateCanisterResponse, Error>
where
    A: CandidType + Send + Sync,
{
    let extra_arg = extra.map(encode_one).transpose()?;

    execute_rpc(CreateCanisterRpc {
        canister_role: canister_role.clone(),
        parent,
        extra_arg,
    })
    .await
}

pub struct CreateCanisterRpc {
    pub canister_role: CanisterRole,
    pub parent: CreateCanisterParent,
    pub extra_arg: Option<Vec<u8>>,
}

impl Rpc for CreateCanisterRpc {
    type Response = CreateCanisterResponse;

    fn into_request(self) -> Request {
        Request::CreateCanister(CreateCanisterRequest {
            canister_role: self.canister_role,
            parent: self.parent,
            extra_arg: self.extra_arg,
        })
    }

    fn try_from_response(resp: Response) -> Result<Self::Response, RequestOpsError> {
        match resp {
            Response::CreateCanister(r) => Ok(r),
            _ => Err(RequestOpsError::InvalidResponseType),
        }
    }
}

///
/// UpgradeCanister
/// Ask root to upgrade a child canister to its latest registered WASM.
///

pub async fn upgrade_canister_request(
    canister_pid: Principal,
) -> Result<UpgradeCanisterResponse, Error> {
    execute_rpc(UpgradeCanisterRpc { canister_pid }).await
}

pub struct UpgradeCanisterRpc {
    pub canister_pid: Principal,
}

impl Rpc for UpgradeCanisterRpc {
    type Response = UpgradeCanisterResponse;

    fn into_request(self) -> Request {
        Request::UpgradeCanister(UpgradeCanisterRequest {
            canister_pid: self.canister_pid,
        })
    }

    fn try_from_response(resp: Response) -> Result<Self::Response, RequestOpsError> {
        match resp {
            Response::UpgradeCanister(r) => Ok(r),
            _ => Err(RequestOpsError::InvalidResponseType),
        }
    }
}

///
/// Cycles
/// Request a cycle transfer from root to the current canister.
///

pub async fn cycles_request(cycles: u128) -> Result<CyclesResponse, Error> {
    OpsError::deny_root()?;

    execute_rpc(CyclesRpc { cycles }).await
}

pub struct CyclesRpc {
    pub cycles: u128,
}

impl Rpc for CyclesRpc {
    type Response = CyclesResponse;

    fn into_request(self) -> Request {
        Request::Cycles(CyclesRequest {
            cycles: self.cycles,
        })
    }

    fn try_from_response(resp: Response) -> Result<Self::Response, RequestOpsError> {
        match resp {
            Response::Cycles(r) => Ok(r),
            _ => Err(RequestOpsError::InvalidResponseType),
        }
    }
}
