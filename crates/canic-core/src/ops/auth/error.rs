use crate::{InternalError, InternalErrorOrigin, ops::prelude::*};
use thiserror::Error as ThisError;

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
