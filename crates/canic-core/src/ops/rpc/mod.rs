pub mod request;

use crate::{
    InternalError, InternalErrorOrigin,
    dto::{
        capability::{
            CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
            NonrootCyclesCapabilityEnvelopeV1, NonrootCyclesCapabilityResponseV1,
            RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
        },
        error::Error,
        rpc::{CreateCanisterParent, Request as DtoRequest},
    },
    ops::{
        OpsError,
        ic::{
            IcOps,
            call::{CallOps, CallResult},
        },
        prelude::*,
        rpc::request::{Request, RequestOpsError, Response},
        runtime::env::EnvOps,
    },
    protocol,
};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error as ThisError;

///
/// RpcOpsError
///

#[derive(Debug, ThisError)]
pub enum RpcOpsError {
    #[error(transparent)]
    RequestOps(#[from] RequestOpsError),

    // Error is a wire-level contract.
    // It is preserved through the ops boundary.
    #[error("rpc rejected: {0}")]
    RemoteRejected(Error),
}

impl From<RpcOpsError> for InternalError {
    fn from(err: RpcOpsError) -> Self {
        match err {
            RpcOpsError::RemoteRejected(err) => Self::public(err),
            other @ RpcOpsError::RequestOps(_) => OpsError::from(other).into(),
        }
    }
}

///
/// Rpc
/// Typed RPC command binding a request variant to its response payload.
///

pub trait Rpc {
    type Response: CandidType + DeserializeOwned;

    fn into_request(self) -> Request;
    fn try_from_response(resp: Response) -> Result<Self::Response, InternalError>;
}

const DEFAULT_CAPABILITY_METADATA_TTL_NS: u64 = 300_000_000_000;
const ROOT_CAPABILITY_NONCE_DOMAIN_V1: &[u8] = b"canic-root-capability-nonce-v1";
static ROOT_CAPABILITY_METADATA_NONCE: AtomicU64 = AtomicU64::new(1);

///
/// RpcOps
///

pub struct RpcOps;

impl RpcOps {
    ///
    /// call_rpc_result
    ///
    /// Calls a method that returns `Result<T, Error>` and
    /// preserves `Error` at the ops boundary.
    ///
    pub async fn call_rpc_result<T>(
        pid: Principal,
        method: &str,
        arg: impl CandidType,
    ) -> Result<T, InternalError>
    where
        T: CandidType + DeserializeOwned,
    {
        let call: CallResult = CallOps::unbounded_wait(pid, method)
            .with_arg(arg)?
            .execute()
            .await?;

        let call_res: Result<T, Error> = call.candid::<Result<T, Error>>()?;

        let res = call_res.map_err(RpcOpsError::RemoteRejected)?;

        Ok(res)
    }

    ///
    /// execute_response_rpc
    ///
    /// Executes a protocol-level RPC via Request/Response.
    ///
    pub(crate) async fn execute_response_rpc<R: Rpc>(
        target_pid: Principal,
        rpc: R,
    ) -> Result<R::Response, InternalError> {
        let request = rpc.into_request();
        if !uses_structural_capability_proof(&request) {
            return Err(non_structural_capability_proof_error(&request));
        }

        let call_res = Self::call_response_capability_v1_structural(target_pid, request).await?;
        let response = R::try_from_response(call_res)?;

        Ok(response)
    }

    async fn call_response_capability_v1_structural(
        target_pid: Principal,
        request: Request,
    ) -> Result<Response, InternalError> {
        let root_pid = EnvOps::root_pid()?;
        if target_pid == root_pid {
            let metadata = capability_metadata_from_request(&request);
            let envelope = RootCapabilityEnvelopeV1 {
                service: CapabilityService::Root,
                capability_version: CAPABILITY_VERSION_V1,
                capability: request,
                proof: CapabilityProof::Structural,
                metadata,
            };

            let response: RootCapabilityResponseV1 =
                Self::call_rpc_result(target_pid, protocol::CANIC_RESPONSE_CAPABILITY_V1, envelope)
                    .await?;

            return Ok(response.response);
        }

        let metadata = capability_metadata_from_request(&request);
        let Request::Cycles(capability) = request else {
            return Err(InternalError::ops(
                InternalErrorOrigin::Ops,
                "structural capability path only supports cycles requests",
            ));
        };
        let envelope = NonrootCyclesCapabilityEnvelopeV1 {
            service: CapabilityService::Root,
            capability_version: CAPABILITY_VERSION_V1,
            capability,
            proof: CapabilityProof::Structural,
            metadata,
        };

        let response: NonrootCyclesCapabilityResponseV1 =
            Self::call_rpc_result(target_pid, protocol::CANIC_RESPONSE_CAPABILITY_V1, envelope)
                .await?;

        Ok(Response::Cycles(response.response))
    }
}

const fn uses_structural_capability_proof(request: &Request) -> bool {
    match request {
        Request::CreateCanister(req) => {
            matches!(&req.parent, CreateCanisterParent::ThisCanister)
        }
        Request::UpgradeCanister(_) | Request::RecycleCanister(_) | Request::Cycles(_) => true,
    }
}

fn non_structural_capability_proof_error(request: &Request) -> InternalError {
    let request: DtoRequest = request.clone();
    InternalError::ops(
        InternalErrorOrigin::Ops,
        format!(
            "non-structural root capability proof is not supported for {}; use a structural capability path or delegated-token endpoint",
            request.family().label()
        ),
    )
}

fn capability_metadata_from_request(request: &Request) -> CapabilityRequestMetadata {
    let metadata = request_metadata(request);
    let request_id = metadata.map_or([0u8; 16], |m| {
        let mut out = [0u8; 16];
        out.copy_from_slice(&m.request_id[..16]);
        out
    });
    let ttl_ns = metadata.map_or(DEFAULT_CAPABILITY_METADATA_TTL_NS, |m| m.ttl_ns);

    CapabilityRequestMetadata {
        request_id,
        nonce: generate_capability_nonce(),
        issued_at_ns: IcOps::now_nanos(),
        ttl_ns,
    }
}

///
/// CapabilitySourceMetadata
///

#[derive(Clone, Copy)]
struct CapabilitySourceMetadata {
    request_id: [u8; 32],
    ttl_ns: u64,
}

fn request_metadata(request: &Request) -> Option<CapabilitySourceMetadata> {
    request.metadata().map(|m| CapabilitySourceMetadata {
        request_id: m.request_id,
        ttl_ns: m.ttl_ns,
    })
}

fn generate_capability_nonce() -> [u8; 16] {
    let nonce = ROOT_CAPABILITY_METADATA_NONCE.fetch_add(1, Ordering::Relaxed);
    let now = IcOps::now_secs();
    let caller = IcOps::metadata_entropy_caller();
    let canister = IcOps::metadata_entropy_canister();

    let mut hasher = Sha256::new();
    hasher.update(ROOT_CAPABILITY_NONCE_DOMAIN_V1);
    hasher.update(now.to_be_bytes());
    hasher.update(nonce.to_be_bytes());
    hasher.update(caller.as_slice());
    hasher.update(canister.as_slice());
    let digest: [u8; 32] = hasher.finalize().into();
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::rpc::{
        CreateCanisterRequest, CyclesRequest, RecycleCanisterRequest, RootRequestMetadata,
        UpgradeCanisterRequest,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn capability_metadata_from_request_uses_request_id_prefix_and_ttl_ns() {
        let request_id = std::array::from_fn(|i| u8::try_from(i).unwrap());
        let request = Request::cycles(CyclesRequest {
            cycles: 1,
            metadata: Some(RootRequestMetadata {
                request_id,
                ttl_ns: u64::MAX,
            }),
        });

        let metadata = capability_metadata_from_request(&request);
        let expected_prefix: [u8; 16] = request_id[..16]
            .try_into()
            .expect("request_id prefix must be 16 bytes");
        assert_eq!(metadata.request_id, expected_prefix);
        assert_eq!(metadata.ttl_ns, u64::MAX);
        assert!(
            metadata.issued_at_ns > 1_700_000_000_000_000_000,
            "issued_at_ns should be host-time nanoseconds in tests"
        );
    }

    #[test]
    fn capability_metadata_from_request_defaults_when_missing() {
        let request = Request::cycles(CyclesRequest {
            cycles: 1,
            metadata: None,
        });

        let metadata = capability_metadata_from_request(&request);
        assert_eq!(metadata.request_id, [0u8; 16]);
        assert_eq!(metadata.ttl_ns, DEFAULT_CAPABILITY_METADATA_TTL_NS);
    }

    #[test]
    fn structural_capability_proof_support_is_exact() {
        assert!(uses_structural_capability_proof(&Request::cycles(
            CyclesRequest {
                cycles: 1,
                metadata: None,
            },
        )));
        assert!(uses_structural_capability_proof(
            &Request::upgrade_canister(UpgradeCanisterRequest {
                canister_pid: p(1),
                metadata: None,
            },)
        ));
        assert!(uses_structural_capability_proof(
            &Request::recycle_canister(RecycleCanisterRequest {
                canister_pid: p(2),
                metadata: None,
            },)
        ));
        assert!(uses_structural_capability_proof(&Request::create_canister(
            CreateCanisterRequest {
                canister_role: CanisterRole::new("child"),
                parent: CreateCanisterParent::ThisCanister,
                extra_arg: None,
                metadata: None,
            },
        )));
        assert!(!uses_structural_capability_proof(
            &Request::create_canister(CreateCanisterRequest {
                canister_role: CanisterRole::new("child"),
                parent: CreateCanisterParent::Root,
                extra_arg: None,
                metadata: None,
            },)
        ));
    }
}
