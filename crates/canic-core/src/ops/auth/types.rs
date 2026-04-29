use crate::{
    cdk::types::Principal,
    dto::auth::{DelegatedTokenV2, DelegationAudienceV2, DelegationProofV2, RootKeyCertificateV2},
};

//
// SignDelegatedTokenV2Input
//

pub struct SignDelegatedTokenV2Input {
    pub proof: DelegationProofV2,
    pub subject: Principal,
    pub audience: DelegationAudienceV2,
    pub scopes: Vec<String>,
    pub ttl_secs: u64,
    pub nonce: [u8; 16],
}

//
// SignDelegationProofV2Input
//

pub struct SignDelegationProofV2Input {
    pub audience: DelegationAudienceV2,
    pub scopes: Vec<String>,
    pub shard_pid: Principal,
    pub cert_ttl_secs: u64,
    pub max_token_ttl_secs: u64,
    pub max_cert_ttl_secs: u64,
    pub issued_at: u64,
    pub root_key_cert: Option<RootKeyCertificateV2>,
}

//
// VerifyDelegatedTokenV2RuntimeInput
//

pub struct VerifyDelegatedTokenV2RuntimeInput<'a> {
    pub token: &'a DelegatedTokenV2,
    pub max_cert_ttl_secs: u64,
    pub max_token_ttl_secs: u64,
    pub required_scopes: &'a [String],
    pub now_secs: u64,
}
