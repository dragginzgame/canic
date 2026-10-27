//! Native managed runtime authority; original directory hashes and stored records retain their encodings.
use crate::{
    deployment::{
        ComponentDeploymentLabel, ComponentDeploymentPurpose, FleetServiceMemberPurpose,
        FleetServicePlacementPolicy,
    },
    dto::fleet_registry::{FleetServiceMode, FleetSubnetRootStatus},
    ids::{
        CanisterRole, ComponentGroupMemberPath, ComponentGroupPlacementId, ComponentGroupSpecId,
        ComponentInstanceId, ComponentSpecId, FleetServiceId,
    },
    protocol::profile::ProtocolProfileDigest,
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Exact native projection of the maintained ComponentRuntimeDirectoryPreparationRequest contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentRuntimeDirectoryPreparationRequest {
    pub operation_id: [u8; 32],
    pub authority: ComponentRuntimeDirectoryAuthority,
    pub direct_children: Vec<ComponentRuntimeDirectChild>,
}

/// Exact native projection of the maintained ComponentRuntimeDirectoryAuthority contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentRuntimeDirectoryAuthority {
    pub fleet: FleetDirectorySnapshot,
    pub component: crate::dto::component_directory::ComponentDirectoryHead,
    pub component_group: Option<ComponentGroupDirectory>,
}

/// Exact native projection of the maintained ComponentRuntimeDirectChild contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentRuntimeDirectChild {
    pub canister_id: Principal,
    pub allocation_operation_id: [u8; 32],
    pub role: CanisterRole,
    pub protocol_profile_digest: ProtocolProfileDigest,
}

/// Exact native projection of the maintained FleetDirectorySnapshot contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetDirectorySnapshot {
    pub provenance: FleetDirectoryProvenance,
    pub fleet_subnet_roots: Vec<FleetSubnetRootDirectoryEntry>,
    pub services: Vec<FleetDirectoryService>,
}

/// Exact native projection of the maintained FleetDirectoryProvenance contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetDirectoryProvenance {
    pub registry: crate::dto::registry_discovery::RegistryVersion,
    pub source_fleet_subnet_root: Principal,
}

/// Exact native projection of the maintained FleetSubnetRootDirectoryEntry contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetSubnetRootDirectoryEntry {
    pub placement_subnet: Principal,
    pub fleet_subnet_root: Principal,
    pub status: FleetSubnetRootStatus,
}

/// Exact native projection of the maintained FleetDirectoryService contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetDirectoryService {
    pub service: FleetServiceId,
    pub role: CanisterRole,
    pub component_spec: ComponentSpecId,
    pub mode: FleetServiceMode,
    pub placement: FleetServicePlacementPolicy,
    pub members: Vec<FleetDirectoryServiceComponent>,
}

/// Exact native projection of the maintained FleetDirectoryServiceComponent contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetDirectoryServiceComponent {
    pub member_purpose: FleetServiceMemberPurpose,
    pub component: ComponentInstanceId,
    pub fleet_subnet_root: Principal,
    pub canister_id: Principal,
    pub group_placement: ComponentGroupPlacementId,
    pub member_path: ComponentGroupMemberPath,
}

/// Exact native projection of the maintained ComponentGroupDirectory contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentGroupDirectory {
    pub provenance: ComponentGroupDirectoryProvenance,
    pub members: Vec<ComponentGroupDirectoryMember>,
}

/// Exact native projection of the maintained ComponentGroupDirectoryProvenance contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentGroupDirectoryProvenance {
    pub authority: crate::dto::component_directory::FleetRegistryAuthority,
    pub fleet_subnet_root: Principal,
    pub group_placement: ComponentGroupPlacementId,
    pub component_group: ComponentGroupSpecId,
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub placement_receipt_content_hash: [u8; 32],
}

/// Exact native projection of the maintained ComponentGroupDirectoryMember contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentGroupDirectoryMember {
    pub member_path: ComponentGroupMemberPath,
    pub component_spec: ComponentSpecId,
    pub purpose: ComponentDeploymentPurpose,
    pub labels: Vec<ComponentDeploymentLabel>,
    pub binding: crate::dto::component_directory::ComponentBinding,
}
