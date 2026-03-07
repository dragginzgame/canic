use crate::{
    InternalError, InternalErrorOrigin,
    dto::auth::{
        DelegationProvisionResponse, DelegationRequest, RoleAttestationRequest,
        SignedRoleAttestation,
    },
    infra::InfraError,
    ops::{
        ic::IcOps,
        prelude::*,
        rpc::{Rpc, RpcOps, RpcOpsError},
    },
};
use candid::encode_one;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
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
    IssueDelegation(DelegationRequest),
    IssueRoleAttestation(RoleAttestationRequest),
}

///
/// RootRequestMetadata
/// Replay and idempotency metadata for mutating root requests.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootRequestMetadata {
    pub request_id: [u8; 32],
    pub ttl_seconds: u64,
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
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
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
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

///
/// CyclesRequest
/// Payload for [`Request::Cycles`]
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CyclesRequest {
    pub cycles: u128,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
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
    DelegationIssued(DelegationProvisionResponse),
    RoleAttestationIssued(SignedRoleAttestation),
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

impl From<RootRequestMetadata> for crate::dto::rpc::RootRequestMetadata {
    fn from(value: RootRequestMetadata) -> Self {
        Self {
            request_id: value.request_id,
            ttl_seconds: value.ttl_seconds,
        }
    }
}

impl From<crate::dto::rpc::RootRequestMetadata> for RootRequestMetadata {
    fn from(value: crate::dto::rpc::RootRequestMetadata) -> Self {
        Self {
            request_id: value.request_id,
            ttl_seconds: value.ttl_seconds,
        }
    }
}

impl From<CreateCanisterRequest> for crate::dto::rpc::CreateCanisterRequest {
    fn from(value: CreateCanisterRequest) -> Self {
        Self {
            canister_role: value.canister_role,
            parent: value.parent.into(),
            extra_arg: value.extra_arg,
            metadata: value.metadata.map(Into::into),
        }
    }
}

impl From<crate::dto::rpc::CreateCanisterRequest> for CreateCanisterRequest {
    fn from(value: crate::dto::rpc::CreateCanisterRequest) -> Self {
        Self {
            canister_role: value.canister_role,
            parent: value.parent.into(),
            extra_arg: value.extra_arg,
            metadata: value.metadata.map(Into::into),
        }
    }
}

impl From<CreateCanisterParent> for crate::dto::rpc::CreateCanisterParent {
    fn from(value: CreateCanisterParent) -> Self {
        match value {
            CreateCanisterParent::Root => Self::Root,
            CreateCanisterParent::ThisCanister => Self::ThisCanister,
            CreateCanisterParent::Parent => Self::Parent,
            CreateCanisterParent::Canister(pid) => Self::Canister(pid),
            CreateCanisterParent::Directory(role) => Self::Directory(role),
        }
    }
}

impl From<crate::dto::rpc::CreateCanisterParent> for CreateCanisterParent {
    fn from(value: crate::dto::rpc::CreateCanisterParent) -> Self {
        match value {
            crate::dto::rpc::CreateCanisterParent::Root => Self::Root,
            crate::dto::rpc::CreateCanisterParent::ThisCanister => Self::ThisCanister,
            crate::dto::rpc::CreateCanisterParent::Parent => Self::Parent,
            crate::dto::rpc::CreateCanisterParent::Canister(pid) => Self::Canister(pid),
            crate::dto::rpc::CreateCanisterParent::Directory(role) => Self::Directory(role),
        }
    }
}

impl From<UpgradeCanisterRequest> for crate::dto::rpc::UpgradeCanisterRequest {
    fn from(value: UpgradeCanisterRequest) -> Self {
        Self {
            canister_pid: value.canister_pid,
            metadata: value.metadata.map(Into::into),
        }
    }
}

impl From<crate::dto::rpc::UpgradeCanisterRequest> for UpgradeCanisterRequest {
    fn from(value: crate::dto::rpc::UpgradeCanisterRequest) -> Self {
        Self {
            canister_pid: value.canister_pid,
            metadata: value.metadata.map(Into::into),
        }
    }
}

impl From<CyclesRequest> for crate::dto::rpc::CyclesRequest {
    fn from(value: CyclesRequest) -> Self {
        Self {
            cycles: value.cycles,
            metadata: value.metadata.map(Into::into),
        }
    }
}

impl From<crate::dto::rpc::CyclesRequest> for CyclesRequest {
    fn from(value: crate::dto::rpc::CyclesRequest) -> Self {
        Self {
            cycles: value.cycles,
            metadata: value.metadata.map(Into::into),
        }
    }
}

impl From<Request> for crate::dto::rpc::Request {
    fn from(value: Request) -> Self {
        match value {
            Request::CreateCanister(req) => Self::CreateCanister(req.into()),
            Request::UpgradeCanister(req) => Self::UpgradeCanister(req.into()),
            Request::Cycles(req) => Self::Cycles(req.into()),
            Request::IssueDelegation(req) => Self::IssueDelegation(req),
            Request::IssueRoleAttestation(req) => Self::IssueRoleAttestation(req),
        }
    }
}

impl From<crate::dto::rpc::Request> for Request {
    fn from(value: crate::dto::rpc::Request) -> Self {
        match value {
            crate::dto::rpc::Request::CreateCanister(req) => Self::CreateCanister(req.into()),
            crate::dto::rpc::Request::UpgradeCanister(req) => Self::UpgradeCanister(req.into()),
            crate::dto::rpc::Request::Cycles(req) => Self::Cycles(req.into()),
            crate::dto::rpc::Request::IssueDelegation(req) => Self::IssueDelegation(req),
            crate::dto::rpc::Request::IssueRoleAttestation(req) => Self::IssueRoleAttestation(req),
        }
    }
}

impl From<CreateCanisterResponse> for crate::dto::rpc::CreateCanisterResponse {
    fn from(value: CreateCanisterResponse) -> Self {
        Self {
            new_canister_pid: value.new_canister_pid,
        }
    }
}

impl From<crate::dto::rpc::CreateCanisterResponse> for CreateCanisterResponse {
    fn from(value: crate::dto::rpc::CreateCanisterResponse) -> Self {
        Self {
            new_canister_pid: value.new_canister_pid,
        }
    }
}

impl From<UpgradeCanisterResponse> for crate::dto::rpc::UpgradeCanisterResponse {
    fn from(_value: UpgradeCanisterResponse) -> Self {
        Self {}
    }
}

impl From<crate::dto::rpc::UpgradeCanisterResponse> for UpgradeCanisterResponse {
    fn from(_value: crate::dto::rpc::UpgradeCanisterResponse) -> Self {
        Self {}
    }
}

impl From<CyclesResponse> for crate::dto::rpc::CyclesResponse {
    fn from(value: CyclesResponse) -> Self {
        Self {
            cycles_transferred: value.cycles_transferred,
        }
    }
}

impl From<crate::dto::rpc::CyclesResponse> for CyclesResponse {
    fn from(value: crate::dto::rpc::CyclesResponse) -> Self {
        Self {
            cycles_transferred: value.cycles_transferred,
        }
    }
}

impl From<Response> for crate::dto::rpc::Response {
    fn from(value: Response) -> Self {
        match value {
            Response::CreateCanister(res) => Self::CreateCanister(res.into()),
            Response::UpgradeCanister(res) => Self::UpgradeCanister(res.into()),
            Response::Cycles(res) => Self::Cycles(res.into()),
            Response::DelegationIssued(res) => Self::DelegationIssued(res),
            Response::RoleAttestationIssued(res) => Self::RoleAttestationIssued(res),
        }
    }
}

impl From<crate::dto::rpc::Response> for Response {
    fn from(value: crate::dto::rpc::Response) -> Self {
        match value {
            crate::dto::rpc::Response::CreateCanister(res) => Self::CreateCanister(res.into()),
            crate::dto::rpc::Response::UpgradeCanister(res) => Self::UpgradeCanister(res.into()),
            crate::dto::rpc::Response::Cycles(res) => Self::Cycles(res.into()),
            crate::dto::rpc::Response::DelegationIssued(res) => Self::DelegationIssued(res),
            crate::dto::rpc::Response::RoleAttestationIssued(res) => {
                Self::RoleAttestationIssued(res)
            }
        }
    }
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

const DEFAULT_ROOT_REQUEST_TTL_SECONDS: u64 = 300;
static ROOT_REQUEST_NONCE: AtomicU64 = AtomicU64::new(1);

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
            metadata: Some(new_request_metadata()),
        })
        .await
    }

    pub async fn upgrade_canister(
        canister_pid: Principal,
    ) -> Result<UpgradeCanisterResponse, InternalError> {
        RpcOps::execute_root_response_rpc(UpgradeCanisterRpc {
            canister_pid,
            metadata: Some(new_request_metadata()),
        })
        .await
    }

    pub async fn request_cycles(cycles: u128) -> Result<CyclesResponse, InternalError> {
        RpcOps::execute_root_response_rpc(CyclesRpc {
            cycles,
            metadata: Some(new_request_metadata()),
        })
        .await
    }
}

///
/// CreateCanisterRpc
///

struct CreateCanisterRpc {
    canister_role: CanisterRole,
    parent: CreateCanisterParent,
    extra_arg: Option<Vec<u8>>,
    metadata: Option<RootRequestMetadata>,
}

impl Rpc for CreateCanisterRpc {
    type Response = CreateCanisterResponse;

    fn into_request(self) -> Request {
        Request::CreateCanister(CreateCanisterRequest {
            canister_role: self.canister_role,
            parent: self.parent,
            extra_arg: self.extra_arg,
            metadata: self.metadata,
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
    pub metadata: Option<RootRequestMetadata>,
}

impl Rpc for UpgradeCanisterRpc {
    type Response = UpgradeCanisterResponse;

    fn into_request(self) -> Request {
        Request::UpgradeCanister(UpgradeCanisterRequest {
            canister_pid: self.canister_pid,
            metadata: self.metadata,
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
    pub metadata: Option<RootRequestMetadata>,
}

impl Rpc for CyclesRpc {
    type Response = CyclesResponse;

    fn into_request(self) -> Request {
        Request::Cycles(CyclesRequest {
            cycles: self.cycles,
            metadata: self.metadata,
        })
    }

    fn try_from_response(resp: Response) -> Result<Self::Response, InternalError> {
        match resp {
            Response::Cycles(r) => Ok(r),
            _ => Err(RequestOpsError::InvalidResponseType.into()),
        }
    }
}

fn new_request_metadata() -> RootRequestMetadata {
    RootRequestMetadata {
        request_id: generate_request_id(),
        ttl_seconds: DEFAULT_ROOT_REQUEST_TTL_SECONDS,
    }
}

fn generate_request_id() -> [u8; 32] {
    if let Ok(bytes) = crate::utils::rand::random_bytes(32)
        && bytes.len() == 32
    {
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        return out;
    }

    // Fallback when RNG is not yet seeded: deterministic but collision-resistant.
    let nonce = ROOT_REQUEST_NONCE.fetch_add(1, Ordering::Relaxed);
    let now = IcOps::now_secs();
    let caller = IcOps::msg_caller();
    let canister = IcOps::canister_self();

    let mut hasher = Sha256::new();
    hasher.update(now.to_be_bytes());
    hasher.update(nonce.to_be_bytes());
    hasher.update(caller.as_slice());
    hasher.update(canister.as_slice());
    hasher.finalize().into()
}
