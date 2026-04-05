use crate::dto::{
    prelude::*,
    rpc::{CyclesRequest, CyclesResponse, Request, Response},
};

pub mod proof;

pub use proof::{DelegatedGrant, DelegatedGrantProof, DelegatedGrantScope, RoleAttestationProof};

//
// CapabilityVersion
//

pub const CAPABILITY_VERSION_V1: u16 = 1;

//
// ProofVersion
//

pub const PROOF_VERSION_V1: u16 = 1;

//
// CapabilityService
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum CapabilityService {
    Root,
}

//
// CapabilityRequestMetadata
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct CapabilityRequestMetadata {
    pub request_id: [u8; 16],
    pub nonce: [u8; 16],
    pub issued_at: u64,
    pub ttl_seconds: u32,
}

//
// CapabilityProofBlob
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CapabilityProofBlob {
    pub proof_version: u16,
    pub capability_hash: [u8; 32],
    pub payload: Vec<u8>,
}

//
// CapabilityProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum CapabilityProof {
    Structural,
    RoleAttestation(CapabilityProofBlob),
    DelegatedGrant(CapabilityProofBlob),
}

//
// RootCapabilityEnvelopeV1
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct RootCapabilityEnvelopeV1 {
    pub service: CapabilityService,
    pub capability_version: u16,
    pub capability: Request,
    pub proof: CapabilityProof,
    pub metadata: CapabilityRequestMetadata,
}

//
// RootCapabilityResponseV1
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct RootCapabilityResponseV1 {
    pub response: Response,
}

//
// NonrootCyclesCapabilityEnvelopeV1
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct NonrootCyclesCapabilityEnvelopeV1 {
    pub service: CapabilityService,
    pub capability_version: u16,
    pub capability: CyclesRequest,
    pub proof: CapabilityProof,
    pub metadata: CapabilityRequestMetadata,
}

//
// NonrootCyclesCapabilityResponseV1
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct NonrootCyclesCapabilityResponseV1 {
    pub response: CyclesResponse,
}
