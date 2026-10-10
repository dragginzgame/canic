//! Passive compiled deployment metadata, quotas, and grants.

pub mod label;

use crate::ids::{CanisterRole, ComponentSpecId, FleetServiceId};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::fmt;

pub use label::*;

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetServiceMemberPurpose {
    #[serde(rename = "authority")]
    Authority,
    #[serde(rename = "replica")]
    Replica,
    #[serde(rename = "pool_member")]
    PoolMember,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ComponentDeploymentPurpose {
    Ordinary,
    FleetServiceMember {
        service: FleetServiceId,
        member_purpose: FleetServiceMemberPurpose,
    },
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDeploymentSpawnGrantLimit {
    pub parent_role: CanisterRole,
    pub child_role: CanisterRole,
    pub maximum_instances_per_parent: u32,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDeploymentLimits {
    pub maximum_descendants: u32,
    pub maximum_registry_bytes: u64,
    pub spawn_grant_reductions: Vec<ComponentDeploymentSpawnGrantLimit>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentProvisioningGrant {
    pub requester_component_spec: ComponentSpecId,
    pub target_component_spec: ComponentSpecId,
    pub maximum_instances_per_requester_per_root: u32,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
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
