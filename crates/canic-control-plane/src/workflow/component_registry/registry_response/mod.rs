//! Root-facing Component Registry and allocation response projection.
//!
//! Boundary: validated Registry views become passive DTOs without storage reads, lifecycle
//! decisions or platform effects.

use super::*;

pub(super) fn response(
    root: candid::Principal,
    prepared: &RootComponentRegistryView,
) -> Result<RootComponentRegistryStatusResponse, InternalError> {
    if prepared.root.fleet_subnet_root != root {
        return Err(InternalError::invariant());
    }
    Ok(RootComponentRegistryStatusResponse {
        fleet_subnet_root: root,
        prepared_against_registry: prepared.prepared_against_registry.clone(),
        release_set: prepared.release_set,
        component_topology_digest: prepared.root.component_topology_digest,
        next_allocation_sequence: prepared.next_allocation_sequence,
        reserved_component_instances: prepared.reserved_component_instances,
        committed_component_instances: prepared.committed_component_instances,
        managed_descendants: prepared.managed_descendants,
        known_created_component_canisters: prepared.known_created_component_canisters,
        encoded_bytes: prepared.encoded_bytes,
        initial_inventory: prepared.initial_inventory.map(|inventory| {
            RootComponentInitialInventoryStatus {
                fleet_activation_operation_id: inventory.fleet_activation_operation_id,
                component_count: inventory.component_count,
                inventory_hash: inventory.inventory_hash,
                sealed_at_ns: inventory.sealed_at_ns,
                directories_converged: inventory.directories_converged,
                root_runtime_activated: inventory.root_runtime_activated,
            }
        }),
    })
}

pub(super) fn allocation_response(
    allocation: RootComponentAllocationView,
) -> Result<RootComponentAllocationResponse, InternalError> {
    if allocation.allocation_sequence == 0 {
        return Err(InternalError::invariant());
    }
    let (phase, creation, installation) = match allocation.progress {
        RootComponentAllocationProgressView::Reserved => {
            (RootComponentAllocationPhase::Reserved, None, None)
        }
        RootComponentAllocationProgressView::CreationIntent(effect) => (
            RootComponentAllocationPhase::CreationIntent,
            Some(creation_evidence(effect, None)),
            None,
        ),
        RootComponentAllocationProgressView::Created { effect, canister } => (
            RootComponentAllocationPhase::Created,
            Some(creation_evidence(effect, Some(canister))),
            None,
        ),
        RootComponentAllocationProgressView::InstallIntent {
            creation,
            canister,
            installation,
        } => (
            RootComponentAllocationPhase::InstallIntent,
            Some(creation_evidence(creation, Some(canister))),
            Some(install_evidence(installation)),
        ),
        RootComponentAllocationProgressView::Installed {
            creation,
            canister,
            installation,
        } => (
            RootComponentAllocationPhase::Installed,
            Some(creation_evidence(creation, Some(canister))),
            Some(install_evidence(installation)),
        ),
        RootComponentAllocationProgressView::Verified {
            creation,
            canister,
            installation,
        } => (
            RootComponentAllocationPhase::Verified,
            Some(creation_evidence(creation, Some(canister))),
            Some(install_evidence(installation)),
        ),
        RootComponentAllocationProgressView::Committed {
            creation,
            canister,
            installation,
            ..
        } => (
            RootComponentAllocationPhase::Committed,
            Some(creation_evidence(creation, Some(canister))),
            Some(install_evidence(installation)),
        ),
        RootComponentAllocationProgressView::Removed {
            creation,
            canister,
            installation,
            ..
        } => (
            RootComponentAllocationPhase::Removed,
            Some(creation_evidence(creation, Some(canister))),
            Some(install_evidence(installation)),
        ),
    };
    Ok(RootComponentAllocationResponse {
        operation_id: allocation.operation_id,
        allocation_sequence: allocation.allocation_sequence,
        component: allocation.component,
        component_spec: allocation.component_spec,
        spec_hash: allocation.spec_hash,
        role: allocation.role,
        provisioning_origin: allocation.provisioning_origin,
        release_set: allocation.release_set,
        phase,
        creation,
        installation,
    })
}

pub(super) fn child_allocation_response(
    allocation: RootComponentChildAllocationView,
) -> RootComponentChildAllocationResponse {
    let (phase, creation, installation) = match allocation.progress {
        RootComponentChildAllocationProgressView::Reserved => {
            (RootComponentAllocationPhase::Reserved, None, None)
        }
        RootComponentChildAllocationProgressView::CreationIntent(effect) => (
            RootComponentAllocationPhase::CreationIntent,
            Some(creation_evidence(effect, None)),
            None,
        ),
        RootComponentChildAllocationProgressView::Created { effect, canister } => (
            RootComponentAllocationPhase::Created,
            Some(creation_evidence(effect, Some(canister))),
            None,
        ),
        RootComponentChildAllocationProgressView::InstallIntent {
            creation,
            canister,
            installation,
        } => (
            RootComponentAllocationPhase::InstallIntent,
            Some(creation_evidence(creation, Some(canister))),
            Some(child_install_evidence(installation)),
        ),
        RootComponentChildAllocationProgressView::Installed {
            creation,
            canister,
            installation,
        } => (
            RootComponentAllocationPhase::Installed,
            Some(creation_evidence(creation, Some(canister))),
            Some(child_install_evidence(installation)),
        ),
        RootComponentChildAllocationProgressView::Verified {
            creation,
            canister,
            installation,
        } => (
            RootComponentAllocationPhase::Verified,
            Some(creation_evidence(creation, Some(canister))),
            Some(child_install_evidence(installation)),
        ),
        RootComponentChildAllocationProgressView::Committed {
            creation,
            canister,
            installation,
            ..
        } => (
            RootComponentAllocationPhase::Committed,
            Some(creation_evidence(creation, Some(canister))),
            Some(child_install_evidence(installation)),
        ),
    };
    RootComponentChildAllocationResponse {
        operation_id: allocation.operation_id,
        component: allocation.component,
        parent_canister_id: allocation.parent_canister_id,
        parent_role: allocation.parent_role,
        child_role: allocation.child_role,
        child_kind: allocation.child_kind,
        maximum_instances_per_parent: allocation.maximum_instances_per_parent,
        maximum_descendants: allocation.maximum_descendants,
        maximum_registry_bytes: allocation.maximum_registry_bytes,
        reserved_against_registry: allocation.reserved_against_registry,
        release_set: allocation.release_set,
        phase,
        creation,
        installation,
    }
}

pub(super) const fn registry_evidence(
    head: &ComponentRegistryHead,
) -> ComponentRegistryVersionEvidence {
    ComponentRegistryVersionEvidence {
        component: head.component,
        revision: head.revision,
        content_hash: head.content_hash,
    }
}

pub(super) fn child_commit_response(
    allocation: RootComponentChildAllocationView,
    partition: ComponentRegistryPartitionView,
) -> Result<RootComponentChildCommitResponse, InternalError> {
    let RootComponentChildAllocationProgressView::Committed { commitment, .. } =
        &allocation.progress
    else {
        return Err(InternalError::invariant());
    };
    if ComponentPartitionSnapshotAuthority::from_child_commitment(commitment)
        != ComponentPartitionSnapshotAuthority::from_partition(&partition)
    {
        return Err(InternalError::invariant());
    }
    let registry = partition_response(partition.clone());
    let directory = component_directory_head(&partition);
    Ok(RootComponentChildCommitResponse {
        allocation: child_allocation_response(allocation),
        registry,
        directory,
    })
}

pub(super) fn commit_response(
    allocation: RootComponentAllocationView,
    partition: ComponentRegistryPartitionView,
) -> Result<RootComponentCommitResponse, InternalError> {
    let RootComponentAllocationProgressView::Committed { commitment, .. } = &allocation.progress
    else {
        return Err(InternalError::invariant());
    };
    let expected_head = ComponentRegistryHead {
        component: partition.binding.component,
        revision: partition.revision,
        content_hash: partition.content_hash,
    };
    if commitment.registry != expected_head
        || commitment.prepared_registry_encoded_bytes != partition.encoded_bytes
        || commitment.directory_synchronized_at_ns != partition.directory_synchronized_at_ns
    {
        return Err(InternalError::invariant());
    }
    let registry = partition_response(partition.clone());
    let directory = component_directory_head(&partition);
    Ok(RootComponentCommitResponse {
        allocation: allocation_response(allocation)?,
        registry,
        directory,
    })
}

pub(super) fn membership_response(
    allocation: RootComponentAllocationView,
    partition: ComponentRegistryPartitionView,
    target: ComponentRuntimeStatusResponse,
) -> Result<RootComponentMembershipActivationResponse, InternalError> {
    let membership = committed_directory_receipt(&allocation)?
        .membership
        .as_ref()
        .ok_or_else(InternalError::invariant)?;
    let encoded_bytes_covered = membership.registry_encoded_bytes <= partition.encoded_bytes;
    if !membership.directory_synchronized
        || !encoded_bytes_covered
        || membership.directory_synchronized_at_ns != partition.directory_synchronized_at_ns
    {
        return Err(InternalError::invariant());
    }
    let directory = component_directory_head(&partition);
    let registry = partition_response(partition);
    Ok(RootComponentMembershipActivationResponse {
        allocation: allocation_response(allocation)?,
        registry,
        directory,
        target,
    })
}

pub(super) fn child_membership_response(
    allocation: RootComponentChildAllocationView,
    committed_partition: ComponentRegistryPartitionView,
    active_partition: ComponentRegistryPartitionView,
    child: ComponentRuntimeStatusResponse,
) -> Result<RootComponentChildMembershipActivationResponse, InternalError> {
    let membership = committed_child_directory_receipt(&allocation)?
        .membership
        .as_ref()
        .ok_or_else(InternalError::invariant)?;
    let membership_matches_partition =
        ComponentPartitionSnapshotAuthority::from_child_membership(membership)
            == ComponentPartitionSnapshotAuthority::from_partition(&active_partition);
    if !membership.directory_synchronized || !membership_matches_partition {
        return Err(InternalError::invariant());
    }
    let directory = component_directory_head(&active_partition);
    let registry = partition_response(active_partition);
    Ok(RootComponentChildMembershipActivationResponse {
        committed: child_commit_response(allocation, committed_partition)?,
        registry,
        directory,
        child,
    })
}

pub(super) fn partition_response(
    partition: ComponentRegistryPartitionView,
) -> ComponentRegistryPartitionResponse {
    ComponentRegistryPartitionResponse {
        head: ComponentRegistryHead {
            component: partition.binding.component,
            revision: partition.revision,
            content_hash: partition.content_hash,
        },
        binding: partition.binding,
        protocol_profile_digest: partition.protocol_profile_digest,
        provisioning_origin: partition.provisioning_origin,
        release_set: partition.release_set,
        status: partition.status,
        reserved_descendants: partition.reserved_descendants,
        committed_descendants: partition.committed_descendants,
        encoded_bytes: partition.encoded_bytes,
    }
}

pub(in crate::workflow) fn component_directory_head(
    partition: &ComponentRegistryPartitionView,
) -> ComponentDirectoryHead {
    ComponentDirectoryHead {
        provenance: ComponentDirectoryProvenance {
            component: partition.binding.clone(),
            source_fleet_subnet_root: partition.binding.fleet_subnet_root,
            component_registry_revision: partition.revision,
            component_registry_content_hash: partition.content_hash,
            synchronized_at_ns: partition.directory_synchronized_at_ns,
        },
        descendant_count: partition.committed_descendants,
    }
}
