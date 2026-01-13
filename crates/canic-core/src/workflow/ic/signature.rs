use crate::{InternalError, ops::ic::signature::SignatureOps, workflow::prelude::*};

///
/// SignatureWorkflow
///

pub struct SignatureWorkflow;

impl SignatureWorkflow {
    /// Prepare a canister signature (update-only).
    pub fn prepare(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<(), InternalError> {
        SignatureOps::prepare(domain, seed, message)
    }

    #[must_use]
    pub fn get(domain: &[u8], seed: &[u8], message: &[u8]) -> Option<Vec<u8>> {
        SignatureOps::get(domain, seed, message)
    }

    pub fn sign(
        domain: &[u8],
        seed: &[u8],
        message: &[u8],
    ) -> Result<Option<Vec<u8>>, InternalError> {
        SignatureOps::sign(domain, seed, message)
    }

    pub fn verify(
        domain: &[u8],
        seed: &[u8],
        message: &[u8],
        signature_cbor: &[u8],
        issuer_pid: Principal,
    ) -> Result<(), InternalError> {
        SignatureOps::verify(domain, seed, message, signature_cbor, issuer_pid)
    }

    #[must_use]
    pub fn root_hash() -> Vec<u8> {
        SignatureOps::root_hash()
    }
}
