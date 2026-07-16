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

/// Root-owned scheduling state for one delegated-token issuer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootIssuerRenewalState {
    pub issuer_pid: Principal,
    pub template_fingerprint: [u8; 32],
    pub last_installed_cert_hash: Option<[u8; 32]>,
    pub last_installed_expires_at_ns: Option<u64>,
    pub last_installed_refresh_after_ns: Option<u64>,
    pub next_attempt_after_ns: u64,
    pub updated_at_ns: u64,
}
