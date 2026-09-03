//! Module: workflow::component_registry::lifecycle_drivers
//!
//! Responsibility: resume bounded Component allocation and retirement operations.
//! Does not own: durable records, endpoint authorization, policy, or platform effects.
//! Boundary: observes one retained operation and schedules only its next existing workflow step.

use super::*;
use std::{cell::RefCell, collections::BTreeSet};

thread_local! {
    static SCHEDULED_COMPONENT_CHILD_ALLOCATIONS: RefCell<BTreeSet<(ComponentInstanceId, [u8; 32])>> =
        const { RefCell::new(BTreeSet::new()) };
}

const MAX_COMPONENT_CHILD_ALLOCATION_PHASES_PER_INVOCATION: usize = 16;

/// Privately advance one accepted ordinary or peer top-level allocation.
pub fn schedule_component_allocation(operation_id: [u8; 32]) {
    schedule_component_allocation_after(operation_id, Duration::ZERO);
}

fn schedule_component_allocation_after(operation_id: [u8; 32], delay: Duration) {
    TimerApi::defer_lifecycle_required(
        delay,
        "Fleet Subnet Root Component allocation",
        async move {
            match Box::pin(advance_component_allocation_once(operation_id)).await {
                Ok(true) => {}
                Ok(false) => schedule_component_allocation_after(operation_id, Duration::ZERO),
                Err(_) => {
                    schedule_component_allocation_after(operation_id, Duration::from_secs(1));
                }
            }
        },
    );
}

async fn advance_component_allocation_once(operation_id: [u8; 32]) -> Result<bool, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    let fleet_directory =
        validate_current_mirror_authority(&authority, root, &preparation_request)?;
    let topology = ConfigOps::component_topology()?;
    let allocation =
        ComponentRegistryOps::allocation(operation_id).ok_or_else(InternalError::unavailable)?;
    if matches!(
        &allocation.provisioning_origin,
        ComponentProvisioningOrigin::ComponentGroup { .. }
    ) {
        return Err(InternalError::invariant());
    }
    validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        operation_id,
    )?;

    match &allocation.progress {
        RootComponentAllocationProgressView::Reserved
        | RootComponentAllocationProgressView::CreationIntent(_) => {
            let plan = creation_plan(root, &store, &allocation)?;
            advance_creation(operation_id, allocation, plan)?;
            Ok(false)
        }
        RootComponentAllocationProgressView::Created { .. }
        | RootComponentAllocationProgressView::InstallIntent { .. }
        | RootComponentAllocationProgressView::Installed { .. } => {
            let plan = component_install_plan(&authority.binding, &store, &allocation).await?;
            Box::pin(advance_install(operation_id, allocation, plan)).await?;
            Ok(false)
        }
        RootComponentAllocationProgressView::Verified { .. } => {
            let plan = component_install_plan(&authority.binding, &store, &allocation).await?;
            let installation = committed_or_verified_installation(&allocation)?;
            validate_install_effect(installation, &plan.durable)?;
            verify_committed_or_verified_install(&allocation, &plan).await?;
            let (committed, partition) = ComponentRegistryOps::commit_verified(
                operation_id,
                IcOps::now_nanos(),
                plan.durable.maximum_registry_bytes,
                fleet_directory,
            )?;
            validate_partition(
                &authority.binding,
                authority.initial_release_set,
                &topology,
                &partition,
            )?;
            commit_response(committed, partition)?;
            Ok(false)
        }
        RootComponentAllocationProgressView::Committed { commitment, .. } => {
            let plan = prepared_component_runtime_plan_for_reconciliation(operation_id).await?;
            if !commitment.directory_prepared {
                prepare_component_directories_with_plan(
                    RootComponentDirectoryPreparationRequest { operation_id },
                    plan,
                )
                .await?;
                return Ok(false);
            }
            if !commitment.runtime_activated {
                Box::pin(activate_component_runtime_with_plan(
                    RootComponentRuntimeActivationRequest { operation_id },
                    plan,
                ))
                .await?;
                return Ok(false);
            }
            if commitment
                .membership
                .as_ref()
                .is_none_or(|membership| !membership.directory_synchronized)
            {
                Box::pin(activate_component_membership_with_plan(
                    RootComponentMembershipActivationRequest { operation_id },
                    plan,
                ))
                .await?;
                return Ok(false);
            }
            Ok(component_allocation_reconciliation_complete(&allocation))
        }
        RootComponentAllocationProgressView::Removed { .. } => Ok(true),
    }
}

pub(super) fn component_allocation_reconciliation_complete(
    allocation: &RootComponentAllocationView,
) -> bool {
    match &allocation.progress {
        RootComponentAllocationProgressView::Committed { commitment, .. } => {
            commitment.directory_prepared
                && commitment.runtime_activated
                && commitment
                    .membership
                    .as_ref()
                    .is_some_and(|membership| membership.directory_synchronized)
        }
        RootComponentAllocationProgressView::Removed { .. } => true,
        _ => false,
    }
}

/// Privately advance one accepted direct-child allocation for its retained parent authority.
pub fn schedule_component_child_allocation(component: ComponentInstanceId, operation_id: [u8; 32]) {
    let inserted = SCHEDULED_COMPONENT_CHILD_ALLOCATIONS
        .with(|scheduled| scheduled.borrow_mut().insert((component, operation_id)));
    if !inserted {
        return;
    }
    schedule_component_child_allocation_after(component, operation_id, Duration::ZERO);
}

fn schedule_component_child_allocation_after(
    component: ComponentInstanceId,
    operation_id: [u8; 32],
    delay: Duration,
) {
    TimerApi::defer_lifecycle_required(
        delay,
        "Fleet Subnet Root Component child allocation",
        async move {
            match Box::pin(advance_component_child_allocation_once(
                component,
                operation_id,
            ))
            .await
            {
                Ok(true) => {
                    SCHEDULED_COMPONENT_CHILD_ALLOCATIONS.with(|scheduled| {
                        scheduled.borrow_mut().remove(&(component, operation_id));
                    });
                }
                Ok(false) => schedule_component_child_allocation_after(
                    component,
                    operation_id,
                    Duration::ZERO,
                ),
                Err(_) => schedule_component_child_allocation_after(
                    component,
                    operation_id,
                    Duration::from_secs(1),
                ),
            }
        },
    );
}

async fn advance_component_child_allocation_once(
    component: ComponentInstanceId,
    operation_id: [u8; 32],
) -> Result<bool, InternalError> {
    let allocation = ComponentRegistryOps::child_allocation(component, operation_id)?
        .ok_or_else(InternalError::unavailable)?;
    let parent_canister_id = allocation.parent_canister_id;
    match &allocation.progress {
        RootComponentChildAllocationProgressView::Reserved
        | RootComponentChildAllocationProgressView::CreationIntent(_) => {
            create_child_allocation_for_parent(
                RootComponentChildCreationRequest {
                    operation_id,
                    component,
                },
                parent_canister_id,
            )
            .await?;
            Ok(false)
        }
        RootComponentChildAllocationProgressView::Created { .. }
        | RootComponentChildAllocationProgressView::InstallIntent { .. }
        | RootComponentChildAllocationProgressView::Installed { .. } => {
            Box::pin(install_child_allocation_for_parent(
                RootComponentChildInstallRequest {
                    operation_id,
                    component,
                },
                parent_canister_id,
            ))
            .await?;
            Ok(false)
        }
        RootComponentChildAllocationProgressView::Verified { .. } => {
            commit_child_allocation_for_parent(
                RootComponentChildCommitRequest {
                    operation_id,
                    component,
                },
                parent_canister_id,
            )
            .await?;
            Ok(false)
        }
        RootComponentChildAllocationProgressView::Committed { commitment, .. } => {
            if !commitment.directory_prepared {
                Box::pin(prepare_child_directories_for_parent(
                    RootComponentChildDirectoryPreparationRequest {
                        operation_id,
                        component,
                    },
                    parent_canister_id,
                ))
                .await?;
                return Ok(false);
            }
            if !commitment.runtime_activated {
                activate_child_runtime_for_parent(
                    RootComponentChildRuntimeActivationRequest {
                        operation_id,
                        component,
                    },
                    parent_canister_id,
                )
                .await?;
                return Ok(false);
            }
            if commitment
                .membership
                .as_ref()
                .is_none_or(|membership| !membership.directory_synchronized)
            {
                Box::pin(activate_child_membership_for_parent(
                    RootComponentChildMembershipActivationRequest {
                        operation_id,
                        component,
                    },
                    parent_canister_id,
                ))
                .await?;
                return Ok(false);
            }
            Ok(true)
        }
    }
}

/// Complete one retained active-parent child allocation within its requesting call.
pub(in crate::workflow) async fn complete_component_child_allocation(
    component: ComponentInstanceId,
    operation_id: [u8; 32],
) -> Result<(), InternalError> {
    for _ in 0..MAX_COMPONENT_CHILD_ALLOCATION_PHASES_PER_INVOCATION {
        if Box::pin(advance_component_child_allocation_once(
            component,
            operation_id,
        ))
        .await?
        {
            return Ok(());
        }
    }

    Err(InternalError::unavailable())
}

/// Privately advance one accepted top-level Component removal.
pub fn schedule_component_removal(component: ComponentInstanceId, operation_id: [u8; 32]) {
    schedule_component_removal_after(component, operation_id, Duration::ZERO);
}

fn schedule_component_removal_after(
    component: ComponentInstanceId,
    operation_id: [u8; 32],
    delay: Duration,
) {
    TimerApi::defer_lifecycle_required(delay, "Fleet Subnet Root Component removal", async move {
        match Box::pin(advance_component_removal_once(component, operation_id)).await {
            Ok(true) => {}
            Ok(false) => {
                schedule_component_removal_after(component, operation_id, Duration::ZERO);
            }
            Err(_) => {
                schedule_component_removal_after(component, operation_id, Duration::from_secs(1));
            }
        }
    });
}

pub(in crate::workflow) async fn advance_component_removal_once(
    component: ComponentInstanceId,
    operation_id: [u8; 32],
) -> Result<bool, InternalError> {
    let draining = ComponentRegistryOps::component_draining(component)?
        .ok_or_else(InternalError::unavailable)?;
    if draining.operation_id != operation_id {
        return Err(InternalError::conflict());
    }
    if !matches!(
        &draining.quiescence,
        Some(RootComponentQuiescenceProgressView::Quiescent(_))
    ) {
        Box::pin(quiesce_component(RootComponentQuiescenceRequest {
            operation_id,
            component,
            expected_registry: draining.registry,
        }))
        .await?;
        return Ok(false);
    }
    if draining.final_inventory.is_none() {
        if draining.descendant_count == 0 {
            finalize_component_inventory(RootComponentFinalInventoryRequest {
                operation_id,
                component,
                expected_registry: draining.registry,
            })
            .await?;
        } else {
            advance_component_draining(RootComponentDrainingAdvanceRequest {
                operation_id,
                component,
            })
            .await?;
        }
        return Ok(false);
    }
    let inventory_hash = draining
        .final_inventory
        .as_ref()
        .ok_or_else(InternalError::invariant)?
        .inventory_hash;
    let request = RootComponentDeletionRequest {
        operation_id,
        component,
        expected_inventory_hash: inventory_hash,
    };
    match draining.deletion {
        None | Some(RootComponentDeletionProgressView::DeleteIntent(_)) => {
            delete_component(request).await?;
            Ok(false)
        }
        Some(RootComponentDeletionProgressView::Deleted(_)) => {
            remove_component_membership(request)?;
            Ok(false)
        }
        Some(RootComponentDeletionProgressView::MembershipRemoved(_)) => Ok(true),
    }
}

pub(super) async fn advance_component_draining_boundary(
    request: RootComponentDrainingAdvanceRequest,
) -> Result<RootComponentDrainingAdvanceResponse, InternalError> {
    let prepared = prepared_component_draining_boundary(request.component).await?;
    match ComponentRegistryOps::advance_component_draining(request.component, request.operation_id)?
    {
        RootComponentDrainingAdvanceView::DescendantSubtreePending { .. } => {
            let removal = ComponentRegistryOps::begin_draining_subtree_removal(
                request.component,
                request.operation_id,
                prepared.maximum_component_registry_bytes,
            )?;
            validate_subtree_removal(
                &prepared.root,
                prepared.release_set,
                &prepared.topology,
                &removal,
                None,
            )?;
            Ok(component_draining_advance_removal_response(
                request, removal,
            ))
        }
        RootComponentDrainingAdvanceView::DescendantRemoval(removal) => {
            validate_subtree_removal(
                &prepared.root,
                prepared.release_set,
                &prepared.topology,
                &removal,
                None,
            )?;
            Ok(component_draining_advance_removal_response(
                request, *removal,
            ))
        }
        RootComponentDrainingAdvanceView::DescendantsEmpty {
            registry,
            descendant_content_hash,
        } => Ok(RootComponentDrainingAdvanceResponse {
            operation_id: request.operation_id,
            component: request.component,
            phase: RootComponentDrainingAdvancePhase::DescendantsEmpty(
                RootComponentDrainingDescendantsEmpty {
                    registry,
                    descendant_content_hash,
                },
            ),
        }),
    }
}

pub(super) async fn prepared_component_draining_boundary(
    component: canic_core::ids::ComponentInstanceId,
) -> Result<PreparedComponentDrainingBoundary, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap,
        expected_fleet_registry: prepared.prepared_against_registry,
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    let fleet_directory =
        validate_current_mirror_authority(&authority, root, &preparation_request)?;
    require_active_root_runtime("Component draining requires an Active Fleet Subnet Root runtime")?;

    let topology = ConfigOps::component_topology()?;
    let partition =
        ComponentRegistryOps::partition(component)?.ok_or_else(InternalError::unavailable)?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_component_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(InternalError::invariant)?
        .limits
        .maximum_registry_bytes;
    Ok(PreparedComponentDrainingBoundary {
        root: authority.binding,
        release_set: authority.initial_release_set,
        topology,
        maximum_component_registry_bytes,
        fleet_directory,
        store,
    })
}

pub(super) async fn advance_subtree_removal_phase(
    removal: RootComponentSubtreeRemovalView,
) -> Result<RootComponentSubtreeRemovalView, InternalError> {
    let action = subtree_removal_action(&removal)?;
    let response = match action {
        ComponentSubtreeRemovalAction::Advance(request) => advance_subtree_removal(request).await?,
        ComponentSubtreeRemovalAction::PrepareStop(request) => {
            prepare_subtree_leaf_stop(request).await?
        }
        ComponentSubtreeRemovalAction::Stop(request) => stop_subtree_leaf(request).await?,
        ComponentSubtreeRemovalAction::PrepareDelete(request) => {
            prepare_subtree_leaf_delete(request).await?
        }
        ComponentSubtreeRemovalAction::Delete(request) => delete_subtree_leaf(request).await?,
        ComponentSubtreeRemovalAction::RemoveMembership(request) => {
            remove_subtree_leaf_membership(request).await?
        }
        ComponentSubtreeRemovalAction::SynchronizeDirectory(request) => {
            Box::pin(synchronize_subtree_leaf_directory(request)).await?
        }
        ComponentSubtreeRemovalAction::FinalizeLeaf(request) => {
            finalize_subtree_leaf(request).await?
        }
    };
    ComponentRegistryOps::subtree_removal(response.component, response.operation_id)?
        .ok_or_else(InternalError::invariant)
}

const fn subtree_removal_action(
    removal: &RootComponentSubtreeRemovalView,
) -> Result<ComponentSubtreeRemovalAction, InternalError> {
    let action = match &removal.progress {
        RootComponentSubtreeRemovalProgressView::Fenced
        | RootComponentSubtreeRemovalProgressView::Traversing { .. } => {
            ComponentSubtreeRemovalAction::Advance(RootComponentSubtreeRemovalAdvanceRequest {
                operation_id: removal.operation_id,
                component: removal.component,
                expected_traversal_steps: removal.traversal_steps,
            })
        }
        RootComponentSubtreeRemovalProgressView::LeafSelected { leaf } => {
            ComponentSubtreeRemovalAction::PrepareStop(
                RootComponentSubtreeRemovalStopPreparationRequest {
                    operation_id: removal.operation_id,
                    component: removal.component,
                    expected_traversal_steps: removal.traversal_steps,
                    expected_leaf_canister_id: leaf.canister_id,
                    expected_leaf_parent_canister_id: leaf.parent_canister_id,
                },
            )
        }
        RootComponentSubtreeRemovalProgressView::StopIntent(stop) => {
            ComponentSubtreeRemovalAction::Stop(RootComponentSubtreeRemovalStopRequest {
                operation_id: removal.operation_id,
                component: removal.component,
                expected_traversal_steps: removal.traversal_steps,
                expected_leaf_canister_id: stop.leaf.canister_id,
                expected_leaf_parent_canister_id: stop.leaf.parent_canister_id,
            })
        }
        RootComponentSubtreeRemovalProgressView::Stopped(stopped) => {
            ComponentSubtreeRemovalAction::PrepareDelete(
                RootComponentSubtreeRemovalDeletePreparationRequest {
                    operation_id: removal.operation_id,
                    component: removal.component,
                    expected_traversal_steps: removal.traversal_steps,
                    expected_leaf_canister_id: stopped.stop.leaf.canister_id,
                    expected_leaf_parent_canister_id: stopped.stop.leaf.parent_canister_id,
                },
            )
        }
        RootComponentSubtreeRemovalProgressView::DeleteIntent(deletion) => {
            ComponentSubtreeRemovalAction::Delete(RootComponentSubtreeRemovalDeleteRequest {
                operation_id: removal.operation_id,
                component: removal.component,
                expected_traversal_steps: removal.traversal_steps,
                expected_leaf_canister_id: deletion.stopped.stop.leaf.canister_id,
                expected_leaf_parent_canister_id: deletion.stopped.stop.leaf.parent_canister_id,
            })
        }
        RootComponentSubtreeRemovalProgressView::Deleted(deleted) => {
            let leaf = &deleted.deletion.stopped.stop.leaf;
            ComponentSubtreeRemovalAction::RemoveMembership(subtree_membership_removal_request(
                removal, leaf,
            ))
        }
        RootComponentSubtreeRemovalProgressView::MembershipRemoved(membership) => {
            let leaf = &membership.deleted.deletion.stopped.stop.leaf;
            ComponentSubtreeRemovalAction::SynchronizeDirectory(
                RootComponentSubtreeRemovalDirectorySynchronizationRequest {
                    operation_id: removal.operation_id,
                    component: removal.component,
                    expected_traversal_steps: removal.traversal_steps,
                    expected_leaf_canister_id: leaf.canister_id,
                    expected_leaf_parent_canister_id: leaf.parent_canister_id,
                },
            )
        }
        RootComponentSubtreeRemovalProgressView::DirectorySynchronized(directory) => {
            let leaf = &directory
                .membership_removed
                .deleted
                .deletion
                .stopped
                .stop
                .leaf;
            ComponentSubtreeRemovalAction::FinalizeLeaf(
                RootComponentSubtreeRemovalLeafFinalizationRequest {
                    operation_id: removal.operation_id,
                    component: removal.component,
                    expected_traversal_steps: removal.traversal_steps,
                    expected_leaf_canister_id: leaf.canister_id,
                    expected_leaf_parent_canister_id: leaf.parent_canister_id,
                },
            )
        }
        RootComponentSubtreeRemovalProgressView::Completed(_) => {
            return Err(InternalError::invariant());
        }
    };
    Ok(action)
}

const fn subtree_membership_removal_request(
    removal: &RootComponentSubtreeRemovalView,
    leaf: &crate::view::component_registry::RootComponentSubtreeRemovalNodeView,
) -> RootComponentSubtreeRemovalMembershipRemovalRequest {
    RootComponentSubtreeRemovalMembershipRemovalRequest {
        operation_id: removal.operation_id,
        component: removal.component,
        expected_traversal_steps: removal.traversal_steps,
        expected_leaf_canister_id: leaf.canister_id,
        expected_leaf_parent_canister_id: leaf.parent_canister_id,
    }
}

pub(super) fn component_draining_advance_removal_response(
    request: RootComponentDrainingAdvanceRequest,
    removal: RootComponentSubtreeRemovalView,
) -> RootComponentDrainingAdvanceResponse {
    RootComponentDrainingAdvanceResponse {
        operation_id: request.operation_id,
        component: request.component,
        phase: RootComponentDrainingAdvancePhase::DescendantRemoval(subtree_removal_response(
            removal,
        )),
    }
}

/// Privately advance one accepted subtree removal through its durable phase journal.
pub fn schedule_subtree_removal(component: ComponentInstanceId, operation_id: [u8; 32]) {
    schedule_subtree_removal_after(component, operation_id, Duration::ZERO);
}

fn schedule_subtree_removal_after(
    component: ComponentInstanceId,
    operation_id: [u8; 32],
    delay: Duration,
) {
    TimerApi::defer_lifecycle_required(
        delay,
        "Fleet Subnet Root Component subtree removal",
        async move {
            let request = RootComponentSubtreeRemovalStatusRequest {
                operation_id,
                component,
            };
            match advance_existing_subtree_removal(request).await {
                Ok(response)
                    if matches!(
                        response.phase,
                        RootComponentSubtreeRemovalPhase::Completed(_)
                    ) => {}
                Ok(_) => schedule_subtree_removal_after(component, operation_id, Duration::ZERO),
                Err(_) => {
                    schedule_subtree_removal_after(component, operation_id, Duration::from_secs(1));
                }
            }
        },
    );
}

/// Read one durable removal when present, preserving absence for nested lifecycle admission.
pub(in crate::workflow) fn existing_subtree_removal(
    request: RootComponentSubtreeRemovalStatusRequest,
) -> Result<Option<RootComponentSubtreeRemovalResponse>, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let Some(removal) =
        ComponentRegistryOps::subtree_removal(request.component, request.operation_id)?
    else {
        return Ok(None);
    };
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &ConfigOps::component_topology()?,
        &removal,
        None,
    )?;
    Ok(Some(subtree_removal_response(removal)))
}

/// Advance one durable subtree-removal phase using its journal as sole cursor authority.
pub(in crate::workflow) async fn advance_existing_subtree_removal(
    request: RootComponentSubtreeRemovalStatusRequest,
) -> Result<RootComponentSubtreeRemovalResponse, InternalError> {
    let removal = ComponentRegistryOps::subtree_removal(request.component, request.operation_id)?
        .ok_or_else(InternalError::unavailable)?;
    let removal = Box::pin(advance_subtree_removal_phase(removal)).await?;
    Ok(subtree_removal_response(removal))
}
