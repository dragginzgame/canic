use crate::{
    InternalError, InternalErrorOrigin,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof},
    ops::{
        config::ConfigOps,
        ic::{IcOps, ecdsa::EcdsaOps},
        prelude::*,
        runtime::metrics::auth::{
            record_verifier_cert_expired, record_verifier_proof_mismatch,
            record_verifier_proof_missing,
        },
        storage::auth::DelegationStateOps,
    },
};
use candid::encode_one;
use sha2::{Digest, Sha256};
use thiserror::Error as ThisError;

const DERIVATION_NAMESPACE: &[u8] = b"canic";
const ROOT_PATH_SEGMENT: &[u8] = b"root";
const SHARD_PATH_SEGMENT: &[u8] = b"shard";
const CERT_SIGNING_DOMAIN: &[u8] = b"CANIC_DELEGATION_CERT_V1";
const TOKEN_SIGNING_DOMAIN: &[u8] = b"CANIC_DELEGATED_TOKEN_V1";

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
        if proof.cert_sig.is_empty() {
            return Err(DelegatedTokenOpsError::CertSignatureUnavailable.into());
        }

        let root_public_key = DelegationStateOps::root_public_key()
            .ok_or(DelegatedTokenOpsError::RootPublicKeyUnavailable)?;
        let hash = cert_hash(&proof.cert)?;
        EcdsaOps::verify_signature(&root_public_key, hash, &proof.cert_sig)
            .map_err(|err| map_signature_error(err, SignatureKind::Delegation))?;

        Ok(())
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
}

// -------------------------------------------------------------------------
// Internal helpers
// -------------------------------------------------------------------------

#[derive(CandidType, Serialize)]
struct TokenSigningPayload {
    cert_hash: Vec<u8>,
    claims: DelegatedTokenClaims,
}

fn encode_candid<T: CandidType>(
    context: &'static str,
    value: &T,
) -> Result<Vec<u8>, InternalError> {
    encode_one(value).map_err(|err| {
        DelegatedTokenOpsError::EncodeFailed {
            context,
            source: err,
        }
        .into()
    })
}

fn cert_hash(cert: &DelegationCert) -> Result<[u8; 32], InternalError> {
    let payload = encode_candid("delegation cert", cert)?;
    Ok(hash_domain_separated(CERT_SIGNING_DOMAIN, &payload))
}

fn token_signing_hash(
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
) -> Result<[u8; 32], InternalError> {
    let payload = TokenSigningPayload {
        cert_hash: cert_hash(cert)?.to_vec(),
        claims: claims.clone(),
    };

    let encoded = encode_candid("token signing payload", &payload)?;
    Ok(hash_domain_separated(TOKEN_SIGNING_DOMAIN, &encoded))
}

fn hash_domain_separated(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((domain.len() as u64).to_be_bytes());
    hasher.update(domain);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    hasher.finalize().into()
}

enum SignatureKind {
    Delegation,
    Token,
}

fn map_signature_error(err: InternalError, kind: SignatureKind) -> InternalError {
    match kind {
        SignatureKind::Delegation => {
            DelegatedTokenOpsError::CertSignatureInvalid(err.to_string()).into()
        }
        SignatureKind::Token => {
            DelegatedTokenOpsError::TokenSignatureInvalid(err.to_string()).into()
        }
    }
}

fn verify_token_sig(token: &DelegatedToken) -> Result<(), InternalError> {
    if token.token_sig.is_empty() {
        return Err(DelegatedTokenOpsError::TokenSignatureUnavailable.into());
    }

    let shard_public_key = DelegationStateOps::shard_public_key(token.proof.cert.shard_pid).ok_or(
        DelegatedTokenOpsError::ShardPublicKeyUnavailable {
            shard_pid: token.proof.cert.shard_pid,
        },
    )?;

    let token_hash = token_signing_hash(&token.claims, &token.proof.cert)?;
    EcdsaOps::verify_signature(&shard_public_key, token_hash, &token.token_sig)
        .map_err(|err| map_signature_error(err, SignatureKind::Token))?;

    Ok(())
}

fn verify_time_bounds(
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
    now_secs: u64,
) -> Result<(), InternalError> {
    if claims.exp < claims.iat {
        return Err(DelegatedTokenOpsError::TokenExpiryBeforeIssued.into());
    }

    if now_secs < claims.iat {
        return Err(DelegatedTokenOpsError::TokenNotYetValid { iat: claims.iat }.into());
    }

    if now_secs > claims.exp {
        return Err(DelegatedTokenOpsError::TokenExpired { exp: claims.exp }.into());
    }

    if now_secs > cert.expires_at {
        record_verifier_cert_expired();
        let local = IcOps::canister_self();
        crate::log!(
            crate::log::Topic::Auth,
            Warn,
            "delegation cert expired local={} shard={} now_secs={} expires_at={}",
            local,
            cert.shard_pid,
            now_secs,
            cert.expires_at
        );
        return Err(DelegatedTokenOpsError::CertExpired {
            expires_at: cert.expires_at,
        }
        .into());
    }

    if claims.iat < cert.issued_at {
        return Err(DelegatedTokenOpsError::TokenIssuedBeforeDelegation {
            token_iat: claims.iat,
            cert_iat: cert.issued_at,
        }
        .into());
    }

    if claims.exp > cert.expires_at {
        return Err(DelegatedTokenOpsError::TokenOutlivesDelegation {
            token_exp: claims.exp,
            cert_exp: cert.expires_at,
        }
        .into());
    }

    Ok(())
}

fn verify_current_proof(proof: &DelegationProof) -> Result<(), InternalError> {
    let Some(stored) = DelegationStateOps::proof_dto() else {
        record_verifier_proof_missing();
        let local = IcOps::canister_self();
        crate::log!(
            crate::log::Topic::Auth,
            Warn,
            "delegation proof missing local={} shard={}",
            local,
            proof.cert.shard_pid
        );
        return Err(DelegatedTokenOpsError::ProofUnavailable.into());
    };

    if proofs_equal(proof, &stored) {
        Ok(())
    } else {
        record_verifier_proof_mismatch();
        let local = IcOps::canister_self();
        crate::log!(
            crate::log::Topic::Auth,
            Warn,
            "delegation proof mismatch local={} shard={} stored_shard={}",
            local,
            proof.cert.shard_pid,
            stored.cert.shard_pid
        );
        Err(DelegatedTokenOpsError::ProofMismatch.into())
    }
}

fn proofs_equal(a: &DelegationProof, b: &DelegationProof) -> bool {
    let a_cert = &a.cert;
    let b_cert = &b.cert;

    a_cert.root_pid == b_cert.root_pid
        && a_cert.shard_pid == b_cert.shard_pid
        && a_cert.issued_at == b_cert.issued_at
        && a_cert.expires_at == b_cert.expires_at
        && a_cert.scopes == b_cert.scopes
        && a_cert.aud == b_cert.aud
        && a.cert_sig == b.cert_sig
}

fn verify_max_ttl(token: &DelegatedToken, max_ttl_secs: u64) -> Result<(), InternalError> {
    let ttl_secs = token.claims.exp - token.claims.iat;
    if ttl_secs > max_ttl_secs {
        return Err(DelegatedTokenOpsError::TokenTtlExceeded {
            ttl_secs,
            max_ttl_secs,
        }
        .into());
    }

    Ok(())
}

fn verify_self_audience(
    claims: &DelegatedTokenClaims,
    self_pid: Principal,
) -> Result<(), InternalError> {
    if claims.aud.contains(&self_pid) {
        Ok(())
    } else {
        Err(DelegatedTokenOpsError::SelfAudienceMissing { self_pid }.into())
    }
}

fn validate_claims_against_cert(
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
) -> Result<(), InternalError> {
    if claims.shard_pid != cert.shard_pid {
        return Err(DelegatedTokenOpsError::ShardPidMismatch {
            expected: cert.shard_pid,
            found: claims.shard_pid,
        }
        .into());
    }

    for aud in &claims.aud {
        if !cert.aud.iter().any(|allowed| allowed == aud) {
            return Err(DelegatedTokenOpsError::AudienceNotAllowed { aud: *aud }.into());
        }
    }

    for scope in &claims.scopes {
        if !cert.scopes.iter().any(|allowed| allowed == scope) {
            return Err(DelegatedTokenOpsError::ScopeNotAllowed {
                scope: scope.clone(),
            }
            .into());
        }
    }

    Ok(())
}

fn delegated_tokens_key_name() -> Result<String, InternalError> {
    let cfg = ConfigOps::delegated_tokens_config()?;
    if cfg.ecdsa_key_name.trim().is_empty() {
        return Err(DelegatedTokenOpsError::EcdsaKeyNameMissing.into());
    }

    Ok(cfg.ecdsa_key_name)
}

fn root_derivation_path() -> Vec<Vec<u8>> {
    vec![DERIVATION_NAMESPACE.to_vec(), ROOT_PATH_SEGMENT.to_vec()]
}

fn shard_derivation_path(shard_pid: Principal) -> Vec<Vec<u8>> {
    vec![
        DERIVATION_NAMESPACE.to_vec(),
        SHARD_PATH_SEGMENT.to_vec(),
        shard_pid.as_slice().to_vec(),
    ]
}

async fn ensure_root_public_key_cached(
    key_name: &str,
    root_pid: Principal,
) -> Result<(), InternalError> {
    if DelegationStateOps::root_public_key().is_some() {
        return Ok(());
    }

    let root_pk = EcdsaOps::public_key_sec1(key_name, root_derivation_path(), root_pid).await?;
    DelegationStateOps::set_root_public_key(root_pk);
    Ok(())
}

async fn ensure_shard_public_key_cached(
    key_name: &str,
    shard_pid: Principal,
) -> Result<(), InternalError> {
    if DelegationStateOps::shard_public_key(shard_pid).is_some() {
        return Ok(());
    }

    let shard_pk =
        EcdsaOps::public_key_sec1(key_name, shard_derivation_path(shard_pid), shard_pid).await?;
    DelegationStateOps::set_shard_public_key(shard_pid, shard_pk);
    Ok(())
}
