use crate::dto::{auth::SignedRoleAttestation, capability::CapabilityService, prelude::*};

//
// RoleAttestationProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RoleAttestationProof {
    pub proof_version: u16,
    pub capability_hash: [u8; 32],
    pub attestation: SignedRoleAttestation,
}

//
// DelegatedGrantScope
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct DelegatedGrantScope {
    pub service: CapabilityService,
    pub capability_family: String,
}

//
// DelegatedGrant
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct DelegatedGrant {
    pub issuer: Principal,
    pub subject: Principal,
    pub audience: Vec<Principal>,
    pub scope: DelegatedGrantScope,
    pub capability_hash: [u8; 32],
    pub quota: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub epoch: u64,
}

//
// DelegatedGrantProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct DelegatedGrantProof {
    pub proof_version: u16,
    pub capability_hash: [u8; 32],
    pub grant: DelegatedGrant,
    pub grant_sig: Vec<u8>,
    pub key_id: u32,
}
