//! Module: ops::fleet_registry::validation
//!
//! Responsibility: validate passive Fleet Registry snapshots and compile genesis.
//! Does not own: stable storage, snapshot publication, transport, or root lifecycle effects.
//! Boundary: the parent ops module calls this against one compiled Component Topology.

use crate::{
    config::{ComponentTopology, ComponentTopologyError},
    dto::fleet_registry::{FleetComponentSpecEntry, FleetRegistry, FleetSubnetRootEntry},
    ids::{AppId, FleetRegistryAuthority},
    ops::fleet_registry::FleetRegistryOpsError,
};
use std::collections::{BTreeMap, BTreeSet};

use candid::Principal;

pub(super) fn compile_genesis(
    configured_app: &AppId,
    authority: FleetRegistryAuthority,
    topology: &ComponentTopology,
) -> Result<FleetRegistry, FleetRegistryOpsError> {
    if authority.epoch != 1 {
        return Err(FleetRegistryOpsError::GenesisAuthorityEpoch(
            authority.epoch,
        ));
    }
    if &authority.binding.fleet.app != configured_app {
        return Err(FleetRegistryOpsError::GenesisAppMismatch {
            expected: configured_app.clone(),
            received: authority.binding.fleet.app,
        });
    }

    let registry = FleetRegistry {
        authority: authority.clone(),
        revision: 1,
        component_specs: topology
            .component_specs
            .iter()
            .map(|spec| FleetComponentSpecEntry {
                component_spec: spec.component_spec.clone(),
                spec_hash: spec.spec_hash,
                component_role: spec.component_role.clone(),
                maximum_fleet_instances: spec.maximum_fleet_instances,
            })
            .collect(),
        fleet_subnet_roots: Vec::new(),
    };
    validate(&authority, topology, &registry)?;
    Ok(registry)
}

pub(super) fn validate(
    expected_authority: &FleetRegistryAuthority,
    topology: &ComponentTopology,
    registry: &FleetRegistry,
) -> Result<(), FleetRegistryOpsError> {
    validate_authority(&registry.authority)?;
    if &registry.authority != expected_authority {
        return Err(FleetRegistryOpsError::AuthorityMismatch);
    }
    if registry.revision == 0 {
        return Err(FleetRegistryOpsError::NonPositiveRevision);
    }

    validate_component_specs(topology, &registry.component_specs)?;
    validate_roots(topology, registry)
}

fn validate_authority(authority: &FleetRegistryAuthority) -> Result<(), FleetRegistryOpsError> {
    if authority.epoch == 0 {
        return Err(FleetRegistryOpsError::NonPositiveAuthorityEpoch);
    }
    if authority.binding.coordinator_subnet.as_principal() == &Principal::anonymous() {
        return Err(FleetRegistryOpsError::AnonymousCoordinatorSubnet);
    }
    if authority.binding.coordinator == Principal::anonymous() {
        return Err(FleetRegistryOpsError::AnonymousCoordinator);
    }
    Ok(())
}

fn validate_component_specs(
    topology: &ComponentTopology,
    entries: &[FleetComponentSpecEntry],
) -> Result<(), FleetRegistryOpsError> {
    if entries.len() != topology.component_specs.len() {
        return Err(FleetRegistryOpsError::FleetComponentSpecSetMismatch);
    }

    for (entry, expected) in entries.iter().zip(&topology.component_specs) {
        if entry.component_spec != expected.component_spec
            || entry.spec_hash != expected.spec_hash
            || entry.component_role != expected.component_role
            || entry.maximum_fleet_instances != expected.maximum_fleet_instances
        {
            return Err(FleetRegistryOpsError::FleetComponentSpecMismatch {
                component_spec: entry.component_spec.clone(),
            });
        }
    }

    Ok(())
}

fn validate_roots(
    topology: &ComponentTopology,
    registry: &FleetRegistry,
) -> Result<(), FleetRegistryOpsError> {
    let mut previous_subnet = None;
    let mut release_build_id = None;
    let mut root_principals = BTreeSet::new();
    let mut admission_totals = topology
        .component_specs
        .iter()
        .map(|spec| (spec.component_spec.clone(), 0_u32))
        .collect::<BTreeMap<_, _>>();

    for root in &registry.fleet_subnet_roots {
        if previous_subnet.is_some_and(|previous| previous >= root.placement_subnet) {
            return Err(FleetRegistryOpsError::NonCanonicalFleetSubnetRootOrder);
        }
        previous_subnet = Some(root.placement_subnet);
        validate_root_identity(registry, root, &mut root_principals)?;
        topology.validate_planned_root(
            &root.component_admissions,
            root.component_topology_digest,
            &root.limits,
        )?;
        let root_release_build_id = root.active_release_set.release_build_id;
        if let Some(expected) = release_build_id {
            if root_release_build_id != expected {
                return Err(FleetRegistryOpsError::RootReleaseBuildMismatch {
                    expected,
                    received: root_release_build_id,
                });
            }
        } else {
            release_build_id = Some(root_release_build_id);
        }

        for admission in &root.component_admissions {
            let total = admission_totals
                .get_mut(&admission.component_spec)
                .expect("planned-root validation admitted only known Component Specs");
            *total = total
                .checked_add(admission.maximum_root_instances)
                .ok_or_else(|| FleetRegistryOpsError::FleetAdmissionsOverflow {
                    component_spec: admission.component_spec.clone(),
                })?;
        }
    }

    for spec in &topology.component_specs {
        let admitted = admission_totals[&spec.component_spec];
        if admitted > spec.maximum_fleet_instances {
            return Err(FleetRegistryOpsError::FleetAdmissionsExceedMaximum {
                component_spec: spec.component_spec.clone(),
                admitted,
                maximum_fleet_instances: spec.maximum_fleet_instances,
            });
        }
    }

    Ok(())
}

fn validate_root_identity(
    registry: &FleetRegistry,
    root: &FleetSubnetRootEntry,
    root_principals: &mut BTreeSet<Principal>,
) -> Result<(), FleetRegistryOpsError> {
    if root.placement_subnet.as_principal() == &Principal::anonymous() {
        return Err(FleetRegistryOpsError::Topology(
            ComponentTopologyError::AnonymousBindingPrincipal {
                field: "fleet_subnet_roots.placement_subnet",
            },
        ));
    }
    if root.fleet_subnet_root == Principal::anonymous() {
        return Err(FleetRegistryOpsError::AnonymousFleetSubnetRoot);
    }
    if root.fleet_subnet_root == registry.authority.binding.coordinator {
        return Err(FleetRegistryOpsError::RootPrincipalConflictsWithCoordinator);
    }
    if !root_principals.insert(root.fleet_subnet_root) {
        return Err(FleetRegistryOpsError::DuplicateFleetSubnetRoot {
            fleet_subnet_root: root.fleet_subnet_root,
        });
    }
    Ok(())
}
