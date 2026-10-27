//! Native role-attestation transport; signed canonical payload bytes are unchanged.
use crate::{
    dto::{auth::RoleAttestationRootProof, rpc::RootRequestMetadata},
    ids::CanisterRole,
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Native RoleAttestationRequest boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RoleAttestationRequest {
    pub subject: Principal,
    pub role: CanisterRole,
    pub subnet_id: Option<Principal>,
    pub audience: Principal,
    pub ttl_ns: u64,
    pub epoch: u64,
    pub metadata: Option<RootRequestMetadata>,
}

/// Native RoleAttestation boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RoleAttestation {
    pub subject: Principal,
    pub role: CanisterRole,
    pub subnet_id: Option<Principal>,
    pub audience: Principal,
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
    pub epoch: u64,
}

/// Native RoleAttestationPrepareResponse boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RoleAttestationPrepareResponse {
    pub payload: RoleAttestation,
    pub payload_hash: [u8; 32],
    pub retrieval_expires_at_ns: u64,
}

/// Native SignedRoleAttestation boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct SignedRoleAttestation {
    pub payload: RoleAttestation,
    pub root_proof: RoleAttestationRootProof,
}
