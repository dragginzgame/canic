use crate::dto::{error::Error, prelude::*, rpc::RootRequestMetadata};

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
/// DelegationProofInstallIntent
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DelegationProofInstallIntent {
    Provisioning,
    Prewarm,
    Repair,
}

///
/// DelegationProofInstallRequest
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationProofInstallRequest {
    pub proof: DelegationProof,
    pub intent: DelegationProofInstallIntent,
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
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

///
/// RoleAttestationRequest
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct RoleAttestationRequest {
    pub subject: Principal,
    pub role: CanisterRole,
    #[serde(default)]
    pub subnet_id: Option<Principal>,
    #[serde(default)]
    pub audience: Option<Principal>,
    pub ttl_secs: u64,
    pub epoch: u64,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

///
/// RoleAttestation
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleAttestation {
    pub subject: Principal,
    pub role: CanisterRole,
    #[serde(default)]
    pub subnet_id: Option<Principal>,
    #[serde(default)]
    pub audience: Option<Principal>,
    pub issued_at: u64,
    pub expires_at: u64,
    pub epoch: u64,
}

///
/// SignedRoleAttestation
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SignedRoleAttestation {
    pub payload: RoleAttestation,
    pub signature: Vec<u8>,
    pub key_id: u32,
}

///
/// AttestationKeyStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AttestationKeyStatus {
    Current,
    Previous,
}

///
/// AttestationKey
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AttestationKey {
    pub key_id: u32,
    pub public_key: Vec<u8>,
    pub status: AttestationKeyStatus,
    #[serde(default)]
    pub valid_from: Option<u64>,
    #[serde(default)]
    pub valid_until: Option<u64>,
}

///
/// AttestationKeySet
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AttestationKeySet {
    pub root_pid: Principal,
    pub generated_at: u64,
    pub keys: Vec<AttestationKey>,
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
/// DelegationVerifierProofPushRequest
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationVerifierProofPushRequest {
    pub proof: DelegationProof,
    pub verifier_targets: Vec<Principal>,
}

///
/// DelegationVerifierProofPushResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationVerifierProofPushResponse {
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
/// DelegationAdminCommand
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum DelegationAdminCommand {
    PrewarmVerifiers(DelegationVerifierProofPushRequest),
    RepairVerifiers(DelegationVerifierProofPushRequest),
}

///
/// DelegationAdminResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum DelegationAdminResponse {
    PrewarmedVerifiers {
        result: DelegationVerifierProofPushResponse,
    },
    RepairedVerifiers {
        result: DelegationVerifierProofPushResponse,
    },
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
