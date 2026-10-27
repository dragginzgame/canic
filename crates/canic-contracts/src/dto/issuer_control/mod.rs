//! Native Root issuer administration, distinct from portable session admission.
use crate::dto::{
    auth::{DelegatedRoleGrant, DelegationAudience},
    issuer_proof::DelegationProof,
    issuer_renewal::RootIssuerRenewalTemplateView,
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Native RootIssuerConfigureRequest boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootIssuerConfigureRequest {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub cert_ttl_ns: u64,
    pub refresh_after_ratio_bps: u16,
}

/// Native RootIssuerPolicyView boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootIssuerPolicyView {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub allowed_audiences: Vec<DelegationAudience>,
    pub allowed_grants: Vec<DelegatedRoleGrant>,
    pub max_cert_ttl_ns: u64,
    pub refresh_after_ratio_bps: u16,
}

/// Native RootIssuerConfigureResponse boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootIssuerConfigureResponse {
    pub issuer: RootIssuerPolicyView,
    pub template: RootIssuerRenewalTemplateView,
}

/// Native RootDelegationProofBatchProof boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootDelegationProofBatchProof {
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
    pub proof: DelegationProof,
}
