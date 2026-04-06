use super::{DelegatedTokenOps, SignedDelegationProof, crypto, keys, verify};
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
    ) -> Result<SignedDelegationProof, InternalError> {
        let local = IcOps::canister_self();
        if cert.root_pid != local {
            return Err(DelegationValidationError::InvalidRootAuthority {
                expected: local,
                found: cert.root_pid,
            }
            .into());
        }

        let key_name = keys::delegated_tokens_key_name()?;
        let hash = crypto::cert_hash(&cert);
        crate::perf!("hash_cert");
        let sig = EcdsaOps::sign_bytes(&key_name, keys::root_derivation_path(), hash).await?;
        crate::perf!("sign_cert");

        Ok(SignedDelegationProof {
            proof: DelegationProof {
                cert,
                cert_sig: sig,
            },
            cert_hash: hash,
        })
    }

    /// Resolve the local shard public key, fetching and caching it on demand.
    pub(crate) async fn local_shard_public_key_sec1(
        shard_pid: Principal,
    ) -> Result<Vec<u8>, InternalError> {
        if let Some(shard_public_key) =
            crate::ops::storage::auth::DelegationStateOps::shard_public_key(shard_pid)
        {
            return Ok(shard_public_key);
        }

        let key_name = keys::delegated_tokens_key_name()?;
        let shard_public_key =
            EcdsaOps::public_key_sec1(&key_name, keys::shard_derivation_path(shard_pid), shard_pid)
                .await?;
        crate::ops::storage::auth::DelegationStateOps::set_shard_public_key(
            shard_pid,
            shard_public_key.clone(),
        );

        Ok(shard_public_key)
    }

    /// Cache root and shard public keys for a delegation certificate.
    ///
    /// Verification paths are intentionally local-only and do not call IC
    /// management APIs, so provisioning must prime this cache.
    pub async fn cache_public_keys_for_cert(cert: &DelegationCert) -> Result<(), InternalError> {
        Self::cache_public_keys_for_cert_with_optional_shard(cert, None).await
    }

    /// Cache root and shard public keys, trusting caller-provided shard key material when present.
    pub async fn cache_public_keys_for_cert_with_optional_shard(
        cert: &DelegationCert,
        shard_public_key: Option<Vec<u8>>,
    ) -> Result<(), InternalError> {
        let key_name = keys::delegated_tokens_key_name()?;
        keys::ensure_root_public_key_cached(&key_name, cert.root_pid).await?;

        if let Some(shard_public_key) = shard_public_key {
            crate::ops::storage::auth::DelegationStateOps::set_shard_public_key(
                cert.shard_pid,
                shard_public_key,
            );
        } else {
            keys::ensure_shard_public_key_cached(&key_name, cert.shard_pid).await?;
        }
        Ok(())
    }

    /// Fetch the shard public key only when it is missing from local verifier state.
    pub(crate) async fn fetch_missing_shard_public_key_for_cert(
        cert: &DelegationCert,
    ) -> Result<Option<Vec<u8>>, InternalError> {
        let key_name = keys::delegated_tokens_key_name()?;
        keys::fetch_missing_shard_public_key(&key_name, cert.shard_pid).await
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
