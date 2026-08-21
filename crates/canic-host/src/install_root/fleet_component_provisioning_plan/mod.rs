//! Module: install_root::fleet_component_provisioning_plan
//!
//! Responsibility: compile explicit root-local placement assignments into one canonical fresh-install plan.
//! Does not own: placement policy, durable progress, Coordinator effects, or runtime activation.
//! Boundary: immutable host input selects roots; checked-in configuration and the live Registry supply every protected field.

#[cfg(test)]
mod tests;

use crate::fleet_install_plan::{FleetInstallPlan, PlannedFleetSubnetRoot};
use std::collections::BTreeMap;

use canic_core::{
    bootstrap::compiled::ConfigModel,
    control_plane_support::{
        config::{ComponentDeploymentConfiguration, FlattenedComponentGroupDeploymentMember},
        ops::{
            component_provisioning_plan::ComponentProvisioningPlanOps,
            fleet_registry::FleetRegistryOps,
        },
    },
    dto::{
        component_provisioning::{
            ComponentGroupPlacementPlan, ComponentGroupPlanEntry,
            FleetComponentProvisioningOperation, FleetComponentProvisioningPlan,
            FleetComponentProvisioningPrepareRequest, FleetSubnetRootProvisioningBatch,
        },
        fleet_registry::{FleetRegistry, FleetSubnetRootEntry, FleetSubnetRootStatus},
    },
    ids::{
        ComponentGroupDeploymentId, ComponentGroupPlacementId, ComponentSpecAdmission,
        ComponentTopologyDigest, FleetSubnetRootBinding, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, SubnetId,
    },
};
use thiserror::Error as ThisError;

/// Complete validated fresh-install plan and its canonical identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CompiledFleetComponentProvisioningPlan {
    pub prepare_request: FleetComponentProvisioningPrepareRequest,
    pub plan_hash: [u8; 32],
}

/// Exact immutable authorities needed to compile initial Component placement.
pub(super) struct CompileFleetComponentProvisioningPlanRequest<'a> {
    pub config: &'a ConfigModel,
    pub fleet_install_plan: &'a FleetInstallPlan,
    pub registry: &'a FleetRegistry,
    pub operation_id: [u8; 32],
}

/// Typed rejection before a provisioning plan can become durable Coordinator intent.
#[derive(Debug, ThisError)]
pub(super) enum FleetComponentProvisioningPlanError {
    #[error("Component deployment configuration is invalid: {0}")]
    Configuration(String),

    #[error("Fleet installation authority differs from the live Fleet Registry")]
    FleetAuthorityMismatch,

    #[error("Fleet install plan root on Subnet {subnet} has no exact live Registry root")]
    MissingRegistryRoot { subnet: SubnetId },

    #[error("Fleet install plan and live Registry root sets differ")]
    RootSetMismatch,

    #[error("initial Component placement references unknown deployment '{deployment}'")]
    UnknownDeployment {
        deployment: ComponentGroupDeploymentId,
    },

    #[error("canonical Component provisioning plan is invalid: {0}")]
    InvalidPlan(String),
}

/// Compile and validate one exact fresh-install Component provisioning request.
pub(super) fn compile_fleet_component_provisioning_plan(
    request: CompileFleetComponentProvisioningPlanRequest<'_>,
) -> Result<CompiledFleetComponentProvisioningPlan, FleetComponentProvisioningPlanError> {
    let configuration = request
        .config
        .compile_component_deployment_configuration()
        .map_err(|error| FleetComponentProvisioningPlanError::Configuration(error.to_string()))?;
    let roots_by_subnet = registry_roots_by_subnet(request.registry);
    validate_fleet_authority(
        request.fleet_install_plan,
        request.registry,
        &roots_by_subnet,
    )?;
    let mut batches = request
        .fleet_install_plan
        .fleet_subnet_roots
        .iter()
        .map(|root| compile_root_batch(root, request.registry, &roots_by_subnet, &configuration))
        .collect::<Result<Vec<_>, _>>()?;
    batches.sort_unstable_by_key(|batch| batch.root.fleet_subnet_root);

    let mut directory_confirmation_roots = request
        .registry
        .fleet_subnet_roots
        .iter()
        .map(|root| root.fleet_subnet_root)
        .collect::<Vec<_>>();
    directory_confirmation_roots.sort_unstable();
    let fleet_registry = FleetRegistryOps::version(
        &request.registry.authority,
        &configuration.component_topology,
        request.registry,
    )
    .map_err(|error| FleetComponentProvisioningPlanError::InvalidPlan(error.to_string()))?;
    let configuration_digest = configuration
        .digest()
        .map_err(|error| FleetComponentProvisioningPlanError::Configuration(error.to_string()))?;
    let plan = FleetComponentProvisioningPlan {
        fleet: request.fleet_install_plan.fleet.clone(),
        fleet_registry,
        configuration_digest,
        operation: FleetComponentProvisioningOperation::FreshInstall,
        directory_confirmation_roots,
        batches,
    };
    let plan_hash =
        ComponentProvisioningPlanOps::hash_compiled(&configuration, request.registry, &plan)
            .map_err(|error| FleetComponentProvisioningPlanError::InvalidPlan(error.to_string()))?;
    Ok(CompiledFleetComponentProvisioningPlan {
        prepare_request: FleetComponentProvisioningPrepareRequest {
            operation_id: request.operation_id,
            plan,
        },
        plan_hash,
    })
}

fn validate_fleet_authority(
    plan: &FleetInstallPlan,
    registry: &FleetRegistry,
    roots_by_subnet: &BTreeMap<SubnetId, &FleetSubnetRootEntry>,
) -> Result<(), FleetComponentProvisioningPlanError> {
    let plan_matches_registry = plan.fleet == registry.authority.binding.fleet;
    let coordinator_subnet_matches =
        plan.coordinator.coordinator_subnet == registry.authority.binding.coordinator_subnet;
    let authority_matches = [plan_matches_registry, coordinator_subnet_matches]
        .into_iter()
        .all(std::convert::identity);
    if !authority_matches {
        return Err(FleetComponentProvisioningPlanError::FleetAuthorityMismatch);
    }
    if plan.fleet_subnet_roots.len() != registry.fleet_subnet_roots.len() {
        return Err(FleetComponentProvisioningPlanError::RootSetMismatch);
    }
    for planned in &plan.fleet_subnet_roots {
        let Some(registered) = roots_by_subnet.get(&planned.placement_subnet) else {
            return Err(FleetComponentProvisioningPlanError::RootSetMismatch);
        };
        let expected = PlannedRootAuthority::from_plan(planned);
        let observed = PlannedRootAuthority::from_registry(registered);
        if observed != expected {
            return Err(FleetComponentProvisioningPlanError::RootSetMismatch);
        }
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct PlannedRootAuthority<'a> {
    placement_subnet: SubnetId,
    component_admissions: &'a [ComponentSpecAdmission],
    component_topology_digest: ComponentTopologyDigest,
    active_release_set: FleetSubnetRootReleaseSet,
    limits: &'a FleetSubnetRootLimits,
    active: bool,
}

impl<'a> PlannedRootAuthority<'a> {
    fn from_plan(root: &'a PlannedFleetSubnetRoot) -> Self {
        Self {
            placement_subnet: root.placement_subnet,
            component_admissions: &root.component_admissions,
            component_topology_digest: root.component_topology_digest,
            active_release_set: root.initial_release_set,
            limits: &root.limits,
            active: true,
        }
    }

    fn from_registry(root: &'a FleetSubnetRootEntry) -> Self {
        Self {
            placement_subnet: root.placement_subnet,
            component_admissions: &root.component_admissions,
            component_topology_digest: root.component_topology_digest,
            active_release_set: root.active_release_set,
            limits: &root.limits,
            active: root.status == FleetSubnetRootStatus::Active,
        }
    }
}

fn registry_roots_by_subnet(registry: &FleetRegistry) -> BTreeMap<SubnetId, &FleetSubnetRootEntry> {
    registry
        .fleet_subnet_roots
        .iter()
        .map(|root| (root.placement_subnet, root))
        .collect()
}

fn compile_root_batch(
    planned: &PlannedFleetSubnetRoot,
    registry: &FleetRegistry,
    roots_by_subnet: &BTreeMap<SubnetId, &FleetSubnetRootEntry>,
    deployments: &ComponentDeploymentConfiguration,
) -> Result<FleetSubnetRootProvisioningBatch, FleetComponentProvisioningPlanError> {
    let registered = roots_by_subnet
        .get(&planned.placement_subnet)
        .copied()
        .ok_or(FleetComponentProvisioningPlanError::MissingRegistryRoot {
            subnet: planned.placement_subnet,
        })?;
    let mut placements = planned
        .component_group_placements
        .iter()
        .map(|assignment| {
            let deployment = deployments
                .deployment_topology
                .get(&assignment.deployment)
                .ok_or_else(|| FleetComponentProvisioningPlanError::UnknownDeployment {
                    deployment: assignment.deployment.clone(),
                })?;
            Ok(ComponentGroupPlacementPlan {
                group_placement: ComponentGroupPlacementId {
                    deployment: assignment.deployment.clone(),
                    ordinal: assignment.ordinal,
                },
                component_group: deployment.component_group.clone(),
                entries: deployment
                    .members
                    .iter()
                    .map(component_plan_entry)
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, FleetComponentProvisioningPlanError>>()?;
    placements.sort_unstable_by(|left, right| left.group_placement.cmp(&right.group_placement));
    Ok(FleetSubnetRootProvisioningBatch {
        root: registry_root_binding(registry, registered),
        active_release_set: registered.active_release_set,
        placements,
    })
}

fn component_plan_entry(
    member: &FlattenedComponentGroupDeploymentMember,
) -> ComponentGroupPlanEntry {
    ComponentGroupPlanEntry {
        member_path: member.member_path.clone(),
        component_spec: member.component_spec.clone(),
        spec_hash: member.component_spec_hash,
        purpose: member.purpose.clone(),
        labels: member.labels.clone(),
        limits: member.limits.clone(),
    }
}

fn registry_root_binding(
    registry: &FleetRegistry,
    root: &FleetSubnetRootEntry,
) -> FleetSubnetRootBinding {
    FleetSubnetRootBinding {
        authority: registry.authority.clone(),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        component_admissions: root.component_admissions.clone(),
        component_topology_digest: root.component_topology_digest,
        limits: root.limits.clone(),
        funding: root.funding.clone(),
    }
}
