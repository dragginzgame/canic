use crate::{
    InternalError, InternalErrorOrigin,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof},
    ops::{
        config::ConfigOps,
        ic::IcOps,
        ic::signature::SignatureOps,
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

pub const DELEGATION_CERT_DOMAIN: &[u8] = b"canic-delegation";
pub const DELEGATED_TOKEN_DOMAIN: &[u8] = b"canic-token";
pub const DELEGATION_CERT_SEED: &[u8] = b"delegation-cert-v1";
pub const DELEGATED_TOKEN_SEED: &[u8] = b"delegated-token-v1";

///
/// DelegatedTokenOpsError
///

#[derive(Debug, ThisError)]
pub enum DelegatedTokenOpsError {
    #[error("audience '{aud}' not allowed by delegation")]
    AudienceNotAllowed { aud: String },

    #[error("delegation cert expired at {expires_at}")]
    CertExpired { expires_at: u64 },

    #[error(
        "delegation cert expires_at ({expires_at}) must be greater than issued_at ({issued_at})"
    )]
    CertInvalidWindow { issued_at: u64, expires_at: u64 },

    #[error("delegation cert signature unavailable")]
    CertSignatureUnavailable,

    #[error("delegation cert signature invalid: {0}")]
    CertSignatureInvalid(String),

    #[error("candid encode failed for {context}: {source}")]
    EncodeFailed {
        context: &'static str,
        source: candid::Error,
    },

    #[error("scope '{scope}' not allowed by delegation")]
    ScopeNotAllowed { scope: String },

    #[error("signer pid mismatch (expected {expected}, found {found})")]
    SignerPidMismatch {
        expected: Principal,
        found: Principal,
    },

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

impl VerifiedDelegatedToken {
    pub(crate) fn dev_bypass() -> Self {
        // DEV ONLY: inert placeholder to mark local auth bypass, never a real authority.
        Self {
            claims: DelegatedTokenClaims {
                sub: Principal::anonymous(),
                aud: "CANIC_DEV_AUTH_BYPASS".to_string(),
                scopes: Vec::new(),
                iat: 0,
                exp: 0,
                ext: None,
                nonce: None,
            },
            cert: DelegationCert {
                v: 0,
                signer_pid: Principal::anonymous(),
                audiences: Vec::new(),
                scopes: Vec::new(),
                issued_at: 0,
                expires_at: 0,
            },
        }
    }
}

///
/// DelegatedTokenOps
///

pub struct DelegatedTokenOps;

impl DelegatedTokenOps {
    // -------------------------------------------------------------------------
    // Delegation cert
    // -------------------------------------------------------------------------

    pub(crate) fn prepare_delegation_cert_signature(
        cert: &DelegationCert,
    ) -> Result<(), InternalError> {
        let hash = cert_hash(cert)?;
        SignatureOps::prepare(DELEGATION_CERT_DOMAIN, DELEGATION_CERT_SEED, &hash)?;

        Ok(())
    }

    pub(crate) fn get_delegation_cert_signature(
        cert: DelegationCert,
    ) -> Result<DelegationProof, InternalError> {
        let hash = cert_hash(&cert)?;
        let sig = SignatureOps::get(DELEGATION_CERT_DOMAIN, DELEGATION_CERT_SEED, &hash)?;

        Ok(DelegationProof {
            cert,
            cert_sig: sig,
        })
    }

    /// Sign a delegation cert in one step.
    ///
    /// This helper exists for compatibility, but IC canister signatures require
    /// update-time preparation and query-time retrieval.
    // SAFETY: this function must only be called by provisioning or rotation workflows.
    pub(crate) fn sign_delegation_cert(
        cert: DelegationCert,
    ) -> Result<DelegationProof, InternalError> {
        Self::prepare_delegation_cert_signature(&cert)?;

        Self::get_delegation_cert_signature(cert)
    }

    /// Structural verification for a delegation proof.
    ///
    /// This phase is always testable and does not require certified data.
    fn verify_delegation_structure(
        proof: &DelegationProof,
        expected_signer: Option<Principal>,
    ) -> Result<(), InternalError> {
        if proof.cert.expires_at <= proof.cert.issued_at {
            return Err(DelegatedTokenOpsError::CertInvalidWindow {
                issued_at: proof.cert.issued_at,
                expires_at: proof.cert.expires_at,
            }
            .into());
        }

        if let Some(expected) = expected_signer
            && proof.cert.signer_pid != expected
        {
            return Err(DelegatedTokenOpsError::SignerPidMismatch {
                expected,
                found: proof.cert.signer_pid,
            }
            .into());
        }

        Ok(())
    }

    /// Cryptographic verification for a delegation proof.
    ///
    /// Purely local verification; does not read certified data or require a
    /// query context.
    fn verify_delegation_signature(
        proof: &DelegationProof,
        authority_pid: Principal,
    ) -> Result<(), InternalError> {
        if proof.cert_sig.is_empty() {
            return Err(DelegatedTokenOpsError::CertSignatureUnavailable.into());
        }

        let hash = cert_hash(&proof.cert)?;
        SignatureOps::verify(
            DELEGATION_CERT_DOMAIN,
            DELEGATION_CERT_SEED,
            &hash,
            &proof.cert_sig,
            authority_pid,
        )
        .map_err(|err| map_signature_error(err, SignatureKind::Delegation))?;

        Ok(())
    }

    /// Full delegation proof verification (structure + signature).
    ///
    /// Purely local verification; does not read certified data or require a
    /// query context.
    pub fn verify_delegation_proof(
        proof: &DelegationProof,
        authority_pid: Principal,
    ) -> Result<(), InternalError> {
        Self::verify_delegation_structure(proof, None)?;
        Self::verify_delegation_signature(proof, authority_pid)?;

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Token signing
    // -------------------------------------------------------------------------

    pub fn prepare_token_signature(
        token_version: u16,
        claims: &DelegatedTokenClaims,
        proof: &DelegationProof,
    ) -> Result<(), InternalError> {
        validate_claims_against_cert(claims, &proof.cert)?;

        let token_hash = token_signing_hash(token_version, claims, &proof.cert)?;
        SignatureOps::prepare(DELEGATED_TOKEN_DOMAIN, DELEGATED_TOKEN_SEED, &token_hash)?;

        Ok(())
    }

    pub fn get_token_signature(
        token_version: u16,
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, InternalError> {
        validate_claims_against_cert(&claims, &proof.cert)?;

        let token_hash = token_signing_hash(token_version, &claims, &proof.cert)?;
        let signature =
            SignatureOps::get(DELEGATED_TOKEN_DOMAIN, DELEGATED_TOKEN_SEED, &token_hash)?;

        Ok(DelegatedToken {
            v: token_version,
            claims,
            proof,
            token_sig: signature,
        })
    }

    pub fn sign_token(
        token_version: u16,
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, InternalError> {
        Self::prepare_token_signature(token_version, &claims, &proof)?;

        Self::get_token_signature(token_version, claims, proof)
    }

    // -------------------------------------------------------------------------
    // Token verification
    // -------------------------------------------------------------------------

    /// Full delegated token verification (structure + signature).
    ///
    /// Purely local verification; does not read certified data or require a
    /// query context.
    // Invariant: All delegated token validation (time, proof binding, config)
    // must flow through this method. No other layer may revalidate.
    pub fn verify_token(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<VerifiedDelegatedToken, InternalError> {
        let cfg = ConfigOps::delegated_tokens_config()?;
        if !cfg.enabled {
            return Err(DelegatedTokenOpsError::DelegatedTokenAuthDisabled.into());
        }

        Self::verify_token_structure(token, now_secs)?;
        if let Some(max_ttl_secs) = cfg.max_ttl_secs {
            verify_max_ttl(token, max_ttl_secs)?;
        }

        verify_current_proof(&token.proof)?;
        Self::verify_token_signature(token, authority_pid)?;

        Ok(VerifiedDelegatedToken {
            claims: token.claims.clone(),
            cert: token.proof.cert.clone(),
        })
    }

    /// Structural verification for a delegated token.
    ///
    /// This phase is always testable and does not require certified data.
    fn verify_token_structure(token: &DelegatedToken, now_secs: u64) -> Result<(), InternalError> {
        Self::verify_delegation_structure(&token.proof, None)?;
        verify_time_bounds(&token.claims, &token.proof.cert, now_secs)?;
        validate_claims_against_cert(&token.claims, &token.proof.cert)?;

        Ok(())
    }

    /// Cryptographic verification for a delegated token.
    ///
    /// Purely local verification; does not read certified data or require a
    /// query context.
    fn verify_token_signature(
        token: &DelegatedToken,
        authority_pid: Principal,
    ) -> Result<(), InternalError> {
        Self::verify_delegation_signature(&token.proof, authority_pid)?;
        verify_token_sig(token)?;

        Ok(())
    }
}

// -------------------------------------------------------------------------
// Internal helpers
// -------------------------------------------------------------------------

#[derive(CandidType, Serialize)]
struct TokenSigningPayload {
    v: u16,
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

fn cert_hash(cert: &DelegationCert) -> Result<Vec<u8>, InternalError> {
    let payload = encode_candid("delegation cert", cert)?;
    Ok(hash_bytes(&payload))
}

fn token_signing_hash(
    token_version: u16,
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
) -> Result<Vec<u8>, InternalError> {
    let hash = cert_hash(cert)?;
    let payload = TokenSigningPayload {
        v: token_version,
        cert_hash: hash,
        claims: claims.clone(),
    };

    let encoded = encode_candid("token signing payload", &payload)?;
    Ok(hash_bytes(&encoded))
}

fn hash_bytes(payload: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hasher.finalize().to_vec()
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

    let token_hash = token_signing_hash(token.v, &token.claims, &token.proof.cert)?;
    SignatureOps::verify(
        DELEGATED_TOKEN_DOMAIN,
        DELEGATED_TOKEN_SEED,
        &token_hash,
        &token.token_sig,
        token.proof.cert.signer_pid,
    )
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
            "delegation cert expired local={} signer={} now_secs={} expires_at={}",
            local,
            cert.signer_pid,
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
            "delegation proof missing local={} signer={}",
            local,
            proof.cert.signer_pid
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
            "delegation proof mismatch local={} signer={} stored_signer={}",
            local,
            proof.cert.signer_pid,
            stored.cert.signer_pid
        );
        Err(DelegatedTokenOpsError::ProofMismatch.into())
    }
}

fn proofs_equal(a: &DelegationProof, b: &DelegationProof) -> bool {
    let a_cert = &a.cert;
    let b_cert = &b.cert;

    a_cert.v == b_cert.v
        && a_cert.signer_pid == b_cert.signer_pid
        && a_cert.audiences == b_cert.audiences
        && a_cert.scopes == b_cert.scopes
        && a_cert.issued_at == b_cert.issued_at
        && a_cert.expires_at == b_cert.expires_at
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

fn validate_claims_against_cert(
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
) -> Result<(), InternalError> {
    if !cert.audiences.iter().any(|aud| aud == &claims.aud) {
        return Err(DelegatedTokenOpsError::AudienceNotAllowed {
            aud: claims.aud.clone(),
        }
        .into());
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
