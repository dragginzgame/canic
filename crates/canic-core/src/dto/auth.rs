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
    Role(CanisterRole),
    Principal(Principal),
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
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
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
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
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
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
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
// InternalInvocationProofRequest
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct InternalInvocationProofRequest {
    pub subject: Principal,
    pub role: CanisterRole,
    #[serde(default)]
    pub subnet_id: Option<Principal>,
    pub audience: Principal,
    pub audience_method: String,
    pub ttl_secs: u64,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

//
// InternalInvocationProofPayloadV1
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InternalInvocationProofPayloadV1 {
    pub subject: Principal,
    pub role: CanisterRole,
    #[serde(default)]
    pub subnet_id: Option<Principal>,
    pub audience: Principal,
    pub audience_method: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub epoch: u64,
}

//
// SignedInternalInvocationProofV1
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SignedInternalInvocationProofV1 {
    pub payload: InternalInvocationProofPayloadV1,
    pub signature: Vec<u8>,
    pub key_id: u32,
}

//
// CanicInternalCallHeaderV1
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanicInternalCallHeaderV1 {
    pub target_canister: Principal,
    pub target_method: String,
}

//
// CanicInternalCallEnvelopeV1
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanicInternalCallEnvelopeV1 {
    pub version: u16,
    pub header: CanicInternalCallHeaderV1,
    pub proof: SignedInternalInvocationProofV1,
    pub args: Vec<u8>,
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

#[cfg(test)]
mod tests {
    #[test]
    fn auth_dtos_remain_passive_boundary_types() {
        let source = include_str!("auth.rs");
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source exists");

        for marker in [
            "impl DelegatedToken",
            "impl DelegatedTokenClaims",
            "impl RoleAttestation",
            "impl SignedRoleAttestation",
            "impl InternalInvocationProofPayloadV1",
            "impl SignedInternalInvocationProofV1",
            "impl CanicInternalCallEnvelopeV1",
            "fn verify",
            "fn sign",
            "fn resolve",
            "fn replay",
            "fn consume",
            "fn policy",
            "fn validate",
        ] {
            assert!(
                !production_source.contains(marker),
                "auth DTOs must stay passive; found marker `{marker}`"
            );
        }
    }
}
