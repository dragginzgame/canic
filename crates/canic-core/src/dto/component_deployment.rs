//! Module: dto::component_deployment
//!
//! Responsibility: carry one runtime's immutable Component deployment context.
//! Does not own: configuration compilation, context validation, persistence, or enforcement.
//! Boundary: roots install this passive context and application policy reads the retained value.

use crate::ids::{
    ComponentBinding, ComponentDeploymentConfigurationDigest, ComponentGroupMemberPath,
    ComponentGroupPlacementId, ComponentGroupSpecId,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};

pub use crate::config::{
    ComponentDeploymentLabel, ComponentDeploymentLabelKey, ComponentDeploymentLabelValue,
    ComponentDeploymentLimits, ComponentDeploymentPurpose, ComponentDeploymentSpawnGrantLimit,
    FleetServiceMemberPurpose,
};

///
/// ProtectedComponentDeployment
///
/// Plan-derived deployment identity and policy retained by one managed Component-tree runtime.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[expect(
    clippy::large_enum_variant,
    reason = "the bounded protected boundary intentionally preserves the design's direct field shape"
)]
pub enum ProtectedComponentDeployment {
    UngroupedOrdinary {
        binding: ComponentBinding,
    },
    GroupMember {
        binding: ComponentBinding,
        configuration_digest: ComponentDeploymentConfigurationDigest,
        group_placement: ComponentGroupPlacementId,
        component_group: ComponentGroupSpecId,
        member_path: ComponentGroupMemberPath,
        purpose: ComponentDeploymentPurpose,
        labels: Vec<ComponentDeploymentLabel>,
        limits: ComponentDeploymentLimits,
    },
}
