//! Module: config::schema::component_group_deployment
//!
//! Responsibility: define strict checked-in Component Group deployment declarations.
//! Does not own: labels, member limits, root selection, planning, or runtime state.
//! Boundary: source declarations select one group and freeze a bounded placement envelope.

use crate::{config::FleetServiceMemberPurpose, ids::ComponentGroupSpecId};
use serde::{Deserialize, Serialize};

/// One independently scalable deployment of a reusable Component Group.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupDeploymentConfig {
    pub component_group: ComponentGroupSpecId,
    pub service_purpose: Option<FleetServiceMemberPurpose>,
    pub initial_placements: u32,
    pub maximum_placements: u32,
    pub placement: ComponentGroupPlacementPolicyConfig,
}

/// Source density and spread envelope for one Component Group deployment.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupPlacementPolicyConfig {
    pub maximum_per_root: u32,
    pub minimum_distinct_roots: u32,
}
