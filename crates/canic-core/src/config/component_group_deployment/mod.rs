//! Module: config::component_group_deployment
//!
//! Responsibility: compile independent Component Group deployments before planning.
//! Does not own: root selection, persistence, or effects.
//! Boundary: strict source deployments become bounded exact flattened Component occurrences.

mod member_limit;
#[cfg(test)]
mod tests;

use crate::{
    config::{
        ComponentDeploymentLabel, ComponentDeploymentLabelKey, ComponentGroupLeafKind,
        ComponentGroupTopology, ComponentGroupTopologyError, ComponentTopology,
        ComponentTopologyError, FlattenedComponentGroup, FlattenedComponentGroupMember,
        FleetServiceMemberPurpose, MAX_COMPONENT_DEPLOYMENT_LABELS,
        component_group::source_labels,
        schema::{ComponentGroupDeploymentConfig, ConfigModel},
    },
    ids::{
        ComponentGroupDeploymentId, ComponentGroupMemberPath, ComponentGroupSpecId,
        ComponentSpecId, FleetServiceId,
    },
};
use std::collections::BTreeMap;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

pub use member_limit::{
    ComponentDeploymentLimits, ComponentDeploymentMemberLimit, ComponentDeploymentMemberLimitError,
    ComponentDeploymentSpawnGrantLimit, MAX_COMPONENT_DEPLOYMENT_MEMBER_LIMITS,
    MAX_COMPONENT_DEPLOYMENT_SPAWN_GRANT_REDUCTIONS,
};

/// Maximum independent Component Group deployments in one App configuration.
pub const MAX_COMPONENT_GROUP_DEPLOYMENTS: usize = 4_096;
/// Maximum flattened Component occurrences across every deployment selection.
pub const MAX_COMPONENT_GROUP_DEPLOYMENT_MEMBERS: usize = 4_096;

impl ConfigModel {
    /// Compile every checked-in deployment to exact Component member occurrences.
    pub fn compile_component_group_deployment_topology(
        &self,
    ) -> Result<ComponentGroupDeploymentTopology, ComponentGroupDeploymentTopologyError> {
        ComponentGroupDeploymentTopology::compile(self)
    }
}

/// Canonical independent Component Group deployments in raw deployment-ID order.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupDeploymentTopology {
    pub component_group_deployments: Vec<ComponentGroupDeploymentSpec>,
}

impl ComponentGroupDeploymentTopology {
    /// Compile source deployment selections and prove their exact group projection and demand.
    pub fn compile(config: &ConfigModel) -> Result<Self, ComponentGroupDeploymentTopologyError> {
        let component_topology = config.compile_component_topology()?;
        let component_group_topology = config.compile_component_group_topology()?;
        Self::compile_from_topologies(config, &component_group_topology, &component_topology)
    }

    /// Compile from the exact topologies already validated by the config boundary.
    pub(super) fn compile_from_topologies(
        config: &ConfigModel,
        component_group_topology: &ComponentGroupTopology,
        component_topology: &ComponentTopology,
    ) -> Result<Self, ComponentGroupDeploymentTopologyError> {
        validate_deployment_count(config.component_group_deployments.len())?;
        let mut component_group_deployments =
            Vec::with_capacity(config.component_group_deployments.len());
        let mut flattened_member_count = 0_usize;

        for (deployment, source) in &config.component_group_deployments {
            validate_placement_envelope(deployment, source)?;
            let deployment_labels = source_labels(&source.labels);
            validate_deployment_labels(deployment, &deployment_labels)?;
            let flattened = component_group_topology
                .flatten(&source.component_group)
                .map_err(ComponentGroupDeploymentTopologyError::ComponentGroupTopology)?;
            validate_deployment_service_purpose(deployment, source.service_purpose, &flattened)?;
            let member_limits = member_limit::compile_member_limits(
                deployment,
                &source.member_limits,
                &flattened.components,
                component_topology,
            )?;
            flattened_member_count = flattened_member_count
                .checked_add(flattened.components.len())
                .ok_or(
                    ComponentGroupDeploymentTopologyError::DeploymentMemberBoundExceeded {
                        actual: usize::MAX,
                        maximum: MAX_COMPONENT_GROUP_DEPLOYMENT_MEMBERS,
                    },
                )?;
            if flattened_member_count > MAX_COMPONENT_GROUP_DEPLOYMENT_MEMBERS {
                return Err(
                    ComponentGroupDeploymentTopologyError::DeploymentMemberBoundExceeded {
                        actual: flattened_member_count,
                        maximum: MAX_COMPONENT_GROUP_DEPLOYMENT_MEMBERS,
                    },
                );
            }
            let members = flattened
                .components
                .into_iter()
                .map(|member| {
                    let purpose =
                        resolve_member_purpose(deployment, source.service_purpose, &member)?;
                    let labels = resolve_effective_labels(
                        deployment,
                        &member.member_path,
                        &deployment_labels,
                        &member.labels,
                    )?;
                    let component_spec = component_topology
                        .get(&member.component_spec)
                        .ok_or_else(|| {
                            ComponentGroupDeploymentTopologyError::UnknownComponentSpec {
                                deployment: deployment.clone(),
                                component_spec: member.component_spec.clone(),
                            }
                        })?;
                    let limits = member_limit::effective_limits(
                        component_spec,
                        &member.member_path,
                        &member_limits,
                    );
                    Ok(FlattenedComponentGroupDeploymentMember {
                        member_path: member.member_path,
                        component_spec: member.component_spec,
                        component_spec_hash: component_spec.spec_hash,
                        purpose,
                        labels,
                        limits,
                    })
                })
                .collect::<Result<Vec<_>, ComponentGroupDeploymentTopologyError>>()?;
            component_group_deployments.push(ComponentGroupDeploymentSpec {
                deployment: deployment.clone(),
                component_group: source.component_group.clone(),
                service_purpose: source.service_purpose,
                labels: deployment_labels,
                member_limits,
                initial_placements: source.initial_placements,
                maximum_placements: source.maximum_placements,
                placement: ComponentGroupPlacementPolicy {
                    maximum_per_root: source.placement.maximum_per_root,
                    minimum_distinct_roots: source.placement.minimum_distinct_roots,
                },
                members,
            });
        }

        let topology = Self {
            component_group_deployments,
        };
        topology.validate(component_group_topology, component_topology)?;
        Ok(topology)
    }

    /// Return one exact canonical deployment projection.
    #[must_use]
    pub fn get(
        &self,
        deployment: &ComponentGroupDeploymentId,
    ) -> Option<&ComponentGroupDeploymentSpec> {
        self.component_group_deployments
            .binary_search_by(|candidate| candidate.deployment.cmp(deployment))
            .ok()
            .map(|index| &self.component_group_deployments[index])
    }

    /// Revalidate a decoded deployment projection against both source authority graphs.
    pub fn validate(
        &self,
        component_group_topology: &ComponentGroupTopology,
        component_topology: &ComponentTopology,
    ) -> Result<(), ComponentGroupDeploymentTopologyError> {
        component_group_topology.canonical_bytes()?;
        component_topology.canonical_bytes()?;
        validate_deployment_count(self.component_group_deployments.len())?;
        let mut previous_deployment: Option<&ComponentGroupDeploymentId> = None;
        let mut validation = DeploymentValidationLedger::new(component_topology);

        for deployment in &self.component_group_deployments {
            if previous_deployment.is_some_and(|previous| previous >= &deployment.deployment) {
                return Err(
                    ComponentGroupDeploymentTopologyError::NonCanonicalDeploymentOrder {
                        deployment: deployment.deployment.clone(),
                    },
                );
            }
            previous_deployment = Some(&deployment.deployment);
            validate_deployment_projection(
                deployment,
                component_group_topology,
                component_topology,
                &mut validation,
            )?;
        }

        validation.validate_spec_maxima(component_topology)
    }
}

/// One canonical independently scalable Component Group selection.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupDeploymentSpec {
    pub deployment: ComponentGroupDeploymentId,
    pub component_group: ComponentGroupSpecId,
    pub service_purpose: Option<FleetServiceMemberPurpose>,
    pub labels: Vec<ComponentDeploymentLabel>,
    pub member_limits: Vec<ComponentDeploymentMemberLimit>,
    pub initial_placements: u32,
    pub maximum_placements: u32,
    pub placement: ComponentGroupPlacementPolicy,
    pub members: Vec<FlattenedComponentGroupDeploymentMember>,
}

/// Protected density and spread envelope before concrete roots are selected.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupPlacementPolicy {
    pub maximum_per_root: u32,
    pub minimum_distinct_roots: u32,
}

/// One exact flattened Component occurrence within an independent deployment.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlattenedComponentGroupDeploymentMember {
    pub member_path: ComponentGroupMemberPath,
    pub component_spec: ComponentSpecId,
    pub component_spec_hash: [u8; 32],
    pub purpose: ComponentDeploymentPurpose,
    pub labels: Vec<ComponentDeploymentLabel>,
    pub limits: ComponentDeploymentLimits,
}

/// Exact typed purpose resolved for one flattened deployment occurrence.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ComponentDeploymentPurpose {
    Ordinary,
    FleetServiceMember {
        service: FleetServiceId,
        member_purpose: FleetServiceMemberPurpose,
    },
}

/// Typed rejection for invalid Component Group deployment compilation.
#[derive(Debug, ThisError)]
pub enum ComponentGroupDeploymentTopologyError {
    #[error(transparent)]
    ComponentGroupTopology(#[from] ComponentGroupTopologyError),

    #[error(transparent)]
    ComponentTopology(#[from] ComponentTopologyError),

    #[error(transparent)]
    MemberLimit(#[from] ComponentDeploymentMemberLimitError),

    #[error("Component Group deployment count {actual} exceeds bound {maximum}")]
    DeploymentBoundExceeded { actual: usize, maximum: usize },

    #[error("flattened Component Group deployment member count {actual} exceeds bound {maximum}")]
    DeploymentMemberBoundExceeded { actual: usize, maximum: usize },

    #[error("Component Group deployment '{deployment}' has zero maximum placements")]
    ZeroMaximumPlacements {
        deployment: ComponentGroupDeploymentId,
    },

    #[error(
        "Component Group deployment '{deployment}' initial placements {initial} exceed maximum {maximum}"
    )]
    InitialPlacementsExceedMaximum {
        deployment: ComponentGroupDeploymentId,
        initial: u32,
        maximum: u32,
    },

    #[error("Component Group deployment '{deployment}' has zero maximum placements per root")]
    ZeroMaximumPerRoot {
        deployment: ComponentGroupDeploymentId,
    },

    #[error(
        "Component Group deployment '{deployment}' maximum placements per root {maximum_per_root} exceed deployment maximum {maximum_placements}"
    )]
    MaximumPerRootExceedsMaximumPlacements {
        deployment: ComponentGroupDeploymentId,
        maximum_per_root: u32,
        maximum_placements: u32,
    },

    #[error("Component Group deployment '{deployment}' has zero minimum distinct roots")]
    ZeroMinimumDistinctRoots {
        deployment: ComponentGroupDeploymentId,
    },

    #[error(
        "Component Group deployment '{deployment}' minimum distinct roots {minimum_distinct_roots} exceed deployment maximum {maximum_placements}"
    )]
    MinimumDistinctRootsExceedMaximumPlacements {
        deployment: ComponentGroupDeploymentId,
        minimum_distinct_roots: u32,
        maximum_placements: u32,
    },

    #[error("Component Group deployment '{deployment}' is not in canonical order")]
    NonCanonicalDeploymentOrder {
        deployment: ComponentGroupDeploymentId,
    },

    #[error("Component Group deployment '{deployment}' does not exactly match its flattened group")]
    MemberProjectionMismatch {
        deployment: ComponentGroupDeploymentId,
    },

    #[error(
        "Component Group deployment '{deployment}' references unknown Component Spec '{component_spec}'"
    )]
    UnknownComponentSpec {
        deployment: ComponentGroupDeploymentId,
        component_spec: ComponentSpecId,
    },

    #[error(
        "Component Group deployment '{deployment}' has the wrong hash for Component Spec '{component_spec}'"
    )]
    ComponentSpecHashMismatch {
        deployment: ComponentGroupDeploymentId,
        component_spec: ComponentSpecId,
        expected: [u8; 32],
        received: [u8; 32],
    },

    #[error(
        "Component Group deployment '{deployment}' assigns Fleet-service purpose without a service-bearing member"
    )]
    InapplicableServicePurposeAssignment {
        deployment: ComponentGroupDeploymentId,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member_path:?}' for service '{service}' has no Fleet-service purpose assignment"
    )]
    MissingServicePurposeAssignment {
        deployment: ComponentGroupDeploymentId,
        member_path: ComponentGroupMemberPath,
        service: FleetServiceId,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member_path:?}' for service '{service}' has {actual} Fleet-service purpose assignments; expected exactly one"
    )]
    MultipleServicePurposeAssignments {
        deployment: ComponentGroupDeploymentId,
        member_path: ComponentGroupMemberPath,
        service: FleetServiceId,
        actual: usize,
    },

    #[error("Component Group deployment '{deployment}' has {actual} labels; maximum is {maximum}")]
    LabelBoundExceeded {
        deployment: ComponentGroupDeploymentId,
        actual: usize,
        maximum: usize,
    },

    #[error(
        "Component Group deployment '{deployment}' label '{label}' is duplicated or not in canonical order"
    )]
    NonCanonicalLabelOrder {
        deployment: ComponentGroupDeploymentId,
        label: ComponentDeploymentLabelKey,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member_path:?}' repeats label key '{label}'"
    )]
    DuplicateEffectiveLabel {
        deployment: ComponentGroupDeploymentId,
        member_path: ComponentGroupMemberPath,
        label: ComponentDeploymentLabelKey,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member_path:?}' has {actual} effective labels; maximum is {maximum}"
    )]
    EffectiveLabelBoundExceeded {
        deployment: ComponentGroupDeploymentId,
        member_path: ComponentGroupMemberPath,
        actual: usize,
        maximum: usize,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member_path:?}' has mismatched effective labels"
    )]
    MemberLabelProjectionMismatch {
        deployment: ComponentGroupDeploymentId,
        member_path: ComponentGroupMemberPath,
    },

    #[error("Component Group deployment demand overflowed for Component Spec '{component_spec}'")]
    ComponentSpecDemandOverflow { component_spec: ComponentSpecId },

    #[error(
        "Component Group deployment demand {required} exceeds Component Spec '{component_spec}' Fleet maximum {maximum_fleet_instances}"
    )]
    ComponentSpecDemandExceedsMaximum {
        component_spec: ComponentSpecId,
        required: u32,
        maximum_fleet_instances: u32,
    },
}

struct DeploymentValidationLedger {
    flattened_member_count: usize,
    spec_demand: BTreeMap<ComponentSpecId, u32>,
}

impl DeploymentValidationLedger {
    fn new(component_topology: &ComponentTopology) -> Self {
        Self {
            flattened_member_count: 0,
            spec_demand: component_topology
                .component_specs
                .iter()
                .map(|spec| (spec.component_spec.clone(), 0_u32))
                .collect(),
        }
    }

    fn record_member_count(
        &mut self,
        member_count: usize,
    ) -> Result<(), ComponentGroupDeploymentTopologyError> {
        self.flattened_member_count = self
            .flattened_member_count
            .checked_add(member_count)
            .ok_or(
                ComponentGroupDeploymentTopologyError::DeploymentMemberBoundExceeded {
                    actual: usize::MAX,
                    maximum: MAX_COMPONENT_GROUP_DEPLOYMENT_MEMBERS,
                },
            )?;
        if self.flattened_member_count > MAX_COMPONENT_GROUP_DEPLOYMENT_MEMBERS {
            return Err(
                ComponentGroupDeploymentTopologyError::DeploymentMemberBoundExceeded {
                    actual: self.flattened_member_count,
                    maximum: MAX_COMPONENT_GROUP_DEPLOYMENT_MEMBERS,
                },
            );
        }
        Ok(())
    }

    fn record_spec_demand(
        &mut self,
        deployment: &ComponentGroupDeploymentSpec,
        component_spec: &ComponentSpecId,
    ) -> Result<(), ComponentGroupDeploymentTopologyError> {
        let demand = self.spec_demand.get_mut(component_spec).ok_or_else(|| {
            ComponentGroupDeploymentTopologyError::UnknownComponentSpec {
                deployment: deployment.deployment.clone(),
                component_spec: component_spec.clone(),
            }
        })?;
        *demand = demand
            .checked_add(deployment.maximum_placements)
            .ok_or_else(
                || ComponentGroupDeploymentTopologyError::ComponentSpecDemandOverflow {
                    component_spec: component_spec.clone(),
                },
            )?;
        Ok(())
    }

    fn validate_spec_maxima(
        &self,
        component_topology: &ComponentTopology,
    ) -> Result<(), ComponentGroupDeploymentTopologyError> {
        for component_spec in &component_topology.component_specs {
            let required = self.spec_demand[&component_spec.component_spec];
            if required > component_spec.maximum_fleet_instances {
                return Err(
                    ComponentGroupDeploymentTopologyError::ComponentSpecDemandExceedsMaximum {
                        component_spec: component_spec.component_spec.clone(),
                        required,
                        maximum_fleet_instances: component_spec.maximum_fleet_instances,
                    },
                );
            }
        }
        Ok(())
    }
}

fn validate_deployment_projection(
    deployment: &ComponentGroupDeploymentSpec,
    component_group_topology: &ComponentGroupTopology,
    component_topology: &ComponentTopology,
    validation: &mut DeploymentValidationLedger,
) -> Result<(), ComponentGroupDeploymentTopologyError> {
    validate_compiled_placement_envelope(deployment)?;
    validate_deployment_labels(&deployment.deployment, &deployment.labels)?;
    let expected = component_group_topology
        .flatten(&deployment.component_group)
        .map_err(ComponentGroupDeploymentTopologyError::ComponentGroupTopology)?;
    validate_deployment_service_purpose(
        &deployment.deployment,
        deployment.service_purpose,
        &expected,
    )?;
    member_limit::validate_member_limits(
        &deployment.deployment,
        &deployment.member_limits,
        &expected.components,
        component_topology,
    )?;
    if expected.components.len() != deployment.members.len() {
        return Err(
            ComponentGroupDeploymentTopologyError::MemberProjectionMismatch {
                deployment: deployment.deployment.clone(),
            },
        );
    }
    validation.record_member_count(deployment.members.len())?;

    for (member, expected_member) in deployment.members.iter().zip(expected.components) {
        let expected_purpose = resolve_member_purpose(
            &deployment.deployment,
            deployment.service_purpose,
            &expected_member,
        )?;
        if !member_projection_matches(member, &expected_member, &expected_purpose) {
            return Err(
                ComponentGroupDeploymentTopologyError::MemberProjectionMismatch {
                    deployment: deployment.deployment.clone(),
                },
            );
        }
        let expected_labels = resolve_effective_labels(
            &deployment.deployment,
            &member.member_path,
            &deployment.labels,
            &expected_member.labels,
        )?;
        if member.labels != expected_labels {
            return Err(
                ComponentGroupDeploymentTopologyError::MemberLabelProjectionMismatch {
                    deployment: deployment.deployment.clone(),
                    member_path: member.member_path.clone(),
                },
            );
        }
        let component_spec = component_topology
            .get(&member.component_spec)
            .ok_or_else(
                || ComponentGroupDeploymentTopologyError::UnknownComponentSpec {
                    deployment: deployment.deployment.clone(),
                    component_spec: member.component_spec.clone(),
                },
            )?;
        if component_spec.spec_hash != member.component_spec_hash {
            return Err(
                ComponentGroupDeploymentTopologyError::ComponentSpecHashMismatch {
                    deployment: deployment.deployment.clone(),
                    component_spec: member.component_spec.clone(),
                    expected: component_spec.spec_hash,
                    received: member.component_spec_hash,
                },
            );
        }
        let expected_limits = member_limit::effective_limits(
            component_spec,
            &member.member_path,
            &deployment.member_limits,
        );
        if member.limits != expected_limits {
            return Err(
                ComponentDeploymentMemberLimitError::EffectiveLimitProjectionMismatch {
                    deployment: deployment.deployment.clone(),
                    member: member.member_path.clone(),
                }
                .into(),
            );
        }
        validation.record_spec_demand(deployment, &member.component_spec)?;
    }
    Ok(())
}

fn member_projection_matches(
    member: &FlattenedComponentGroupDeploymentMember,
    expected: &FlattenedComponentGroupMember,
    expected_purpose: &ComponentDeploymentPurpose,
) -> bool {
    member.member_path == expected.member_path
        && member.component_spec == expected.component_spec
        && member.purpose == *expected_purpose
}

fn validate_deployment_labels(
    deployment: &ComponentGroupDeploymentId,
    labels: &[ComponentDeploymentLabel],
) -> Result<(), ComponentGroupDeploymentTopologyError> {
    if labels.len() > MAX_COMPONENT_DEPLOYMENT_LABELS {
        return Err(ComponentGroupDeploymentTopologyError::LabelBoundExceeded {
            deployment: deployment.clone(),
            actual: labels.len(),
            maximum: MAX_COMPONENT_DEPLOYMENT_LABELS,
        });
    }
    let mut previous: Option<&ComponentDeploymentLabelKey> = None;
    for label in labels {
        if previous.is_some_and(|key| key >= &label.key) {
            return Err(
                ComponentGroupDeploymentTopologyError::NonCanonicalLabelOrder {
                    deployment: deployment.clone(),
                    label: label.key.clone(),
                },
            );
        }
        previous = Some(&label.key);
    }
    Ok(())
}

fn resolve_effective_labels(
    deployment: &ComponentGroupDeploymentId,
    member_path: &ComponentGroupMemberPath,
    deployment_labels: &[ComponentDeploymentLabel],
    member_labels: &[ComponentDeploymentLabel],
) -> Result<Vec<ComponentDeploymentLabel>, ComponentGroupDeploymentTopologyError> {
    let mut labels = BTreeMap::new();
    for label in deployment_labels.iter().chain(member_labels) {
        if labels.insert(&label.key, &label.value).is_some() {
            return Err(
                ComponentGroupDeploymentTopologyError::DuplicateEffectiveLabel {
                    deployment: deployment.clone(),
                    member_path: member_path.clone(),
                    label: label.key.clone(),
                },
            );
        }
    }
    if labels.len() > MAX_COMPONENT_DEPLOYMENT_LABELS {
        return Err(
            ComponentGroupDeploymentTopologyError::EffectiveLabelBoundExceeded {
                deployment: deployment.clone(),
                member_path: member_path.clone(),
                actual: labels.len(),
                maximum: MAX_COMPONENT_DEPLOYMENT_LABELS,
            },
        );
    }
    Ok(labels
        .into_iter()
        .map(|(key, value)| ComponentDeploymentLabel {
            key: key.clone(),
            value: value.clone(),
        })
        .collect())
}

fn validate_deployment_service_purpose(
    deployment: &ComponentGroupDeploymentId,
    service_purpose: Option<FleetServiceMemberPurpose>,
    flattened: &FlattenedComponentGroup,
) -> Result<(), ComponentGroupDeploymentTopologyError> {
    if service_purpose.is_some()
        && !flattened
            .components
            .iter()
            .any(FlattenedComponentGroupMember::is_fleet_service)
    {
        return Err(
            ComponentGroupDeploymentTopologyError::InapplicableServicePurposeAssignment {
                deployment: deployment.clone(),
            },
        );
    }
    Ok(())
}

fn resolve_member_purpose(
    deployment: &ComponentGroupDeploymentId,
    deployment_purpose: Option<FleetServiceMemberPurpose>,
    member: &FlattenedComponentGroupMember,
) -> Result<ComponentDeploymentPurpose, ComponentGroupDeploymentTopologyError> {
    let ComponentGroupLeafKind::FleetService { service } = &member.kind else {
        return Ok(ComponentDeploymentPurpose::Ordinary);
    };
    let assignment_count =
        member.service_purpose_assignments.len() + usize::from(deployment_purpose.is_some());
    if assignment_count == 0 {
        return Err(
            ComponentGroupDeploymentTopologyError::MissingServicePurposeAssignment {
                deployment: deployment.clone(),
                member_path: member.member_path.clone(),
                service: service.clone(),
            },
        );
    }
    if assignment_count > 1 {
        return Err(
            ComponentGroupDeploymentTopologyError::MultipleServicePurposeAssignments {
                deployment: deployment.clone(),
                member_path: member.member_path.clone(),
                service: service.clone(),
                actual: assignment_count,
            },
        );
    }
    let member_purpose = deployment_purpose
        .or_else(|| member.service_purpose_assignments.first().copied())
        .ok_or_else(
            || ComponentGroupDeploymentTopologyError::MissingServicePurposeAssignment {
                deployment: deployment.clone(),
                member_path: member.member_path.clone(),
                service: service.clone(),
            },
        )?;
    Ok(ComponentDeploymentPurpose::FleetServiceMember {
        service: service.clone(),
        member_purpose,
    })
}

const fn validate_deployment_count(
    deployment_count: usize,
) -> Result<(), ComponentGroupDeploymentTopologyError> {
    if deployment_count > MAX_COMPONENT_GROUP_DEPLOYMENTS {
        return Err(
            ComponentGroupDeploymentTopologyError::DeploymentBoundExceeded {
                actual: deployment_count,
                maximum: MAX_COMPONENT_GROUP_DEPLOYMENTS,
            },
        );
    }
    Ok(())
}

fn validate_placement_envelope(
    deployment: &ComponentGroupDeploymentId,
    source: &ComponentGroupDeploymentConfig,
) -> Result<(), ComponentGroupDeploymentTopologyError> {
    validate_placement_values(
        deployment,
        source.initial_placements,
        source.maximum_placements,
        source.placement.maximum_per_root,
        source.placement.minimum_distinct_roots,
    )
}

fn validate_compiled_placement_envelope(
    deployment: &ComponentGroupDeploymentSpec,
) -> Result<(), ComponentGroupDeploymentTopologyError> {
    validate_placement_values(
        &deployment.deployment,
        deployment.initial_placements,
        deployment.maximum_placements,
        deployment.placement.maximum_per_root,
        deployment.placement.minimum_distinct_roots,
    )
}

fn validate_placement_values(
    deployment: &ComponentGroupDeploymentId,
    initial_placements: u32,
    maximum_placements: u32,
    maximum_per_root: u32,
    minimum_distinct_roots: u32,
) -> Result<(), ComponentGroupDeploymentTopologyError> {
    if maximum_placements == 0 {
        return Err(
            ComponentGroupDeploymentTopologyError::ZeroMaximumPlacements {
                deployment: deployment.clone(),
            },
        );
    }
    if initial_placements > maximum_placements {
        return Err(
            ComponentGroupDeploymentTopologyError::InitialPlacementsExceedMaximum {
                deployment: deployment.clone(),
                initial: initial_placements,
                maximum: maximum_placements,
            },
        );
    }
    if maximum_per_root == 0 {
        return Err(ComponentGroupDeploymentTopologyError::ZeroMaximumPerRoot {
            deployment: deployment.clone(),
        });
    }
    if maximum_per_root > maximum_placements {
        return Err(
            ComponentGroupDeploymentTopologyError::MaximumPerRootExceedsMaximumPlacements {
                deployment: deployment.clone(),
                maximum_per_root,
                maximum_placements,
            },
        );
    }
    if minimum_distinct_roots == 0 {
        return Err(
            ComponentGroupDeploymentTopologyError::ZeroMinimumDistinctRoots {
                deployment: deployment.clone(),
            },
        );
    }
    if minimum_distinct_roots > maximum_placements {
        return Err(
            ComponentGroupDeploymentTopologyError::MinimumDistinctRootsExceedMaximumPlacements {
                deployment: deployment.clone(),
                minimum_distinct_roots,
                maximum_placements,
            },
        );
    }
    Ok(())
}
