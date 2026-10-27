//! Native operation-specific Root capability transport; no replay or storage ownership.
use crate::{
    dto::{
        capability::{CapabilityProof, CapabilityRequestMetadata, CapabilityService},
        rpc::RootRequestMetadata,
    },
    ids::CanisterRole,
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Structural authority and original replay metadata carried with one typed operation.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CapabilityRequest<T> {
    pub service: CapabilityService,
    pub capability_version: u16,
    pub capability: T,
    pub proof: CapabilityProof,
    pub metadata: CapabilityRequestMetadata,
}

/// Native CreateCanisterRequest boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CreateCanisterRequest {
    pub canister_role: CanisterRole,
    pub parent: CreateCanisterParent,
    pub extra_arg: Option<Vec<u8>>,
    pub metadata: Option<RootRequestMetadata>,
}

/// Native CreateCanisterParent boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum CreateCanisterParent {
    Root,
    // Use the requesting canister.
    ThisCanister,
    // Use the caller's parent.
    Parent,
    Canister(Principal),
    Directory(CanisterRole),
}

/// Native RecycleCanisterRequest boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RecycleCanisterRequest {
    pub allocation_operation_id: [u8; 32],
    pub canister_pid: Principal,
    pub metadata: Option<RootRequestMetadata>,
}

/// Native CreateCanisterResponse boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CreateCanisterResponse {
    pub new_canister_pid: Principal,
}
