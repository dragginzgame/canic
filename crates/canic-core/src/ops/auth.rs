use crate::{
    InternalError, InternalErrorOrigin,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof},
    ops::{ic::signature::SignatureOps, prelude::*, runtime::env::EnvOps},
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

    #[error("certified query required")]
    UncertifiedRuntime,
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
    // Delegation cert issuance
    // -------------------------------------------------------------------------

    pub fn prepare_delegation_cert(cert: &DelegationCert) -> Result<(), InternalError> {
        let hash = cert_hash(cert)?;
        SignatureOps::prepare(DELEGATION_CERT_DOMAIN, DELEGATION_CERT_SEED, &hash)?;
        Ok(())
    }

    /// Retrieve a prepared delegation proof (query-only).
    ///
    /// Requires a certified query context; will fail under PocketIC or
    /// uncertified query engines.
    pub fn get_delegation_proof(cert: DelegationCert) -> Result<DelegationProof, InternalError> {
        let hash = cert_hash(&cert)?;
        let sig = SignatureOps::get(DELEGATION_CERT_DOMAIN, DELEGATION_CERT_SEED, &hash)
            .ok_or(DelegatedTokenOpsError::CertSignatureUnavailable)?;

        Ok(DelegationProof {
            cert,
            cert_sig: sig,
        })
    }

    /// Sign a delegation cert in one step.
    ///
    /// Requires a certified query context; will fail under PocketIC or
    /// uncertified query engines.
    pub fn sign_delegation_cert(cert: DelegationCert) -> Result<DelegationProof, InternalError> {
        let hash = cert_hash(&cert)?;
        let sig = SignatureOps::sign(DELEGATION_CERT_DOMAIN, DELEGATION_CERT_SEED, &hash)?
            .ok_or(DelegatedTokenOpsError::CertSignatureUnavailable)?;

        Ok(DelegationProof {
            cert,
            cert_sig: sig,
        })
    }

    /// Structural verification for a delegation proof.
    ///
    /// This phase is always testable and does not require certified data.
    pub fn verify_delegation_structure(
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
    /// Requires a certified query context; will fail under PocketIC or
    /// uncertified query engines.
    pub fn verify_delegation_signature(
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
    /// Signature verification requires a certified query context and will fail
    /// under PocketIC or uncertified query engines.
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

    pub fn sign_token(
        token_version: u16,
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, InternalError> {
        validate_claims_against_cert(&claims, &proof.cert)?;

        let token_hash = token_signing_hash(token_version, &claims, &proof.cert)?;
        let signature =
            SignatureOps::sign(DELEGATED_TOKEN_DOMAIN, DELEGATED_TOKEN_SEED, &token_hash)?
                .ok_or(DelegatedTokenOpsError::TokenSignatureUnavailable)?;

        Ok(DelegatedToken {
            v: token_version,
            claims,
            proof,
            token_sig: signature,
        })
    }

    // -------------------------------------------------------------------------
    // Token verification
    // -------------------------------------------------------------------------

    /// Full delegated token verification (structure + signature).
    ///
    /// Signature verification requires a certified query context and will fail
    /// under PocketIC or uncertified query engines.
    pub fn verify_token(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<VerifiedDelegatedToken, InternalError> {
        Self::verify_token_structure(token, now_secs)?;
        Self::verify_token_signature(token, authority_pid)?;

        Ok(VerifiedDelegatedToken {
            claims: token.claims.clone(),
            cert: token.proof.cert.clone(),
        })
    }

    /// Structural verification for a delegated token.
    ///
    /// This phase is always testable and does not require certified data.
    pub fn verify_token_structure(
        token: &DelegatedToken,
        now_secs: u64,
    ) -> Result<(), InternalError> {
        Self::verify_delegation_structure(&token.proof, None)?;
        verify_time_bounds(&token.claims, &token.proof.cert, now_secs)?;
        validate_claims_against_cert(&token.claims, &token.proof.cert)?;

        Ok(())
    }

    /// Cryptographic verification for a delegated token.
    ///
    /// Requires a certified query context; will fail under PocketIC or
    /// uncertified query engines.
    pub fn verify_token_signature(
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
    if EnvOps::is_uncertified_runtime() && is_certified_query_error(&err) {
        return DelegatedTokenOpsError::UncertifiedRuntime.into();
    }

    match kind {
        SignatureKind::Delegation => {
            DelegatedTokenOpsError::CertSignatureInvalid(err.to_string()).into()
        }
        SignatureKind::Token => {
            DelegatedTokenOpsError::TokenSignatureInvalid(err.to_string()).into()
        }
    }
}

fn is_certified_query_error(err: &InternalError) -> bool {
    let message = err.to_string();
    message.contains("certified_data")
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
    if now_secs < claims.iat {
        return Err(DelegatedTokenOpsError::TokenNotYetValid { iat: claims.iat }.into());
    }

    if now_secs > claims.exp {
        return Err(DelegatedTokenOpsError::TokenExpired { exp: claims.exp }.into());
    }

    if now_secs > cert.expires_at {
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
