use crate::{PublicError, ops::ic::signature as sig_ops};

///
/// Signature
/// wrappers over IC signature helpers for endpoints
///

pub fn prepare(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<(), PublicError> {
    sig_ops::prepare(domain, seed, message).map_err(PublicError::from)
}

#[must_use]
pub fn get(domain: &[u8], seed: &[u8], message: &[u8]) -> Option<Vec<u8>> {
    sig_ops::get(domain, seed, message)
}

pub fn sign(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<Option<Vec<u8>>, PublicError> {
    sig_ops::sign(domain, seed, message).map_err(PublicError::from)
}

pub fn verify(
    domain: &[u8],
    seed: &[u8],
    message: &[u8],
    signature_cbor: &[u8],
    issuer_pid: crate::cdk::types::Principal,
) -> Result<(), PublicError> {
    sig_ops::verify(domain, seed, message, signature_cbor, issuer_pid).map_err(PublicError::from)
}

#[must_use]
pub fn root_hash() -> Vec<u8> {
    sig_ops::root_hash()
}
