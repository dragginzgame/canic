pub mod request;

use crate::{
    InternalError, InternalErrorOrigin,
    dto::{
        auth::{RoleAttestationRequest, SignedRoleAttestation},
        capability::{
            CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
            NonrootCyclesCapabilityEnvelopeV1, NonrootCyclesCapabilityResponseV1, PROOF_VERSION_V1,
            RoleAttestationProof, RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
        },
        error::Error,
        rpc::{CreateCanisterParent, RootRequestMetadata},
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
use std::{
    cell::RefCell,
    sync::atomic::{AtomicU64, Ordering},
};
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

const NS_PER_SEC: u64 = 1_000_000_000;
const DEFAULT_ROOT_ATTESTATION_REQUEST_TTL_NS: u64 = 300_000_000_000;
const DEFAULT_CAPABILITY_METADATA_TTL_NS: u64 = 300_000_000_000;
const CAPABILITY_HASH_DOMAIN_V1: &[u8] = b"CANIC_CAPABILITY_V1";
const ROOT_ATTESTATION_REQUEST_ID_DOMAIN_V1: &[u8] = b"canic-root-attestation-request-id-v1";
const ROOT_CAPABILITY_NONCE_DOMAIN_V1: &[u8] = b"canic-root-capability-nonce-v1";
static ROOT_ATTESTATION_REQUEST_NONCE: AtomicU64 = AtomicU64::new(1);
static ROOT_CAPABILITY_METADATA_NONCE: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static ROOT_RESPONSE_ATTESTATION_CACHE: RefCell<Option<CachedRootResponseAttestation>> =
        const { RefCell::new(None) };
}

#[derive(Clone)]
struct CachedRootResponseAttestation {
    root_pid: Principal,
    audience_pid: Principal,
    subject_pid: Principal,
    role: CanisterRole,
    attestation: SignedRoleAttestation,
}

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
        let call_res = if uses_structural_capability_proof(&request) {
            Self::call_response_capability_v1_structural(target_pid, request).await?
        } else {
            let root_pid = EnvOps::root_pid()?;
            let attestation = Self::request_root_response_attestation(root_pid, target_pid).await?;
            Self::call_response_capability_v1(target_pid, request, attestation).await?
        };

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
        let min_accepted_epoch = cfg
            .min_accepted_epoch_by_role
            .get(role.as_str())
            .copied()
            .unwrap_or(0);

        if let Some(attestation) = cached_root_response_attestation(
            root_pid,
            audience_pid,
            self_pid,
            &role,
            min_accepted_epoch,
            IcOps::now_nanos(),
        ) {
            return Ok(attestation);
        }

        let request = RoleAttestationRequest {
            subject: self_pid,
            role,
            subnet_id: None,
            audience: audience_pid,
            ttl_ns: cfg.max_ttl_secs.checked_mul(NS_PER_SEC).ok_or_else(|| {
                InternalError::ops(
                    InternalErrorOrigin::Ops,
                    "auth.role_attestation.max_ttl_secs overflows nanoseconds",
                )
            })?,
            epoch: min_accepted_epoch,
            metadata: Some(new_root_attestation_request_metadata()),
        };

        let attestation: SignedRoleAttestation =
            Self::call_rpc_result(root_pid, protocol::CANIC_REQUEST_ROLE_ATTESTATION, request)
                .await?;
        cache_root_response_attestation(root_pid, audience_pid, self_pid, attestation.clone());
        Ok(attestation)
    }

    async fn call_response_capability_v1(
        target_pid: Principal,
        request: Request,
        attestation: SignedRoleAttestation,
    ) -> Result<Response, InternalError> {
        let dto_request: crate::dto::rpc::Request = request.clone();
        let capability_hash = root_capability_hash(target_pid, &dto_request)?;
        let proof = RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash,
            attestation,
        }
        .try_into()
        .map_err(InternalError::public)?;
        let envelope = RootCapabilityEnvelopeV1 {
            service: CapabilityService::Root,
            capability_version: CAPABILITY_VERSION_V1,
            capability: dto_request,
            proof,
            metadata: capability_metadata_from_request(&request),
        };

        let response: RootCapabilityResponseV1 =
            Self::call_rpc_result(target_pid, protocol::CANIC_RESPONSE_CAPABILITY_V1, envelope)
                .await?;

        Ok(response.response)
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
        Request::IssueRoleAttestation(_) | Request::IssueInternalInvocationProof(_) => false,
    }
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

// cached_root_response_attestation
//
// Reuse a still-valid root-issued role attestation for the same audience.
fn cached_root_response_attestation(
    root_pid: Principal,
    audience_pid: Principal,
    subject_pid: Principal,
    role: &CanisterRole,
    min_accepted_epoch: u64,
    now_ns: u64,
) -> Option<SignedRoleAttestation> {
    ROOT_RESPONSE_ATTESTATION_CACHE.with_borrow_mut(|entry| {
        let cached = entry.as_ref()?;
        let payload = &cached.attestation.payload;
        let valid = cached.root_pid == root_pid
            && cached.audience_pid == audience_pid
            && cached.subject_pid == subject_pid
            && &cached.role == role
            && payload.subject == subject_pid
            && &payload.role == role
            && payload.audience == audience_pid
            && payload.epoch >= min_accepted_epoch
            && now_ns < payload.expires_at_ns;

        if !valid {
            *entry = None;
            return None;
        }

        Some(cached.attestation.clone())
    })
}

// cache_root_response_attestation
//
// Store the latest root-issued role attestation for repeated outbound RPC use.
fn cache_root_response_attestation(
    root_pid: Principal,
    audience_pid: Principal,
    subject_pid: Principal,
    attestation: SignedRoleAttestation,
) {
    ROOT_RESPONSE_ATTESTATION_CACHE.with_borrow_mut(|entry| {
        *entry = Some(CachedRootResponseAttestation {
            root_pid,
            audience_pid,
            subject_pid,
            role: attestation.payload.role.clone(),
            attestation,
        });
    });
}

fn root_capability_hash(
    target_canister: Principal,
    capability: &crate::dto::rpc::Request,
) -> Result<[u8; 32], InternalError> {
    let canonical = capability.clone().canonical_capability_payload();
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
        ttl_ns: DEFAULT_ROOT_ATTESTATION_REQUEST_TTL_NS,
    }
}

fn generate_root_attestation_request_id() -> [u8; 32] {
    let nonce = ROOT_ATTESTATION_REQUEST_NONCE.fetch_add(1, Ordering::Relaxed);
    let now = IcOps::now_secs();
    let caller = IcOps::metadata_entropy_caller();
    let canister = IcOps::metadata_entropy_canister();

    let mut hasher = Sha256::new();
    hasher.update(ROOT_ATTESTATION_REQUEST_ID_DOMAIN_V1);
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
    use crate::dto::{
        auth::RoleAttestation,
        rpc::{CyclesRequest, RootRequestMetadata},
    };

    const USER_HUB_ROLE: CanisterRole = CanisterRole::new("user_hub");

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn sample_attestation(
        subject: Principal,
        audience: Principal,
        expires_at_ns: u64,
        epoch: u64,
    ) -> SignedRoleAttestation {
        SignedRoleAttestation {
            payload: RoleAttestation {
                subject,
                role: USER_HUB_ROLE,
                subnet_id: None,
                audience,
                issued_at_ns: 100,
                expires_at_ns,
                epoch,
            },
            signature: vec![1, 2, 3],
            key_id: 7,
        }
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
    fn cached_root_response_attestation_reuses_matching_entry() {
        let root_pid = p(1);
        let audience_pid = p(2);
        let subject_pid = p(3);
        let attestation = sample_attestation(subject_pid, audience_pid, 500, 9);

        cache_root_response_attestation(root_pid, audience_pid, subject_pid, attestation.clone());

        let cached = cached_root_response_attestation(
            root_pid,
            audience_pid,
            subject_pid,
            &USER_HUB_ROLE,
            9,
            400,
        )
        .expect("matching cached attestation");

        assert_eq!(cached, attestation);
    }

    #[test]
    fn cached_root_response_attestation_reuses_root_newer_epoch_above_local_floor() {
        let root_pid = p(11);
        let audience_pid = p(12);
        let subject_pid = p(13);
        let attestation = sample_attestation(subject_pid, audience_pid, 500, 12);

        cache_root_response_attestation(root_pid, audience_pid, subject_pid, attestation.clone());

        let cached = cached_root_response_attestation(
            root_pid,
            audience_pid,
            subject_pid,
            &USER_HUB_ROLE,
            7,
            400,
        )
        .expect("root-issued newer epoch above the local floor should be reusable");

        assert_eq!(cached, attestation);
    }

    #[test]
    fn cached_root_response_attestation_rejects_epoch_below_local_floor() {
        let root_pid = p(21);
        let audience_pid = p(22);
        let subject_pid = p(23);

        cache_root_response_attestation(
            root_pid,
            audience_pid,
            subject_pid,
            sample_attestation(subject_pid, audience_pid, 500, 3),
        );

        assert!(
            cached_root_response_attestation(
                root_pid,
                audience_pid,
                subject_pid,
                &USER_HUB_ROLE,
                7,
                400,
            )
            .is_none(),
            "cached attestation below the local epoch floor must not be reused"
        );
    }

    #[test]
    fn cached_root_response_attestation_invalidates_expired_entry() {
        let root_pid = p(4);
        let audience_pid = p(5);
        let subject_pid = p(6);

        cache_root_response_attestation(
            root_pid,
            audience_pid,
            subject_pid,
            sample_attestation(subject_pid, audience_pid, 120, 11),
        );

        assert!(
            cached_root_response_attestation(
                root_pid,
                audience_pid,
                subject_pid,
                &USER_HUB_ROLE,
                11,
                121,
            )
            .is_none(),
            "expired cache entry must not be reused"
        );
    }

    #[test]
    fn cached_root_response_attestation_rejects_payload_subject_drift() {
        let root_pid = p(7);
        let audience_pid = p(8);
        let subject_pid = p(9);
        let mut attestation = sample_attestation(subject_pid, audience_pid, 500, 13);
        attestation.payload.subject = p(10);

        cache_root_response_attestation(root_pid, audience_pid, subject_pid, attestation);

        assert!(
            cached_root_response_attestation(
                root_pid,
                audience_pid,
                subject_pid,
                &USER_HUB_ROLE,
                13,
                400,
            )
            .is_none(),
            "cached attestation payload must still bind the requested subject"
        );
    }
}
