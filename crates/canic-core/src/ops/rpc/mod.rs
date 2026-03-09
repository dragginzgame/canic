pub mod request;

use crate::{
    InternalError, InternalErrorOrigin,
    dto::{
        auth::{RoleAttestationRequest, SignedRoleAttestation},
        capability::{
            CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
            PROOF_VERSION_V1, RoleAttestationProof, RootCapabilityEnvelopeV1,
            RootCapabilityResponseV1,
        },
        error::Error,
        rpc::RootRequestMetadata,
    },
    ops::{
        OpsError,
        config::ConfigOps,
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
use candid::encode_one;
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

///
/// RpcOps
///

pub struct RpcOps;

const DEFAULT_ROOT_ATTESTATION_REQUEST_TTL_SECONDS: u64 = 300;
const DEFAULT_CAPABILITY_METADATA_TTL_SECONDS: u32 = 300;
const CAPABILITY_HASH_DOMAIN_V1: &[u8] = b"CANIC_CAPABILITY_V1";
static ROOT_ATTESTATION_REQUEST_NONCE: AtomicU64 = AtomicU64::new(1);
static ROOT_CAPABILITY_METADATA_NONCE: AtomicU64 = AtomicU64::new(1);

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
        let root_pid = EnvOps::root_pid()?;
        let request = rpc.into_request();
        let attestation = Self::request_root_response_attestation(root_pid, target_pid).await?;
        let call_res = Self::call_response_capability_v1(target_pid, request, attestation).await?;

        let response = R::try_from_response(call_res)?;

        Ok(response)
    }

    async fn request_root_response_attestation(
        root_pid: Principal,
        audience_pid: Principal,
    ) -> Result<SignedRoleAttestation, InternalError> {
        let self_pid = IcOps::canister_self();
        let role = EnvOps::canister_role()?;
        let cfg = ConfigOps::role_attestation_config()?;
        let epoch = cfg
            .min_accepted_epoch_by_role
            .get(role.as_str())
            .copied()
            .unwrap_or(0);

        let request = RoleAttestationRequest {
            subject: self_pid,
            role,
            subnet_id: None,
            audience: Some(audience_pid),
            ttl_secs: cfg.max_ttl_secs,
            epoch,
            metadata: Some(new_root_attestation_request_metadata()),
        };

        Self::call_rpc_result(root_pid, protocol::CANIC_REQUEST_ROLE_ATTESTATION, request).await
    }

    async fn call_response_capability_v1(
        target_pid: Principal,
        request: Request,
        attestation: SignedRoleAttestation,
    ) -> Result<Response, InternalError> {
        let dto_request: crate::dto::rpc::Request = request.clone();
        let capability_hash = root_capability_hash(target_pid, &dto_request)?;
        let envelope = RootCapabilityEnvelopeV1 {
            service: CapabilityService::Root,
            capability_version: CAPABILITY_VERSION_V1,
            capability: dto_request,
            proof: CapabilityProof::RoleAttestation(RoleAttestationProof {
                proof_version: PROOF_VERSION_V1,
                capability_hash,
                attestation,
            }),
            metadata: capability_metadata_from_request(&request),
        };

        let response: RootCapabilityResponseV1 =
            Self::call_rpc_result(target_pid, protocol::CANIC_RESPONSE_CAPABILITY_V1, envelope)
                .await?;

        Ok(response.response)
    }
}

fn capability_metadata_from_request(request: &Request) -> CapabilityRequestMetadata {
    let metadata = request_metadata(request);
    let request_id = metadata.map_or([0u8; 16], |m| {
        let mut out = [0u8; 16];
        out.copy_from_slice(&m.request_id[..16]);
        out
    });
    let ttl_seconds = metadata.map_or(DEFAULT_CAPABILITY_METADATA_TTL_SECONDS, |m| {
        u32::try_from(m.ttl_seconds.min(u64::from(u32::MAX))).expect("ttl_seconds bounded to u32")
    });

    CapabilityRequestMetadata {
        request_id,
        nonce: generate_capability_nonce(),
        issued_at: IcOps::now_secs(),
        ttl_seconds,
    }
}

#[derive(Clone, Copy)]
struct CapabilitySourceMetadata {
    request_id: [u8; 32],
    ttl_seconds: u64,
}

fn request_metadata(request: &Request) -> Option<CapabilitySourceMetadata> {
    request.metadata().map(|m| CapabilitySourceMetadata {
        request_id: m.request_id,
        ttl_seconds: m.ttl_seconds,
    })
}

fn generate_capability_nonce() -> [u8; 16] {
    if let Ok(bytes) = crate::utils::rand::random_bytes(16)
        && bytes.len() == 16
    {
        let mut out = [0u8; 16];
        out.copy_from_slice(&bytes);
        return out;
    }

    let nonce = ROOT_CAPABILITY_METADATA_NONCE.fetch_add(1, Ordering::Relaxed);
    let now = IcOps::now_secs();
    let caller = IcOps::msg_caller();
    let canister = IcOps::canister_self();

    let mut hasher = Sha256::new();
    hasher.update(now.to_be_bytes());
    hasher.update(nonce.to_be_bytes());
    hasher.update(caller.as_slice());
    hasher.update(canister.as_slice());
    let digest: [u8; 32] = hasher.finalize().into();
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

fn root_capability_hash(
    target_canister: Principal,
    capability: &crate::dto::rpc::Request,
) -> Result<[u8; 32], InternalError> {
    let canonical = capability.clone().without_metadata();
    let payload = encode_one(&(
        target_canister,
        CapabilityService::Root,
        CAPABILITY_VERSION_V1,
        canonical,
    ))
    .map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Ops,
            format!("failed to encode capability payload: {err}"),
        )
    })?;

    let mut hasher = Sha256::new();
    hasher.update(CAPABILITY_HASH_DOMAIN_V1);
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn new_root_attestation_request_metadata() -> RootRequestMetadata {
    RootRequestMetadata {
        request_id: generate_root_attestation_request_id(),
        ttl_seconds: DEFAULT_ROOT_ATTESTATION_REQUEST_TTL_SECONDS,
    }
}

fn generate_root_attestation_request_id() -> [u8; 32] {
    if let Ok(bytes) = crate::utils::rand::random_bytes(32)
        && bytes.len() == 32
    {
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        return out;
    }

    let nonce = ROOT_ATTESTATION_REQUEST_NONCE.fetch_add(1, Ordering::Relaxed);
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

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::rpc::{CyclesRequest, RootRequestMetadata};

    #[test]
    #[expect(clippy::cast_possible_truncation)]
    fn capability_metadata_from_request_uses_request_id_prefix_and_ttl_clamp() {
        crate::utils::rand::seed_from([7u8; 32]);

        let request_id = std::array::from_fn(|i| i as u8);
        let request = Request::cycles(CyclesRequest {
            cycles: 1,
            metadata: Some(RootRequestMetadata {
                request_id,
                ttl_seconds: u64::MAX,
            }),
        });

        let metadata = capability_metadata_from_request(&request);
        let expected_prefix: [u8; 16] = request_id[..16]
            .try_into()
            .expect("request_id prefix must be 16 bytes");
        assert_eq!(metadata.request_id, expected_prefix);
        assert_eq!(metadata.ttl_seconds, u32::MAX);
        assert!(
            metadata.issued_at > 1_700_000_000,
            "issued_at should be host-time seconds in tests"
        );
    }

    #[test]
    fn capability_metadata_from_request_defaults_when_missing() {
        crate::utils::rand::seed_from([9u8; 32]);

        let request = Request::cycles(CyclesRequest {
            cycles: 1,
            metadata: None,
        });

        let metadata = capability_metadata_from_request(&request);
        assert_eq!(metadata.request_id, [0u8; 16]);
        assert_eq!(
            metadata.ttl_seconds,
            DEFAULT_CAPABILITY_METADATA_TTL_SECONDS
        );
    }
}
