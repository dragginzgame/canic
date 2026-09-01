//! Module: component_topology
//!
//! Responsibility: finalize Fleet Subnet Root topology bindings from config and explicit host input.
//! Does not own: Subnet selection, Canister creation, release sets, Registry commit, or runtime.
//! Boundary: accepts resolved authority/placement facts and emits validated immutable bindings.

#[cfg(test)]
mod tests;

use candid::Principal;
use canic_core::{
    bootstrap::compiled::{ComponentTopology, ConfigModel},
    ids::{
        ComponentSpecAdmission, ComponentSpecId, ComponentTopologyDigest, FleetRegistryAuthority,
        FleetSubnetRootBinding, FleetSubnetRootFundingAuthority, FleetSubnetRootLimits, SubnetId,
    },
};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

///
/// RootComponentAdmissionInput
///
/// Operator-planned positive instance capacity for one Spec on one resolved root.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentAdmissionInput {
    pub component_spec: ComponentSpecId,
    pub maximum_root_instances: u32,
}

///
/// FleetSubnetRootTopologyInput
///
/// Exact resolved physical root placement, admissions, and immutable aggregate limits.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetSubnetRootTopologyInput {
    pub placement_subnet: SubnetId,
    pub fleet_subnet_root: Principal,
    pub component_admissions: Vec<RootComponentAdmissionInput>,
    pub limits: FleetSubnetRootLimits,
    pub funding: FleetSubnetRootFundingAuthority,
}

///
/// PlannedFleetSubnetRootTopologyInput
///
/// Exact root placement, admissions, and limits resolved before Canister creation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedFleetSubnetRootTopologyInput {
    pub placement_subnet: SubnetId,
    pub component_admissions: Vec<RootComponentAdmissionInput>,
    pub limits: FleetSubnetRootLimits,
}

///
/// PlannedFleetSubnetRootTopology
///
/// Canonical pre-creation root topology with no fabricated Canister principal.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedFleetSubnetRootTopology {
    pub placement_subnet: SubnetId,
    pub component_admissions: Vec<ComponentSpecAdmission>,
    pub component_topology_digest: ComponentTopologyDigest,
    pub limits: FleetSubnetRootLimits,
}

///
/// PlannedFleetTopology
///
/// Canonical Component Topology and every validated pre-creation root plan.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedFleetTopology {
    pub component_topology: ComponentTopology,
    pub fleet_subnet_roots: Vec<PlannedFleetSubnetRootTopology>,
}

/// Root-local admitted Component demand checked against one empty-pool target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootPoolCapacityInput {
    pub component_admissions: Vec<ComponentSpecAdmission>,
    pub pool_target_cycles: u128,
    pub root: String,
}

/// Exact retained import count checked against one Root's initial pool capacity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootPoolImportCapacityInput {
    pub import_count: usize,
    pub maximum_size: u32,
    pub root: String,
}

/// Exact fail-closed mismatch between admitted Component demand and pool capacity.
#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum RootPoolCapacityError {
    #[error(
        "Root {root} pool target {pool_target_cycles} cycles is below admitted Component Spec '{component_spec}' initial demand {required_cycles} cycles"
    )]
    Insufficient {
        component_spec: ComponentSpecId,
        pool_target_cycles: u128,
        required_cycles: u128,
        root: String,
    },

    #[error("Root {root} admits unknown Component Spec '{component_spec}'")]
    UnknownComponentSpec {
        component_spec: ComponentSpecId,
        root: String,
    },
}

/// Exact fail-closed mismatch between retained imports and Root initialisation capacity.
#[derive(Debug, Eq, PartialEq, ThisError)]
#[error(
    "Root {root} has {import_count} retained pool imports, above initialisation maximum {maximum_size}"
)]
pub struct RootPoolImportCapacityError {
    pub import_count: usize,
    pub maximum_size: u32,
    pub root: String,
}

///
/// FleetTopologyPlan
///
/// Canonical Fleet topology plus every validated root-local binding.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetTopologyPlan {
    pub component_topology: ComponentTopology,
    pub fleet_subnet_roots: Vec<FleetSubnetRootBinding>,
}

///
/// FleetTopologyPlanError
///
/// Typed rejection while finalizing immutable root topology input.
///

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum FleetTopologyPlanError {
    #[error(
        "Fleet authority App '{authority_app}' does not match configured App '{configured_app}'"
    )]
    AppMismatch {
        configured_app: String,
        authority_app: String,
    },

    #[error("root input repeats Component Spec admission '{component_spec}'")]
    DuplicateAdmission { component_spec: ComponentSpecId },

    #[error("root input repeats placement Subnet '{placement_subnet}'")]
    DuplicatePlacementSubnet { placement_subnet: SubnetId },

    #[error("root placement Subnet must not be anonymous")]
    AnonymousPlacementSubnet,

    #[error(transparent)]
    Topology(#[from] canic_core::bootstrap::compiled::ComponentTopologyError),

    #[error("root input references unknown Component Spec '{component_spec}'")]
    UnknownComponentSpec { component_spec: ComponentSpecId },
}

/// Reject any admitted Component whose exact initial demand exceeds its Root's pool target.
pub fn validate_root_pool_capacity(
    config: &ConfigModel,
    roots: &[RootPoolCapacityInput],
) -> Result<(), RootPoolCapacityError> {
    for root in roots {
        for admission in &root.component_admissions {
            let component = config
                .component_specs
                .get(&admission.component_spec)
                .ok_or_else(|| RootPoolCapacityError::UnknownComponentSpec {
                    component_spec: admission.component_spec.clone(),
                    root: root.root.clone(),
                })?;
            let required_cycles = component.initial_cycles.to_u128();
            if required_cycles > root.pool_target_cycles {
                return Err(RootPoolCapacityError::Insufficient {
                    component_spec: admission.component_spec.clone(),
                    pool_target_cycles: root.pool_target_cycles,
                    required_cycles,
                    root: root.root.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Reject a Root bootstrap whose complete retained import set cannot be initialised.
pub fn validate_root_pool_import_capacity(
    input: &RootPoolImportCapacityInput,
) -> Result<(), RootPoolImportCapacityError> {
    if let Ok(import_count) = u32::try_from(input.import_count)
        && import_count <= input.maximum_size
    {
        return Ok(());
    }
    Err(RootPoolImportCapacityError {
        import_count: input.import_count,
        maximum_size: input.maximum_size,
        root: input.root.clone(),
    })
}

/// Finalize canonical pre-creation root plans without inventing Canister principals.
pub fn plan_initial_fleet_topology(
    config: &ConfigModel,
    root_inputs: Vec<PlannedFleetSubnetRootTopologyInput>,
) -> Result<PlannedFleetTopology, FleetTopologyPlanError> {
    let component_topology = config.compile_component_topology()?;
    let mut fleet_subnet_roots = root_inputs
        .into_iter()
        .map(|input| finalize_planned_root(&component_topology, input))
        .collect::<Result<Vec<_>, _>>()?;
    fleet_subnet_roots.sort_by_key(|root| root.placement_subnet);

    let mut placement_subnets = BTreeSet::new();
    for root in &fleet_subnet_roots {
        if root.placement_subnet.as_principal() == &Principal::anonymous() {
            return Err(FleetTopologyPlanError::AnonymousPlacementSubnet);
        }
        if !placement_subnets.insert(root.placement_subnet) {
            return Err(FleetTopologyPlanError::DuplicatePlacementSubnet {
                placement_subnet: root.placement_subnet,
            });
        }
        component_topology.validate_planned_root(
            &root.component_admissions,
            root.component_topology_digest,
            &root.limits,
        )?;
    }
    let admissions = fleet_subnet_roots
        .iter()
        .map(|root| root.component_admissions.as_slice())
        .collect::<Vec<_>>();
    component_topology.validate_fleet_admissions(&admissions)?;

    Ok(PlannedFleetTopology {
        component_topology,
        fleet_subnet_roots,
    })
}

/// Finalize canonical root bindings without accepting caller-supplied hashes or digests.
pub fn plan_fleet_topology(
    config: &ConfigModel,
    authority: FleetRegistryAuthority,
    root_inputs: Vec<FleetSubnetRootTopologyInput>,
) -> Result<FleetTopologyPlan, FleetTopologyPlanError> {
    if config.app_id() != &authority.binding.fleet.app {
        return Err(FleetTopologyPlanError::AppMismatch {
            configured_app: config.app_id().to_string(),
            authority_app: authority.binding.fleet.app.to_string(),
        });
    }

    let component_topology = config.compile_component_topology()?;
    let mut fleet_subnet_roots = Vec::with_capacity(root_inputs.len());

    for input in root_inputs {
        fleet_subnet_roots.push(finalize_root_binding(
            &component_topology,
            &authority,
            input,
        )?);
    }
    fleet_subnet_roots.sort_by(|left, right| {
        left.placement_subnet
            .cmp(&right.placement_subnet)
            .then_with(|| left.fleet_subnet_root.cmp(&right.fleet_subnet_root))
    });

    component_topology.validate_fleet_subnet_root_bindings(&fleet_subnet_roots)?;
    Ok(FleetTopologyPlan {
        component_topology,
        fleet_subnet_roots,
    })
}

fn finalize_root_binding(
    component_topology: &ComponentTopology,
    authority: &FleetRegistryAuthority,
    mut input: FleetSubnetRootTopologyInput,
) -> Result<FleetSubnetRootBinding, FleetTopologyPlanError> {
    input
        .component_admissions
        .sort_by(|left, right| left.component_spec.cmp(&right.component_spec));

    let mut seen = BTreeSet::new();
    let mut component_admissions = Vec::with_capacity(input.component_admissions.len());
    for admission in input.component_admissions {
        if !seen.insert(admission.component_spec.clone()) {
            return Err(FleetTopologyPlanError::DuplicateAdmission {
                component_spec: admission.component_spec,
            });
        }
        let component_spec = component_topology
            .get(&admission.component_spec)
            .ok_or_else(|| FleetTopologyPlanError::UnknownComponentSpec {
                component_spec: admission.component_spec.clone(),
            })?;
        component_admissions.push(ComponentSpecAdmission {
            component_spec: admission.component_spec,
            spec_hash: component_spec.spec_hash,
            maximum_root_instances: admission.maximum_root_instances,
        });
    }

    let projection = component_topology.project_for_admissions(&component_admissions)?;
    Ok(FleetSubnetRootBinding {
        authority: authority.clone(),
        placement_subnet: input.placement_subnet,
        fleet_subnet_root: input.fleet_subnet_root,
        component_admissions,
        component_topology_digest: projection.digest()?,
        limits: input.limits,
        funding: input.funding,
    })
}

fn finalize_planned_root(
    component_topology: &ComponentTopology,
    mut input: PlannedFleetSubnetRootTopologyInput,
) -> Result<PlannedFleetSubnetRootTopology, FleetTopologyPlanError> {
    input
        .component_admissions
        .sort_by(|left, right| left.component_spec.cmp(&right.component_spec));

    let mut seen = BTreeSet::new();
    let mut component_admissions = Vec::with_capacity(input.component_admissions.len());
    for admission in input.component_admissions {
        if !seen.insert(admission.component_spec.clone()) {
            return Err(FleetTopologyPlanError::DuplicateAdmission {
                component_spec: admission.component_spec,
            });
        }
        let component_spec = component_topology
            .get(&admission.component_spec)
            .ok_or_else(|| FleetTopologyPlanError::UnknownComponentSpec {
                component_spec: admission.component_spec.clone(),
            })?;
        component_admissions.push(ComponentSpecAdmission {
            component_spec: admission.component_spec,
            spec_hash: component_spec.spec_hash,
            maximum_root_instances: admission.maximum_root_instances,
        });
    }

    let projection = component_topology.project_for_admissions(&component_admissions)?;
    Ok(PlannedFleetSubnetRootTopology {
        placement_subnet: input.placement_subnet,
        component_admissions,
        component_topology_digest: projection.digest()?,
        limits: input.limits,
    })
}
