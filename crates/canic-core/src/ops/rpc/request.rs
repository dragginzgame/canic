use crate::{
    InternalError, InternalErrorOrigin,
    infra::InfraError,
    ops::{
        prelude::*,
        rpc::{Rpc, RpcOps, RpcOpsError},
    },
};
use candid::encode_one;
use thiserror::Error as ThisError;

///
/// RequestOpsError
/// Errors produced during request dispatch or response handling
///

#[derive(Debug, ThisError)]
pub enum RequestOpsError {
    #[error(transparent)]
    Infra(#[from] InfraError),

    #[error("invalid response type")]
    InvalidResponseType,
}

///
/// Request
/// Root-directed orchestration commands (ops-local).
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum Request {
    CreateCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    Cycles(CyclesRequest),
}

///
/// CreateCanisterRequest
/// Payload for [`Request::CreateCanister`]
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CreateCanisterRequest {
    pub canister_role: CanisterRole,
    pub parent: CreateCanisterParent,
    pub extra_arg: Option<Vec<u8>>,
}

///
/// CreateCanisterParent
/// Parent-location choices for a new canister
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
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

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct UpgradeCanisterRequest {
    pub canister_pid: Principal,
}

///
/// CyclesRequest
/// Payload for [`Request::Cycles`]
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CyclesRequest {
    pub cycles: u128,
}

///
/// Response
/// Response payloads produced by root for orchestration requests (ops-local).
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum Response {
    CreateCanister(CreateCanisterResponse),
    UpgradeCanister(UpgradeCanisterResponse),
    Cycles(CyclesResponse),
}

///
/// CreateCanisterResponse
/// Result of creating and installing a new canister.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CreateCanisterResponse {
    pub new_canister_pid: Principal,
}

///
/// UpgradeCanisterResponse
/// Result of an upgrade request (currently empty, reserved for metadata)
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct UpgradeCanisterResponse {}

///
/// CyclesResponse
/// Result of transferring cycles to a child canister
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CyclesResponse {
    pub cycles_transferred: u128,
}

impl From<RequestOpsError> for InternalError {
    fn from(err: RequestOpsError) -> Self {
        RpcOpsError::from(err).into()
    }
}

///
/// RequestOps
/// Ops-level helpers for request/response RPCs.
///

pub struct RequestOps;

impl RequestOps {
    pub async fn create_canister<A>(
        canister_role: &CanisterRole,
        parent: CreateCanisterParent,
        extra: Option<A>,
    ) -> Result<CreateCanisterResponse, InternalError>
    where
        A: CandidType + Send + Sync,
    {
        let extra_arg = extra.map(encode_one).transpose().map_err(|err| {
            InternalError::invariant(
                InternalErrorOrigin::Ops,
                format!("failed to encode create_canister extra arg: {err}"),
            )
        })?;

        RpcOps::execute_root_response_rpc(CreateCanisterRpc {
            canister_role: canister_role.clone(),
            parent,
            extra_arg,
        })
        .await
    }

    pub async fn upgrade_canister(
        canister_pid: Principal,
    ) -> Result<UpgradeCanisterResponse, InternalError> {
        RpcOps::execute_root_response_rpc(UpgradeCanisterRpc { canister_pid }).await
    }

    pub async fn request_cycles(cycles: u128) -> Result<CyclesResponse, InternalError> {
        RpcOps::execute_root_response_rpc(CyclesRpc { cycles }).await
    }
}

///
/// CreateCanisterRpc
///

struct CreateCanisterRpc {
    canister_role: CanisterRole,
    parent: CreateCanisterParent,
    extra_arg: Option<Vec<u8>>,
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

    fn try_from_response(resp: Response) -> Result<Self::Response, InternalError> {
        match resp {
            Response::CreateCanister(r) => Ok(r),
            _ => Err(RequestOpsError::InvalidResponseType.into()),
        }
    }
}

///
/// UpgradeCanisterRpc
///

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

    fn try_from_response(resp: Response) -> Result<Self::Response, InternalError> {
        match resp {
            Response::UpgradeCanister(r) => Ok(r),
            _ => Err(RequestOpsError::InvalidResponseType.into()),
        }
    }
}

///
/// CyclesRpc
///

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

    fn try_from_response(resp: Response) -> Result<Self::Response, InternalError> {
        match resp {
            Response::Cycles(r) => Ok(r),
            _ => Err(RequestOpsError::InvalidResponseType.into()),
        }
    }
}
