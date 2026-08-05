//! Module: config::schema::component_group_deployment
//!
//! Responsibility: define strict checked-in Component Group deployment declarations.
//! Does not own: root selection, planning, or runtime state.
//! Boundary: source declarations select one group and freeze placement plus member-limit envelopes.

use crate::{
    config::{
        ComponentDeploymentLabelKey, ComponentDeploymentLabelValue, FleetServiceMemberPurpose,
    },
    ids::{CanisterRole, ComponentGroupMemberPath, ComponentGroupSpecId},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One independently scalable deployment of a reusable Component Group.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupDeploymentConfig {
    pub component_group: ComponentGroupSpecId,
    pub service_purpose: Option<FleetServiceMemberPurpose>,
    #[serde(default)]
    pub labels: BTreeMap<ComponentDeploymentLabelKey, ComponentDeploymentLabelValue>,
    #[serde(default)]
    pub member_limits: Vec<ComponentDeploymentMemberLimitConfig>,
    pub initial_placements: u32,
    pub maximum_placements: u32,
    pub placement: ComponentGroupPlacementPolicyConfig,
}

/// Reduction-only limits for one exact flattened deployment member.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDeploymentMemberLimitConfig {
    pub member: ComponentGroupMemberPath,
    pub maximum_descendants: Option<u32>,
    pub maximum_registry_bytes: Option<u64>,
    #[serde(default)]
    pub spawn_grants: Vec<ComponentDeploymentSpawnGrantLimitConfig>,
}

/// Reduction-only ceiling for one exact Component Spec spawn grant.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDeploymentSpawnGrantLimitConfig {
    pub parent_role: CanisterRole,
    pub child_role: CanisterRole,
    pub maximum_instances_per_parent: u32,
}

/// Source density and spread envelope for one Component Group deployment.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupPlacementPolicyConfig {
    pub maximum_per_root: u32,
    pub minimum_distinct_roots: u32,
}
