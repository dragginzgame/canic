//! Module: workflow::rpc::capability::hash
//!
//! Responsibility: compute canonical root capability proof-binding hashes.
//! Does not own: proof validation, request dispatch, or replay metadata.
//! Boundary: encodes canonical capability payloads with capability hash domain separation.

use crate::{
    cdk::types::Principal,
    dto::{capability::CapabilityService, error::Error, rpc::Request},
    workflow::rpc::capability::CAPABILITY_HASH_DOMAIN_V1,
};
use candid::encode_one;
use sha2::{Digest, Sha256};

pub(super) fn root_capability_hash(
    target_canister: Principal,
    capability_version: u16,
    capability: &Request,
) -> Result<[u8; 32], Error> {
    let canonical = capability.clone().canonical_capability_payload();
    let payload = encode_one(&(
        target_canister,
        CapabilityService::Root,
        capability_version,
        canonical,
    ))
    .map_err(|err| Error::internal(format!("failed to encode capability payload: {err}")))?;
    let mut hasher = Sha256::new();
    hasher.update(CAPABILITY_HASH_DOMAIN_V1);
    hasher.update(payload);
    Ok(hasher.finalize().into())
}
