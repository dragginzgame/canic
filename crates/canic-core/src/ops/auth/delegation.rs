use super::{DelegatedTokenOps, crypto, keys, verify};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{DelegationCert, DelegationProof},
    ops::{
        auth::DelegationValidationError,
        ic::{IcOps, ecdsa::EcdsaOps},
    },
};

impl DelegatedTokenOps {
    /// Sign a delegation cert in one step using threshold ECDSA.
    pub(crate) async fn sign_delegation_cert(
        cert: DelegationCert,
    ) -> Result<DelegationProof, InternalError> {
        let local = IcOps::canister_self();
        if cert.root_pid != local {
            return Err(DelegationValidationError::InvalidRootAuthority {
                expected: local,
                found: cert.root_pid,
            }
            .into());
        }

        let key_name = keys::delegated_tokens_key_name()?;
        keys::ensure_root_public_key_cached(&key_name, cert.root_pid).await?;
        let hash = crypto::cert_hash(&cert)?;
        let sig = EcdsaOps::sign_bytes(&key_name, keys::root_derivation_path(), hash).await?;

        Ok(DelegationProof {
            cert,
            cert_sig: sig,
        })
    }

    /// Cache root and shard public keys for a delegation certificate.
    ///
    /// Verification paths are intentionally local-only and do not call IC
    /// management APIs, so provisioning must prime this cache.
    pub async fn cache_public_keys_for_cert(cert: &DelegationCert) -> Result<(), InternalError> {
        let key_name = keys::delegated_tokens_key_name()?;
        keys::ensure_root_public_key_cached(&key_name, cert.root_pid).await?;
        keys::ensure_shard_public_key_cached(&key_name, cert.shard_pid).await?;
        Ok(())
    }

    /// Cache only the shard public key for a delegation certificate.
    ///
    /// Root-controlled issuance already primes the root public key cache during
    /// signing, so the root verifier path only needs the shard key here.
    pub(crate) async fn cache_shard_public_key_for_cert(
        cert: &DelegationCert,
    ) -> Result<(), InternalError> {
        let key_name = keys::delegated_tokens_key_name()?;
        keys::ensure_shard_public_key_cached(&key_name, cert.shard_pid).await?;
        Ok(())
    }

    /// Structural verification for a delegation proof.
    pub(super) fn verify_delegation_structure(
        proof: &DelegationProof,
        expected_root: Option<Principal>,
    ) -> Result<(), InternalError> {
        if proof.cert.expires_at <= proof.cert.issued_at {
            return Err(DelegationValidationError::CertInvalidWindow {
                issued_at: proof.cert.issued_at,
                expires_at: proof.cert.expires_at,
            }
            .into());
        }

        if let Some(expected) = expected_root
            && proof.cert.root_pid != expected
        {
            return Err(DelegationValidationError::InvalidRootAuthority {
                expected,
                found: proof.cert.root_pid,
            }
            .into());
        }

        Ok(())
    }

    /// Cryptographic verification for a delegation proof.
    pub(super) fn verify_delegation_signature(
        proof: &DelegationProof,
    ) -> Result<(), InternalError> {
        verify::verify_delegation_signature(proof)
    }

    /// Full delegation proof verification (structure + signature).
    pub fn verify_delegation_proof(
        proof: &DelegationProof,
        authority_pid: Principal,
    ) -> Result<(), InternalError> {
        Self::verify_delegation_structure(proof, Some(authority_pid))?;
        Self::verify_delegation_signature(proof)?;
        Ok(())
    }
}
