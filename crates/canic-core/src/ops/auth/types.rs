use crate::{
    cdk::types::Principal,
    dto::auth::{DelegatedToken, DelegationAudience, DelegationProof},
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
// VerifyDelegatedTokenRuntimeInput
//

pub struct VerifyDelegatedTokenRuntimeInput<'a> {
    pub token: &'a DelegatedToken,
    pub max_cert_ttl_secs: u64,
    pub max_token_ttl_secs: u64,
    pub required_scopes: &'a [String],
    pub now_secs: u64,
}
