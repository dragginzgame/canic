use crate::{
    dto::{
        auth::{DelegationRequest, RoleAttestationRequest},
        rpc::RootRequestMetadata,
    },
    ops::ic::IcOps,
};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};

const DEFAULT_ROOT_REQUEST_TTL_SECONDS: u64 = 300;
static ROOT_REQUEST_NONCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn with_root_request_metadata(mut request: DelegationRequest) -> DelegationRequest {
    if request.metadata.is_none() {
        request.metadata = Some(new_request_metadata());
    }
    request
}

pub(super) fn with_root_attestation_request_metadata(
    mut request: RoleAttestationRequest,
) -> RoleAttestationRequest {
    if request.metadata.is_none() {
        request.metadata = Some(new_request_metadata());
    }
    request
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
    hasher.update(now.to_be_bytes());
    hasher.update(nonce.to_be_bytes());
    hasher.update(caller.as_slice());
    hasher.update(canister.as_slice());
    hasher.finalize().into()
}
