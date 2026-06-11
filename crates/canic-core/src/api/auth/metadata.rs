use crate::{
    dto::auth::{AuthRequestMetadata, DelegationProofIssueRequest},
    ops::ic::IcOps,
};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};

const DEFAULT_AUTH_REQUEST_TTL_NS: u64 = 300_000_000_000;
const AUTH_ROOT_REQUEST_METADATA_DOMAIN: &[u8] = b"canic-auth-root-request-metadata-v1";
static ROOT_REQUEST_NONCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn with_delegation_request_metadata(
    mut request: DelegationProofIssueRequest,
) -> DelegationProofIssueRequest {
    if request.metadata.is_none() {
        request.metadata = Some(new_auth_request_metadata());
    }
    request
}

fn new_auth_request_metadata() -> AuthRequestMetadata {
    AuthRequestMetadata {
        request_id: generate_request_id(),
        ttl_ns: DEFAULT_AUTH_REQUEST_TTL_NS,
    }
}

fn generate_request_id() -> [u8; 32] {
    let nonce = ROOT_REQUEST_NONCE.fetch_add(1, Ordering::Relaxed);
    let now = IcOps::now_secs();
    let caller = IcOps::metadata_entropy_caller();
    let canister = IcOps::metadata_entropy_canister();

    let mut hasher = Sha256::new();
    hasher.update(AUTH_ROOT_REQUEST_METADATA_DOMAIN);
    hasher.update(now.to_be_bytes());
    hasher.update(nonce.to_be_bytes());
    hasher.update(caller.as_slice());
    hasher.update(canister.as_slice());
    hasher.finalize().into()
}
