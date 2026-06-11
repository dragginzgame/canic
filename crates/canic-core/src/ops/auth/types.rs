use crate::{
    cdk::types::Principal,
    dto::auth::{DelegatedRoleGrant, DelegatedToken, DelegationAudience, DelegationCert},
    ops::auth::delegated::mint::PreparedDelegatedToken,
};

//
// SignDelegatedTokenInput
//

pub struct SignDelegatedTokenInput {
    pub subject: Principal,
    pub audience: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub ttl_ns: u64,
    pub nonce: [u8; 16],
    pub ext: Option<Vec<u8>>,
}

//
// SignDelegationProofInput
//

pub struct SignDelegationProofInput {
    pub operation_id: [u8; 32],
    pub audience: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub issuer_pid: Principal,
    pub cert_ttl_ns: u64,
    pub max_token_ttl_ns: u64,
    pub max_cert_ttl_ns: u64,
    pub issued_at_ns: u64,
}

//
// PreparedRootDelegationProof
//

#[derive(Clone)]
pub struct PreparedRootDelegationProof {
    pub cert: DelegationCert,
    pub cert_hash: [u8; 32],
    pub retrieval_expires_at_ns: u64,
}

//
// PreparedDelegatedTokenIssuerProof
//

pub struct PreparedDelegatedTokenIssuerProof {
    pub prepared: PreparedDelegatedToken,
    pub claims_hash: [u8; 32],
    pub retrieval_expires_at_ns: u64,
}

//
// DelegatedTokenVerifierConfig
//

pub struct DelegatedTokenVerifierConfig {
    pub root_canister_id: Principal,
    pub ic_root_public_key_raw: Vec<u8>,
}

//
// VerifyDelegatedTokenRuntimeInput
//

pub struct VerifyDelegatedTokenRuntimeInput<'a> {
    pub token: &'a DelegatedToken,
    pub caller: Principal,
    pub max_cert_ttl_ns: u64,
    pub max_token_ttl_ns: u64,
    pub required_scopes: &'a [String],
    pub now_ns: u64,
}
