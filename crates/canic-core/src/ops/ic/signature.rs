use crate::{Error, infra::ic::signature as sig_infra};
use candid::Principal;

/// Prepare a canister signature (update-only).
pub fn prepare(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<(), Error> {
    sig_infra::prepare(domain, seed, message).map_err(Error::from)
}

#[must_use]
pub fn get(domain: &[u8], seed: &[u8], message: &[u8]) -> Option<Vec<u8>> {
    sig_infra::get(domain, seed, message)
}

pub fn sign(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<Option<Vec<u8>>, Error> {
    sig_infra::sign(domain, seed, message).map_err(Error::from)
}

pub fn verify(
    domain: &[u8],
    seed: &[u8],
    message: &[u8],
    signature_cbor: &[u8],
    issuer_pid: Principal,
) -> Result<(), Error> {
    sig_infra::verify(domain, seed, message, signature_cbor, issuer_pid).map_err(Error::from)
}

#[must_use]
pub fn root_hash() -> Vec<u8> {
    sig_infra::root_hash()
}
