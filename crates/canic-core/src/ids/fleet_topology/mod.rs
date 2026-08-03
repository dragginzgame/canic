//! Module: ids::fleet_topology
//!
//! Responsibility: define protected Fleet topology, admission, limit, and binding facts.
//! Does not own: configuration compilation, placement decisions, Registry mutation, or storage.
//! Boundary: these passive cross-layer contracts are validated before authoritative use.

use crate::{
    cdk::types::Cycles,
    ids::{CanisterRole, ComponentInstanceId, ComponentSpecId, FleetBinding, SubnetId},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::fmt;

///
/// ComponentTopologyDigest
///
/// SHA-256 identity of one canonical root-local Component Topology projection.
///

#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(transparent)]
pub struct ComponentTopologyDigest([u8; 32]);

impl ComponentTopologyDigest {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ComponentTopologyDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

///
/// CyclesFundingBudget
///
/// Positive aggregate cycles-funding ceiling applied over one bounded window.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CyclesFundingBudget {
    pub window_secs: u64,
    pub maximum_cycles: Cycles,
}

///
/// FleetSubnetCanisterPoolConfig
///
/// Immutable prepaid empty-Canister inventory policy for one Fleet Subnet Root.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetCanisterPoolConfig {
    /// Ready empty Canisters the root continuously attempts to retain.
    pub minimum_size: u32,
    /// Ceiling for configured imports and proactive refill inventory.
    ///
    /// Recycled assets remain tracked even when their return temporarily exceeds this target.
    pub maximum_size: u32,
    /// Cycles placed on each Canister created by the root for this pool.
    pub canister_cycles: Cycles,
}

///
/// ComponentSpecAdmission
///
/// Immutable permission and concrete-instance ceiling for one Spec on one Fleet Subnet Root.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentSpecAdmission {
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub maximum_root_instances: u32,
}

///
/// FleetSubnetRootLimits
///
/// Immutable aggregate policy ceilings for one Fleet Subnet Root.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetRootLimits {
    pub maximum_component_instances: u32,
    pub maximum_managed_canisters: u32,
    pub maximum_registry_bytes: u64,
    pub maximum_wasm_store_bytes: u64,
    pub canister_pool: FleetSubnetCanisterPoolConfig,
    pub cycles_funding: CyclesFundingBudget,
}

///
/// FleetCoordinatorBinding
///
/// Immutable identity and exact physical placement of one Fleet Coordinator.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetCoordinatorBinding {
    pub fleet: FleetBinding,
    pub coordinator_subnet: SubnetId,
    pub coordinator: Principal,
}

///
/// FleetRegistryAuthority
///
/// Exact Coordinator binding and reinstall-local authority epoch for one Fleet Registry.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetRegistryAuthority {
    pub binding: FleetCoordinatorBinding,
    pub epoch: u64,
}

///
/// FleetSubnetRootBinding
///
/// Complete immutable identity, placement, admissions, and limits of one Fleet Subnet Root.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetRootBinding {
    pub authority: FleetRegistryAuthority,
    pub placement_subnet: SubnetId,
    pub fleet_subnet_root: Principal,
    pub component_admissions: Vec<ComponentSpecAdmission>,
    pub component_topology_digest: ComponentTopologyDigest,
    pub limits: FleetSubnetRootLimits,
}

///
/// ComponentBinding
///
/// Complete immutable identity and placement of one concrete Component.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentBinding {
    pub authority: FleetRegistryAuthority,
    pub component: ComponentInstanceId,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub role: CanisterRole,
    pub placement_subnet: SubnetId,
    pub fleet_subnet_root: Principal,
    pub canister_id: Principal,
}

///
/// ComponentChildBinding
///
/// Complete immutable identity of one child at any depth in one exact Component tree.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentChildBinding {
    pub component: ComponentBinding,
    pub parent_canister_id: Principal,
    pub role: CanisterRole,
    pub canister_id: Principal,
}

///
/// ManagedCanisterBinding
///
/// Immutable Registry-issued identity retained by one managed application Canister.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum ManagedCanisterBinding {
    Component(ComponentBinding),
    ComponentChild(ComponentChildBinding),
}
