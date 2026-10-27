//! Native registry snapshot boundary; persisted identities and canonical hashing retain their owners.
use crate::{
    deployment::{FleetServiceMemberPurpose, FleetServicePlacementPolicy},
    dto::{
        component_directory::FleetRegistryAuthority,
        fleet_registry::{FleetComponentSpecEntry, FleetServiceMode, FleetSubnetRootStatus},
    },
    ids::{
        CanisterRole, ComponentGroupMemberPath, ComponentGroupPlacementId, ComponentInstanceId,
        ComponentSpecAdmission, ComponentSpecId, ComponentTopologyDigest, FleetBinding,
        FleetServiceId, FleetSubnetRootAutomaticIcpRefillPolicy, FleetSubnetRootFundingPolicy,
        FleetSubnetRootLimits, FleetSubnetRootReleaseSet,
    },
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Native projection of the maintained FleetRegistry contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetRegistry {
    pub authority: FleetRegistryAuthority,
    pub revision: u64,
    pub admission: FleetAdmissionPolicy,
    pub component_specs: Vec<FleetComponentSpecEntry>,
    pub fleet_subnet_roots: Vec<FleetSubnetRootEntry>,
    pub services: Vec<FleetServiceBinding>,
}

/// Native projection of the maintained FleetSubnetRootEntry contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetSubnetRootEntry {
    pub placement_subnet: Principal,
    pub fleet_subnet_root: Principal,
    pub component_admissions: Vec<ComponentSpecAdmission>,
    pub component_topology_digest: ComponentTopologyDigest,
    pub active_release_set: FleetSubnetRootReleaseSet,
    pub limits: FleetSubnetRootLimits,
    pub funding: FleetSubnetRootFundingAuthority,
    pub status: FleetSubnetRootStatus,
}

/// Native projection of the maintained FleetServiceBinding contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetServiceBinding {
    pub service: FleetServiceId,
    pub role: CanisterRole,
    pub component_spec: ComponentSpecId,
    pub mode: FleetServiceMode,
    pub placement: FleetServicePlacementPolicy,
    pub members: Vec<FleetServiceComponentBinding>,
}

/// Native projection of the maintained FleetServiceComponentBinding contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetServiceComponentBinding {
    pub member_purpose: FleetServiceMemberPurpose,
    pub component: ComponentInstanceId,
    pub fleet_subnet_root: Principal,
    pub canister_id: Principal,
    pub group_placement: ComponentGroupPlacementId,
    pub member_path: ComponentGroupMemberPath,
}

/// Native projection of the maintained FleetAdmissionSelector contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum FleetAdmissionSelector {
    #[serde(rename = "fleet")]
    Fleet,
    #[serde(rename = "component_spec")]
    ComponentSpec(ComponentSpecId),
    #[serde(rename = "component_instance")]
    ComponentInstance(ComponentInstanceId),
    #[serde(rename = "fleet_subnet_root")]
    FleetSubnetRoot(Principal),
}

/// Native projection of the maintained FleetAdmissionRule contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetAdmissionRule {
    pub selector: FleetAdmissionSelector,
    pub principals: Vec<Principal>,
}

/// Native projection of the maintained FleetAdmissionPolicy contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetAdmissionPolicy {
    pub schema_version: u16,
    pub fleet: FleetBinding,
    pub generation: u64,
    pub fleet_principals: Vec<Principal>,
    pub rules: Vec<FleetAdmissionRule>,
    pub policy_digest: [u8; 32],
}

/// Native projection of the maintained FleetSubnetRootFundingAuthority contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetSubnetRootFundingAuthority {
    pub root_funding: FleetSubnetRootFundingPolicy,
    pub icp_refill: Option<FleetSubnetRootIcpRefillPolicy>,
}

/// Native projection of the maintained FleetSubnetRootIcpRefillPolicy contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetSubnetRootIcpRefillPolicy {
    pub max_refill_e8s_per_call: u64,
    pub window_secs: u64,
    pub maximum_refill_e8s: u64,
    pub minimum_icp_balance_e8s: u64,
    pub min_xdr_permyriad_per_icp: Option<u64>,
    pub ledger_canister_id: Option<Principal>,
    pub cmc_canister_id: Option<Principal>,
    pub allow_ic_system_canister_overrides: bool,
    pub automatic: Option<FleetSubnetRootAutomaticIcpRefillPolicy>,
}

/// One Root's exact active registry and physical inventory counters.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootInventory {
    pub fleet_registry: crate::dto::registry_discovery::RegistryVersion,
    pub placement_subnet: Principal,
    pub fleet_subnet_root: Principal,
    pub status: FleetSubnetRootStatus,
    pub infrastructure_canisters: u32,
    pub component_canisters: u32,
    pub pooled_canisters: u32,
    pub total_canisters: u32,
}

/// Exact Root binding in native transport; stored authority remains unchanged.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetSubnetRootBinding {
    pub authority: crate::dto::component_directory::FleetRegistryAuthority,
    pub placement_subnet: Principal,
    pub fleet_subnet_root: Principal,
    pub component_admissions: Vec<ComponentSpecAdmission>,
    pub component_topology_digest: ComponentTopologyDigest,
    pub limits: FleetSubnetRootLimits,
    pub funding: FleetSubnetRootFundingAuthority,
}
