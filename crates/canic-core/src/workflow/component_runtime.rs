//! Module: workflow::component_runtime
//!
//! Responsibility: validate exact Directory authority and activate one managed Component runtime.
//! Does not own: root distribution, Registry mutation, or endpoint authorization.
//! Boundary: only exact protected binding and Directory evidence may cross each runtime phase.

use crate::{
    InternalError,
    dto::{
        component_deployment::{ComponentDeploymentPurpose, ProtectedComponentDeployment},
        component_provisioning::ComponentGroupDirectory,
        component_registry::{
            ComponentDirectoryProvenance, ComponentRuntimeActivationRequest,
            ComponentRuntimeDirectChild, ComponentRuntimeDirectoryAuthority,
            ComponentRuntimeDirectoryPreparationRequest,
            ComponentRuntimeDirectorySynchronizationRequest, ComponentRuntimePhase,
            ComponentRuntimeStatusResponse,
        },
        fleet_registry::{FleetDirectorySnapshot, FleetServiceMode, FleetSubnetRootStatus},
    },
    ids::{ComponentBinding, FleetRegistryAuthority, FleetServiceId, ManagedCanisterBinding},
    ops::{
        component_runtime::ComponentRuntimeOps,
        ic::IcOps,
        storage::{
            StorageOpsError, children::CanisterChildrenOps, fleet_activation::FleetActivationOps,
        },
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
    let direct_children_hash = validate_direct_children(&request.direct_children)?;
    let direct_children = request.direct_children.clone();
    let status = FleetActivationOps::prepare_component_runtime_directory(
        request,
        authority_hash,
        direct_children_hash,
    )
    .map_err(StorageOpsError::from)
    .map_err(InternalError::from)?;
    apply_direct_children(direct_children);
    Ok(status)
}

/// Converge one managed runtime to the requested current Directory and active state.
pub fn configure(
    request: ComponentRuntimeDirectoryPreparationRequest,
) -> Result<crate::view::fleet_activation::ComponentRuntimeActivationTransition, InternalError> {
    configure_with_runtime(
        request,
        crate::workflow::runtime::RuntimeWorkflow::start_all,
    )
}

pub fn configure_with_automatic_topup(
    request: ComponentRuntimeDirectoryPreparationRequest,
) -> Result<crate::view::fleet_activation::ComponentRuntimeActivationTransition, InternalError> {
    configure_with_runtime(
        request,
        crate::workflow::runtime::RuntimeWorkflow::start_all_with_automatic_topup,
    )
}

fn configure_with_runtime(
    request: ComponentRuntimeDirectoryPreparationRequest,
    start_runtime: fn() -> Result<(), InternalError>,
) -> Result<crate::view::fleet_activation::ComponentRuntimeActivationTransition, InternalError> {
    let current = status()?;
    match current.phase {
        ComponentRuntimePhase::AwaitingDirectory | ComponentRuntimePhase::DirectoryPrepared => {
            let prepared = prepare_directory(request.clone())?;
            let directory_authority_hash = prepared
                .authority_hash
                .ok_or_else(InternalError::invariant)?;
            activate_with_runtime(
                ComponentRuntimeActivationRequest {
                    operation_id: request.operation_id,
                    directory_authority_hash,
                },
                start_runtime,
            )
        }
        ComponentRuntimePhase::Active => {
            let status = synchronize_directory(ComponentRuntimeDirectorySynchronizationRequest {
                operation_id: request.operation_id,
                authority: request.authority,
                direct_children: request.direct_children,
            })?;
            Ok(
                crate::view::fleet_activation::ComponentRuntimeActivationTransition {
                    status,
                    transitioned: false,
                    application_init_args: None,
                },
            )
        }
    }
}

/// Synchronize one exact next current Directory authority on an Active Component runtime.
pub fn synchronize_directory(
    request: ComponentRuntimeDirectorySynchronizationRequest,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let current = status()?;
    if request.operation_id != current.operation_id {
        return Err(InternalError::conflict());
    }
    validate_binding(&current.binding)?;
    validate_authority(&current.binding, &current.deployment, &request.authority)?;
    if current.phase != ComponentRuntimePhase::Active || current.activation.is_none() {
        return Err(InternalError::conflict());
    }
    let current_authority = current
        .authority
        .as_ref()
        .ok_or_else(InternalError::invariant)?;
    validate_directory_progression(current_authority, &request.authority)?;
    let authority_hash = ComponentRuntimeOps::directory_authority_hash(&request.authority)?;
    let direct_children_hash = validate_direct_children(&request.direct_children)?;
    let direct_children = request.direct_children.clone();
    let status = FleetActivationOps::synchronize_component_runtime_directory(
        request,
        authority_hash,
        direct_children_hash,
    )
    .map_err(StorageOpsError::from)
    .map_err(InternalError::from)?;
    apply_direct_children(direct_children);
    Ok(status)
}

fn validate_direct_children(
    direct_children: &[ComponentRuntimeDirectChild],
) -> Result<[u8; 32], InternalError> {
    let mut canonical = direct_children.to_vec();
    canonical.sort();
    canonical.dedup();
    if canonical != direct_children {
        return Err(InternalError::conflict());
    }
    if direct_children
        .iter()
        .any(|child| child.canister_id == IcOps::canister_self())
    {
        return Err(InternalError::conflict());
    }
    ComponentRuntimeOps::direct_children_hash(direct_children)
}

fn apply_direct_children(direct_children: Vec<ComponentRuntimeDirectChild>) {
    CanisterChildrenOps::import_direct_children(
        IcOps::canister_self(),
        direct_children
            .into_iter()
            .map(|child| (child.canister_id, child.role))
            .collect(),
    );
}

/// Independently validate and return the target-local Directory preparation state.
pub fn status() -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let status = FleetActivationOps::component_runtime_status().map_err(StorageOpsError::from)?;
    validate_binding(&status.binding)?;
    match (&status.authority, status.authority_hash) {
        (None, None) => {}
        (Some(authority), Some(authority_hash)) => {
            validate_authority(&status.binding, &status.deployment, authority)?;
            if ComponentRuntimeOps::directory_authority_hash(authority)? != authority_hash {
                return Err(InternalError::invariant());
            }
        }
        _ => {
            return Err(InternalError::invariant());
        }
    }
    Ok(status)
}

/// Require this exact active Component tree to own one service's protected write authority.
pub fn require_service_authority(service: &FleetServiceId) -> Result<(), InternalError> {
    if service_authority_matches(service)? {
        return Ok(());
    }
    Err(InternalError::forbidden())
}

/// Resolve the protected runtime before evaluating the exact service-Authority predicate.
pub fn service_authority_matches(service: &FleetServiceId) -> Result<bool, InternalError> {
    let current = status()?;
    Ok(active_service_authority_matches(
        current.phase,
        &current.deployment,
        service,
    ))
}

fn active_service_authority_matches(
    phase: ComponentRuntimePhase,
    deployment: &ProtectedComponentDeployment,
    expected_service: &FleetServiceId,
) -> bool {
    matches!(
        (phase, deployment),
        (
            ComponentRuntimePhase::Active,
            ProtectedComponentDeployment::GroupMember {
                purpose: ComponentDeploymentPurpose::FleetServiceMember {
                    service,
                    member_purpose: crate::config::FleetServiceMemberPurpose::Authority,
                },
                ..
            }
        ) if service == expected_service
    )
}

/// Activate one exact Directory-prepared Component runtime.
pub fn activate(
    request: ComponentRuntimeActivationRequest,
) -> Result<crate::view::fleet_activation::ComponentRuntimeActivationTransition, InternalError> {
    activate_with_runtime(
        request,
        crate::workflow::runtime::RuntimeWorkflow::start_all,
    )
}

fn activate_with_runtime(
    request: ComponentRuntimeActivationRequest,
    start_runtime: fn() -> Result<(), InternalError>,
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
        return Err(InternalError::conflict());
    }
    let transition = FleetActivationOps::activate_component_runtime(request, IcOps::now_nanos())
        .map_err(StorageOpsError::from)
        .map_err(InternalError::from)?;
    if transition.transitioned
        && let Err(error) = start_runtime()
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
        return Err(InternalError::conflict());
    }
    validate_binding(&current.binding)?;
    validate_authority(&current.binding, &current.deployment, &request.authority)
}

fn validate_binding(binding: &ManagedCanisterBinding) -> Result<(), InternalError> {
    let canister = match binding {
        ManagedCanisterBinding::Component(component) => component.canister_id,
        ManagedCanisterBinding::ComponentChild(child) => child.canister_id,
    };
    if canister != IcOps::canister_self() {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_authority(
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
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
        return Err(InternalError::invalid_input());
    }
    validate_fleet_directory(component, deployment, &authority.fleet)?;
    validate_component_group_directory(component, deployment, authority.component_group.as_ref())
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
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_fleet_directory(
    component: &ComponentBinding,
    deployment: &ProtectedComponentDeployment,
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
        return Err(InternalError::invalid_input());
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
            return Err(InternalError::invalid_input());
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
                return Err(InternalError::invalid_input());
            }
            found_source = true;
        }
    }
    if !found_source {
        return Err(InternalError::invalid_input());
    }
    validate_fleet_services(component, deployment, directory)
}

fn validate_fleet_services(
    component: &ComponentBinding,
    deployment: &ProtectedComponentDeployment,
    directory: &FleetDirectorySnapshot,
) -> Result<(), InternalError> {
    let mut previous_service = None;
    let mut matched_membership = None;
    for service in &directory.services {
        let service_is_valid = [
            previous_service.is_none_or(|previous| previous < &service.service),
            !service.members.is_empty(),
            service.placement.maximum_members_per_root > 0,
            service.placement.minimum_distinct_roots > 0,
        ]
        .into_iter()
        .all(|valid| valid);
        if !service_is_valid {
            return Err(InternalError::invalid_input());
        }
        previous_service = Some(&service.service);
        let mut previous_member = None;
        let mut authority_count = 0_u32;
        for member in &service.members {
            let key = fleet_service_member_key(member);
            if previous_member.is_some_and(|previous| previous >= key) {
                return Err(InternalError::invalid_input());
            }
            previous_member = Some(key);
            if member.member_purpose == crate::config::FleetServiceMemberPurpose::Authority {
                authority_count = authority_count
                    .checked_add(1)
                    .ok_or_else(InternalError::resource_exhausted)?;
            }
            if member.component == component.component {
                let protected_membership_is_exact = [
                    matched_membership.is_none(),
                    member.fleet_subnet_root == component.fleet_subnet_root,
                    member.canister_id == component.canister_id,
                    service.component_spec == component.component_spec,
                ]
                .into_iter()
                .all(|valid| valid);
                if !protected_membership_is_exact {
                    return Err(InternalError::invalid_input());
                }
                matched_membership = Some((service, member));
            }
        }
        let mode_is_valid = fleet_service_mode_is_valid(service, authority_count);
        if !mode_is_valid {
            return Err(InternalError::invalid_input());
        }
    }
    validate_component_service_membership(deployment, matched_membership)
}

fn fleet_service_mode_is_valid(
    service: &crate::dto::fleet_registry::FleetDirectoryService,
    authority_count: u32,
) -> bool {
    match service.mode {
        FleetServiceMode::AuthorityReplica => {
            authority_count == 1
                && service.members.iter().all(|member| {
                    member.member_purpose != crate::config::FleetServiceMemberPurpose::PoolMember
                })
        }
        FleetServiceMode::ActivePool => {
            authority_count == 0
                && service.members.iter().all(|member| {
                    member.member_purpose == crate::config::FleetServiceMemberPurpose::PoolMember
                })
        }
    }
}

type FleetServiceMemberKey<'a> = (
    u8,
    &'a crate::ids::ComponentGroupPlacementId,
    &'a crate::ids::ComponentGroupMemberPath,
    &'a crate::ids::ComponentInstanceId,
);

const fn fleet_service_member_key(
    member: &crate::dto::fleet_registry::FleetDirectoryServiceComponent,
) -> FleetServiceMemberKey<'_> {
    let purpose = match member.member_purpose {
        crate::config::FleetServiceMemberPurpose::Authority => 0,
        crate::config::FleetServiceMemberPurpose::Replica => 1,
        crate::config::FleetServiceMemberPurpose::PoolMember => 2,
    };
    (
        purpose,
        &member.group_placement,
        &member.member_path,
        &member.component,
    )
}

fn validate_component_service_membership(
    deployment: &ProtectedComponentDeployment,
    membership: Option<(
        &crate::dto::fleet_registry::FleetDirectoryService,
        &crate::dto::fleet_registry::FleetDirectoryServiceComponent,
    )>,
) -> Result<(), InternalError> {
    let expected = match deployment {
        ProtectedComponentDeployment::UngroupedOrdinary { .. }
        | ProtectedComponentDeployment::GroupMember {
            purpose: ComponentDeploymentPurpose::Ordinary,
            ..
        } => None,
        ProtectedComponentDeployment::GroupMember {
            group_placement,
            member_path,
            purpose:
                ComponentDeploymentPurpose::FleetServiceMember {
                    service,
                    member_purpose,
                },
            ..
        } => Some((service, member_purpose, group_placement, member_path)),
    };
    match (expected, membership) {
        (None, None) => Ok(()),
        (Some(expected), Some(actual)) if fleet_service_membership_matches(expected, actual) => {
            Ok(())
        }
        _ => Err(InternalError::invalid_input()),
    }
}

type ExpectedFleetServiceMembership<'a> = (
    &'a crate::ids::FleetServiceId,
    &'a crate::config::FleetServiceMemberPurpose,
    &'a crate::ids::ComponentGroupPlacementId,
    &'a crate::ids::ComponentGroupMemberPath,
);

type ActualFleetServiceMembership<'a> = (
    &'a crate::dto::fleet_registry::FleetDirectoryService,
    &'a crate::dto::fleet_registry::FleetDirectoryServiceComponent,
);

fn fleet_service_membership_matches(
    expected: ExpectedFleetServiceMembership<'_>,
    actual: ActualFleetServiceMembership<'_>,
) -> bool {
    let (service, purpose, placement, path) = expected;
    let (actual_service, actual_member) = actual;
    [
        &actual_service.service == service,
        &actual_member.member_purpose == purpose,
        &actual_member.group_placement == placement,
        &actual_member.member_path == path,
    ]
    .into_iter()
    .all(|matches| matches)
}

fn validate_component_group_directory(
    component: &ComponentBinding,
    deployment: &ProtectedComponentDeployment,
    directory: Option<&ComponentGroupDirectory>,
) -> Result<(), InternalError> {
    let ProtectedComponentDeployment::GroupMember {
        group_placement,
        component_group,
        member_path,
        purpose,
        labels,
        ..
    } = deployment
    else {
        return if directory.is_none() {
            Ok(())
        } else {
            Err(InternalError::invalid_input())
        };
    };
    let directory = directory.ok_or_else(InternalError::invalid_input)?;
    let provenance = &directory.provenance;
    let provenance_is_exact = [
        provenance.authority == component.authority,
        provenance.fleet_subnet_root == component.fleet_subnet_root,
        &provenance.group_placement == group_placement,
        &provenance.component_group == component_group,
        provenance.operation_id != [0; 32],
        provenance.plan_hash != [0; 32],
        provenance.placement_receipt_content_hash != [0; 32],
        !directory.members.is_empty(),
    ]
    .into_iter()
    .all(|matches| matches);
    if !provenance_is_exact {
        return Err(InternalError::invalid_input());
    }
    let mut previous_path = None;
    let mut own_member_found = false;
    let mut principals = std::collections::BTreeSet::new();
    let mut components = std::collections::BTreeSet::new();
    for member in &directory.members {
        let member_is_valid = [
            previous_path.is_none_or(|previous| previous < &member.member_path),
            principals.insert(member.binding.canister_id),
            components.insert(member.binding.component),
            member.binding.authority == component.authority,
            member.binding.fleet_subnet_root == component.fleet_subnet_root,
        ]
        .into_iter()
        .all(|valid| valid);
        if !member_is_valid {
            return Err(InternalError::invalid_input());
        }
        previous_path = Some(&member.member_path);
        if &member.member_path == member_path {
            let own_member_is_exact = [
                !own_member_found,
                member.component_spec == component.component_spec,
                &member.purpose == purpose,
                &member.labels == labels,
                member.binding == *component,
            ]
            .into_iter()
            .all(|matches| matches);
            if !own_member_is_exact {
                return Err(InternalError::invalid_input());
            }
            own_member_found = true;
        }
    }
    if !own_member_found {
        return Err(InternalError::invalid_input());
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
        config::{
            ComponentDeploymentLimits, ComponentDeploymentPurpose, FleetServiceMemberPurpose,
            FleetServicePlacementPolicy,
        },
        dto::{
            component_deployment::ProtectedComponentDeployment,
            component_provisioning::{
                ComponentGroupDirectory, ComponentGroupDirectoryMember,
                ComponentGroupDirectoryProvenance,
            },
            component_registry::{ComponentDirectoryHead, ComponentDirectoryProvenance},
            fleet_registry::{
                FleetDirectoryProvenance, FleetDirectoryService, FleetDirectoryServiceComponent,
                FleetRegistryVersion, FleetServiceMode, FleetSubnetRootDirectoryEntry,
            },
        },
        ids::{
            AppId, CanisterRole, CanonicalNetworkId, ComponentDeploymentConfigurationDigest,
            ComponentGroupMemberPath, ComponentGroupPlacementId, ComponentInstanceId, FleetBinding,
            FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, FleetServiceId,
            SubnetId,
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
        let deployment = ProtectedComponentDeployment::UngroupedOrdinary {
            binding: component.clone(),
        };
        authority.fleet.fleet_subnet_roots[0].status = FleetSubnetRootStatus::Draining;
        validate_fleet_directory(&component, &deployment, &authority.fleet)
            .expect("Draining source remains current for admitted Component lifecycle");

        authority.fleet.fleet_subnet_roots[0].status = FleetSubnetRootStatus::Joining;
        assert!(validate_fleet_directory(&component, &deployment, &authority.fleet).is_err());

        authority.fleet.fleet_subnet_roots[0].status = FleetSubnetRootStatus::Removed;
        assert!(validate_fleet_directory(&component, &deployment, &authority.fleet).is_err());
    }

    #[test]
    fn grouped_service_member_requires_exact_service_and_group_directories() {
        let mut authority = directory_authority();
        let component = authority.component.provenance.component.clone();
        let placement = ComponentGroupPlacementId {
            deployment: "projects".parse().expect("deployment ID"),
            ordinal: 0,
        };
        let member_path =
            ComponentGroupMemberPath::try_from(vec!["database".parse().expect("member ID")])
                .expect("member path");
        let service: FleetServiceId = "database".parse().expect("service ID");
        let purpose = ComponentDeploymentPurpose::FleetServiceMember {
            service: service.clone(),
            member_purpose: FleetServiceMemberPurpose::Authority,
        };
        let deployment = ProtectedComponentDeployment::GroupMember {
            binding: component.clone(),
            configuration_digest: ComponentDeploymentConfigurationDigest::from_bytes([20; 32]),
            group_placement: placement.clone(),
            component_group: "project_cell".parse().expect("Component Group ID"),
            member_path: member_path.clone(),
            purpose: purpose.clone(),
            labels: vec![],
            limits: ComponentDeploymentLimits {
                maximum_descendants: 20_000,
                maximum_registry_bytes: 16_777_216,
                spawn_grant_reductions: vec![],
            },
        };
        authority.fleet.services = vec![FleetDirectoryService {
            service,
            role: component.role.clone(),
            component_spec: component.component_spec.clone(),
            mode: FleetServiceMode::AuthorityReplica,
            placement: FleetServicePlacementPolicy {
                maximum_members_per_root: 1,
                minimum_distinct_roots: 1,
            },
            members: vec![FleetDirectoryServiceComponent {
                member_purpose: FleetServiceMemberPurpose::Authority,
                component: component.component,
                fleet_subnet_root: component.fleet_subnet_root,
                canister_id: component.canister_id,
                group_placement: placement.clone(),
                member_path: member_path.clone(),
            }],
        }];
        authority.component_group = Some(ComponentGroupDirectory {
            provenance: ComponentGroupDirectoryProvenance {
                authority: component.authority.clone(),
                fleet_subnet_root: component.fleet_subnet_root,
                group_placement: placement,
                component_group: "project_cell".parse().expect("Component Group ID"),
                operation_id: [21; 32],
                plan_hash: [22; 32],
                placement_receipt_content_hash: [23; 32],
            },
            members: vec![ComponentGroupDirectoryMember {
                member_path,
                component_spec: component.component_spec.clone(),
                purpose,
                labels: vec![],
                binding: component.clone(),
            }],
        });
        let binding = ManagedCanisterBinding::Component(component);
        validate_authority(&binding, &deployment, &authority)
            .expect("exact service and group Directories");

        authority.fleet.services[0].members[0].canister_id = Principal::from_slice(&[24; 29]);
        assert!(validate_authority(&binding, &deployment, &authority).is_err());
    }

    #[test]
    fn only_matching_active_authority_purpose_grants_service_write_authority() {
        let component = directory_authority().component.provenance.component;
        let database: FleetServiceId = "database".parse().expect("service ID");
        let other: FleetServiceId = "other".parse().expect("service ID");
        let authority = grouped_deployment(
            component.clone(),
            ComponentDeploymentPurpose::FleetServiceMember {
                service: database.clone(),
                member_purpose: FleetServiceMemberPurpose::Authority,
            },
        );
        assert!(active_service_authority_matches(
            ComponentRuntimePhase::Active,
            &authority,
            &database,
        ));
        assert!(!active_service_authority_matches(
            ComponentRuntimePhase::DirectoryPrepared,
            &authority,
            &database,
        ));
        assert!(!active_service_authority_matches(
            ComponentRuntimePhase::Active,
            &authority,
            &other,
        ));

        for member_purpose in [
            FleetServiceMemberPurpose::Replica,
            FleetServiceMemberPurpose::PoolMember,
        ] {
            let deployment = grouped_deployment(
                component.clone(),
                ComponentDeploymentPurpose::FleetServiceMember {
                    service: database.clone(),
                    member_purpose,
                },
            );
            assert!(!active_service_authority_matches(
                ComponentRuntimePhase::Active,
                &deployment,
                &database,
            ));
        }
        let grouped_ordinary =
            grouped_deployment(component.clone(), ComponentDeploymentPurpose::Ordinary);
        assert!(!active_service_authority_matches(
            ComponentRuntimePhase::Active,
            &grouped_ordinary,
            &database,
        ));
        let ungrouped = ProtectedComponentDeployment::UngroupedOrdinary { binding: component };
        assert!(!active_service_authority_matches(
            ComponentRuntimePhase::Active,
            &ungrouped,
            &database,
        ));
    }

    fn grouped_deployment(
        binding: ComponentBinding,
        purpose: ComponentDeploymentPurpose,
    ) -> ProtectedComponentDeployment {
        ProtectedComponentDeployment::GroupMember {
            binding,
            configuration_digest: ComponentDeploymentConfigurationDigest::from_bytes([25; 32]),
            group_placement: ComponentGroupPlacementId {
                deployment: "projects".parse().expect("deployment ID"),
                ordinal: 0,
            },
            component_group: "project_cell".parse().expect("Component Group ID"),
            member_path: ComponentGroupMemberPath::try_from(vec![
                "database".parse().expect("member ID"),
            ])
            .expect("member path"),
            purpose,
            labels: vec![],
            limits: ComponentDeploymentLimits {
                maximum_descendants: 20_000,
                maximum_registry_bytes: 16_777_216,
                spawn_grant_reductions: vec![],
            },
        }
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
                services: vec![],
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
            component_group: None,
        }
    }
}
