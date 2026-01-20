use crate::{
    InternalError,
    infra::ic::signature::SignatureInfra,
    ops::{ic::IcOpsError, prelude::*},
};

///
/// SignatureOps
///

pub struct SignatureOps;

impl SignatureOps {
    /// Prepare a canister signature (update-only).
    pub fn prepare(domain: &[u8], seed: &[u8], message: &[u8]) -> Result<(), InternalError> {
        SignatureInfra::prepare(domain, seed, message).map_err(IcOpsError::from)?;

        Ok(())
    }

    #[must_use]
    pub fn get(domain: &[u8], seed: &[u8], message: &[u8]) -> Option<Vec<u8>> {
        SignatureInfra::get(domain, seed, message)
    }

    pub fn sign(
        domain: &[u8],
        seed: &[u8],
        message: &[u8],
    ) -> Result<Option<Vec<u8>>, InternalError> {
        let signature = SignatureInfra::sign(domain, seed, message).map_err(IcOpsError::from)?;

        Ok(signature)
    }

    pub fn verify(
        domain: &[u8],
        seed: &[u8],
        message: &[u8],
        signature_cbor: &[u8],
        issuer_pid: Principal,
    ) -> Result<(), InternalError> {
        SignatureInfra::verify(domain, seed, message, signature_cbor, issuer_pid)
            .map_err(IcOpsError::from)?;

        Ok(())
    }

    #[must_use]
    pub fn root_hash() -> Vec<u8> {
        SignatureInfra::root_hash()
    }

    /// Resynchronize certified_data with the current signature map.
    pub fn sync_certified_data() {
        SignatureInfra::sync_certified_data();
    }
}
