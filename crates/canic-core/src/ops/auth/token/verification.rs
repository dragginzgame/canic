//! Root delegation proof verification used by native qualification.
use super::*;
use crate::dto::auth::{DelegationCert, RootProof};
use crate::ops::auth::delegated::cert_rules::DelegatedAuthTtlLimits;
use crate::ops::auth::delegated::root_proof::{
    RootDelegationProofError, VerifyRootDelegationProofInput, verify_root_delegation_proof,
};

impl AuthOps {
    #[cfg(test)]
    pub(crate) fn verify_delegation_root_proof(
        cert: &DelegationCert,
        root_proof: &RootProof,
        verifier_cfg: &AuthProofVerifierConfig,
        now_ns: u64,
        ttl_limits: DelegatedAuthTtlLimits,
    ) -> Result<(), InternalError> {
        let root_pid = cert.root_pid;
        if root_pid != verifier_cfg.root_canister_id {
            return Err(InternalError::invalid_input());
        }

        let Some(chain_key_root) = verifier_cfg.chain_key_root.as_ref() else {
            return Err(InternalError::auth_material_stale());
        };
        verify_root_delegation_proof(VerifyRootDelegationProofInput {
            cert,
            root_proof,
            expected_issuer: cert.issuer_pid,
            verifier: chain_key_root,
            now_ns,
            ttl_limits,
        })
        .map(|_evidence| ())
        .map_err(map_root_delegation_proof_error)
    }
}

#[cfg(test)]
pub(super) fn map_root_delegation_proof_error(cause: RootDelegationProofError) -> InternalError {
    match cause {
        RootDelegationProofError::Verification(cause) => {
            crate::ops::auth::delegated::root_proof::map_verification_error(cause)
        }
        _ => InternalError::invalid_input(),
    }
}
