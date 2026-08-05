//! Module: config::component_group_deployment
//!
//! Responsibility: compile independent Component Group deployments before planning.
//! Does not own: purpose, labels, member limits, root selection, persistence, or effects.
//! Boundary: strict source deployments become bounded exact flattened Component occurrences.

#[cfg(test)]
mod tests;

use crate::{
    config::{
        ComponentGroupTopology, ComponentGroupTopologyError, ComponentTopology,
        ComponentTopologyError, FlattenedComponentGroupMember,
        schema::{ComponentGroupDeploymentConfig, ConfigModel},
    },
    ids::{
        ComponentGroupDeploymentId, ComponentGroupMemberPath, ComponentGroupSpecId, ComponentSpecId,
    },
};
use std::collections::BTreeMap;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

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
            let flattened = component_group_topology
                .flatten(&source.component_group)
                .map_err(ComponentGroupDeploymentTopologyError::ComponentGroupTopology)?;
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
                    let component_spec = component_topology
                        .get(&member.component_spec)
                        .ok_or_else(|| {
                            ComponentGroupDeploymentTopologyError::UnknownComponentSpec {
                                deployment: deployment.clone(),
                                component_spec: member.component_spec.clone(),
                            }
                        })?;
                    Ok(FlattenedComponentGroupDeploymentMember {
                        member_path: member.member_path,
                        component_spec: member.component_spec,
                        component_spec_hash: component_spec.spec_hash,
                    })
                })
                .collect::<Result<Vec<_>, ComponentGroupDeploymentTopologyError>>()?;
            component_group_deployments.push(ComponentGroupDeploymentSpec {
                deployment: deployment.clone(),
                component_group: source.component_group.clone(),
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
}

/// Typed rejection for invalid Component Group deployment compilation.
#[derive(Debug, ThisError)]
pub enum ComponentGroupDeploymentTopologyError {
    #[error(transparent)]
    ComponentGroupTopology(#[from] ComponentGroupTopologyError),

    #[error(transparent)]
    ComponentTopology(#[from] ComponentTopologyError),

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
    let expected = component_group_topology
        .flatten(&deployment.component_group)
        .map_err(ComponentGroupDeploymentTopologyError::ComponentGroupTopology)?;
    if expected.components.len() != deployment.members.len() {
        return Err(
            ComponentGroupDeploymentTopologyError::MemberProjectionMismatch {
                deployment: deployment.deployment.clone(),
            },
        );
    }
    validation.record_member_count(deployment.members.len())?;

    for (member, expected_member) in deployment.members.iter().zip(expected.components) {
        if !member_projection_matches(member, &expected_member) {
            return Err(
                ComponentGroupDeploymentTopologyError::MemberProjectionMismatch {
                    deployment: deployment.deployment.clone(),
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
        validation.record_spec_demand(deployment, &member.component_spec)?;
    }
    Ok(())
}

fn member_projection_matches(
    member: &FlattenedComponentGroupDeploymentMember,
    expected: &FlattenedComponentGroupMember,
) -> bool {
    member.member_path == expected.member_path && member.component_spec == expected.component_spec
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
