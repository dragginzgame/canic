use crate::{
    cdk::types::Principal,
    dto::auth::{
        DelegatedToken, DelegationAudience, DelegationCert, DelegationProof,
        InternalInvocationProofPayloadV1, RoleAttestation,
    },
};

//
// SignDelegatedTokenInput
//

pub struct SignDelegatedTokenInput {
    pub proof: DelegationProof,
    pub subject: Principal,
    pub audience: DelegationAudience,
    pub scopes: Vec<String>,
    pub ttl_secs: u64,
    pub nonce: [u8; 16],
}

//
// SignDelegationProofInput
//

pub struct SignDelegationProofInput {
    pub audience: DelegationAudience,
    pub scopes: Vec<String>,
    pub shard_pid: Principal,
    pub cert_ttl_secs: u64,
    pub max_token_ttl_secs: u64,
    pub max_cert_ttl_secs: u64,
    pub issued_at: u64,
}

//
// PreparedRootDelegationProof
//

pub struct PreparedRootDelegationProof {
    pub cert: DelegationCert,
    pub cert_hash: [u8; 32],
    pub key_name: String,
    pub root_derivation_path: Vec<Vec<u8>>,
}

//
// PreparedRoleAttestationSignature
//

pub struct PreparedRoleAttestationSignature {
    pub payload: RoleAttestation,
    pub message_hash: [u8; 32],
    pub key_name: String,
    pub derivation_path: Vec<Vec<u8>>,
}

//
// PreparedInternalInvocationProofSignature
//

pub struct PreparedInternalInvocationProofSignature {
    pub payload: InternalInvocationProofPayloadV1,
    pub message_hash: [u8; 32],
    pub key_name: String,
    pub derivation_path: Vec<Vec<u8>>,
}

//
// VerifyDelegatedTokenRuntimeInput
//

pub struct VerifyDelegatedTokenRuntimeInput<'a> {
    pub token: &'a DelegatedToken,
    pub max_cert_ttl_secs: u64,
    pub max_token_ttl_secs: u64,
    pub required_scopes: &'a [String],
    pub now_secs: u64,
}
