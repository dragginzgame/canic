//! Module: dto::auth::attestation
//!
//! Responsibility: root-signed role-attestation DTOs.
//! Does not own: attestation signing, cache state, or verification.
//! Boundary: passive role-attestation request and proof contracts.

//
// RoleAttestationRequest
//

use super::IcCanisterSignatureProofV1;
use crate::dto::{prelude::*, rpc::RootRequestMetadata};

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct RoleAttestationRequest {
    pub subject: Principal,
    pub role: CanisterRole,
    pub subnet_id: Option<Principal>,
    pub audience: Principal,
    pub ttl_ns: u64,
    pub epoch: u64,
    pub metadata: Option<RootRequestMetadata>,
}

//
// RoleAttestation
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleAttestation {
    pub subject: Principal,
    pub role: CanisterRole,
    pub subnet_id: Option<Principal>,
    pub audience: Principal,
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
    pub epoch: u64,
}

//
// RoleAttestationRootProof
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum RoleAttestationRootProof {
    IcCanisterSignatureV1(IcCanisterSignatureProofV1),
}

//
// RoleAttestationPrepareResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleAttestationPrepareResponse {
    pub payload: RoleAttestation,
    pub payload_hash: [u8; 32],
    pub retrieval_expires_at_ns: u64,
}

//
// RoleAttestationGetRequest
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RoleAttestationGetRequest {
    pub payload_hash: [u8; 32],
}

//
// SignedRoleAttestation
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SignedRoleAttestation {
    pub payload: RoleAttestation,
    pub root_proof: RoleAttestationRootProof,
}
