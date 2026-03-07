use crate::{
    InternalError, InternalErrorOrigin,
    dto::auth::{
        AttestationKey, AttestationKeySet, DelegatedToken, DelegatedTokenClaims, DelegationCert,
        DelegationProof, RoleAttestation, SignedRoleAttestation,
    },
    ops::{
        config::ConfigOps,
        ic::{IcOps, ecdsa::EcdsaOps},
        prelude::*,
        storage::auth::DelegationStateOps,
    },
};
use thiserror::Error as ThisError;

mod crypto;
mod keys;
mod verify;

const DERIVATION_NAMESPACE: &[u8] = b"canic";
const ROOT_PATH_SEGMENT: &[u8] = b"root";
const SHARD_PATH_SEGMENT: &[u8] = b"shard";
const ATTESTATION_PATH_SEGMENT: &[u8] = b"attestation";
const CERT_SIGNING_DOMAIN: &[u8] = b"CANIC_DELEGATION_CERT_V1";
const TOKEN_SIGNING_DOMAIN: &[u8] = b"CANIC_DELEGATED_TOKEN_V1";
const ROLE_ATTESTATION_SIGNING_DOMAIN: &[u8] = b"CANIC_ROLE_ATTESTATION_V1";
const ROLE_ATTESTATION_KEY_ID_V1: u32 = 1;

///
/// DelegatedTokenOpsError
///

#[derive(Debug, ThisError)]
pub enum DelegatedTokenOpsError {
    #[error("audience principal '{aud}' not allowed by delegation")]
    AudienceNotAllowed { aud: Principal },

    #[error("delegation cert expired at {expires_at}")]
    CertExpired { expires_at: u64 },

    #[error(
        "delegation cert expires_at ({expires_at}) must be greater than issued_at ({issued_at})"
    )]
    CertInvalidWindow { issued_at: u64, expires_at: u64 },

    #[error("delegation cert root pid mismatch (expected {expected}, found {found})")]
    InvalidRootAuthority {
        expected: Principal,
        found: Principal,
    },

    #[error("delegation cert signature unavailable")]
    CertSignatureUnavailable,

    #[error("delegation cert signature invalid: {0}")]
    CertSignatureInvalid(String),

    #[error("candid encode failed for {context}: {source}")]
    EncodeFailed {
        context: &'static str,
        source: candid::Error,
    },

    #[error("ecdsa key name missing in configuration")]
    EcdsaKeyNameMissing,

    #[error("attestation signing key name missing in configuration")]
    AttestationKeyNameMissing,

    #[error("attestation key_id {key_id} not found in local key cache")]
    AttestationUnknownKeyId { key_id: u32 },

    #[error(
        "attestation key_id {key_id} is not valid yet (valid_from {valid_from}, now {now_secs})"
    )]
    AttestationKeyNotYetValid {
        key_id: u32,
        valid_from: u64,
        now_secs: u64,
    },

    #[error("attestation key_id {key_id} expired at {valid_until} (now {now_secs})")]
    AttestationKeyExpired {
        key_id: u32,
        valid_until: u64,
        now_secs: u64,
    },

    #[error("attestation signature unavailable")]
    AttestationSignatureUnavailable,

    #[error("attestation signature invalid: {0}")]
    AttestationSignatureInvalid(String),

    #[error("attestation subject mismatch (expected caller {expected}, found {found})")]
    AttestationSubjectMismatch {
        expected: Principal,
        found: Principal,
    },

    #[error("attestation expired at {expires_at} (now {now_secs})")]
    AttestationExpired { expires_at: u64, now_secs: u64 },

    #[error("attestation audience mismatch (expected {expected}, found {found})")]
    AttestationAudienceMismatch {
        expected: Principal,
        found: Principal,
    },

    #[error("attestation subnet mismatch (expected {expected}, found {found})")]
    AttestationSubnetMismatch {
        expected: Principal,
        found: Principal,
    },

    #[error("attestation subnet was set but verifier subnet is unavailable")]
    AttestationSubnetUnavailable,

    #[error("attestation epoch {epoch} below minimum accepted epoch {min_accepted_epoch}")]
    AttestationEpochRejected { epoch: u64, min_accepted_epoch: u64 },

    #[error("root public key unavailable for delegation verification")]
    RootPublicKeyUnavailable,

    #[error("shard public key unavailable for shard '{shard_pid}'")]
    ShardPublicKeyUnavailable { shard_pid: Principal },

    #[error("scope '{scope}' not allowed by delegation")]
    ScopeNotAllowed { scope: String },

    #[error("token shard pid mismatch (expected {expected}, found {found})")]
    ShardPidMismatch {
        expected: Principal,
        found: Principal,
    },

    #[error("token audience does not include local canister '{self_pid}'")]
    SelfAudienceMissing { self_pid: Principal },

    #[error("token expired at {exp}")]
    TokenExpired { exp: u64 },

    #[error("token signature unavailable")]
    TokenSignatureUnavailable,

    #[error("token signature invalid: {0}")]
    TokenSignatureInvalid(String),

    #[error("token not yet valid (iat {iat})")]
    TokenNotYetValid { iat: u64 },

    #[error("token issued before delegation (iat {token_iat} < cert {cert_iat})")]
    TokenIssuedBeforeDelegation { token_iat: u64, cert_iat: u64 },

    #[error("token expires after delegation (exp {token_exp} > cert {cert_exp})")]
    TokenOutlivesDelegation { token_exp: u64, cert_exp: u64 },

    #[error("delegated token auth disabled (set auth.delegated_tokens.enabled=true in canic.toml)")]
    DelegatedTokenAuthDisabled,

    #[error("delegation proof unavailable")]
    ProofUnavailable,

    #[error("delegation proof does not match current proof")]
    ProofMismatch,

    #[error("delegated token expiry precedes issued_at")]
    TokenExpiryBeforeIssued,

    #[error("delegated token ttl exceeds max {max_ttl_secs}s (ttl {ttl_secs}s)")]
    TokenTtlExceeded { ttl_secs: u64, max_ttl_secs: u64 },
}

impl From<DelegatedTokenOpsError> for InternalError {
    fn from(err: DelegatedTokenOpsError) -> Self {
        Self::ops(InternalErrorOrigin::Ops, err.to_string())
    }
}

///
/// VerifiedDelegatedToken
///

pub struct VerifiedDelegatedToken {
    pub claims: DelegatedTokenClaims,
    pub cert: DelegationCert,
}

///
/// DelegatedTokenOps
///

pub struct DelegatedTokenOps;

impl DelegatedTokenOps {
    // -------------------------------------------------------------------------
    // Delegation cert
    // -------------------------------------------------------------------------

    /// Sign a delegation cert in one step using threshold ECDSA.
    pub(crate) async fn sign_delegation_cert(
        cert: DelegationCert,
    ) -> Result<DelegationProof, InternalError> {
        let local = IcOps::canister_self();
        if cert.root_pid != local {
            return Err(DelegatedTokenOpsError::InvalidRootAuthority {
                expected: local,
                found: cert.root_pid,
            }
            .into());
        }

        let key_name = delegated_tokens_key_name()?;
        ensure_root_public_key_cached(&key_name, cert.root_pid).await?;
        let hash = cert_hash(&cert)?;
        let sig = EcdsaOps::sign_bytes(&key_name, root_derivation_path(), hash).await?;

        Ok(DelegationProof {
            cert,
            cert_sig: sig,
        })
    }

    /// Sign a role attestation payload using the attestation domain.
    pub(crate) async fn sign_role_attestation(
        payload: RoleAttestation,
    ) -> Result<SignedRoleAttestation, InternalError> {
        let key_name = attestation_key_name()?;
        ensure_attestation_key_cached(&key_name, IcOps::canister_self(), IcOps::now_secs()).await?;
        let hash = role_attestation_hash(&payload)?;
        let signature =
            EcdsaOps::sign_bytes(&key_name, attestation_derivation_path(), hash).await?;

        Ok(SignedRoleAttestation {
            payload,
            signature,
            key_id: ROLE_ATTESTATION_KEY_ID_V1,
        })
    }

    pub async fn attestation_key_set() -> Result<AttestationKeySet, InternalError> {
        let root_pid = IcOps::canister_self();
        let now_secs = IcOps::now_secs();
        let key_name = attestation_key_name()?;
        ensure_attestation_key_cached(&key_name, root_pid, now_secs).await?;

        Ok(AttestationKeySet {
            root_pid,
            generated_at: now_secs,
            keys: attestation_keys_sorted(),
        })
    }

    /// Cache root and shard public keys for a delegation certificate.
    ///
    /// Verification paths are intentionally local-only and do not call IC
    /// management APIs, so provisioning must prime this cache.
    pub async fn cache_public_keys_for_cert(cert: &DelegationCert) -> Result<(), InternalError> {
        let key_name = delegated_tokens_key_name()?;
        ensure_root_public_key_cached(&key_name, cert.root_pid).await?;
        ensure_shard_public_key_cached(&key_name, cert.shard_pid).await?;
        Ok(())
    }

    /// Structural verification for a delegation proof.
    fn verify_delegation_structure(
        proof: &DelegationProof,
        expected_root: Option<Principal>,
    ) -> Result<(), InternalError> {
        if proof.cert.expires_at <= proof.cert.issued_at {
            return Err(DelegatedTokenOpsError::CertInvalidWindow {
                issued_at: proof.cert.issued_at,
                expires_at: proof.cert.expires_at,
            }
            .into());
        }

        if let Some(expected) = expected_root
            && proof.cert.root_pid != expected
        {
            return Err(DelegatedTokenOpsError::InvalidRootAuthority {
                expected,
                found: proof.cert.root_pid,
            }
            .into());
        }

        Ok(())
    }

    /// Cryptographic verification for a delegation proof.
    fn verify_delegation_signature(proof: &DelegationProof) -> Result<(), InternalError> {
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

    // -------------------------------------------------------------------------
    // Token signing
    // -------------------------------------------------------------------------

    pub async fn sign_token(
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, InternalError> {
        validate_claims_against_cert(&claims, &proof.cert)?;

        let local = IcOps::canister_self();
        if claims.shard_pid != local {
            return Err(DelegatedTokenOpsError::ShardPidMismatch {
                expected: local,
                found: claims.shard_pid,
            }
            .into());
        }

        let key_name = delegated_tokens_key_name()?;
        ensure_shard_public_key_cached(&key_name, claims.shard_pid).await?;
        let token_hash = token_signing_hash(&claims, &proof.cert)?;
        let token_sig = EcdsaOps::sign_bytes(
            &key_name,
            shard_derivation_path(claims.shard_pid),
            token_hash,
        )
        .await?;

        Ok(DelegatedToken {
            claims,
            proof,
            token_sig,
        })
    }

    // -------------------------------------------------------------------------
    // Token verification
    // -------------------------------------------------------------------------

    pub fn verify_token(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
        self_pid: Principal,
    ) -> Result<VerifiedDelegatedToken, InternalError> {
        let cfg = ConfigOps::delegated_tokens_config()?;
        if !cfg.enabled {
            return Err(DelegatedTokenOpsError::DelegatedTokenAuthDisabled.into());
        }

        Self::verify_token_structure(token, authority_pid, now_secs, self_pid)?;
        if let Some(max_ttl_secs) = cfg.max_ttl_secs {
            verify_max_ttl(token, max_ttl_secs)?;
        }

        verify_current_proof(&token.proof)?;
        Self::verify_token_signature(token)?;

        Ok(VerifiedDelegatedToken {
            claims: token.claims.clone(),
            cert: token.proof.cert.clone(),
        })
    }

    fn verify_token_structure(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
        self_pid: Principal,
    ) -> Result<(), InternalError> {
        Self::verify_delegation_structure(&token.proof, Some(authority_pid))?;
        verify_time_bounds(&token.claims, &token.proof.cert, now_secs)?;
        validate_claims_against_cert(&token.claims, &token.proof.cert)?;
        verify_self_audience(&token.claims, self_pid)?;

        Ok(())
    }

    fn verify_token_signature(token: &DelegatedToken) -> Result<(), InternalError> {
        Self::verify_delegation_signature(&token.proof)?;
        verify_token_sig(token)?;
        Ok(())
    }

    pub fn replace_attestation_key_set(key_set: AttestationKeySet) {
        DelegationStateOps::set_attestation_key_set(key_set);
    }

    pub(crate) fn verify_role_attestation_cached(
        attestation: &SignedRoleAttestation,
        caller: Principal,
        self_pid: Principal,
        verifier_subnet: Option<Principal>,
        now_secs: u64,
        min_accepted_epoch: u64,
    ) -> Result<RoleAttestation, DelegatedTokenOpsError> {
        if attestation.signature.is_empty() {
            return Err(DelegatedTokenOpsError::AttestationSignatureUnavailable);
        }

        let key = DelegationStateOps::attestation_public_key(attestation.key_id).ok_or(
            DelegatedTokenOpsError::AttestationUnknownKeyId {
                key_id: attestation.key_id,
            },
        )?;
        verify_attestation_key_validity(&key, now_secs)?;

        let public_key = key.public_key;
        let hash = role_attestation_hash(&attestation.payload)
            .map_err(|err| DelegatedTokenOpsError::AttestationSignatureInvalid(err.to_string()))?;
        EcdsaOps::verify_signature(&public_key, hash, &attestation.signature)
            .map_err(|err| DelegatedTokenOpsError::AttestationSignatureInvalid(err.to_string()))?;

        verify_role_attestation_claims(
            &attestation.payload,
            caller,
            self_pid,
            verifier_subnet,
            now_secs,
            min_accepted_epoch,
        )?;

        Ok(attestation.payload.clone())
    }
}

const fn verify_attestation_key_validity(
    key: &AttestationKey,
    now_secs: u64,
) -> Result<(), DelegatedTokenOpsError> {
    verify::verify_attestation_key_validity(key, now_secs)
}

fn cert_hash(cert: &DelegationCert) -> Result<[u8; 32], InternalError> {
    crypto::cert_hash(cert)
}

fn token_signing_hash(
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
) -> Result<[u8; 32], InternalError> {
    crypto::token_signing_hash(claims, cert)
}

fn verify_token_sig(token: &DelegatedToken) -> Result<(), InternalError> {
    verify::verify_token_sig(token)
}

fn verify_time_bounds(
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
    now_secs: u64,
) -> Result<(), InternalError> {
    verify::verify_time_bounds(claims, cert, now_secs)
}

fn verify_current_proof(proof: &DelegationProof) -> Result<(), InternalError> {
    verify::verify_current_proof(proof)
}

fn verify_max_ttl(token: &DelegatedToken, max_ttl_secs: u64) -> Result<(), InternalError> {
    verify::verify_max_ttl(token, max_ttl_secs)
}

fn verify_self_audience(
    claims: &DelegatedTokenClaims,
    self_pid: Principal,
) -> Result<(), InternalError> {
    verify::verify_self_audience(claims, self_pid)
}

fn validate_claims_against_cert(
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
) -> Result<(), InternalError> {
    verify::validate_claims_against_cert(claims, cert)
}

fn verify_role_attestation_claims(
    payload: &RoleAttestation,
    caller: Principal,
    self_pid: Principal,
    verifier_subnet: Option<Principal>,
    now_secs: u64,
    min_accepted_epoch: u64,
) -> Result<(), DelegatedTokenOpsError> {
    verify::verify_role_attestation_claims(
        payload,
        caller,
        self_pid,
        verifier_subnet,
        now_secs,
        min_accepted_epoch,
    )
}

fn attestation_keys_sorted() -> Vec<AttestationKey> {
    keys::attestation_keys_sorted()
}

fn delegated_tokens_key_name() -> Result<String, InternalError> {
    keys::delegated_tokens_key_name()
}

fn attestation_key_name() -> Result<String, InternalError> {
    keys::attestation_key_name()
}

fn root_derivation_path() -> Vec<Vec<u8>> {
    keys::root_derivation_path()
}

fn shard_derivation_path(shard_pid: Principal) -> Vec<Vec<u8>> {
    keys::shard_derivation_path(shard_pid)
}

fn attestation_derivation_path() -> Vec<Vec<u8>> {
    keys::attestation_derivation_path()
}

fn role_attestation_hash(attestation: &RoleAttestation) -> Result<[u8; 32], InternalError> {
    crypto::role_attestation_hash(attestation)
}

async fn ensure_attestation_key_cached(
    key_name: &str,
    root_pid: Principal,
    now_secs: u64,
) -> Result<(), InternalError> {
    keys::ensure_attestation_key_cached(key_name, root_pid, now_secs).await
}

async fn ensure_root_public_key_cached(
    key_name: &str,
    root_pid: Principal,
) -> Result<(), InternalError> {
    keys::ensure_root_public_key_cached(key_name, root_pid).await
}

async fn ensure_shard_public_key_cached(
    key_name: &str,
    shard_pid: Principal,
) -> Result<(), InternalError> {
    keys::ensure_shard_public_key_cached(key_name, shard_pid).await
}

#[cfg(test)]
mod tests;
