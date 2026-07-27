//! Module: model::fleet_registry
//!
//! Responsibility: own canonical Fleet Registry snapshot invariants and genesis construction.
//! Does not own: stable storage, snapshot publication, transport, or root lifecycle effects.
//! Boundary: ops validates passive Registry DTOs against one compiled Component Topology.

use crate::{
    config::{ComponentTopology, ComponentTopologyError},
    dto::fleet_registry::{FleetComponentSpecEntry, FleetRegistry, FleetSubnetRootEntry},
    ids::{AppId, ComponentSpecId, FleetRegistryAuthority, ReleaseBuildId},
};
use candid::Principal;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error as ThisError;

///
/// FleetRegistryModelError
///
/// Typed rejection for a malformed or contradictory Fleet Registry snapshot.
///

#[derive(Debug, ThisError)]
pub enum FleetRegistryModelError {
    #[error("Fleet Registry Coordinator principal must not be anonymous")]
    AnonymousCoordinator,

    #[error("Fleet Registry Coordinator Subnet must not be anonymous")]
    AnonymousCoordinatorSubnet,

    #[error("Fleet Registry authority does not match the protected expected authority")]
    AuthorityMismatch,

    #[error("Fleet Registry root principal must not be anonymous")]
    AnonymousFleetSubnetRoot,

    #[error("Fleet Registry genesis App '{received}' does not match configured App '{expected}'")]
    GenesisAppMismatch { expected: AppId, received: AppId },

    #[error("Fleet Registry contains duplicate root principal {fleet_subnet_root}")]
    DuplicateFleetSubnetRoot { fleet_subnet_root: Principal },

    #[error(
        "Fleet Registry Component Spec '{component_spec}' does not match the compiled topology"
    )]
    FleetComponentSpecMismatch { component_spec: ComponentSpecId },

    #[error("Fleet Registry Component Specs are not the complete compiled topology")]
    FleetComponentSpecSetMismatch,

    #[error(
        "Fleet Registry admissions for Component Spec '{component_spec}' exceed its Fleet maximum {maximum_fleet_instances}: {admitted}"
    )]
    FleetAdmissionsExceedMaximum {
        component_spec: ComponentSpecId,
        admitted: u32,
        maximum_fleet_instances: u32,
    },

    #[error("Fleet Registry admission total overflowed for Component Spec '{component_spec}'")]
    FleetAdmissionsOverflow { component_spec: ComponentSpecId },

    #[error("Fleet Registry genesis requires authority epoch 1, got {0}")]
    GenesisAuthorityEpoch(u64),

    #[error("Fleet Registry root order is not strictly ascending by physical Subnet")]
    NonCanonicalFleetSubnetRootOrder,

    #[error("Fleet Registry revision must be positive")]
    NonPositiveRevision,

    #[error("Fleet Registry authority epoch must be positive")]
    NonPositiveAuthorityEpoch,

    #[error("Fleet Registry root principal conflicts with its Coordinator")]
    RootPrincipalConflictsWithCoordinator,

    #[error(
        "Fleet Registry roots carry different active release builds: expected {expected}, got {received}"
    )]
    RootReleaseBuildMismatch {
        expected: ReleaseBuildId,
        received: ReleaseBuildId,
    },

    #[error(transparent)]
    Topology(#[from] ComponentTopologyError),
}

pub fn compile_genesis(
    configured_app: &AppId,
    authority: FleetRegistryAuthority,
    topology: &ComponentTopology,
) -> Result<FleetRegistry, FleetRegistryModelError> {
    if authority.epoch != 1 {
        return Err(FleetRegistryModelError::GenesisAuthorityEpoch(
            authority.epoch,
        ));
    }
    if &authority.binding.fleet.app != configured_app {
        return Err(FleetRegistryModelError::GenesisAppMismatch {
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

pub fn validate(
    expected_authority: &FleetRegistryAuthority,
    topology: &ComponentTopology,
    registry: &FleetRegistry,
) -> Result<(), FleetRegistryModelError> {
    validate_authority(&registry.authority)?;
    if &registry.authority != expected_authority {
        return Err(FleetRegistryModelError::AuthorityMismatch);
    }
    if registry.revision == 0 {
        return Err(FleetRegistryModelError::NonPositiveRevision);
    }

    validate_component_specs(topology, &registry.component_specs)?;
    validate_roots(topology, registry)
}

fn validate_authority(authority: &FleetRegistryAuthority) -> Result<(), FleetRegistryModelError> {
    if authority.epoch == 0 {
        return Err(FleetRegistryModelError::NonPositiveAuthorityEpoch);
    }
    if authority.binding.coordinator_subnet.as_principal() == &Principal::anonymous() {
        return Err(FleetRegistryModelError::AnonymousCoordinatorSubnet);
    }
    if authority.binding.coordinator == Principal::anonymous() {
        return Err(FleetRegistryModelError::AnonymousCoordinator);
    }
    Ok(())
}

fn validate_component_specs(
    topology: &ComponentTopology,
    entries: &[FleetComponentSpecEntry],
) -> Result<(), FleetRegistryModelError> {
    if entries.len() != topology.component_specs.len() {
        return Err(FleetRegistryModelError::FleetComponentSpecSetMismatch);
    }

    for (entry, expected) in entries.iter().zip(&topology.component_specs) {
        if entry.component_spec != expected.component_spec
            || entry.spec_hash != expected.spec_hash
            || entry.component_role != expected.component_role
            || entry.maximum_fleet_instances != expected.maximum_fleet_instances
        {
            return Err(FleetRegistryModelError::FleetComponentSpecMismatch {
                component_spec: entry.component_spec.clone(),
            });
        }
    }

    Ok(())
}

fn validate_roots(
    topology: &ComponentTopology,
    registry: &FleetRegistry,
) -> Result<(), FleetRegistryModelError> {
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
            return Err(FleetRegistryModelError::NonCanonicalFleetSubnetRootOrder);
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
                return Err(FleetRegistryModelError::RootReleaseBuildMismatch {
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
                .ok_or_else(|| FleetRegistryModelError::FleetAdmissionsOverflow {
                    component_spec: admission.component_spec.clone(),
                })?;
        }
    }

    for spec in &topology.component_specs {
        let admitted = admission_totals[&spec.component_spec];
        if admitted > spec.maximum_fleet_instances {
            return Err(FleetRegistryModelError::FleetAdmissionsExceedMaximum {
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
) -> Result<(), FleetRegistryModelError> {
    if root.placement_subnet.as_principal() == &Principal::anonymous() {
        return Err(FleetRegistryModelError::Topology(
            ComponentTopologyError::AnonymousBindingPrincipal {
                field: "fleet_subnet_roots.placement_subnet",
            },
        ));
    }
    if root.fleet_subnet_root == Principal::anonymous() {
        return Err(FleetRegistryModelError::AnonymousFleetSubnetRoot);
    }
    if root.fleet_subnet_root == registry.authority.binding.coordinator {
        return Err(FleetRegistryModelError::RootPrincipalConflictsWithCoordinator);
    }
    if !root_principals.insert(root.fleet_subnet_root) {
        return Err(FleetRegistryModelError::DuplicateFleetSubnetRoot {
            fleet_subnet_root: root.fleet_subnet_root,
        });
    }
    Ok(())
}
