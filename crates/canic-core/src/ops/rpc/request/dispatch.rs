use super::{
    CreateCanisterParent, CreateCanisterRequest, CreateCanisterResponse, CyclesRequest,
    CyclesResponse, RecycleCanisterRequest, RecycleCanisterResponse, Request, RequestOpsError,
    Response, RootRequestMetadata, UpgradeCanisterRequest, UpgradeCanisterResponse,
};
use crate::{
    InternalError, InternalErrorOrigin,
    ops::{
        ic::IcOps,
        prelude::*,
        rpc::{Rpc, RpcOps},
        runtime::env::EnvOps,
    },
};
use candid::encode_one;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};

const DEFAULT_ROOT_REQUEST_TTL_SECONDS: u64 = 300;
const ROOT_REQUEST_METADATA_DOMAIN_V1: &[u8] = b"canic-root-request-metadata-v1";
static ROOT_REQUEST_NONCE: AtomicU64 = AtomicU64::new(1);

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

        let root_pid = EnvOps::root_pid()?;
        RpcOps::execute_response_rpc(
            root_pid,
            CreateCanisterRpc {
                canister_role: canister_role.clone(),
                parent,
                extra_arg,
                metadata: Some(new_request_metadata()),
            },
        )
        .await
    }

    pub async fn upgrade_canister(
        canister_pid: Principal,
    ) -> Result<UpgradeCanisterResponse, InternalError> {
        let root_pid = EnvOps::root_pid()?;
        RpcOps::execute_response_rpc(
            root_pid,
            UpgradeCanisterRpc {
                canister_pid,
                metadata: Some(new_request_metadata()),
            },
        )
        .await
    }

    pub async fn recycle_canister(
        canister_pid: Principal,
    ) -> Result<RecycleCanisterResponse, InternalError> {
        let root_pid = EnvOps::root_pid()?;
        RpcOps::execute_response_rpc(
            root_pid,
            RecycleCanisterRpc {
                canister_pid,
                metadata: Some(new_request_metadata()),
            },
        )
        .await
    }

    pub async fn request_cycles(cycles: u128) -> Result<CyclesResponse, InternalError> {
        let parent_pid = EnvOps::parent_pid()?;
        RpcOps::execute_response_rpc(
            parent_pid,
            CyclesRpc {
                cycles,
                metadata: Some(new_request_metadata()),
            },
        )
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
        Request::create_canister(CreateCanisterRequest {
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
        Request::upgrade_canister(UpgradeCanisterRequest {
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
/// RecycleCanisterRpc
///

pub struct RecycleCanisterRpc {
    pub canister_pid: Principal,
    pub metadata: Option<RootRequestMetadata>,
}

impl Rpc for RecycleCanisterRpc {
    type Response = RecycleCanisterResponse;

    fn into_request(self) -> Request {
        Request::recycle_canister(RecycleCanisterRequest {
            canister_pid: self.canister_pid,
            metadata: self.metadata,
        })
    }

    fn try_from_response(resp: Response) -> Result<Self::Response, InternalError> {
        match resp {
            Response::RecycleCanister(r) => Ok(r),
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
        Request::cycles(CyclesRequest {
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
    let nonce = ROOT_REQUEST_NONCE.fetch_add(1, Ordering::Relaxed);
    let now = IcOps::now_secs();
    let caller = IcOps::metadata_entropy_caller();
    let canister = IcOps::metadata_entropy_canister();

    let mut hasher = Sha256::new();
    hasher.update(ROOT_REQUEST_METADATA_DOMAIN_V1);
    hasher.update(now.to_be_bytes());
    hasher.update(nonce.to_be_bytes());
    hasher.update(caller.as_slice());
    hasher.update(canister.as_slice());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn metadata(id: u8) -> RootRequestMetadata {
        RootRequestMetadata {
            request_id: [id; 32],
            ttl_seconds: 123,
        }
    }

    #[test]
    fn upgrade_canister_rpc_carries_replay_metadata_into_request() {
        let canister_pid = p(42);
        let metadata = metadata(7);

        let request = UpgradeCanisterRpc {
            canister_pid,
            metadata: Some(metadata),
        }
        .into_request();

        let Request::UpgradeCanister(request) = request else {
            panic!("upgrade RPC must encode an upgrade request");
        };

        assert_eq!(request.canister_pid, canister_pid);
        assert_eq!(request.metadata, Some(metadata));
    }

    #[test]
    fn upgrade_canister_rpc_accepts_only_upgrade_response() {
        UpgradeCanisterRpc::try_from_response(Response::UpgradeCanister(
            UpgradeCanisterResponse {},
        ))
        .expect("upgrade response accepted");

        UpgradeCanisterRpc::try_from_response(Response::RecycleCanister(
            RecycleCanisterResponse {},
        ))
        .expect_err("wrong response variant rejected");
    }
}
