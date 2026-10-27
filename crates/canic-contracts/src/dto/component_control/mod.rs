//! Native allocation and removal intent; authoritative records retain their original shapes.
use crate::{
    dto::{component_registry::ComponentRegistryHead, registry_discovery::RegistryVersion},
    ids::{ComponentInstanceId, ComponentSpecId, FleetServiceId},
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};
/// Exact subtree target and observed partition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct SubtreeRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub target_canister_id: Principal,
    pub expected_registry: ComponentRegistryHead,
}
/// Peer proof mode; the Root derives actual caller authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum PeerRequester {
    SameRoot,
    FleetService {
        service: FleetServiceId,
        expected_registry: Box<RegistryVersion>,
    },
}
/// Exact requested peer specification and replay identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PeerRequest {
    pub operation_id: [u8; 32],
    pub component_spec: ComponentSpecId,
    pub requester: PeerRequester,
}

/// Native RootComponentRegistryPreparationRequest projection; canonical durable authority is unchanged.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PreparationRequest {
    pub store_bootstrap: crate::dto::root_store::RootStoreBootstrapRequest,
    pub expected_fleet_registry: RegistryVersion,
}

/// Native RootComponentRegistryStatusResponse projection; canonical durable authority is unchanged.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RegistryStatus {
    pub fleet_subnet_root: Principal,
    pub prepared_against_registry: RegistryVersion,
    pub release_set: crate::ids::FleetSubnetRootReleaseSet,
    pub component_topology_digest: crate::ids::ComponentTopologyDigest,
    pub next_allocation_sequence: u64,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    pub managed_descendants: u32,
    pub known_created_component_canisters: u32,
    pub encoded_bytes: u64,
    pub initial_inventory:
        Option<crate::dto::component_registry::RootComponentInitialInventoryStatus>,
}
