//! Module: model::auth::root_issuer
//!
//! Responsibility: define root-owned issuer policy and renewal state.
//! Does not own: admission decisions, DTO conversion, or persisted record layout.

use crate::{cdk::types::Principal, ids::CanisterRole};

/// Audience admitted by one root issuer policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootDelegationAudiencePolicy {
    Canister(Principal),
    CanicSubnet(Principal),
    Project(String),
}

/// Role and scopes admitted by one root issuer policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootDelegatedRoleGrantPolicy {
    pub target: CanisterRole,
    pub scopes: Vec<String>,
}

/// Authoritative root issuer policy state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootIssuerPolicy {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub allowed_audiences: Vec<RootDelegationAudiencePolicy>,
    pub allowed_grants: Vec<RootDelegatedRoleGrantPolicy>,
    pub max_cert_ttl_ns: u64,
    pub refresh_after_ratio_bps: u16,
}

/// Root-managed desired renewal state for one delegated-token issuer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootIssuerRenewalTemplate {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub audience: RootDelegationAudiencePolicy,
    pub grants: Vec<RootDelegatedRoleGrantPolicy>,
    pub cert_ttl_ns: u64,
}

/// Last root-managed renewal outcome for one delegated-token issuer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootIssuerRenewalOutcome {
    AlreadyInstalled,
    DriftDetected,
    InstallDeadlineExpired,
    Installed,
    IssuerCallFailed,
    NeverRun,
    PolicyRejected,
    ProofMismatch,
    QuotaExceeded,
    RejectedByIssuer,
    RetrievalExpired,
    TemplateChanged,
    TemplateDisabled,
}

/// Root-owned scheduling state for one delegated-token issuer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootIssuerRenewalState {
    pub issuer_pid: Principal,
    pub template_fingerprint: [u8; 32],
    pub last_installed_cert_hash: Option<[u8; 32]>,
    pub last_installed_expires_at_ns: Option<u64>,
    pub last_installed_refresh_after_ns: Option<u64>,
    pub active_attempt_id: Option<[u8; 32]>,
    pub last_outcome: RootIssuerRenewalOutcome,
    pub consecutive_failures: u32,
    pub next_attempt_after_ns: u64,
    pub updated_at_ns: u64,
}

/// Root-owned pointer to one prepared renewal proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootIssuerRenewalProofRef {
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
}

/// Per-issuer lifecycle state for one scheduled renewal attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootIssuerRenewalAttemptStatus {
    Prepared,
    Installing,
    Installed,
    FailedRetryable,
    FailedTerminal,
    Disabled,
    Expired,
}

/// Root-owned issuer-level scheduled renewal attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootIssuerRenewalAttempt {
    pub attempt_id: [u8; 32],
    pub issuer_pid: Principal,
    pub template_fingerprint: [u8; 32],
    pub batch_id: [u8; 32],
    pub proof_ref: RootIssuerRenewalProofRef,
    pub status: RootIssuerRenewalAttemptStatus,
    pub prepared_at_ns: u64,
    pub retrieval_expires_at_ns: u64,
    pub install_deadline_ns: u64,
    pub prepared_cert_hash: [u8; 32],
    pub prepared_expires_at_ns: u64,
    pub prepared_refresh_after_ns: u64,
    pub failure: Option<RootIssuerRenewalOutcome>,
}
