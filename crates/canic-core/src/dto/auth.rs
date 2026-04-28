use crate::dto::{error::Error, prelude::*, rpc::RootRequestMetadata};

//
// DelegationAudience
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DelegationAudience {
    Any,
    Roles(Vec<CanisterRole>),
}

//
// DelegationCert
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationCert {
    pub root_pid: Principal,
    pub shard_pid: Principal,
    pub issued_at: u64,
    pub expires_at: u64,
    pub scopes: Vec<String>,
    pub aud: DelegationAudience,
}

//
// DelegationProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationProof {
    pub cert: DelegationCert,
    pub cert_sig: Vec<u8>,
}

//
// DelegationProofInstallIntent
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum DelegationProofInstallIntent {
    Provisioning,
    Repair,
}

//
// DelegationProofInstallRequest
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct DelegationProofInstallRequest {
    pub proof: DelegationProof,
    pub intent: DelegationProofInstallIntent,
    #[serde(default)]
    pub root_public_key_sec1: Option<Vec<u8>>,
    pub shard_public_key_sec1: Vec<u8>,
}

//
// DelegatedTokenClaims
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct DelegatedTokenClaims {
    pub sub: Principal,
    pub shard_pid: Principal,
    pub scopes: Vec<String>,
    pub aud: DelegationAudience,
    pub iat: u64,
    pub exp: u64,
    // Optional signed application payload. CANIC preserves this field but does
    // not interpret it; applications own its schema and authorization meaning.
    #[serde(default)]
    pub ext: Option<Vec<u8>>,
}

//
// DelegatedToken
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct DelegatedToken {
    pub claims: DelegatedTokenClaims,
    pub proof: DelegationProof,
    pub token_sig: Vec<u8>,
}

//
// DelegationRequest
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct DelegationRequest {
    pub shard_pid: Principal,
    pub scopes: Vec<String>,
    pub aud: DelegationAudience,
    pub ttl_secs: u64,
    pub shard_public_key_sec1: Vec<u8>,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

//
// RoleAttestationRequest
//

#[derive(CandidType, Clone, Debug, Deserialize)]
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

//
// RoleAttestation
//

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

//
// SignedRoleAttestation
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SignedRoleAttestation {
    pub payload: RoleAttestation,
    pub signature: Vec<u8>,
    pub key_id: u32,
}

//
// AttestationKeyStatus
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AttestationKeyStatus {
    Current,
    Previous,
}

//
// AttestationKey
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct AttestationKey {
    pub key_id: u32,
    pub public_key: Vec<u8>,
    pub status: AttestationKeyStatus,
    #[serde(default)]
    pub valid_from: Option<u64>,
    #[serde(default)]
    pub valid_until: Option<u64>,
}

//
// AttestationKeySet
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct AttestationKeySet {
    pub root_pid: Principal,
    pub generated_at: u64,
    pub keys: Vec<AttestationKey>,
}

// Canonical delegation issuance response. Fanout results are verifier-only.
//
// DelegationProvisionResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationProvisionResponse {
    pub proof: DelegationProof,
    pub results: Vec<DelegationProvisionTargetResponse>,
}

//
// DelegationVerifierProofPushRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct DelegationVerifierProofPushRequest {
    pub proof: DelegationProof,
    pub verifier_targets: Vec<Principal>,
}

//
// DelegationVerifierProofPushResponse
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct DelegationVerifierProofPushResponse {
    pub results: Vec<DelegationProvisionTargetResponse>,
}

//
// DelegationProofStatus
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct DelegationProofStatus {
    pub shard_pid: Principal,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DelegationProvisionStatus {
    Ok,
    Failed,
}

//
// DelegationAdminCommand
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum DelegationAdminCommand {
    RepairVerifiers(DelegationVerifierProofPushRequest),
}

//
// DelegationAdminResponse
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum DelegationAdminResponse {
    RepairedVerifiers {
        result: DelegationVerifierProofPushResponse,
    },
}

//
// DelegationProvisionTargetResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationProvisionTargetResponse {
    pub target: Principal,
    pub status: DelegationProvisionStatus,
    pub error: Option<Error>,
}
