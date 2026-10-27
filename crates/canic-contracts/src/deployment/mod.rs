//! Passive compiled deployment metadata, quotas, and grants.

pub mod label;

use crate::ids::{CanisterRole, ComponentSpecId, FleetServiceId};
use serde::{Deserialize, Serialize};
use std::fmt;

pub use label::*;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum FleetServiceMemberPurpose {
    #[serde(rename = "authority")]
    Authority,
    #[serde(rename = "replica")]
    Replica,
    #[serde(rename = "pool_member")]
    PoolMember,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum ComponentDeploymentPurpose {
    Ordinary,
    FleetServiceMember {
        service: FleetServiceId,
        member_purpose: FleetServiceMemberPurpose,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentDeploymentSpawnGrantLimit {
    pub parent_role: CanisterRole,
    pub child_role: CanisterRole,
    pub maximum_instances_per_parent: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentDeploymentLimits {
    pub maximum_descendants: u32,
    pub maximum_registry_bytes: u64,
    pub spawn_grant_reductions: Vec<ComponentDeploymentSpawnGrantLimit>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentProvisioningGrant {
    pub requester_component_spec: ComponentSpecId,
    pub target_component_spec: ComponentSpecId,
    pub maximum_instances_per_requester_per_root: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ComponentChildKind {
    #[serde(rename = "singleton")]
    Singleton,

    #[serde(rename = "replica")]
    Replica,

    #[serde(rename = "shard")]
    Shard,

    #[serde(rename = "instance")]
    Instance,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetServicePlacementPolicy {
    pub maximum_members_per_root: u32,
    pub minimum_distinct_roots: u32,
}

impl std::fmt::Display for ComponentChildKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Singleton => "singleton",
            Self::Replica => "replica",
            Self::Shard => "shard",
            Self::Instance => "instance",
        };
        formatter.write_str(label)
    }
}

#[cfg(all(feature = "wire-schema", not(target_arch = "wasm32")))]
mod schema;
