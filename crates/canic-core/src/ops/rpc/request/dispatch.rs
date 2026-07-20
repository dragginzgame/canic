//! Module: ops::rpc::request::dispatch
//!
//! Responsibility: build typed root RPC requests and decode typed responses.
//! Does not own: workflow policy, capability proof verification, or replay storage.
//! Boundary: delegates transport to `RpcOps` after attaching request metadata.

use super::RequestOpsError;
use crate::model::replay::OperationId;
use crate::{
    InternalError, InternalErrorOrigin,
    dto::rpc::{
        AcknowledgePlacementReceiptRequest, AcknowledgePlacementReceiptResponse,
        CreateCanisterParent, CreateCanisterRequest, CreateCanisterResponse, CyclesRequest,
        CyclesResponse, RecycleCanisterRequest, RecycleCanisterResponse, Request, Response,
        RootRequestMetadata, UpgradeCanisterRequest, UpgradeCanisterResponse,
    },
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

const DEFAULT_ROOT_REQUEST_TTL_NS: u64 = 300_000_000_000;
const ROOT_REQUEST_METADATA_DOMAIN_V1: &[u8] = b"canic-root-request-metadata-v1";
static ROOT_REQUEST_NONCE: AtomicU64 = AtomicU64::new(1);

///
/// RequestOps
///
/// Ops-level helpers for request/response RPCs.
///

pub struct RequestOps;

impl RequestOps {
    /// Dispatch a create-canister request to the configured root canister.
    pub async fn create_canister<A>(
        canister_role: &CanisterRole,
        parent: CreateCanisterParent,
        extra: Option<A>,
    ) -> Result<CreateCanisterResponse, InternalError>
    where
        A: CandidType + Send + Sync,
    {
        Self::create_canister_with_metadata(canister_role, parent, extra, new_request_metadata())
            .await
    }

    /// Dispatch a placement-child request under a caller-owned durable operation identity.
    pub(crate) async fn allocate_placement_child<A>(
        canister_role: &CanisterRole,
        parent: CreateCanisterParent,
        extra: Option<A>,
        operation_id: OperationId,
    ) -> Result<CreateCanisterResponse, InternalError>
    where
        A: CandidType + Send + Sync,
    {
        Self::allocate_placement_child_with_metadata(
            canister_role,
            parent,
            extra,
            operation_request_metadata(operation_id),
        )
        .await
    }

    /// Acknowledge a placement-child response after the caller durably owns its result.
    pub(crate) async fn acknowledge_placement_receipt(
        root_pid: Principal,
        operation_id: OperationId,
    ) -> Result<AcknowledgePlacementReceiptResponse, InternalError> {
        RpcOps::execute_response_rpc(
            root_pid,
            AcknowledgePlacementReceiptRpc {
                operation_id,
                metadata: Some(operation_request_metadata(operation_id)),
            },
        )
        .await
    }

    async fn create_canister_with_metadata<A>(
        canister_role: &CanisterRole,
        parent: CreateCanisterParent,
        extra: Option<A>,
        metadata: RootRequestMetadata,
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
                command: CreateCanisterCommand::Generic,
                canister_role: canister_role.clone(),
                parent,
                extra_arg,
                metadata: Some(metadata),
            },
        )
        .await
    }

    async fn allocate_placement_child_with_metadata<A>(
        canister_role: &CanisterRole,
        parent: CreateCanisterParent,
        extra: Option<A>,
        metadata: RootRequestMetadata,
    ) -> Result<CreateCanisterResponse, InternalError>
    where
        A: CandidType + Send + Sync,
    {
        let extra_arg = extra.map(encode_one).transpose().map_err(|err| {
            InternalError::invariant(
                InternalErrorOrigin::Ops,
                format!("failed to encode placement child extra arg: {err}"),
            )
        })?;

        let root_pid = EnvOps::root_pid()?;
        RpcOps::execute_response_rpc(
            root_pid,
            CreateCanisterRpc {
                command: CreateCanisterCommand::Placement,
                canister_role: canister_role.clone(),
                parent,
                extra_arg,
                metadata: Some(metadata),
            },
        )
        .await
    }

    /// Dispatch an upgrade request for a child canister through root RPC.
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

    /// Dispatch a recycle request for a child canister through root RPC.
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

    /// Dispatch a cycles request to the current parent canister.
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
/// AcknowledgePlacementReceiptRpc
///
/// Internal command adapter for placement-receipt acknowledgement RPCs.
///

struct AcknowledgePlacementReceiptRpc {
    operation_id: OperationId,
    metadata: Option<RootRequestMetadata>,
}

impl Rpc for AcknowledgePlacementReceiptRpc {
    type Response = AcknowledgePlacementReceiptResponse;

    fn into_request(self) -> Request {
        Request::acknowledge_placement_receipt(AcknowledgePlacementReceiptRequest {
            operation_id: self.operation_id.into_bytes(),
            metadata: self.metadata,
        })
    }

    fn try_from_response(resp: Response) -> Result<Self::Response, InternalError> {
        match resp {
            Response::AcknowledgePlacementReceipt(response) => Ok(response),
            _ => Err(RequestOpsError::InvalidResponseType.into()),
        }
    }
}

///
/// CreateCanisterRpc
///
/// Internal command adapter for create-canister RPCs.
///

enum CreateCanisterCommand {
    Generic,
    Placement,
}

struct CreateCanisterRpc {
    command: CreateCanisterCommand,
    canister_role: CanisterRole,
    parent: CreateCanisterParent,
    extra_arg: Option<Vec<u8>>,
    metadata: Option<RootRequestMetadata>,
}

impl Rpc for CreateCanisterRpc {
    type Response = CreateCanisterResponse;

    fn into_request(self) -> Request {
        let request = CreateCanisterRequest {
            canister_role: self.canister_role,
            parent: self.parent,
            extra_arg: self.extra_arg,
            metadata: self.metadata,
        };
        match self.command {
            CreateCanisterCommand::Generic => Request::create_canister(request),
            CreateCanisterCommand::Placement => Request::allocate_placement_child(request),
        }
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
/// Internal command adapter for upgrade-canister RPCs.
///

struct UpgradeCanisterRpc {
    canister_pid: Principal,
    metadata: Option<RootRequestMetadata>,
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
/// Internal command adapter for recycle-canister RPCs.
///

struct RecycleCanisterRpc {
    canister_pid: Principal,
    metadata: Option<RootRequestMetadata>,
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
/// Internal command adapter for cycles-funding RPCs.
///

struct CyclesRpc {
    cycles: u128,
    metadata: Option<RootRequestMetadata>,
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
        ttl_ns: DEFAULT_ROOT_REQUEST_TTL_NS,
    }
}

const fn operation_request_metadata(operation_id: OperationId) -> RootRequestMetadata {
    RootRequestMetadata {
        request_id: operation_id.into_bytes(),
        ttl_ns: DEFAULT_ROOT_REQUEST_TTL_NS,
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn metadata(id: u8) -> RootRequestMetadata {
        RootRequestMetadata {
            request_id: [id; 32],
            ttl_ns: 123_000_000_000,
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

    #[test]
    fn private_request_adapters_preserve_request_shapes() {
        let canister_pid = p(43);
        let recycle_metadata = metadata(8);
        let recycle_request = RecycleCanisterRpc {
            canister_pid,
            metadata: Some(recycle_metadata),
        }
        .into_request();
        let Request::RecycleCanister(recycle_request) = recycle_request else {
            panic!("recycle RPC must encode a recycle request");
        };
        assert_eq!(recycle_request.canister_pid, canister_pid);
        assert_eq!(recycle_request.metadata, Some(recycle_metadata));

        let cycles_metadata = metadata(9);
        let cycles_request = CyclesRpc {
            cycles: 1_000_000,
            metadata: Some(cycles_metadata),
        }
        .into_request();
        let Request::Cycles(cycles_request) = cycles_request else {
            panic!("cycles RPC must encode a cycles request");
        };
        assert_eq!(cycles_request.cycles, 1_000_000);
        assert_eq!(cycles_request.metadata, Some(cycles_metadata));
    }

    #[test]
    fn operation_metadata_preserves_caller_owned_request_id() {
        let operation_id = OperationId::from_bytes([11; 32]);

        let metadata = operation_request_metadata(operation_id);

        assert_eq!(metadata.request_id, operation_id.into_bytes());
        assert_eq!(metadata.ttl_ns, DEFAULT_ROOT_REQUEST_TTL_NS);
    }

    #[test]
    fn operation_bound_create_uses_the_placement_command() {
        let metadata = operation_request_metadata(OperationId::from_bytes([11; 32]));
        let request = CreateCanisterRpc {
            command: CreateCanisterCommand::Placement,
            canister_role: CanisterRole::new("worker"),
            parent: CreateCanisterParent::ThisCanister,
            extra_arg: None,
            metadata: Some(metadata),
        }
        .into_request();

        let Request::AllocatePlacementChild(request) = request else {
            panic!("operation-bound create must use the placement command");
        };
        assert_eq!(request.metadata, Some(metadata));
    }

    #[test]
    fn placement_receipt_acknowledgement_carries_the_target_operation_id() {
        let operation_id = OperationId::from_bytes([12; 32]);
        let metadata = operation_request_metadata(operation_id);

        let request = AcknowledgePlacementReceiptRpc {
            operation_id,
            metadata: Some(metadata),
        }
        .into_request();
        let Request::AcknowledgePlacementReceipt(request) = request else {
            panic!("acknowledgement RPC must encode its request variant");
        };
        assert_eq!(request.operation_id, operation_id.into_bytes());
        assert_eq!(request.metadata, Some(metadata));
        assert_eq!(metadata.request_id, operation_id.into_bytes());
    }
}
