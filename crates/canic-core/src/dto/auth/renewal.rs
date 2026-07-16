//! Module: dto::auth::renewal
//!
//! Responsibility: root issuer policy and renewal status DTOs.
//! Does not own: scheduling, proof batch creation, or install result mutation.
//! Boundary: passive operator/root endpoint contracts for issuer renewal state.

use super::{DelegatedRoleGrant, DelegationAudience, DelegationProof};
use crate::dto::prelude::*;

//
// RootDelegationProofBatchProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationProofBatchProof {
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
    pub proof: DelegationProof,
}

//
// RootIssuerPolicyUpsertRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerPolicyUpsertRequest {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub allowed_audiences: Vec<DelegationAudience>,
    pub allowed_grants: Vec<DelegatedRoleGrant>,
    pub max_cert_ttl_ns: u64,
    pub refresh_after_ratio_bps: u16,
}

//
// RootIssuerPolicyView
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerPolicyView {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub allowed_audiences: Vec<DelegationAudience>,
    pub allowed_grants: Vec<DelegatedRoleGrant>,
    pub max_cert_ttl_ns: u64,
    pub refresh_after_ratio_bps: u16,
}

//
// RootIssuerPolicyResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerPolicyResponse {
    pub issuer: RootIssuerPolicyView,
}

//
// RootIssuerRenewalTemplateUpsertRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalTemplateUpsertRequest {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub cert_ttl_ns: u64,
}

//
// RootIssuerRenewalTemplateView
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalTemplateView {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub cert_ttl_ns: u64,
}

//
// RootIssuerRenewalTemplateResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalTemplateResponse {
    pub template: RootIssuerRenewalTemplateView,
}

//
// RootIssuerRenewalStatusRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalStatusRequest {
    pub issuer_pid: Principal,
}

//
// RootIssuerRenewalBatchStatus
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootIssuerRenewalBatchStatus {
    Prepared,
    Signing,
    Signed,
    Installing,
    Installed,
    FailedRetryable,
}

//
// RootIssuerRenewalBatchView
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalBatchView {
    pub batch_id: [u8; 32],
    pub status: RootIssuerRenewalBatchStatus,
    pub cert_hash: [u8; 32],
    pub proof_epoch: u64,
    pub prepared_at_ns: u64,
    pub expires_at_ns: u64,
    pub installed_at_ns: Option<u64>,
    pub retry_after_ns: Option<u64>,
    pub failure: Option<String>,
}

//
// RootIssuerRenewalStateView
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalStateView {
    pub issuer_pid: Principal,
    pub template_fingerprint: [u8; 32],
    pub last_installed_cert_hash: Option<[u8; 32]>,
    pub last_installed_expires_at_ns: Option<u64>,
    pub last_installed_refresh_after_ns: Option<u64>,
    pub next_attempt_after_ns: u64,
    pub updated_at_ns: u64,
}

//
// RootIssuerRenewalStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalStatusResponse {
    pub template: Option<RootIssuerRenewalTemplateView>,
    pub state: Option<RootIssuerRenewalStateView>,
    pub latest_batch: Option<RootIssuerRenewalBatchView>,
}
