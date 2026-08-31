//! Module: workflow::component_registry::authority_validation
//!
//! Responsibility: validate exact caller, provisioning, effect, topology, and lifecycle authority.
//! Does not own: durable records, orchestration, platform effects, or transport authentication.
//! Boundary: composes retained and observed evidence into fail-closed workflow decisions.

use super::*;

pub(super) fn validate_allocation_caller(
    allocation: &RootComponentAllocationView,
) -> Result<(), InternalError> {
    match &allocation.provisioning_origin {
        ComponentProvisioningOrigin::FleetAdministrator { caller }
            if *caller != IcOps::msg_caller() =>
        {
            Err(InternalError::conflict())
        }
        ComponentProvisioningOrigin::ComponentGroup { .. } => Err(InternalError::public(
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        )),
        ComponentProvisioningOrigin::FleetAdministrator { .. }
        | ComponentProvisioningOrigin::Component { .. }
        | ComponentProvisioningOrigin::FleetServiceComponent { .. } => Ok(()),
    }
}

#[derive(Clone, Copy)]
pub(super) struct PeerRequesterAccessEvidence<'a> {
    pub(super) caller: candid::Principal,
    pub(super) indexed_component: Option<canic_core::ids::ComponentInstanceId>,
    pub(super) retained: &'a canic_core::ids::ComponentBinding,
    pub(super) current: &'a canic_core::ids::ComponentBinding,
    pub(super) current_status: ComponentLifecycleStatus,
}

impl PeerRequesterAccessEvidence<'_> {
    pub(super) fn is_exact_active(&self) -> bool {
        [
            self.retained.canister_id == self.caller,
            self.indexed_component == Some(self.retained.component),
            self.current == self.retained,
            self.current_status == ComponentLifecycleStatus::Active,
        ]
        .into_iter()
        .all(|exact| exact)
    }
}

pub(super) fn require_active_peer_allocation_caller(
    operation_id: [u8; 32],
) -> Result<(), InternalError> {
    let caller = IcOps::msg_caller();
    let (authority, _) = root_authority()?;
    require_active_root_runtime(
        "peer Component lifecycle requires an Active Fleet Subnet Root runtime",
    )?;
    let allocation =
        ComponentRegistryOps::allocation(operation_id).ok_or_else(InternalError::unavailable)?;
    revalidate_retained_peer_origin(
        &authority,
        &ConfigOps::component_topology()?,
        &allocation.provisioning_origin,
        caller,
    )
}

pub(super) fn revalidate_peer_provisioning_origin(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    request: &PeerComponentRequester,
    origin: &ComponentProvisioningOrigin,
    caller: candid::Principal,
) -> Result<(), InternalError> {
    let request_matches_origin = match (request, origin) {
        (PeerComponentRequester::SameRoot, ComponentProvisioningOrigin::Component { .. }) => true,
        (
            PeerComponentRequester::FleetService {
                service,
                expected_registry,
            },
            ComponentProvisioningOrigin::FleetServiceComponent {
                requester,
                registry,
                ..
            },
        ) => service == &requester.service && expected_registry.as_ref() == registry.as_ref(),
        _ => false,
    };
    if !request_matches_origin {
        return Err(InternalError::conflict());
    }
    revalidate_retained_peer_origin(authority, topology, origin, caller)
}

pub(super) fn revalidate_retained_peer_origin(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    origin: &ComponentProvisioningOrigin,
    caller: candid::Principal,
) -> Result<(), InternalError> {
    match origin {
        ComponentProvisioningOrigin::Component { requester, grant } => {
            revalidate_same_root_peer_origin(authority, topology, requester, grant, caller)
        }
        ComponentProvisioningOrigin::FleetServiceComponent {
            requester,
            registry,
            grant,
        } => revalidate_fleet_service_peer_origin(
            authority, topology, requester, registry, grant, caller,
        ),
        ComponentProvisioningOrigin::FleetAdministrator { .. }
        | ComponentProvisioningOrigin::ComponentGroup { .. } => Err(InternalError::public(
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        )),
    }
}

pub(super) fn revalidate_same_root_peer_origin(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    requester: &ComponentBinding,
    grant: &canic_core::control_plane_support::config::ComponentProvisioningGrant,
    caller: candid::Principal,
) -> Result<(), InternalError> {
    topology
        .validate_component_binding(&authority.binding, requester)
        .map_err(|_| {
            InternalError::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
        })?;
    let current = ComponentRegistryOps::partition(requester.component)?.ok_or_else(|| {
        InternalError::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
    })?;
    let evidence = PeerRequesterAccessEvidence {
        caller,
        indexed_component: ComponentRegistryOps::component_for_principal(caller),
        retained: requester,
        current: &current.binding,
        current_status: current.status,
    };
    if !evidence.is_exact_active() {
        return Err(InternalError::public(
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    validate_retained_peer_grant(topology, requester, grant)
}

pub(super) fn revalidate_fleet_service_peer_origin(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    requester: &FleetServiceComponentRequester,
    registry: &FleetRegistryVersion,
    grant: &canic_core::control_plane_support::config::ComponentProvisioningGrant,
    caller: candid::Principal,
) -> Result<(), InternalError> {
    let mirror =
        FleetRegistryMirrorOps::validated_current(authority, authority.binding.fleet_subnet_root)?;
    if !ComponentRegistryOps::registry_covers_preparation(registry, &mirror.active.snapshot.version)
    {
        return Err(InternalError::conflict());
    }
    let current = FleetServicePeerOps::resolve(
        &authority.binding,
        topology,
        &mirror,
        caller,
        &requester.service,
    )?;
    if &current.requester != requester {
        return Err(InternalError::public(
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    validate_retained_peer_grant(topology, &requester.component, grant)
}

pub(super) fn validate_retained_peer_grant(
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    requester: &ComponentBinding,
    grant: &canic_core::control_plane_support::config::ComponentProvisioningGrant,
) -> Result<(), InternalError> {
    let current =
        topology.provisioning_grant(&requester.component_spec, &grant.target_component_spec);
    if current != Some(grant) {
        return Err(InternalError::invariant());
    }
    Ok(())
}

pub(super) fn validate_creation_effect(
    effect: &RootComponentCreationEffectView,
    expected: &RootComponentCreationPlan,
) -> Result<(), InternalError> {
    if !expected.matches_effect(effect) {
        return Err(InternalError::invariant());
    }
    Ok(())
}

pub(super) fn validate_install_effect(
    effect: &RootComponentInstallEffectView,
    expected: &RootComponentInstallPlan,
) -> Result<(), InternalError> {
    if !expected.matches_effect(effect) {
        return Err(InternalError::invariant());
    }
    Ok(())
}

pub(super) fn validate_child_install_effect(
    effect: &RootComponentChildInstallEffectView,
    expected: &RootComponentChildInstallPlan,
) -> Result<(), InternalError> {
    if !expected.matches_effect(effect) {
        return Err(InternalError::invariant());
    }
    Ok(())
}

pub(super) const fn creation_evidence(
    effect: RootComponentCreationEffectView,
    canister: Option<candid::Principal>,
) -> RootComponentCreationEvidence {
    RootComponentCreationEvidence {
        wasm_store: effect.wasm_store,
        payload_hash: effect.payload_hash,
        payload_size_bytes: effect.payload_size_bytes,
        initial_cycles: effect.initial_cycles,
        controller: effect.controller,
        canister,
    }
}

pub(super) fn install_evidence(
    effect: RootComponentInstallEffectView,
) -> RootComponentInstallEvidence {
    RootComponentInstallEvidence {
        raw_module_hash: effect.raw_module_hash,
        chunk_hashes: effect.chunk_hashes,
        binding: effect.binding,
    }
}

pub(super) fn child_install_evidence(
    effect: RootComponentChildInstallEffectView,
) -> RootComponentChildInstallEvidence {
    RootComponentChildInstallEvidence {
        raw_module_hash: effect.raw_module_hash,
        chunk_hashes: effect.chunk_hashes,
        binding: effect.binding,
    }
}

pub(in crate::workflow) fn validate_allocation_record(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    allocation: &RootComponentAllocationView,
    expected_operation_id: [u8; 32],
) -> Result<(), InternalError> {
    if allocation.operation_id == [0; 32] || allocation.operation_id != expected_operation_id {
        return Err(InternalError::invariant());
    }
    if allocation.allocation_sequence == 0
        || allocation.component
            != canic_core::ids::ComponentInstanceId::from_root_allocation(
                root.authority.binding.fleet.fleet,
                root.authority.epoch,
                root.fleet_subnet_root,
                allocation.allocation_sequence,
            )
    {
        return Err(InternalError::invariant());
    }
    if allocation.release_set != release_set {
        return Err(InternalError::invariant());
    }
    let admission = root
        .component_admissions
        .binary_search_by(|candidate| candidate.component_spec.cmp(&allocation.component_spec))
        .ok()
        .map(|index| &root.component_admissions[index])
        .ok_or_else(InternalError::invariant)?;
    let spec = topology
        .get(&allocation.component_spec)
        .ok_or_else(InternalError::invariant)?;
    if allocation.spec_hash != admission.spec_hash {
        return Err(InternalError::invariant());
    }
    if allocation.spec_hash != spec.spec_hash {
        return Err(InternalError::invariant());
    }
    if allocation.role != spec.component_role {
        return Err(InternalError::invariant());
    }
    validate_provisioning_origin(root, topology, allocation)?;
    Ok(())
}

pub(super) fn validate_provisioning_origin(
    root: &canic_core::ids::FleetSubnetRootBinding,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    allocation: &RootComponentAllocationView,
) -> Result<(), InternalError> {
    match &allocation.provisioning_origin {
        ComponentProvisioningOrigin::FleetAdministrator { .. } => {}
        ComponentProvisioningOrigin::Component { requester, grant } => {
            topology
                .validate_component_binding(root, requester)
                .map_err(|_error| InternalError::invariant())?;
            let expected =
                topology.provisioning_grant(&requester.component_spec, &allocation.component_spec);
            if expected != Some(grant.as_ref()) {
                return Err(InternalError::invariant());
            }
        }
        ComponentProvisioningOrigin::FleetServiceComponent {
            requester,
            registry,
            grant,
        } => {
            FleetServicePeerOps::validate_origin(
                root,
                topology,
                &allocation.component_spec,
                requester,
                registry,
                grant,
            )?;
        }
        origin @ ComponentProvisioningOrigin::ComponentGroup { .. } => {
            crate::ops::component_provisioning::RootComponentProvisioningOps::
                validate_member_provisioning_origin(
                    origin,
                    &allocation.component_spec,
                    allocation.spec_hash,
                )?;
        }
    }
    Ok(())
}

pub(super) const fn retained_subtree_stop_controller(
    progress: &RootComponentSubtreeRemovalProgressView,
) -> Option<candid::Principal> {
    match progress {
        RootComponentSubtreeRemovalProgressView::StopIntent(effect) => Some(effect.controller),
        RootComponentSubtreeRemovalProgressView::Stopped(receipt) => Some(receipt.stop.controller),
        RootComponentSubtreeRemovalProgressView::DeleteIntent(deletion) => {
            Some(deletion.stopped.stop.controller)
        }
        RootComponentSubtreeRemovalProgressView::Deleted(receipt) => {
            Some(receipt.deletion.stopped.stop.controller)
        }
        RootComponentSubtreeRemovalProgressView::MembershipRemoved(receipt) => {
            Some(receipt.deleted.deletion.stopped.stop.controller)
        }
        RootComponentSubtreeRemovalProgressView::DirectorySynchronized(receipt) => Some(
            receipt
                .membership_removed
                .deleted
                .deletion
                .stopped
                .stop
                .controller,
        ),
        RootComponentSubtreeRemovalProgressView::Fenced
        | RootComponentSubtreeRemovalProgressView::Traversing { .. }
        | RootComponentSubtreeRemovalProgressView::LeafSelected { .. }
        | RootComponentSubtreeRemovalProgressView::Completed(_) => None,
    }
}

pub(super) fn validate_subtree_removal(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    removal: &RootComponentSubtreeRemovalView,
    request: Option<&RootComponentSubtreeRemovalRequest>,
) -> Result<(), InternalError> {
    let partition =
        ComponentRegistryOps::partition(removal.component)?.ok_or_else(InternalError::invariant)?;
    validate_partition(root, release_set, topology, &partition)?;
    validate_subtree_removal_target(root, topology, removal)?;
    let reserved_registry_is_valid = removal.reserved_against_registry.component
        == removal.component
        && removal.reserved_against_registry.revision > 0;
    let partition_covers_reservation = match partition
        .revision
        .cmp(&removal.reserved_against_registry.revision)
    {
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => {
            partition.content_hash == removal.reserved_against_registry.content_hash
        }
        std::cmp::Ordering::Greater => true,
    };
    if removal.operation_id == [0; 32]
        || removal.maximum_completed_leaves == 0
        || removal.completed_leaves > removal.maximum_completed_leaves
        || !reserved_registry_is_valid
        || !partition_covers_reservation
    {
        return Err(InternalError::invariant());
    }
    let stop_controller = retained_subtree_stop_controller(&removal.progress);
    if stop_controller.is_some_and(|controller| controller != root.fleet_subnet_root) {
        return Err(InternalError::invariant());
    }
    if let Some(request) = request {
        let request_identity = (
            request.operation_id,
            request.component,
            request.target_canister_id,
            &request.expected_registry,
        );
        let durable_identity = (
            removal.operation_id,
            removal.component,
            removal.target_canister_id,
            &removal.reserved_against_registry,
        );
        if request_identity != durable_identity {
            return Err(InternalError::conflict());
        }
    }
    Ok(())
}

pub(super) fn validate_subtree_removal_target(
    root: &canic_core::ids::FleetSubnetRootBinding,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    removal: &RootComponentSubtreeRemovalView,
) -> Result<(), InternalError> {
    let registered_target =
        ComponentRegistryOps::registered_parent(removal.component, removal.target_canister_id)?;
    if subtree_target_membership_is_removed(&removal.progress) {
        if registered_target.is_some() {
            return Err(InternalError::invariant());
        }
    } else {
        let (target, _current_status) = registered_target.ok_or_else(InternalError::invariant)?;
        let ManagedCanisterBinding::ComponentChild(target) = target else {
            return Err(InternalError::invariant());
        };
        topology
            .validate_component_child_binding(root, &target)
            .map_err(|_error| InternalError::invariant())?;
        let target_identity = (
            removal.component,
            removal.target_parent_canister_id,
            &removal.target_role,
            removal.target_status,
        );
        let registered_target_identity = (
            target.component.component,
            target.parent_canister_id,
            &target.role,
            ComponentLifecycleStatus::Active,
        );
        if target_identity != registered_target_identity {
            return Err(InternalError::invariant());
        }
    }
    Ok(())
}

pub(super) const fn subtree_target_membership_is_removed(
    progress: &RootComponentSubtreeRemovalProgressView,
) -> bool {
    matches!(
        progress,
        RootComponentSubtreeRemovalProgressView::MembershipRemoved(_)
            | RootComponentSubtreeRemovalProgressView::DirectorySynchronized(_)
            | RootComponentSubtreeRemovalProgressView::Completed(_)
    )
}

pub(super) fn validate_child_allocation(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    parent: &ManagedCanisterBinding,
    allocation: &RootComponentChildAllocationView,
    request: Option<&RootComponentChildAllocationRequest>,
) -> Result<(), InternalError> {
    let (parent_component, parent_canister_id, parent_role) = match parent {
        ManagedCanisterBinding::Component(binding) => {
            topology
                .validate_component_binding(root, binding)
                .map_err(|_error| InternalError::invariant())?;
            (binding, binding.canister_id, &binding.role)
        }
        ManagedCanisterBinding::ComponentChild(binding) => {
            topology
                .validate_component_child_binding(root, binding)
                .map_err(|_error| InternalError::invariant())?;
            (&binding.component, binding.canister_id, &binding.role)
        }
    };
    if parent_canister_id != allocation.parent_canister_id {
        return Err(InternalError::public(
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    let spec = topology
        .get(&parent_component.component_spec)
        .ok_or_else(InternalError::invariant)?;
    let child = spec
        .child(&allocation.child_role)
        .ok_or_else(InternalError::invariant)?;
    let grant = spec
        .spawn_grant(parent_role, &allocation.child_role)
        .ok_or_else(InternalError::invariant)?;
    let partition = ComponentRegistryOps::partition(parent_component.component)?
        .ok_or_else(InternalError::invariant)?;
    if partition.binding != *parent_component {
        return Err(InternalError::invariant());
    }
    let deployment_limits = component_deployment_limits(&partition, topology)?;
    let maximum_instances_per_parent = deployment_spawn_grant_maximum(
        &deployment_limits,
        parent_role,
        &allocation.child_role,
        grant.maximum_instances_per_parent,
    )?;
    let expected_authority = ComponentChildAllocationAuthority {
        component: parent_component.component,
        parent_role,
        child_kind: child.kind,
        maximum_instances_per_parent,
        maximum_descendants: deployment_limits.maximum_descendants,
        maximum_registry_bytes: deployment_limits.maximum_registry_bytes,
        release_set,
        reserved_component: parent_component.component,
    };
    let reservation_is_versioned = allocation.reserved_against_registry.revision > 0;
    if ComponentChildAllocationAuthority::from_allocation(allocation) != expected_authority
        || !reservation_is_versioned
    {
        return Err(InternalError::invariant());
    }
    if request.is_some_and(|request| !child_allocation_request_matches(request, allocation)) {
        return Err(InternalError::conflict());
    }
    Ok(())
}

pub(super) fn deployment_spawn_grant_maximum(
    limits: &ComponentDeploymentLimits,
    parent_role: &CanisterRole,
    child_role: &CanisterRole,
    spec_maximum: u32,
) -> Result<u32, InternalError> {
    let maximum = limits
        .spawn_grant_reductions
        .iter()
        .find(|limit| &limit.parent_role == parent_role && &limit.child_role == child_role)
        .map_or(spec_maximum, |limit| limit.maximum_instances_per_parent);
    if maximum == 0 || maximum > spec_maximum {
        return Err(InternalError::invariant());
    }
    Ok(maximum)
}

pub(super) fn child_allocation_request_matches(
    request: &RootComponentChildAllocationRequest,
    allocation: &RootComponentChildAllocationView,
) -> bool {
    ComponentChildRequestIdentity::from(request) == ComponentChildRequestIdentity::from(allocation)
}

#[derive(Eq, PartialEq)]
pub(super) struct ComponentChildRequestIdentity<'a> {
    operation_id: [u8; 32],
    component: ComponentInstanceId,
    child_role: &'a CanisterRole,
    application_init_args: &'a Option<Vec<u8>>,
}

impl<'a> From<&'a RootComponentChildAllocationRequest> for ComponentChildRequestIdentity<'a> {
    fn from(request: &'a RootComponentChildAllocationRequest) -> Self {
        Self {
            operation_id: request.operation_id,
            component: request.component,
            child_role: &request.child_role,
            application_init_args: &request.application_init_args,
        }
    }
}

impl<'a> From<&'a RootComponentChildAllocationView> for ComponentChildRequestIdentity<'a> {
    fn from(allocation: &'a RootComponentChildAllocationView) -> Self {
        Self {
            operation_id: allocation.operation_id,
            component: allocation.component,
            child_role: &allocation.child_role,
            application_init_args: &allocation.application_init_args,
        }
    }
}

pub(super) fn validate_partition(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    partition: &ComponentRegistryPartitionView,
) -> Result<(), InternalError> {
    topology
        .validate_component_binding(root, &partition.binding)
        .map_err(|_error| InternalError::invariant())?;
    let root_authority_matches = partition.release_set == release_set
        && partition.binding.fleet_subnet_root == root.fleet_subnet_root
        && partition.binding.placement_subnet == root.placement_subnet;
    let lifecycle_is_committed = matches!(
        partition.status,
        ComponentLifecycleStatus::Prepared
            | ComponentLifecycleStatus::Active
            | ComponentLifecycleStatus::Draining
    ) && partition.revision > 0
        && partition.directory_synchronized_at_ns > 0;
    let principal_index_matches =
        ComponentRegistryOps::component_for_principal(partition.binding.canister_id)
            == Some(partition.binding.component);
    if !root_authority_matches || !lifecycle_is_committed || !principal_index_matches {
        return Err(InternalError::invariant());
    }
    Ok(())
}

pub(super) fn validate_component_draining(
    partition: &ComponentRegistryPartitionView,
    draining: &RootComponentDrainingView,
    request: Option<&RootComponentDrainingRequest>,
    fleet_directory: Option<&FleetDirectorySnapshot>,
) -> Result<(), InternalError> {
    let current_covers_receipt = partition.binding.component == draining.component
        && partition.status == ComponentLifecycleStatus::Draining
        && partition.revision >= draining.registry.revision
        && partition.committed_descendants <= draining.descendant_count
        && (partition.revision != draining.registry.revision
            || (partition.content_hash == draining.registry.content_hash
                && partition.descendant_content_hash == draining.descendant_content_hash
                && partition.committed_descendants == draining.descendant_count
                && partition.directory_synchronized_at_ns == draining.started_at_ns));
    let request_matches = match request {
        None => true,
        Some(request) => {
            let operation_matches = request.operation_id == draining.operation_id;
            let component_matches = request.component == draining.component;
            let registry_matches = request.expected_registry == draining.previous_registry;
            operation_matches && component_matches && registry_matches
        }
    };
    if !current_covers_receipt || !request_matches {
        return Err(InternalError::conflict());
    }
    if let Some(fleet_directory) = fleet_directory {
        let authority = ComponentRuntimeDirectoryAuthority {
            fleet: fleet_directory.clone(),
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: partition.binding.clone(),
                    source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                    component_registry_revision: draining.registry.revision,
                    component_registry_content_hash: draining.registry.content_hash,
                    synchronized_at_ns: draining.started_at_ns,
                },
                descendant_count: draining.descendant_count,
            },
            component_group: None,
        };
        if ComponentRuntimeOps::directory_authority_hash(&authority)?
            != draining.directory_authority_hash
        {
            return Err(InternalError::invariant());
        }
    }
    Ok(())
}

pub(super) fn validate_directory_member(
    root: &canic_core::ids::FleetSubnetRootBinding,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    partition: &ComponentRegistryPartitionView,
    member: &ManagedCanisterBinding,
) -> Result<(), InternalError> {
    match member {
        ManagedCanisterBinding::Component(binding) if binding == &partition.binding => Ok(()),
        ManagedCanisterBinding::ComponentChild(binding)
            if binding.component == partition.binding =>
        {
            topology
                .validate_component_child_binding(root, binding)
                .map_err(|_error| InternalError::invariant())
        }
        ManagedCanisterBinding::Component(_) | ManagedCanisterBinding::ComponentChild(_) => {
            Err(InternalError::invariant())
        }
    }
}

pub(super) const fn component_directory_member_can_read(status: ComponentLifecycleStatus) -> bool {
    matches!(
        status,
        ComponentLifecycleStatus::Active | ComponentLifecycleStatus::Draining
    )
}
