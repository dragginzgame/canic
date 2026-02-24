use crate::dto::{error::Error, prelude::*};

///
/// DelegationCert
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationCert {
    pub root_pid: Principal,
    pub shard_pid: Principal,
    pub issued_at: u64,
    pub expires_at: u64,
    pub scopes: Vec<String>,
    pub aud: Vec<Principal>,
}

///
/// DelegationProof
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationProof {
    pub cert: DelegationCert,
    pub cert_sig: Vec<u8>,
}

///
/// DelegatedTokenClaims
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegatedTokenClaims {
    pub sub: Principal,
    pub shard_pid: Principal,
    pub scopes: Vec<String>,
    pub aud: Vec<Principal>,
    pub iat: u64,
    pub exp: u64,
}

///
/// DelegatedToken
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegatedToken {
    pub claims: DelegatedTokenClaims,
    pub proof: DelegationProof,
    pub token_sig: Vec<u8>,
}

///
/// DelegationRequest
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationRequest {
    pub shard_pid: Principal,
    pub scopes: Vec<String>,
    pub aud: Vec<Principal>,
    pub ttl_secs: u64,
    pub verifier_targets: Vec<Principal>,
    pub include_root_verifier: bool,
}

// admin-only: not part of canonical delegation flow.
// used for controlled provisioning and tooling flows.
///
/// DelegationProvisionRequest
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationProvisionRequest {
    pub cert: DelegationCert,
    pub signer_targets: Vec<Principal>,
    pub verifier_targets: Vec<Principal>,
}

// admin-only: not part of canonical delegation flow.
// used for controlled provisioning and tooling flows.
///
/// DelegationProvisionResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationProvisionResponse {
    pub proof: DelegationProof,
    pub results: Vec<DelegationProvisionTargetResponse>,
}

///
/// DelegationProofStatus
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationProofStatus {
    pub shard_pid: Principal,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DelegationProvisionTargetKind {
    Signer,
    Verifier,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DelegationProvisionStatus {
    Ok,
    Failed,
}

///
/// DelegationProvisionTargetResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationProvisionTargetResponse {
    pub target: Principal,
    pub kind: DelegationProvisionTargetKind,
    pub status: DelegationProvisionStatus,
    pub error: Option<Error>,
}
