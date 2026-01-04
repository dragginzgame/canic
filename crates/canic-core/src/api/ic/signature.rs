use crate::{PublicError, ops};

///
/// Signature
/// wrappers over IC signature helpers for endpoints
///

pub fn prepare(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<(), PublicError> {
    ops::ic::signature::prepare(domain, seed, message).map_err(PublicError::from)
}

#[must_use]
pub fn get(domain: &[u8], seed: &[u8], message: &[u8]) -> Option<Vec<u8>> {
    ops::ic::signature::get(domain, seed, message)
}

pub fn sign(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<Option<Vec<u8>>, PublicError> {
    ops::ic::signature::sign(domain, seed, message).map_err(PublicError::from)
}

pub fn verify(
    domain: &[u8],
    seed: &[u8],
    message: &[u8],
    signature_cbor: &[u8],
    issuer_pid: crate::cdk::types::Principal,
) -> Result<(), PublicError> {
    ops::ic::signature::verify(domain, seed, message, signature_cbor, issuer_pid)
        .map_err(PublicError::from)
}

#[must_use]
pub fn root_hash() -> Vec<u8> {
    ops::ic::signature::root_hash()
}
