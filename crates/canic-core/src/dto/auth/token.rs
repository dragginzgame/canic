//! Module: dto::auth::token
//!
//! Responsibility: delegated token request, response, and bearer token DTOs.
//! Does not own: token issuance, replay checks, or proof verification.
//! Boundary: passive issuer/client token transport contracts.

use super::{
    AuthRequestMetadata, DelegatedRoleGrant, DelegationAudience, DelegationProof, IssuerProof,
};
use crate::dto::prelude::*;

//
// DelegatedTokenClaims
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedTokenClaims {
    pub subject: Principal,
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub nonce: [u8; 16],
    #[serde(default)]
    pub ext: Option<Vec<u8>>,
}

//
// DelegatedToken
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedToken {
    pub claims: DelegatedTokenClaims,
    pub proof: DelegationProof,
    pub issuer_proof: IssuerProof,
}

//
// DelegatedTokenPrepareRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedTokenPrepareRequest {
    #[serde(default)]
    pub metadata: Option<AuthRequestMetadata>,
    pub subject: Principal,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub ttl_ns: u64,
    #[serde(default)]
    pub ext: Option<Vec<u8>>,
}

//
// DelegatedTokenPrepareResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedTokenPrepareResponse {
    pub claims: DelegatedTokenClaims,
    pub claims_hash: [u8; 32],
    pub retrieval_expires_at_ns: u64,
}

//
// DelegatedTokenGetRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedTokenGetRequest {
    pub claims_hash: [u8; 32],
}
