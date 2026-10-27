//! Native Root renewal reporting; persisted issuer state and deadlines are unchanged.
use crate::dto::auth::{DelegatedRoleGrant, DelegationAudience, RootIssuerRenewalBatchView};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Native RootIssuerRenewalTemplateView boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootIssuerRenewalTemplateView {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub cert_ttl_ns: u64,
}

/// Native RootIssuerRenewalStatusRequest boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootIssuerRenewalStatusRequest {
    pub issuer_pid: Principal,
}

/// Native RootIssuerRenewalStateView boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootIssuerRenewalStateView {
    pub issuer_pid: Principal,
    pub template_fingerprint: [u8; 32],
    pub last_installed_cert_hash: Option<[u8; 32]>,
    pub last_installed_expires_at_ns: Option<u64>,
    pub last_installed_refresh_after_ns: Option<u64>,
    pub next_attempt_after_ns: u64,
    pub updated_at_ns: u64,
}

/// Native RootIssuerRenewalStatusResponse boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootIssuerRenewalStatusResponse {
    pub template: Option<RootIssuerRenewalTemplateView>,
    pub state: Option<RootIssuerRenewalStateView>,
    pub latest_batch: Option<RootIssuerRenewalBatchView>,
}
