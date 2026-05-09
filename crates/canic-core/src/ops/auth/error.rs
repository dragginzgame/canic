use crate::{InternalError, InternalErrorOrigin, ids::CanisterRole, ops::prelude::*};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum AuthOpsError {
    #[error(transparent)]
    Validation(#[from] AuthValidationError),

    #[error(transparent)]
    Signature(#[from] AuthSignatureError),

    #[error(transparent)]
    Scope(#[from] AuthScopeError),

    #[error(transparent)]
    Expiry(#[from] AuthExpiryError),
}

#[derive(Debug, ThisError)]
pub enum AuthValidationError {
    #[error(
        "delegation cert expires_at ({expires_at}) must be greater than issued_at ({issued_at})"
    )]
    CertInvalidWindow { issued_at: u64, expires_at: u64 },

    #[error("delegation cert root pid mismatch (expected {expected}, found {found})")]
    InvalidRootAuthority {
        expected: Principal,
        found: Principal,
    },

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

    #[error("attestation subnet was set but verifier subnet is unavailable")]
    AttestationSubnetUnavailable,

    #[error("delegated token auth disabled (set auth.delegated_tokens.enabled=true in canic.toml)")]
    DelegatedTokenAuthDisabled,

    #[error("delegated token replay rejected; use a fresh token")]
    DelegatedTokenReplay,

    #[error("delegated token replay store capacity reached ({capacity})")]
    DelegatedTokenReplayStoreCapacityReached { capacity: usize },

    #[error("auth validation failed: {0}")]
    Auth(String),
}

#[derive(Debug, ThisError)]
pub enum AuthSignatureError {
    #[error("delegation cert signature unavailable")]
    CertSignatureUnavailable,

    #[error("delegation cert signature invalid: {0}")]
    CertSignatureInvalid(String),

    #[error("token signature unavailable")]
    TokenSignatureUnavailable,

    #[error("token signature invalid: {0}")]
    TokenSignatureInvalid(String),

    #[error("attestation signature unavailable")]
    AttestationSignatureUnavailable,

    #[error("attestation signature invalid: {0}")]
    AttestationSignatureInvalid(String),

    #[error("root public key unavailable for delegated-token verification")]
    RootPublicKeyUnavailable,

    #[error("shard public key unavailable for shard '{shard_pid}'")]
    ShardPublicKeyUnavailable { shard_pid: Principal },
}

#[derive(Debug, ThisError)]
pub enum AuthScopeError {
    #[error("audience principal '{aud}' not allowed by delegation")]
    AudienceNotAllowed { aud: Principal },

    #[error("audience role '{role}' not allowed by delegation")]
    AudienceRoleNotAllowed { role: CanisterRole },

    #[error("wildcard verifier audience not allowed by role-scoped delegation")]
    AudienceAnyNotAllowed,

    #[error("token audience role list must not be empty")]
    AudienceRoleListEmpty,

    #[error("scope '{scope}' not allowed by delegation")]
    ScopeNotAllowed { scope: String },

    #[error("token shard pid mismatch (expected {expected}, found {found})")]
    ShardPidMismatch {
        expected: Principal,
        found: Principal,
    },

    #[error("token audience does not include local canister '{self_pid}'")]
    SelfAudienceMissing { self_pid: Principal },

    #[error("token audience does not include local canister role '{role}' for '{self_pid}'")]
    SelfRoleAudienceMissing {
        self_pid: Principal,
        role: CanisterRole,
    },

    #[error("local canister '{self_pid}' is not configured as a delegated auth verifier")]
    SelfVerifierUnavailable { self_pid: Principal },

    #[error("attestation subject mismatch (expected caller {expected}, found {found})")]
    AttestationSubjectMismatch {
        expected: Principal,
        found: Principal,
    },

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
}

#[derive(Debug, ThisError)]
pub enum AuthExpiryError {
    #[error("delegation cert expired at {expires_at}")]
    CertExpired { expires_at: u64 },

    #[error("token expired at {exp}")]
    TokenExpired { exp: u64 },

    #[error("token not yet valid (iat {iat})")]
    TokenNotYetValid { iat: u64 },

    #[error("token issued before delegation (iat {token_iat} < cert {cert_iat})")]
    TokenIssuedBeforeDelegation { token_iat: u64, cert_iat: u64 },

    #[error("token expires after delegation (exp {token_exp} > cert {cert_exp})")]
    TokenOutlivesDelegation { token_exp: u64, cert_exp: u64 },

    #[error("delegated token expiry precedes issued_at")]
    TokenExpiryBeforeIssued,

    #[error("delegated token ttl exceeds max {max_ttl_secs}s (ttl {ttl_secs}s)")]
    TokenTtlExceeded { ttl_secs: u64, max_ttl_secs: u64 },

    #[error("attestation expired at {expires_at} (now {now_secs})")]
    AttestationExpired { expires_at: u64, now_secs: u64 },

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

    #[error("attestation epoch {epoch} below minimum accepted epoch {min_accepted_epoch}")]
    AttestationEpochRejected { epoch: u64, min_accepted_epoch: u64 },
}

impl From<AuthOpsError> for InternalError {
    fn from(err: AuthOpsError) -> Self {
        Self::ops(InternalErrorOrigin::Ops, err.to_string())
    }
}

impl From<AuthValidationError> for InternalError {
    fn from(err: AuthValidationError) -> Self {
        AuthOpsError::from(err).into()
    }
}

impl From<AuthSignatureError> for InternalError {
    fn from(err: AuthSignatureError) -> Self {
        AuthOpsError::from(err).into()
    }
}

impl From<AuthScopeError> for InternalError {
    fn from(err: AuthScopeError) -> Self {
        AuthOpsError::from(err).into()
    }
}

impl From<AuthExpiryError> for InternalError {
    fn from(err: AuthExpiryError) -> Self {
        AuthOpsError::from(err).into()
    }
}
