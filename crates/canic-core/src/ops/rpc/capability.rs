//! Module: ops::rpc::capability
//!
//! Responsibility: compute protocol-level RPC capability hashes.
//! Does not own: workflow proof validation, replay orchestration, or request dispatch.
//! Boundary: owns canonical wire encoding used for capability proof binding.

use crate::{
    cdk::types::Principal,
    dto::{capability::CapabilityService, error::Error, rpc::Request},
    ops::rpc::request::RequestConversionOps,
};
use candid::encode_one;
use sha2::{Digest, Sha256};

const CAPABILITY_HASH_DOMAIN_V1: &[u8] = b"CANIC_CAPABILITY_V1";

/// Compute the canonical root capability hash for proof binding.
pub fn root_capability_hash(
    target_canister: Principal,
    capability_version: u16,
    capability: &Request,
) -> Result<[u8; 32], Error> {
    let canonical = RequestConversionOps::canonical_capability_payload(capability);
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
