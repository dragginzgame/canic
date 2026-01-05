use crate::{PublicError, ops::ic::signature::SignatureOps};

///
/// SignatureApi
/// wrappers over IC signature helpers for endpoints
///

pub struct SignatureApi;

impl SignatureApi {
    pub fn prepare(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<(), PublicError> {
        SignatureOps::prepare(domain, seed, message).map_err(PublicError::from)
    }

    #[must_use]
    pub fn get(domain: &[u8], seed: &[u8], message: &[u8]) -> Option<Vec<u8>> {
        SignatureOps::get(domain, seed, message)
    }

    pub fn sign(
        domain: &[u8],
        seed: &[u8],
        message: &[u8],
    ) -> Result<Option<Vec<u8>>, PublicError> {
        SignatureOps::sign(domain, seed, message).map_err(PublicError::from)
    }

    pub fn verify(
        domain: &[u8],
        seed: &[u8],
        message: &[u8],
        signature_cbor: &[u8],
        issuer_pid: crate::cdk::types::Principal,
    ) -> Result<(), PublicError> {
        SignatureOps::verify(domain, seed, message, signature_cbor, issuer_pid)
            .map_err(PublicError::from)
    }

    #[must_use]
    pub fn root_hash() -> Vec<u8> {
        SignatureOps::root_hash()
    }
}
