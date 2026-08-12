//! Module: fleet_install_plan::initial_placement_policy
//!
//! Responsibility: purely validate complete initial Component Group placement assignments.
//! Does not own: file paths, serialization, durable publication, artifact projection, or effects.
//! Boundary: compiled deployment/service policy plus planned roots yields acceptance or typed error.

use super::model::PlannedFleetSubnetRoot;
use std::collections::BTreeMap;

use canic_core::{
    bootstrap::compiled::ConfigModel,
    control_plane_support::config::{
        ComponentDeploymentPurpose, ComponentGroupDeploymentSpec, FleetServiceTopology,
    },
    ids::{ComponentGroupDeploymentId, ComponentSpecId, FleetServiceId, SubnetId},
};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub(super) enum InitialPlacementPolicyError {
    #[error("{0}")]
    Configuration(String),

    #[error("root {root} assignments are not strictly canonical")]
    NonCanonicalRootAssignments { root: SubnetId },

    #[error("root {root} exceeds maximum_group_placements")]
    RootPlacementLimitExceeded { root: SubnetId },

    #[error("root {root} references unknown deployment '{deployment}'")]
    UnknownDeployment {
        root: SubnetId,
        deployment: ComponentGroupDeploymentId,
    },

    #[error("deployment '{deployment}' ordinal {ordinal} is outside its initial placement set")]
    OrdinalOutsideInitialSet {
        deployment: ComponentGroupDeploymentId,
        ordinal: u32,
    },

    #[error("deployment '{deployment}' ordinal {ordinal} is assigned more than once")]
    DuplicateAssignment {
        deployment: ComponentGroupDeploymentId,
        ordinal: u32,
    },

    #[error("{subject} count overflowed")]
    CountOverflow { subject: &'static str },

    #[error("{subject} count does not fit u32")]
    CountDoesNotFitU32 { subject: &'static str },

    #[error("root {root} initial Components exceed protected root capacity")]
    RootComponentCapacityExceeded { root: SubnetId },

    #[error(
        "root {root} initial atomic Component batch requires {required} Ready prepaid Canisters but its configured minimum/import target is {available}"
    )]
    ReadyAssetCapacityExceeded {
        root: SubnetId,
        required: u32,
        available: u32,
    },

    #[error("root {root} does not admit Component Spec '{component_spec}'")]
    MissingComponentAdmission {
        root: SubnetId,
        component_spec: ComponentSpecId,
    },

    #[error("root {root} exceeds admission for Component Spec '{component_spec}'")]
    ComponentAdmissionExceeded {
        root: SubnetId,
        component_spec: ComponentSpecId,
    },

    #[error("deployment '{deployment}' does not assign every initial ordinal exactly once")]
    IncompleteDeploymentAssignments {
        deployment: ComponentGroupDeploymentId,
    },

    #[error("deployment '{deployment}' violates its root density or spread policy")]
    DeploymentPlacementPolicy {
        deployment: ComponentGroupDeploymentId,
    },

    #[error("Fleet service '{service}' violates its root density or spread policy")]
    ServicePlacementPolicy { service: FleetServiceId },
}

pub(super) fn validate_initial_component_group_assignments(
    config: &ConfigModel,
    roots: &[PlannedFleetSubnetRoot],
) -> Result<(), InitialPlacementPolicyError> {
    let configuration = config
        .compile_component_deployment_configuration()
        .map_err(|error| InitialPlacementPolicyError::Configuration(error.to_string()))?;
    let deployments = &configuration
        .deployment_topology
        .component_group_deployments;
    let mut assignments = BTreeMap::<
        (ComponentGroupDeploymentId, u32),
        (SubnetId, &ComponentGroupDeploymentSpec),
    >::new();
    let mut service_roots = BTreeMap::<FleetServiceId, BTreeMap<SubnetId, u32>>::new();

    for root in roots {
        validate_root_assignment_order(root)?;
        validate_root_initial_assignment_capacity(
            root,
            deployments,
            &mut assignments,
            &mut service_roots,
        )?;
    }

    for deployment in deployments {
        validate_deployment_assignment(deployment, &assignments)?;
    }
    validate_service_assignments(&configuration.fleet_service_topology, &service_roots)
}

fn validate_root_assignment_order(
    root: &PlannedFleetSubnetRoot,
) -> Result<(), InitialPlacementPolicyError> {
    let assignments_are_sorted = root.component_group_placements.is_sorted();
    let assignments_are_unique = root
        .component_group_placements
        .windows(2)
        .all(|window| window[0] != window[1]);
    if !assignments_are_sorted || !assignments_are_unique {
        return Err(InitialPlacementPolicyError::NonCanonicalRootAssignments {
            root: root.placement_subnet,
        });
    }
    if root.component_group_placements.len() > root.limits.maximum_group_placements as usize {
        return Err(InitialPlacementPolicyError::RootPlacementLimitExceeded {
            root: root.placement_subnet,
        });
    }
    Ok(())
}

fn validate_root_initial_assignment_capacity<'a>(
    root: &PlannedFleetSubnetRoot,
    deployments: &'a [ComponentGroupDeploymentSpec],
    assignments: &mut BTreeMap<
        (ComponentGroupDeploymentId, u32),
        (SubnetId, &'a ComponentGroupDeploymentSpec),
    >,
    service_roots: &mut BTreeMap<FleetServiceId, BTreeMap<SubnetId, u32>>,
) -> Result<(), InitialPlacementPolicyError> {
    let mut component_counts = BTreeMap::<ComponentSpecId, u32>::new();
    let mut component_count = 0_u32;
    for assignment in &root.component_group_placements {
        let deployment = deployments
            .binary_search_by(|candidate| candidate.deployment.cmp(&assignment.deployment))
            .ok()
            .map(|index| &deployments[index])
            .ok_or_else(|| InitialPlacementPolicyError::UnknownDeployment {
                root: root.placement_subnet,
                deployment: assignment.deployment.clone(),
            })?;
        if assignment.ordinal >= deployment.initial_placements {
            return Err(InitialPlacementPolicyError::OrdinalOutsideInitialSet {
                deployment: assignment.deployment.clone(),
                ordinal: assignment.ordinal,
            });
        }
        if assignments
            .insert(
                (assignment.deployment.clone(), assignment.ordinal),
                (root.placement_subnet, deployment),
            )
            .is_some()
        {
            return Err(InitialPlacementPolicyError::DuplicateAssignment {
                deployment: assignment.deployment.clone(),
                ordinal: assignment.ordinal,
            });
        }
        component_count = component_count
            .checked_add(u32::try_from(deployment.members.len()).map_err(|_| {
                InitialPlacementPolicyError::CountDoesNotFitU32 {
                    subject: "deployment member",
                }
            })?)
            .ok_or(InitialPlacementPolicyError::CountOverflow {
                subject: "root Component",
            })?;
        record_member_capacity(root, deployment, &mut component_counts, service_roots)?;
    }
    if component_count > root.limits.maximum_component_instances {
        return Err(InitialPlacementPolicyError::RootComponentCapacityExceeded {
            root: root.placement_subnet,
        });
    }
    validate_initial_pool_capacity(root, component_count)?;
    validate_component_admissions(root, component_counts)
}

fn record_member_capacity(
    root: &PlannedFleetSubnetRoot,
    deployment: &ComponentGroupDeploymentSpec,
    component_counts: &mut BTreeMap<ComponentSpecId, u32>,
    service_roots: &mut BTreeMap<FleetServiceId, BTreeMap<SubnetId, u32>>,
) -> Result<(), InitialPlacementPolicyError> {
    for member in &deployment.members {
        let count = component_counts
            .entry(member.component_spec.clone())
            .or_default();
        *count = count
            .checked_add(1)
            .ok_or(InitialPlacementPolicyError::CountOverflow {
                subject: "root Component Spec",
            })?;
        if let ComponentDeploymentPurpose::FleetServiceMember { service, .. } = &member.purpose {
            let count = service_roots
                .entry(service.clone())
                .or_default()
                .entry(root.placement_subnet)
                .or_default();
            *count = count
                .checked_add(1)
                .ok_or(InitialPlacementPolicyError::CountOverflow {
                    subject: "Fleet-service member",
                })?;
        }
    }
    Ok(())
}

fn validate_initial_pool_capacity(
    root: &PlannedFleetSubnetRoot,
    component_count: u32,
) -> Result<(), InitialPlacementPolicyError> {
    let imported_assets = u32::try_from(root.canister_pool_imports.len()).map_err(|_| {
        InitialPlacementPolicyError::CountDoesNotFitU32 {
            subject: "root Canister pool import",
        }
    })?;
    let automatic_ready_target = root.limits.canister_pool.minimum_size.max(imported_assets);
    if component_count > automatic_ready_target {
        return Err(InitialPlacementPolicyError::ReadyAssetCapacityExceeded {
            root: root.placement_subnet,
            required: component_count,
            available: automatic_ready_target,
        });
    }
    Ok(())
}

fn validate_component_admissions(
    root: &PlannedFleetSubnetRoot,
    component_counts: BTreeMap<ComponentSpecId, u32>,
) -> Result<(), InitialPlacementPolicyError> {
    for (component_spec, count) in component_counts {
        let admission = root
            .component_admissions
            .binary_search_by(|admission| admission.component_spec.cmp(&component_spec))
            .ok()
            .map(|index| &root.component_admissions[index])
            .ok_or_else(|| InitialPlacementPolicyError::MissingComponentAdmission {
                root: root.placement_subnet,
                component_spec: component_spec.clone(),
            })?;
        if count > admission.maximum_root_instances {
            return Err(InitialPlacementPolicyError::ComponentAdmissionExceeded {
                root: root.placement_subnet,
                component_spec,
            });
        }
    }
    Ok(())
}

fn validate_deployment_assignment(
    deployment: &ComponentGroupDeploymentSpec,
    assignments: &BTreeMap<
        (ComponentGroupDeploymentId, u32),
        (SubnetId, &ComponentGroupDeploymentSpec),
    >,
) -> Result<(), InitialPlacementPolicyError> {
    let matching = assignments
        .iter()
        .filter(|((candidate, _), _)| candidate == &deployment.deployment)
        .collect::<Vec<_>>();
    let ordinals_are_exact = matching
        .iter()
        .map(|((_, ordinal), _)| *ordinal)
        .eq(0..deployment.initial_placements);
    if matching.len() != deployment.initial_placements as usize || !ordinals_are_exact {
        return Err(
            InitialPlacementPolicyError::IncompleteDeploymentAssignments {
                deployment: deployment.deployment.clone(),
            },
        );
    }
    let mut root_counts = BTreeMap::<SubnetId, u32>::new();
    for (_, (root, _)) in matching {
        let count = root_counts.entry(*root).or_default();
        *count = count
            .checked_add(1)
            .ok_or(InitialPlacementPolicyError::CountOverflow {
                subject: "deployment root",
            })?;
    }
    let density_is_valid = root_counts
        .values()
        .all(|count| *count <= deployment.placement.maximum_per_root);
    let required_roots = deployment
        .initial_placements
        .min(deployment.placement.minimum_distinct_roots) as usize;
    if !density_is_valid || root_counts.len() < required_roots {
        return Err(InitialPlacementPolicyError::DeploymentPlacementPolicy {
            deployment: deployment.deployment.clone(),
        });
    }
    Ok(())
}

fn validate_service_assignments(
    topology: &FleetServiceTopology,
    service_roots: &BTreeMap<FleetServiceId, BTreeMap<SubnetId, u32>>,
) -> Result<(), InitialPlacementPolicyError> {
    for target in &topology.targets {
        let roots = service_roots
            .get(&target.service)
            .cloned()
            .unwrap_or_default();
        let density_is_valid = roots
            .values()
            .all(|count| *count <= target.placement.maximum_members_per_root);
        let members = roots.values().try_fold(0_u32, |total, count| {
            total
                .checked_add(*count)
                .ok_or(InitialPlacementPolicyError::CountOverflow {
                    subject: "Fleet-service member",
                })
        })?;
        let required_roots = members.min(target.placement.minimum_distinct_roots) as usize;
        if !density_is_valid || roots.len() < required_roots {
            return Err(InitialPlacementPolicyError::ServicePlacementPolicy {
                service: target.service.clone(),
            });
        }
    }
    Ok(())
}
