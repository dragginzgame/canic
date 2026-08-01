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
        FleetSubnetRootBinding, FleetSubnetRootLimits, SubnetId,
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
