use crate::dto::{prelude::*, rpc::RootRequestMetadata};

//
// SignatureAlgorithm
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SignatureAlgorithm {
    EcdsaP256Sha256,
}

//
// DelegationAudience
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DelegationAudience {
    Roles(Vec<CanisterRole>),
    Principals(Vec<Principal>),
    RolesOrPrincipals {
        roles: Vec<CanisterRole>,
        principals: Vec<Principal>,
    },
}

//
// RootPublicKey
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootPublicKey {
    pub root_pid: Principal,
    pub key_id: String,
    pub alg: SignatureAlgorithm,
    pub public_key_sec1: Vec<u8>,
    pub key_hash: [u8; 32],
    pub not_before: u64,
    pub not_after: Option<u64>,
}

//
// RootTrustAnchor
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootTrustAnchor {
    pub root_pid: Principal,
    pub root_key: RootPublicKey,
}

//
// ShardKeyBinding
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ShardKeyBinding {
    IcThresholdEcdsa {
        key_name_hash: [u8; 32],
        derivation_path_hash: [u8; 32],
    },
}

//
// DelegationCert
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationCert {
    pub version: u16,
    pub root_pid: Principal,
    pub root_key_id: String,
    pub root_key_hash: [u8; 32],
    pub alg: SignatureAlgorithm,
    pub shard_pid: Principal,
    pub shard_key_id: String,
    pub shard_public_key_sec1: Vec<u8>,
    pub shard_key_hash: [u8; 32],
    pub shard_key_binding: ShardKeyBinding,
    pub issued_at: u64,
    pub expires_at: u64,
    pub max_token_ttl_secs: u64,
    pub scopes: Vec<String>,
    pub aud: DelegationAudience,
    pub verifier_role_hash: Option<[u8; 32]>,
}

//
// DelegationProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationProof {
    pub cert: DelegationCert,
    pub root_sig: Vec<u8>,
}

//
// DelegatedTokenClaims
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedTokenClaims {
    pub version: u16,
    pub subject: Principal,
    pub issuer_shard_pid: Principal,
    pub cert_hash: [u8; 32],
    pub issued_at: u64,
    pub expires_at: u64,
    pub aud: DelegationAudience,
    pub scopes: Vec<String>,
    pub nonce: [u8; 16],
}

//
// DelegatedToken
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedToken {
    pub claims: DelegatedTokenClaims,
    pub proof: DelegationProof,
    pub shard_sig: Vec<u8>,
}

//
// DelegationProofIssueRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationProofIssueRequest {
    pub shard_pid: Principal,
    pub scopes: Vec<String>,
    pub aud: DelegationAudience,
    pub cert_ttl_secs: u64,
}

//
// DelegatedTokenIssueRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedTokenIssueRequest {
    pub proof: DelegationProof,
    pub subject: Principal,
    pub aud: DelegationAudience,
    pub scopes: Vec<String>,
    pub ttl_secs: u64,
    pub nonce: [u8; 16],
}

//
// DelegatedTokenMintRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedTokenMintRequest {
    pub subject: Principal,
    pub aud: DelegationAudience,
    pub scopes: Vec<String>,
    pub token_ttl_secs: u64,
    pub cert_ttl_secs: u64,
    pub nonce: [u8; 16],
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
    pub audience: Principal,
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
    pub audience: Principal,
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
    pub key_name: String,
    pub key_hash: [u8; 32],
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
