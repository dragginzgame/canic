use crate::{
    cdk::types::Principal,
    dto::{capability::CapabilityService, error::Error, rpc::Request},
};
use candid::encode_one;
use sha2::{Digest, Sha256};

pub(super) fn root_capability_hash(
    target_canister: Principal,
    capability_version: u16,
    capability: &Request,
) -> Result<[u8; 32], Error> {
    let canonical = capability.clone().without_metadata();
    let payload = encode_one(&(
        target_canister,
        CapabilityService::Root,
        capability_version,
        canonical,
    ))
    .map_err(|err| Error::internal(format!("failed to encode capability payload: {err}")))?;
    let mut hasher = Sha256::new();
    hasher.update(super::CAPABILITY_HASH_DOMAIN_V1);
    hasher.update(payload);
    Ok(hasher.finalize().into())
}
