//! Module: role_contract::descriptor
//!
//! Responsibility: validate owner descriptors and join them to resolved allocations.
//! Does not own: role selection, state records, audit rendering, or stable-memory access.
//! Boundary: every materialized manifest passes through one complete registry validation.

use canic_control_plane::state_contract::canic_control_plane_state_descriptors;
use canic_core::{
    role_contract::{
        AllocationLifecycle, MemoryId, ResolvedRoleContract, RoleContractFinding,
        StateAllocationKey,
        allocation::{allocation_definitions, validate_canonical_allocations},
    },
    state_contract::{
        STATE_MANIFEST_SCHEMA_VERSION, StateAllocationDescriptor, StateManifest, StateRoleManifest,
        canic_state_descriptors,
    },
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct StateDescriptorRegistry {
    descriptors: BTreeMap<StateAllocationKey, StateAllocationDescriptor>,
}

impl StateDescriptorRegistry {
    #[must_use]
    pub fn descriptor(&self, key: StateAllocationKey) -> Option<&StateAllocationDescriptor> {
        self.descriptors.get(&key)
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &StateAllocationDescriptor> {
        self.descriptors.values()
    }
}

pub fn validate_state_descriptor_registry()
-> Result<StateDescriptorRegistry, Vec<RoleContractFinding>> {
    validate_descriptors(
        canic_state_descriptors()
            .into_iter()
            .chain(canic_control_plane_state_descriptors()),
    )
}

fn validate_descriptors(
    descriptors: impl IntoIterator<Item = StateAllocationDescriptor>,
) -> Result<StateDescriptorRegistry, Vec<RoleContractFinding>> {
    let mut errors = Vec::new();
    if let Err(error) = validate_canonical_allocations() {
        errors.push(error);
    }

    let mut by_key = BTreeMap::new();
    for descriptor in descriptors {
        let key = descriptor.allocation;
        if by_key.insert(key, descriptor).is_some() {
            errors.push(RoleContractFinding::AllocationDescriptorDuplicate { key });
        }
    }

    for definition in allocation_definitions() {
        match (definition.lifecycle, by_key.get(&definition.key)) {
            (AllocationLifecycle::Active, None) => {
                errors.push(RoleContractFinding::AllocationDescriptorMissing {
                    key: definition.key,
                });
            }
            (AllocationLifecycle::Active, Some(descriptor)) => {
                if descriptor.owner != definition.owner {
                    errors.push(RoleContractFinding::CatalogInvalid {
                        reason: format!(
                            "allocation {:?} descriptor owner {} does not match canonical owner {}",
                            definition.key,
                            descriptor.owner.as_str(),
                            definition.owner.as_str()
                        ),
                    });
                }

                let expected = sorted_ids(definition.memory_ids.iter().copied());
                let actual = descriptor_active_ids(descriptor);
                if actual != expected {
                    errors.push(RoleContractFinding::AllocationDescriptorIdMismatch {
                        key: definition.key,
                        expected,
                        actual,
                    });
                }
                validate_descriptor_owners(descriptor, &mut errors);
                validate_removed_state(descriptor, &mut errors);
            }
            (AllocationLifecycle::Reserved | AllocationLifecycle::RetiredNeverReuse, Some(_)) => {
                errors.push(RoleContractFinding::CatalogInvalid {
                    reason: format!(
                        "inactive allocation {:?} must not have an active state descriptor",
                        definition.key
                    ),
                });
            }
            (AllocationLifecycle::Reserved | AllocationLifecycle::RetiredNeverReuse, None) => {}
        }
    }

    for key in by_key.keys() {
        if !allocation_definitions()
            .iter()
            .any(|definition| definition.key == *key)
        {
            errors.push(RoleContractFinding::CatalogInvalid {
                reason: format!("descriptor references unknown allocation {key:?}"),
            });
        }
    }

    if errors.is_empty() {
        Ok(StateDescriptorRegistry {
            descriptors: by_key,
        })
    } else {
        Err(errors)
    }
}

fn descriptor_active_ids(descriptor: &StateAllocationDescriptor) -> Vec<MemoryId> {
    sorted_ids(
        descriptor
            .state
            .iter()
            .filter_map(|domain| domain.memory_id)
            .chain(
                descriptor
                    .reserved_memory
                    .iter()
                    .map(|reservation| reservation.memory_id),
            )
            .map(MemoryId::new),
    )
}

fn sorted_ids(ids: impl IntoIterator<Item = MemoryId>) -> Vec<MemoryId> {
    let mut ids = ids.into_iter().collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

fn validate_descriptor_owners(
    descriptor: &StateAllocationDescriptor,
    errors: &mut Vec<RoleContractFinding>,
) {
    let expected = descriptor.owner.as_str();
    let owners = descriptor
        .state
        .iter()
        .map(|domain| domain.owner.as_str())
        .chain(
            descriptor
                .reserved_memory
                .iter()
                .map(|reservation| reservation.owner.as_str()),
        );
    if owners.into_iter().any(|owner| owner != expected) {
        errors.push(RoleContractFinding::CatalogInvalid {
            reason: format!(
                "allocation {:?} contains state metadata owned outside {expected}",
                descriptor.allocation
            ),
        });
    }
}

fn validate_removed_state(
    descriptor: &StateAllocationDescriptor,
    errors: &mut Vec<RoleContractFinding>,
) {
    for removed in &descriptor.removed_state {
        let Some(memory_id) = removed.memory_id else {
            continue;
        };
        let owners = allocation_definitions()
            .iter()
            .filter(|definition| {
                definition.memory_ids.contains(&MemoryId::new(memory_id))
                    && definition.lifecycle == AllocationLifecycle::RetiredNeverReuse
                    && definition.owner == descriptor.owner
            })
            .count();
        if owners != 1 {
            errors.push(RoleContractFinding::CatalogInvalid {
                reason: format!(
                    "removed state {} in allocation {:?} does not map to one retired canonical ID",
                    removed.domain, descriptor.allocation
                ),
            });
        }
    }
}

pub fn materialize_state_manifest(
    contracts: &[ResolvedRoleContract],
) -> Result<StateManifest, Vec<RoleContractFinding>> {
    let registry = validate_state_descriptor_registry()?;
    let mut roles = contracts
        .iter()
        .map(|contract| materialize_role(&registry, contract))
        .collect::<Result<Vec<_>, _>>()?;
    roles.sort_by(|left, right| left.canister_role.cmp(&right.canister_role));
    Ok(StateManifest {
        schema_version: STATE_MANIFEST_SCHEMA_VERSION,
        roles,
    })
}

fn materialize_role(
    registry: &StateDescriptorRegistry,
    contract: &ResolvedRoleContract,
) -> Result<StateRoleManifest, Vec<RoleContractFinding>> {
    let mut state = Vec::new();
    let mut removed_state = Vec::new();
    let mut reserved_memory = Vec::new();
    let mut errors = Vec::new();
    let mut selected = BTreeSet::new();

    for allocation in &contract.allocations {
        if !selected.insert(allocation.key) {
            continue;
        }
        let Some(descriptor) = registry.descriptor(allocation.key) else {
            errors.push(RoleContractFinding::AllocationDescriptorMissing {
                key: allocation.key,
            });
            continue;
        };
        state.extend(descriptor.state.clone());
        removed_state.extend(descriptor.removed_state.clone());
        reserved_memory.extend(descriptor.reserved_memory.clone());
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    state.sort_by(|left, right| left.domain.cmp(&right.domain));
    removed_state.sort_by(|left, right| left.domain.cmp(&right.domain));
    reserved_memory.sort_by_key(|reservation| reservation.memory_id);
    Ok(StateRoleManifest {
        canister_role: contract.role.as_str().to_string(),
        state,
        removed_state,
        reserved_memory,
    })
}

#[cfg(test)]
mod tests;
