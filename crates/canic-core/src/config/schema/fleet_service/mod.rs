//! Module: config::schema::fleet_service
//!
//! Responsibility: define strict checked-in Fleet-service target declarations.
//! Does not own: occurrence resolution, placement assignment, publication, or runtime state.
//! Boundary: source targets name logical service topology without physical roots or Canisters.

use crate::ids::{
    CanisterRole, ComponentGroupDeploymentId, ComponentGroupMemberPath, ComponentSpecId,
    FleetServiceId,
};
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Top-level namespace for application service declarations.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServicesConfig {
    #[serde(default)]
    pub fleet: FleetServicesConfig,
}

/// Fleet-wide logical service targets in canonical service-ID key space.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetServicesConfig {
    #[serde(default)]
    pub targets: BTreeMap<FleetServiceId, FleetServiceTargetConfig>,
}

/// Strict source declaration for one Fleet service and its mode.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", deny_unknown_fields)]
pub enum FleetServiceTargetConfig {
    #[serde(rename = "authority_replica")]
    AuthorityReplica {
        role: CanisterRole,
        component_spec: ComponentSpecId,
        authority_deployment: ComponentGroupDeploymentId,
        authority_member: ComponentGroupMemberPath,
        placement: FleetServicePlacementPolicyConfig,
    },

    #[serde(rename = "active_pool")]
    ActivePool {
        role: CanisterRole,
        component_spec: ComponentSpecId,
        placement: FleetServicePlacementPolicyConfig,
    },
}

/// Source density and spread envelope shared by all members of one service.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetServicePlacementPolicyConfig {
    pub maximum_members_per_root: u32,
    pub minimum_distinct_roots: u32,
}
