use crate::dto::{
    auth::SignedRoleAttestation,
    prelude::*,
    rpc::{Request, Response},
};

///
/// CapabilityVersion
///

pub const CAPABILITY_VERSION_V1: u16 = 1;

///
/// ProofVersion
///

pub const PROOF_VERSION_V1: u16 = 1;

///
/// CapabilityService
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CapabilityService {
    Root,
    ProjectRegistry,
    ProjectInstance,
    Cycles,
    CanisterLifecycle,
}

///
/// CapabilityRequestMetadata
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityRequestMetadata {
    pub request_id: [u8; 16],
    pub nonce: [u8; 16],
    pub issued_at: u64,
    pub ttl_seconds: u32,
}

///
/// RoleAttestationProof
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleAttestationProof {
    pub proof_version: u16,
    pub capability_hash: [u8; 32],
    pub attestation: SignedRoleAttestation,
}

///
/// DelegatedGrantScope
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedGrantScope {
    pub service: CapabilityService,
    pub capability_family: String,
}

///
/// DelegatedGrant
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

///
/// DelegatedGrantProof
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedGrantProof {
    pub proof_version: u16,
    pub capability_hash: [u8; 32],
    pub grant: DelegatedGrant,
    pub grant_sig: Vec<u8>,
    pub key_id: u32,
}

///
/// CapabilityProof
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CapabilityProof {
    Structural,
    RoleAttestation(RoleAttestationProof),
    DelegatedGrant(DelegatedGrantProof),
}

///
/// RootCapabilityEnvelopeV1
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct RootCapabilityEnvelopeV1 {
    pub service: CapabilityService,
    pub capability_version: u16,
    pub capability: Request,
    pub proof: CapabilityProof,
    pub metadata: CapabilityRequestMetadata,
}

///
/// RootCapabilityResponseV1
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct RootCapabilityResponseV1 {
    pub response: Response,
}
