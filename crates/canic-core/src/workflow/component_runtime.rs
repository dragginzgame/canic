//! Module: workflow::component_runtime
//!
//! Responsibility: validate exact Directory authority and activate one managed Component runtime.
//! Does not own: root distribution, Registry mutation, or endpoint authorization.
//! Boundary: only exact protected binding and Directory evidence may cross each runtime phase.

use crate::{
    InternalError, InternalErrorOrigin,
    dto::{
        component_registry::{
            ComponentDirectoryProvenance, ComponentRuntimeActivationRequest,
            ComponentRuntimeDirectoryAuthority, ComponentRuntimeDirectoryPreparationRequest,
            ComponentRuntimeDirectorySynchronizationRequest, ComponentRuntimePhase,
            ComponentRuntimeStatusResponse,
        },
        fleet_registry::{FleetDirectorySnapshot, FleetSubnetRootStatus},
    },
    ids::{ComponentBinding, FleetRegistryAuthority, ManagedCanisterBinding},
    ops::{
        component_runtime::ComponentRuntimeOps,
        ic::IcOps,
        storage::{StorageOpsError, fleet_activation::FleetActivationOps},
    },
};

#[derive(Debug, Eq, PartialEq)]
struct ComponentDirectoryIdentity<'a> {
    component: &'a ComponentBinding,
    source_fleet_subnet_root: &'a crate::cdk::types::Principal,
}

impl<'a> ComponentDirectoryIdentity<'a> {
    const fn from_component(component: &'a ComponentBinding) -> Self {
        Self {
            component,
            source_fleet_subnet_root: &component.fleet_subnet_root,
        }
    }

    const fn from_provenance(provenance: &'a ComponentDirectoryProvenance) -> Self {
        Self {
            component: &provenance.component,
            source_fleet_subnet_root: &provenance.source_fleet_subnet_root,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct FleetDirectoryIdentity<'a> {
    authority: &'a FleetRegistryAuthority,
    source_fleet_subnet_root: &'a crate::cdk::types::Principal,
}

impl<'a> FleetDirectoryIdentity<'a> {
    const fn from_component(component: &'a ComponentBinding) -> Self {
        Self {
            authority: &component.authority,
            source_fleet_subnet_root: &component.fleet_subnet_root,
        }
    }

    const fn from_directory(directory: &'a FleetDirectorySnapshot) -> Self {
        Self {
            authority: &directory.provenance.registry.authority,
            source_fleet_subnet_root: &directory.provenance.source_fleet_subnet_root,
        }
    }
}

/// Prepare one exact Fleet and Component Directory authority while runtime remains Prepared.
pub fn prepare_directory(
    request: ComponentRuntimeDirectoryPreparationRequest,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let current = FleetActivationOps::component_runtime_status().map_err(StorageOpsError::from)?;
    validate_request(&current, &request)?;
    let authority_hash = ComponentRuntimeOps::directory_authority_hash(&request.authority)?;
    FleetActivationOps::prepare_component_runtime_directory(request, authority_hash)
        .map_err(StorageOpsError::from)
        .map_err(InternalError::from)
}

/// Synchronize one exact next current Directory authority on an Active Component runtime.
pub fn synchronize_directory(
    request: ComponentRuntimeDirectorySynchronizationRequest,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let current = status()?;
    if request.operation_id != current.operation_id {
        return Err(InternalError::conflict(
            "Component runtime Directory operation differs from protected installation identity",
        ));
    }
    validate_binding(&current.binding)?;
    validate_authority(&current.binding, &request.authority)?;
    if current.phase != ComponentRuntimePhase::Active || current.activation.is_none() {
        return Err(InternalError::conflict(
            "current Component Directory synchronization requires an Active runtime",
        ));
    }
    let current_authority = current.authority.as_ref().ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Active Component runtime has no current Directory authority",
        )
    })?;
    validate_directory_progression(current_authority, &request.authority)?;
    let authority_hash = ComponentRuntimeOps::directory_authority_hash(&request.authority)?;
    FleetActivationOps::synchronize_component_runtime_directory(request, authority_hash)
        .map_err(StorageOpsError::from)
        .map_err(InternalError::from)
}

/// Independently validate and return the target-local Directory preparation state.
pub fn status() -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let status = FleetActivationOps::component_runtime_status().map_err(StorageOpsError::from)?;
    validate_binding(&status.binding)?;
    match (&status.authority, status.authority_hash) {
        (None, None) => {}
        (Some(authority), Some(authority_hash)) => {
            validate_authority(&status.binding, authority)?;
            if ComponentRuntimeOps::directory_authority_hash(authority)? != authority_hash {
                return Err(InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    "retained Component runtime Directory authority hash does not match its value",
                ));
            }
        }
        _ => {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "retained Component runtime Directory authority and hash are incomplete",
            ));
        }
    }
    Ok(status)
}

/// Activate one exact Directory-prepared Component runtime.
pub fn activate(
    request: ComponentRuntimeActivationRequest,
) -> Result<crate::view::fleet_activation::ComponentRuntimeActivationTransition, InternalError> {
    let current = status()?;
    let expected_authority_hash = match current.phase {
        ComponentRuntimePhase::AwaitingDirectory | ComponentRuntimePhase::DirectoryPrepared => {
            current.authority_hash
        }
        ComponentRuntimePhase::Active => current
            .activation
            .map(|activation| activation.directory_authority_hash),
    };
    if request.operation_id != current.operation_id
        || request.directory_authority_hash == [0; 32]
        || expected_authority_hash != Some(request.directory_authority_hash)
    {
        return Err(InternalError::conflict(
            "Component runtime activation differs from its protected Directory authority",
        ));
    }
    let transition = FleetActivationOps::activate_component_runtime(request, IcOps::now_nanos())
        .map_err(StorageOpsError::from)
        .map_err(InternalError::from)?;
    if transition.transitioned
        && let Err(error) = crate::workflow::runtime::RuntimeWorkflow::start_all()
    {
        IcOps::trap(format!(
            "Component runtime activation could not establish runtime services: {error}"
        ));
    }
    Ok(transition)
}

fn validate_request(
    current: &ComponentRuntimeStatusResponse,
    request: &ComponentRuntimeDirectoryPreparationRequest,
) -> Result<(), InternalError> {
    if request.operation_id != current.operation_id {
        return Err(InternalError::conflict(
            "Component runtime Directory operation differs from protected installation identity",
        ));
    }
    validate_binding(&current.binding)?;
    validate_authority(&current.binding, &request.authority)
}

fn validate_binding(binding: &ManagedCanisterBinding) -> Result<(), InternalError> {
    let canister = match binding {
        ManagedCanisterBinding::Component(component) => component.canister_id,
        ManagedCanisterBinding::ComponentChild(child) => child.canister_id,
    };
    if canister != IcOps::canister_self() {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "protected Component runtime binding does not name this Canister",
        ));
    }
    Ok(())
}

fn validate_authority(
    binding: &ManagedCanisterBinding,
    authority: &ComponentRuntimeDirectoryAuthority,
) -> Result<(), InternalError> {
    let component = owning_component(binding);
    let provenance = &authority.component.provenance;
    let identity_matches = ComponentDirectoryIdentity::from_provenance(provenance)
        == ComponentDirectoryIdentity::from_component(component);
    let head_is_versioned = [
        provenance.component_registry_revision > 0,
        provenance.component_registry_content_hash != [0; 32],
        provenance.synchronized_at_ns > 0,
    ]
    .into_iter()
    .all(|valid| valid);
    if !identity_matches || !head_is_versioned {
        return Err(InternalError::invalid_input(
            "Component Directory does not match the protected Component-tree binding",
        ));
    }
    validate_fleet_directory(component, &authority.fleet)
}

fn validate_directory_progression(
    current: &ComponentRuntimeDirectoryAuthority,
    next: &ComponentRuntimeDirectoryAuthority,
) -> Result<(), InternalError> {
    if current == next {
        return Ok(());
    }
    let current_component = &current.component.provenance;
    let next_component = &next.component.provenance;
    let current_fleet_revision = current.fleet.provenance.registry.revision;
    let next_fleet_revision = next.fleet.provenance.registry.revision;
    let component_identity_is_stable = next_component.component == current_component.component
        && next_component.source_fleet_subnet_root == current_component.source_fleet_subnet_root;
    let component_authority_advances = next_component.component_registry_revision
        > current_component.component_registry_revision
        && next_component.component_registry_content_hash
            != current_component.component_registry_content_hash
        && next_component.synchronized_at_ns > current_component.synchronized_at_ns;
    let fleet_authority_is_monotonic = match next_fleet_revision.cmp(&current_fleet_revision) {
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => next.fleet == current.fleet,
        std::cmp::Ordering::Greater => true,
    };
    if !component_identity_is_stable
        || !component_authority_advances
        || !fleet_authority_is_monotonic
    {
        return Err(InternalError::conflict(
            "next Component Directory does not advance exact current authority",
        ));
    }
    Ok(())
}

fn validate_fleet_directory(
    component: &ComponentBinding,
    directory: &FleetDirectorySnapshot,
) -> Result<(), InternalError> {
    let identity_matches = FleetDirectoryIdentity::from_directory(directory)
        == FleetDirectoryIdentity::from_component(component);
    let version_is_present = directory.provenance.registry.revision > 0
        && directory.provenance.registry.content_hash != [0; 32];
    let root_set_is_present = !directory.fleet_subnet_roots.is_empty();
    if ![identity_matches, version_is_present, root_set_is_present]
        .into_iter()
        .all(|valid| valid)
    {
        return Err(InternalError::invalid_input(
            "Fleet Directory does not match the protected Component-tree binding",
        ));
    }

    let mut previous: Option<(&[u8], &[u8])> = None;
    let mut found_source = false;
    for entry in &directory.fleet_subnet_roots {
        let key = (
            entry.placement_subnet.as_principal().as_slice(),
            entry.fleet_subnet_root.as_slice(),
        );
        let status_is_published = entry.status != FleetSubnetRootStatus::Joining;
        let order_is_canonical = previous.is_none_or(|previous| previous < key);
        if !status_is_published || !order_is_canonical {
            return Err(InternalError::invalid_input(
                "Fleet Directory root entries are not unique, canonical and published",
            ));
        }
        previous = Some(key);
        if entry.fleet_subnet_root == component.fleet_subnet_root {
            let source_is_current = matches!(
                entry.status,
                FleetSubnetRootStatus::Active | FleetSubnetRootStatus::Draining
            );
            let source_is_exact = [
                !found_source,
                entry.placement_subnet == component.placement_subnet,
                source_is_current,
            ]
            .into_iter()
            .all(|valid| valid);
            if !source_is_exact {
                return Err(InternalError::invalid_input(
                    "Fleet Directory source root differs from the current Component binding",
                ));
            }
            found_source = true;
        }
    }
    if !found_source {
        return Err(InternalError::invalid_input(
            "Fleet Directory omits the Component's source root",
        ));
    }
    Ok(())
}

const fn owning_component(binding: &ManagedCanisterBinding) -> &ComponentBinding {
    match binding {
        ManagedCanisterBinding::Component(component) => component,
        ManagedCanisterBinding::ComponentChild(child) => &child.component,
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::{
            component_registry::{ComponentDirectoryHead, ComponentDirectoryProvenance},
            fleet_registry::{
                FleetDirectoryProvenance, FleetRegistryVersion, FleetSubnetRootDirectoryEntry,
            },
        },
        ids::{
            AppId, CanisterRole, CanonicalNetworkId, ComponentInstanceId, FleetBinding,
            FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, SubnetId,
        },
    };
    use candid::Principal;

    #[test]
    fn directory_progression_accepts_exact_replay_and_skipped_revisions() {
        let current = directory_authority();
        assert!(validate_directory_progression(&current, &current).is_ok());

        let mut skipped = current.clone();
        skipped.component.provenance.component_registry_revision = 3;
        skipped.component.provenance.component_registry_content_hash = [13; 32];
        skipped.component.provenance.synchronized_at_ns = 14;
        skipped.component.descendant_count = 2;
        assert!(validate_directory_progression(&current, &skipped).is_ok());

        let mut conflicting = current.clone();
        conflicting
            .component
            .provenance
            .component_registry_content_hash = [15; 32];
        assert!(validate_directory_progression(&current, &conflicting).is_err());
    }

    #[test]
    fn fleet_directory_accepts_draining_but_not_joining_or_removed_source() {
        let mut authority = directory_authority();
        let component = authority.component.provenance.component.clone();
        authority.fleet.fleet_subnet_roots[0].status = FleetSubnetRootStatus::Draining;
        validate_fleet_directory(&component, &authority.fleet)
            .expect("Draining source remains current for admitted Component lifecycle");

        authority.fleet.fleet_subnet_roots[0].status = FleetSubnetRootStatus::Joining;
        assert!(validate_fleet_directory(&component, &authority.fleet).is_err());

        authority.fleet.fleet_subnet_roots[0].status = FleetSubnetRootStatus::Removed;
        assert!(validate_fleet_directory(&component, &authority.fleet).is_err());
    }

    fn directory_authority() -> ComponentRuntimeDirectoryAuthority {
        let subnet = SubnetId::from_principal(Principal::from_slice(&[2; 29]));
        let root = Principal::from_slice(&[3; 29]);
        let authority = FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([4; 32]),
                    },
                    app: AppId::from("test"),
                },
                coordinator_subnet: subnet,
                coordinator: Principal::from_slice(&[5; 29]),
            },
            epoch: 1,
        };
        let component = ComponentBinding {
            authority: authority.clone(),
            component: ComponentInstanceId::from_generated_bytes([6; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [7; 32],
            role: CanisterRole::new("project_hub"),
            placement_subnet: subnet,
            fleet_subnet_root: root,
            canister_id: Principal::from_slice(&[8; 29]),
        };
        ComponentRuntimeDirectoryAuthority {
            fleet: FleetDirectorySnapshot {
                provenance: FleetDirectoryProvenance {
                    registry: FleetRegistryVersion {
                        authority,
                        revision: 9,
                        content_hash: [10; 32],
                    },
                    source_fleet_subnet_root: root,
                },
                fleet_subnet_roots: vec![FleetSubnetRootDirectoryEntry {
                    placement_subnet: subnet,
                    fleet_subnet_root: root,
                    status: FleetSubnetRootStatus::Active,
                }],
            },
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component,
                    source_fleet_subnet_root: root,
                    component_registry_revision: 1,
                    component_registry_content_hash: [11; 32],
                    synchronized_at_ns: 12,
                },
                descendant_count: 0,
            },
        }
    }
}
