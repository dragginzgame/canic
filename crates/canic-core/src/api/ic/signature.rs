use crate::{Error, workflow::ic::signature::SignatureWorkflow};

///
/// SignatureApi
/// wrappers over IC signature helpers for endpoints
///

pub struct SignatureApi;

impl SignatureApi {
    pub fn prepare(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<(), Error> {
        SignatureWorkflow::prepare(domain, seed, message).map_err(Error::from)
    }

    #[must_use]
    pub fn get(domain: &[u8], seed: &[u8], message: &[u8]) -> Option<Vec<u8>> {
        SignatureWorkflow::get(domain, seed, message)
    }

    pub fn sign(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        SignatureWorkflow::sign(domain, seed, message).map_err(Error::from)
    }

    pub fn verify(
        domain: &[u8],
        seed: &[u8],
        message: &[u8],
        signature_cbor: &[u8],
        issuer_pid: crate::cdk::types::Principal,
    ) -> Result<(), Error> {
        SignatureWorkflow::verify(domain, seed, message, signature_cbor, issuer_pid)
            .map_err(Error::from)
    }

    #[must_use]
    pub fn root_hash() -> Vec<u8> {
        SignatureWorkflow::root_hash()
    }
}
