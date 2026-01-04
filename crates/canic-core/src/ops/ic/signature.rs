use crate::{Error, infra, ops::prelude::*};

/// Prepare a canister signature (update-only).
pub fn prepare(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<(), Error> {
    infra::ic::signature::prepare(domain, seed, message).map_err(Error::from)
}

#[must_use]
pub fn get(domain: &[u8], seed: &[u8], message: &[u8]) -> Option<Vec<u8>> {
    infra::ic::signature::get(domain, seed, message)
}

pub fn sign(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<Option<Vec<u8>>, Error> {
    infra::ic::signature::sign(domain, seed, message).map_err(Error::from)
}

pub fn verify(
    domain: &[u8],
    seed: &[u8],
    message: &[u8],
    signature_cbor: &[u8],
    issuer_pid: Principal,
) -> Result<(), Error> {
    infra::ic::signature::verify(domain, seed, message, signature_cbor, issuer_pid)
        .map_err(Error::from)
}

#[must_use]
pub fn root_hash() -> Vec<u8> {
    infra::ic::signature::root_hash()
}
