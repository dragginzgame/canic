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

    #[cfg(test)]
    #[allow(dead_code)]
    #[must_use]
    pub fn root_hash() -> Vec<u8> {
        SignatureInfra::root_hash()
    }

    /// Resynchronize certified_data with the current signature map.
    pub fn sync_certified_data() {
        SignatureInfra::sync_certified_data();
    }
}
