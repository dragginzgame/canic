//! Module: config::schema::component_group
//!
//! Responsibility: define strict checked-in Component Group declarations.
//! Does not own: graph compilation, deployments, placement, purpose, or runtime state.
//! Boundary: TOML input is decoded into bounded identifier-keyed maps before compilation.

use crate::{
    config::FleetServiceMemberPurpose,
    ids::{ComponentGroupMemberId, ComponentGroupSpecId, ComponentSpecId, FleetServiceId},
};
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Reusable configuration-only composition of Components and included groups.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupSpecConfig {
    #[serde(default)]
    pub components: BTreeMap<ComponentGroupMemberId, ComponentGroupComponentConfig>,

    #[serde(default)]
    pub groups: BTreeMap<ComponentGroupMemberId, ComponentGroupIncludeConfig>,
}

/// One direct Component occurrence declared inside a Component Group.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupComponentConfig {
    pub component_spec: ComponentSpecId,

    pub service: Option<FleetServiceId>,

    pub service_purpose: Option<FleetServiceMemberPurpose>,
}

/// One configuration-only inclusion edge to another Component Group.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupIncludeConfig {
    pub component_group: ComponentGroupSpecId,

    pub service_purpose: Option<FleetServiceMemberPurpose>,
}
