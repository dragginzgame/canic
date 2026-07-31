//! Module: workflow::component_registry
//!
//! Responsibility: prepare Component Registry authority and advance root-executed Component-tree lifecycle.
//! Does not own: topology compilation, Store publication, or root runtime activation.
//! Boundary: every mutation follows exact Store and active Registry Mirror/Directory verification.

use crate::{
    ops::{
        component_registry::{
            ComponentRegistryOps, RootComponentChildInstallPlan, RootComponentCreationPlan,
            RootComponentInstallPlan,
        },
        fleet_registry_mirror::FleetRegistryMirrorOps,
    },
    view::component_registry::{
        ComponentDirectoryCanonicalCursor, ComponentDirectoryPageSelection,
        ComponentRegistryPartitionView, RootComponentAllocationProgressView,
        RootComponentAllocationView, RootComponentChildAllocationProgressView,
        RootComponentChildAllocationView, RootComponentChildCommitmentView,
        RootComponentChildInstallEffectView, RootComponentChildMembershipView,
        RootComponentCreationEffectView, RootComponentDeletionIntentView,
        RootComponentDeletionProgressView, RootComponentDrainingAdvanceView,
        RootComponentDrainingView, RootComponentFinalInventoryView,
        RootComponentInitialInventoryView, RootComponentInstallEffectView,
        RootComponentMembershipRemovedView, RootComponentQuiescenceProgressView,
        RootComponentQuiescenceStopIntentView, RootComponentRegistryView,
        RootComponentSubtreeDeleteEffectView, RootComponentSubtreeDirectoryConvergenceView,
        RootComponentSubtreeDirectorySynchronizedView, RootComponentSubtreeMembershipRemovedView,
        RootComponentSubtreeRemovalProgressView, RootComponentSubtreeRemovalView,
        RootComponentSubtreeStopEffectView, RootComponentSubtreeStoppedEffectView,
    },
    workflow::{
        bootstrap::root_store, deployment, runtime::template::resolved_root_store_module_source,
    },
};
use candid::CandidType;
use canic_core::api::runtime::install::{ApprovedModulePayload, ApprovedModuleSource};
use canic_core::{
    api::fleet_activation::FleetActivationApi,
    control_plane_support::{
        config::schema::ComponentChildKind,
        error::{InternalError, InternalErrorOrigin},
        ops::{
            component_runtime::ComponentRuntimeOps,
            config::ConfigOps,
            ic::{
                IcOps,
                call::CallOps,
                mgmt::{
                    CanisterInstallMode, CanisterStatus, CanisterStatusObservation,
                    CanisterStatusType, MgmtOps,
                },
            },
        },
        policy::{
            component_allocation::{TopLevelComponentAllocationInput, reserve_top_level_component},
            component_child_allocation::{
                ComponentChildAllocationInput, ComponentChildAllocationReadiness,
                ComponentRegistryVersionEvidence, reserve_component_child,
            },
        },
        workflow::{cost_guard::CostGuardWorkflow, runtime::install::ModuleInstallWorkflow},
    },
    dto::{
        abi::v1::{CanisterInitAuthority, CanisterInitPayload},
        component_registry::{
            ComponentDirectoryChildEntry, ComponentDirectoryHead, ComponentDirectoryHeadRequest,
            ComponentDirectoryPageCursor, ComponentDirectoryPageRequest,
            ComponentDirectoryPageResponse, ComponentDirectoryProvenance, ComponentLifecycleStatus,
            ComponentProvisioningOrigin, ComponentRegistryHead, ComponentRegistryPartitionRequest,
            ComponentRegistryPartitionResponse, ComponentRuntimeActivationRequest,
            ComponentRuntimeDirectoryAuthority, ComponentRuntimeDirectoryConvergenceEvidence,
            ComponentRuntimeDirectoryPreparationRequest,
            ComponentRuntimeDirectorySynchronizationRequest, ComponentRuntimePhase,
            ComponentRuntimeStatusResponse, RootComponentAllocationPhase,
            RootComponentAllocationRequest, RootComponentAllocationResponse,
            RootComponentAllocationStatusRequest, RootComponentChildAllocationRequest,
            RootComponentChildAllocationResponse, RootComponentChildAllocationStatusRequest,
            RootComponentChildCommitRequest, RootComponentChildCommitResponse,
            RootComponentChildCreationRequest, RootComponentChildDirectoryPreparationRequest,
            RootComponentChildDirectoryPreparationResponse, RootComponentChildInstallEvidence,
            RootComponentChildInstallRequest, RootComponentChildMembershipActivationRequest,
            RootComponentChildMembershipActivationResponse,
            RootComponentChildRuntimeActivationRequest,
            RootComponentChildRuntimeActivationResponse, RootComponentCommitRequest,
            RootComponentCommitResponse, RootComponentCreationEvidence,
            RootComponentCreationRequest, RootComponentDeletedReceipt, RootComponentDeletionIntent,
            RootComponentDeletionPhase, RootComponentDeletionRequest,
            RootComponentDeletionResponse, RootComponentDeletionStatusRequest,
            RootComponentDirectoryPreparationRequest, RootComponentDirectoryPreparationResponse,
            RootComponentDrainingAdvancePhase, RootComponentDrainingAdvanceRequest,
            RootComponentDrainingAdvanceResponse, RootComponentDrainingDescendantsEmpty,
            RootComponentDrainingRequest, RootComponentDrainingResponse,
            RootComponentDrainingStatusRequest, RootComponentFinalInventory,
            RootComponentFinalInventoryRequest, RootComponentFinalInventoryResponse,
            RootComponentInitialInventoryStatus, RootComponentInstallEvidence,
            RootComponentInstallRequest, RootComponentMembershipActivationRequest,
            RootComponentMembershipActivationResponse, RootComponentMembershipRemovedReceipt,
            RootComponentQuiescencePhase, RootComponentQuiescenceRequest,
            RootComponentQuiescenceResponse, RootComponentQuiescenceStatusRequest,
            RootComponentQuiescenceStopIntent, RootComponentQuiescentReceipt,
            RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
            RootComponentRuntimeActivationRequest, RootComponentRuntimeActivationResponse,
            RootComponentSubtreeRemovalAdvanceRequest, RootComponentSubtreeRemovalCompletedReceipt,
            RootComponentSubtreeRemovalDeleteIntent,
            RootComponentSubtreeRemovalDeletePreparationRequest,
            RootComponentSubtreeRemovalDeleteRequest, RootComponentSubtreeRemovalDeletedReceipt,
            RootComponentSubtreeRemovalDirectoryConvergenceEvidence,
            RootComponentSubtreeRemovalDirectorySynchronizationRequest,
            RootComponentSubtreeRemovalDirectorySynchronizedReceipt,
            RootComponentSubtreeRemovalLeafFinalizationRequest,
            RootComponentSubtreeRemovalMembershipRemovalRequest,
            RootComponentSubtreeRemovalMembershipRemovedReceipt, RootComponentSubtreeRemovalNode,
            RootComponentSubtreeRemovalPhase, RootComponentSubtreeRemovalRequest,
            RootComponentSubtreeRemovalResponse, RootComponentSubtreeRemovalStatusRequest,
            RootComponentSubtreeRemovalStopIntent,
            RootComponentSubtreeRemovalStopPreparationRequest,
            RootComponentSubtreeRemovalStopRequest, RootComponentSubtreeRemovalStoppedReceipt,
        },
        error::Error,
        fleet_activation::FleetActivationPhase,
        fleet_registry::{FleetDirectorySnapshot, FleetRegistryVersion, FleetSubnetRootStatus},
        root_store::{RootStoreBootstrapRequest, RootStoreBootstrapResponse},
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentInstanceId, FleetSubnetRootBinding,
        FleetSubnetRootReleaseSet, ManagedCanisterBinding,
    },
    protocol,
};
use serde::Deserialize;

const MAX_COMPONENT_DIRECTORY_PAGE_ENTRIES: u16 = 100;
const MAX_COMPONENT_DIRECTORY_CURSOR_BYTES: usize = 2_048;

#[derive(Debug, Eq, PartialEq)]
struct ComponentRegistryPreparationAuthority<'a> {
    root: &'a FleetSubnetRootBinding,
    prepared_against_registry: &'a FleetRegistryVersion,
    release_set: FleetSubnetRootReleaseSet,
    store_bootstrap: &'a RootStoreBootstrapRequest,
}

impl<'a> ComponentRegistryPreparationAuthority<'a> {
    const fn new(
        root: &'a FleetSubnetRootBinding,
        prepared_against_registry: &'a FleetRegistryVersion,
        release_set: FleetSubnetRootReleaseSet,
        store_bootstrap: &'a RootStoreBootstrapRequest,
    ) -> Self {
        Self {
            root,
            prepared_against_registry,
            release_set,
            store_bootstrap,
        }
    }

    const fn from_registry(registry: &'a RootComponentRegistryView) -> Self {
        Self::new(
            &registry.root,
            &registry.prepared_against_registry,
            registry.release_set,
            &registry.store_bootstrap,
        )
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ComponentDirectoryCursorBinding<'a> {
    directory: &'a ComponentDirectoryHead,
    parent_canister_id: Option<candid::Principal>,
    role: Option<&'a CanisterRole>,
    status: Option<ComponentLifecycleStatus>,
}

impl<'a> ComponentDirectoryCursorBinding<'a> {
    const fn from_payload(payload: &'a ComponentDirectoryCursorPayload) -> Self {
        Self {
            directory: &payload.directory,
            parent_canister_id: payload.parent_canister_id,
            role: payload.role.as_ref(),
            status: payload.status,
        }
    }

    const fn from_request(request: &'a ComponentDirectoryPageRequest) -> Self {
        Self {
            directory: &request.directory,
            parent_canister_id: request.parent_canister_id,
            role: request.role.as_ref(),
            status: request.status,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ComponentChildAllocationAuthority<'a> {
    component: ComponentInstanceId,
    parent_role: &'a CanisterRole,
    child_kind: ComponentChildKind,
    maximum_instances_per_parent: u32,
    maximum_descendants: u32,
    maximum_registry_bytes: u64,
    release_set: FleetSubnetRootReleaseSet,
    reserved_component: ComponentInstanceId,
}

impl<'a> ComponentChildAllocationAuthority<'a> {
    const fn from_allocation(allocation: &'a RootComponentChildAllocationView) -> Self {
        Self {
            component: allocation.component,
            parent_role: &allocation.parent_role,
            child_kind: allocation.child_kind,
            maximum_instances_per_parent: allocation.maximum_instances_per_parent,
            maximum_descendants: allocation.maximum_descendants,
            maximum_registry_bytes: allocation.maximum_registry_bytes,
            release_set: allocation.release_set,
            reserved_component: allocation.reserved_against_registry.component,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ComponentPartitionStateAuthority {
    registry: ComponentRegistryHead,
    descendant_content_hash: [u8; 32],
    reserved_descendants: u32,
    committed_descendants: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ComponentPartitionSnapshotAuthority {
    state: ComponentPartitionStateAuthority,
    registry_encoded_bytes: u64,
    directory_synchronized_at_ns: u64,
}

impl ComponentPartitionSnapshotAuthority {
    const fn from_partition(partition: &ComponentRegistryPartitionView) -> Self {
        Self {
            state: ComponentPartitionStateAuthority {
                registry: ComponentRegistryHead {
                    component: partition.binding.component,
                    revision: partition.revision,
                    content_hash: partition.content_hash,
                },
                descendant_content_hash: partition.descendant_content_hash,
                reserved_descendants: partition.reserved_descendants,
                committed_descendants: partition.committed_descendants,
            },
            registry_encoded_bytes: partition.encoded_bytes,
            directory_synchronized_at_ns: partition.directory_synchronized_at_ns,
        }
    }

    fn from_child_commitment(commitment: &RootComponentChildCommitmentView) -> Self {
        Self {
            state: ComponentPartitionStateAuthority {
                registry: commitment.registry.clone(),
                descendant_content_hash: commitment.descendant_content_hash,
                reserved_descendants: commitment.reserved_descendants,
                committed_descendants: commitment.committed_descendants,
            },
            registry_encoded_bytes: commitment.registry_encoded_bytes,
            directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
        }
    }

    fn from_child_membership(membership: &RootComponentChildMembershipView) -> Self {
        Self {
            state: ComponentPartitionStateAuthority {
                registry: membership.registry.clone(),
                descendant_content_hash: membership.descendant_content_hash,
                reserved_descendants: membership.reserved_descendants,
                committed_descendants: membership.committed_descendants,
            },
            registry_encoded_bytes: membership.registry_encoded_bytes,
            directory_synchronized_at_ns: membership.directory_synchronized_at_ns,
        }
    }
}

struct PreparedComponentRuntimePlan {
    root_binding: canic_core::ids::FleetSubnetRootBinding,
    allocation: RootComponentAllocationView,
    partition: ComponentRegistryPartitionView,
    target_canister: candid::Principal,
    target_binding: ManagedCanisterBinding,
    directory_request: ComponentRuntimeDirectoryPreparationRequest,
    directory_authority_hash: [u8; 32],
    maximum_component_registry_bytes: u64,
}

struct PreparedChildRuntimePlan {
    root_binding: canic_core::ids::FleetSubnetRootBinding,
    allocation: RootComponentChildAllocationView,
    committed_partition: ComponentRegistryPartitionView,
    child_canister: candid::Principal,
    child_binding: ManagedCanisterBinding,
    owning_component_binding: ManagedCanisterBinding,
    requesting_parent_binding: ManagedCanisterBinding,
    parent_binding: Option<ManagedCanisterBinding>,
    directory_request: ComponentRuntimeDirectoryPreparationRequest,
    directory_authority_hash: [u8; 32],
}

#[derive(Clone, Debug)]
struct PreparedSubtreeLeafStopPlan {
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    traversal_steps: u32,
    stop: RootComponentSubtreeStopEffectView,
    expected_status_module_hash: [u8; 32],
    maximum_component_registry_bytes: u64,
    progressed_beyond_stopped: bool,
}

#[derive(Clone, Debug)]
struct PreparedComponentQuiescencePlan {
    component: ComponentInstanceId,
    operation_id: [u8; 32],
    stop: RootComponentQuiescenceStopIntentView,
    expected_status_module_hash: [u8; 32],
    already_quiescent: bool,
}

struct PreparedComponentDrainingBoundary {
    root: FleetSubnetRootBinding,
    release_set: FleetSubnetRootReleaseSet,
    topology: canic_core::control_plane_support::config::ComponentTopology,
    maximum_component_registry_bytes: u64,
    fleet_directory: FleetDirectorySnapshot,
    store: RootStoreBootstrapResponse,
}

#[derive(Clone, Debug)]
struct PreparedComponentDeletionPlan {
    component: ComponentInstanceId,
    operation_id: [u8; 32],
    deletion: RootComponentDeletionIntentView,
    expected_status_module_hash: [u8; 32],
    already_deleted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ComponentDeletionRequestAuthority {
    operation_id: [u8; 32],
    component: ComponentInstanceId,
    inventory_hash: [u8; 32],
}

impl ComponentDeletionRequestAuthority {
    const fn from_request(request: &RootComponentDeletionRequest) -> Self {
        Self {
            operation_id: request.operation_id,
            component: request.component,
            inventory_hash: request.expected_inventory_hash,
        }
    }

    const fn from_durable(
        draining: &RootComponentDrainingView,
        deletion: &RootComponentDeletionIntentView,
    ) -> Self {
        Self {
            operation_id: draining.operation_id,
            component: draining.component,
            inventory_hash: deletion.final_inventory.inventory_hash,
        }
    }
}

fn terminal_component_membership_removal_response(
    request: &RootComponentDeletionRequest,
) -> Result<Option<RootComponentDeletionResponse>, InternalError> {
    let Some(draining) = ComponentRegistryOps::component_draining(request.component)? else {
        return Ok(None);
    };
    let Some(RootComponentDeletionProgressView::MembershipRemoved(receipt)) = &draining.deletion
    else {
        return Ok(None);
    };
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let request_authority = ComponentDeletionRequestAuthority::from_request(request);
    let durable_authority =
        ComponentDeletionRequestAuthority::from_durable(&draining, &receipt.deleted.deletion);
    if request_authority != durable_authority {
        return Err(InternalError::conflict(
            "Component deletion request differs from terminal removal authority",
        ));
    }
    component_deletion_response(draining).map(Some)
}

enum ComponentDrainingRemovalAction {
    Advance(RootComponentSubtreeRemovalAdvanceRequest),
    PrepareStop(RootComponentSubtreeRemovalStopPreparationRequest),
    Stop(RootComponentSubtreeRemovalStopRequest),
    PrepareDelete(RootComponentSubtreeRemovalDeletePreparationRequest),
    Delete(RootComponentSubtreeRemovalDeleteRequest),
    RemoveMembership(RootComponentSubtreeRemovalMembershipRemovalRequest),
    SynchronizeDirectory(RootComponentSubtreeRemovalDirectorySynchronizationRequest),
    FinalizeLeaf(RootComponentSubtreeRemovalLeafFinalizationRequest),
}

#[derive(Clone, Debug)]
struct PreparedSubtreeLeafDeletePlan {
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    traversal_steps: u32,
    deletion: RootComponentSubtreeDeleteEffectView,
    expected_status_module_hash: [u8; 32],
    maximum_component_registry_bytes: u64,
    already_deleted: bool,
}

#[derive(CandidType, Deserialize)]
struct ComponentDirectoryCursorPayload {
    directory: ComponentDirectoryHead,
    parent_canister_id: Option<candid::Principal>,
    role: Option<CanisterRole>,
    status: Option<ComponentLifecycleStatus>,
    last_parent_canister_id: candid::Principal,
    last_role: CanisterRole,
    last_canister_id: candid::Principal,
}

/// Prepare the one empty Component Registry meta record under exact active root authority.
pub async fn prepare(
    request: RootComponentRegistryPreparationRequest,
) -> Result<RootComponentRegistryStatusResponse, InternalError> {
    let (authority, root) = root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;
    if ComponentRegistryOps::current().is_some() {
        validate_current_mirror_authority(&authority, root, &request)?;
    } else {
        validate_preparation_authority(&authority, root, &request)?;
    }

    let prepared = ComponentRegistryOps::prepare(
        authority.binding,
        request.expected_fleet_registry,
        authority.initial_release_set,
        request.store_bootstrap,
    )?;
    response(root, &prepared)
}

/// Independently verify the durable Component Registry meta record without mutation.
pub async fn status(
    request: RootComponentRegistryPreparationRequest,
) -> Result<RootComponentRegistryStatusResponse, InternalError> {
    let (authority, root) = root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &request)?;

    let prepared = ComponentRegistryOps::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    let expected = ComponentRegistryPreparationAuthority::new(
        &authority.binding,
        &request.expected_fleet_registry,
        authority.initial_release_set,
        &request.store_bootstrap,
    );
    if ComponentRegistryPreparationAuthority::from_registry(&prepared) != expected {
        return Err(InternalError::conflict(
            "durable Component Registry authority differs from the active root",
        ));
    }
    response(root, &prepared)
}

/// Durably reserve one admitted top-level Component identity and root-local capacity.
pub async fn reserve_allocation(
    request: RootComponentAllocationRequest,
) -> Result<RootComponentAllocationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;

    let provisioning_origin = ComponentProvisioningOrigin::FleetAdministrator {
        caller: IcOps::msg_caller(),
    };
    let topology = ConfigOps::component_topology()?;
    if let Some(existing) = ComponentRegistryOps::allocation(request.operation_id) {
        if existing.component_spec != request.component_spec
            || existing.provisioning_origin != provisioning_origin
        {
            return Err(InternalError::conflict(
                "Component allocation operation is already bound to different intent",
            ));
        }
        validate_allocation_record(
            &authority.binding,
            authority.initial_release_set,
            &topology,
            &existing,
            request.operation_id,
        )?;
        return allocation_response(existing);
    }

    ComponentRegistryOps::require_top_level_allocation_open()?;
    let counts = ComponentRegistryOps::component_spec_counts(&request.component_spec)?;
    let decision = reserve_top_level_component(TopLevelComponentAllocationInput {
        operation_id: request.operation_id,
        component_spec: &request.component_spec,
        root: &authority.binding,
        topology: &topology,
        next_allocation_sequence: prepared.next_allocation_sequence,
        reserved_component_instances: prepared.reserved_component_instances,
        committed_component_instances: prepared.committed_component_instances,
        managed_descendants: prepared.managed_descendants,
        reserved_spec_instances: counts.reserved,
        committed_spec_instances: counts.committed,
    })
    .map_err(InternalError::from)?;
    let reserved = ComponentRegistryOps::reserve_allocation(
        decision,
        request.operation_id,
        provisioning_origin,
        FleetActivationApi::status()
            .map_err(InternalError::public)?
            .phase
            == FleetActivationPhase::Active,
    )?;
    allocation_response(reserved)
}

/// Read one durable top-level Component allocation reservation without mutation.
pub fn allocation_status(
    request: RootComponentAllocationStatusRequest,
) -> Result<RootComponentAllocationResponse, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let allocation = ComponentRegistryOps::allocation(request.operation_id).ok_or_else(|| {
        InternalError::unavailable("Component allocation operation has not been reserved")
    })?;
    let topology = ConfigOps::component_topology()?;
    validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        request.operation_id,
    )?;
    allocation_response(allocation)
}

/// Durably reserve one direct child for the exact registered parent caller.
pub async fn reserve_child_allocation(
    request: RootComponentChildAllocationRequest,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;

    let caller = IcOps::msg_caller();
    let topology = ConfigOps::component_topology()?;
    let parent =
        ComponentRegistryOps::registered_parent(request.component, caller)?.ok_or_else(|| {
            InternalError::public(Error::forbidden(format!(
                "caller {caller} is not a registered member of Component {}",
                request.component
            )))
        })?;
    if let Some(existing) =
        ComponentRegistryOps::child_allocation(request.component, request.operation_id)?
    {
        validate_child_allocation(
            &authority.binding,
            authority.initial_release_set,
            &topology,
            &parent.0,
            &existing,
            Some(&request),
        )?;
        return Ok(child_allocation_response(existing));
    }

    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let current_registry = ComponentRegistryHead {
        component: partition.binding.component,
        revision: partition.revision,
        content_hash: partition.content_hash,
    };
    let fleet_activation = FleetActivationApi::status().map_err(InternalError::public)?;
    let readiness = if fleet_activation.phase != FleetActivationPhase::Active {
        ComponentChildAllocationReadiness::RootRuntimeInactive
    } else if partition.status != ComponentLifecycleStatus::Active {
        ComponentChildAllocationReadiness::ComponentRegistryInactive
    } else if parent.1 != ComponentLifecycleStatus::Active {
        ComponentChildAllocationReadiness::ParentRegistryMemberInactive
    } else {
        ComponentChildAllocationReadiness::Ready
    };
    let component_descendants = partition
        .reserved_descendants
        .checked_add(partition.committed_descendants)
        .ok_or_else(|| InternalError::resource_exhausted("Component descendant count overflow"))?;
    let parent_role_instances = ComponentRegistryOps::parent_role_instances(
        request.component,
        caller,
        &request.child_role,
    )?;
    let decision = reserve_component_child(ComponentChildAllocationInput {
        operation_id: request.operation_id,
        caller,
        component: &partition.binding,
        parent: &parent.0,
        child_role: &request.child_role,
        expected_registry: registry_evidence(&request.expected_registry),
        current_registry: registry_evidence(&current_registry),
        readiness,
        root: &authority.binding,
        topology: &topology,
        reserved_component_instances: prepared.reserved_component_instances,
        committed_component_instances: prepared.committed_component_instances,
        component_descendants,
        root_managed_descendants: prepared.managed_descendants,
        parent_role_instances,
    })
    .map_err(InternalError::from)?;
    let reserved = ComponentRegistryOps::reserve_child_allocation(
        decision,
        request.operation_id,
        request.expected_registry,
    )?;
    Ok(child_allocation_response(reserved))
}

/// Read one durable direct-child reservation for its exact registered parent caller.
pub fn child_allocation_status(
    request: RootComponentChildAllocationStatusRequest,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let caller = IcOps::msg_caller();
    let parent =
        ComponentRegistryOps::registered_parent(request.component, caller)?.ok_or_else(|| {
            InternalError::public(Error::forbidden(format!(
                "caller {caller} is not a registered member of Component {}",
                request.component
            )))
        })?;
    let allocation =
        ComponentRegistryOps::child_allocation(request.component, request.operation_id)?
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
    validate_child_allocation(
        &authority.binding,
        authority.initial_release_set,
        &ConfigOps::component_topology()?,
        &parent.0,
        &allocation,
        None,
    )?;
    Ok(child_allocation_response(allocation))
}

/// Durably advance one exact active Component into Draining.
pub async fn begin_component_draining(
    request: RootComponentDrainingRequest,
) -> Result<RootComponentDrainingResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    let fleet_directory =
        validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component draining requires an Active Fleet Subnet Root runtime",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "draining Component Spec is absent from the protected topology",
            )
        })?
        .limits
        .maximum_registry_bytes;
    let draining = ComponentRegistryOps::begin_component_draining(
        request.component,
        request.operation_id,
        request.expected_registry.clone(),
        IcOps::now_nanos(),
        maximum_registry_bytes,
        fleet_directory.clone(),
    )?;
    let current = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "draining Component partition disappeared after mutation",
        )
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &current,
    )?;
    validate_component_draining(&current, &draining, Some(&request), Some(&fleet_directory))?;
    Ok(component_draining_response(draining))
}

/// Read one durable top-level Component draining operation without mutation.
pub fn component_draining_status(
    request: RootComponentDrainingStatusRequest,
) -> Result<RootComponentDrainingResponse, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let draining =
        ComponentRegistryOps::component_draining(request.component)?.ok_or_else(|| {
            InternalError::unavailable("Component draining operation has not been durably fenced")
        })?;
    if draining.operation_id != request.operation_id {
        return Err(InternalError::conflict(
            "Component draining operation is bound to different intent",
        ));
    }
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component draining authority has no Registry partition",
        )
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &ConfigOps::component_topology()?,
        &partition,
    )?;
    validate_component_draining(&partition, &draining, None, None)?;
    Ok(component_draining_response(draining))
}

/// Converge and stop one exact draining top-level Component before descendant removal.
pub async fn quiesce_component(
    request: RootComponentQuiescenceRequest,
) -> Result<RootComponentQuiescenceResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    let fleet_directory =
        validate_current_mirror_authority(&authority, root, &preparation_request)?;
    require_active_root_runtime(
        "Component quiescence requires an Active Fleet Subnet Root runtime",
    )?;

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_component_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "quiescing Component Spec is absent from the protected topology",
            )
        })?
        .limits
        .maximum_registry_bytes;
    let draining =
        ComponentRegistryOps::component_draining(request.component)?.ok_or_else(|| {
            InternalError::unavailable("Component draining operation has not been durably fenced")
        })?;
    validate_component_draining(&partition, &draining, None, None)?;
    let operation_matches = request.operation_id == draining.operation_id;
    let registry_matches = request.expected_registry == draining.registry;
    if !operation_matches || !registry_matches {
        return Err(InternalError::conflict(
            "Component quiescence request differs from its durable draining authority",
        ));
    }

    let draining = if draining.quiescence.is_none() {
        let component_authority = ComponentRuntimeDirectoryAuthority {
            fleet: fleet_directory,
            component: component_directory_head(&partition),
        };
        let authority_hash = ComponentRuntimeOps::directory_authority_hash(&component_authority)?;
        let binding = ManagedCanisterBinding::Component(partition.binding.clone());
        let convergence =
            converge_active_member_directory(&binding, &component_authority, authority_hash)
                .await?;
        let artifact = exact_store_artifact(&store, &partition.binding.role)?;
        ComponentRegistryOps::prepare_component_quiescence(
            request.component,
            request.operation_id,
            request.expected_registry.clone(),
            convergence,
            artifact.payload_hash,
            IcOps::now_nanos(),
            maximum_component_registry_bytes,
        )?
    } else {
        draining
    };
    let plan =
        prepared_component_quiescence_plan(&authority.binding, &store, &partition, &draining)?;
    if plan.already_quiescent {
        return component_quiescence_response(draining);
    }
    observe_or_stop_component(&plan).await?;
    let quiescent = ComponentRegistryOps::mark_component_quiescent(
        plan.component,
        plan.operation_id,
        plan.expected_status_module_hash,
        IcOps::now_nanos(),
    )?;
    let current = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "quiescent Component partition disappeared after terminal mutation",
        )
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &current,
    )?;
    validate_component_draining(&current, &quiescent, None, None)?;
    component_quiescence_response(quiescent)
}

/// Read one draining Component's durable quiescence progress without mutation.
pub fn component_quiescence_status(
    request: RootComponentQuiescenceStatusRequest,
) -> Result<RootComponentQuiescenceResponse, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let draining =
        ComponentRegistryOps::component_draining(request.component)?.ok_or_else(|| {
            InternalError::unavailable("Component draining operation has not been durably fenced")
        })?;
    if request.operation_id != draining.operation_id {
        return Err(InternalError::conflict(
            "Component quiescence status is bound to different draining intent",
        ));
    }
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component quiescence authority has no Registry partition",
        )
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &ConfigOps::component_topology()?,
        &partition,
    )?;
    validate_component_draining(&partition, &draining, None, None)?;
    component_quiescence_response(draining)
}

/// Advance at most one deterministic post-order phase for a quiescent Component drain.
pub async fn advance_component_draining(
    request: RootComponentDrainingAdvanceRequest,
) -> Result<RootComponentDrainingAdvanceResponse, InternalError> {
    match ComponentRegistryOps::advance_component_draining(request.component, request.operation_id)?
    {
        RootComponentDrainingAdvanceView::DescendantRemoval(removal) => {
            let removal = advance_draining_removal_phase(*removal).await?;
            Ok(component_draining_advance_removal_response(
                request, removal,
            ))
        }
        RootComponentDrainingAdvanceView::DescendantSubtreePending { .. }
        | RootComponentDrainingAdvanceView::DescendantsEmpty { .. } => {
            advance_component_draining_boundary(request).await
        }
    }
}

/// Freeze exact empty Component Registry and current Fleet Directory authority.
pub async fn finalize_component_inventory(
    request: RootComponentFinalInventoryRequest,
) -> Result<RootComponentFinalInventoryResponse, InternalError> {
    let prepared = prepared_component_draining_boundary(request.component).await?;
    let inventory = ComponentRegistryOps::finalize_component_inventory(
        request.component,
        request.operation_id,
        request.expected_registry,
        prepared.fleet_directory,
        IcOps::now_nanos(),
    )?;
    Ok(component_final_inventory_response(
        request.operation_id,
        request.component,
        inventory,
    ))
}

/// Reconcile one qualified top-level deletion and commit only independently observed absence.
pub async fn delete_component(
    request: RootComponentDeletionRequest,
) -> Result<RootComponentDeletionResponse, InternalError> {
    if let Some(response) = terminal_component_membership_removal_response(&request)? {
        return Ok(response);
    }
    let prepared = prepared_component_draining_boundary(request.component).await?;
    let draining = ComponentRegistryOps::prepare_component_deletion(
        request.component,
        request.operation_id,
        request.expected_inventory_hash,
        IcOps::now_nanos(),
    )?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_component_draining(&partition, &draining, None, None)?;
    let plan = prepared_component_deletion_plan(
        &prepared.root,
        &prepared.store,
        &partition,
        &draining,
        &request,
    )?;
    if plan.already_deleted {
        return component_deletion_response(draining);
    }

    observe_or_delete_component(&plan).await?;
    let deleted = ComponentRegistryOps::mark_component_deleted(
        plan.component,
        plan.operation_id,
        plan.deletion.final_inventory.inventory_hash,
        IcOps::now_nanos(),
    )?;
    validate_component_draining(&partition, &deleted, None, None)?;
    component_deletion_response(deleted)
}

/// Atomically remove one independently deleted Component from local membership.
pub fn remove_component_membership(
    request: RootComponentDeletionRequest,
) -> Result<RootComponentDeletionResponse, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    if let Some(partition) = ComponentRegistryOps::partition(request.component)? {
        validate_partition(
            &authority.binding,
            authority.initial_release_set,
            &ConfigOps::component_topology()?,
            &partition,
        )?;
    }
    let removed = ComponentRegistryOps::remove_component_membership(
        request.component,
        request.operation_id,
        request.expected_inventory_hash,
        IcOps::now_nanos(),
    )?;
    component_deletion_response(removed)
}

/// Read one finalized Component's durable top-level deletion progress without mutation.
pub fn component_deletion_status(
    request: RootComponentDeletionStatusRequest,
) -> Result<RootComponentDeletionResponse, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let draining =
        ComponentRegistryOps::component_draining(request.component)?.ok_or_else(|| {
            InternalError::unavailable("Component draining operation has not been durably fenced")
        })?;
    if request.operation_id != draining.operation_id {
        return Err(InternalError::conflict(
            "Component deletion status is bound to different draining intent",
        ));
    }
    if let Some(partition) = ComponentRegistryOps::partition(request.component)? {
        validate_partition(
            &authority.binding,
            authority.initial_release_set,
            &ConfigOps::component_topology()?,
            &partition,
        )?;
        validate_component_draining(&partition, &draining, None, None)?;
    } else if !matches!(
        draining.deletion,
        Some(RootComponentDeletionProgressView::MembershipRemoved(_))
    ) {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component deletion authority has no live partition or terminal removal receipt",
        ));
    }
    component_deletion_response(draining)
}

async fn advance_component_draining_boundary(
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

async fn prepared_component_draining_boundary(
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
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component draining requires an Active Fleet Subnet Root runtime",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_component_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "draining Component Spec is absent from the protected topology",
            )
        })?
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

async fn advance_draining_removal_phase(
    removal: RootComponentSubtreeRemovalView,
) -> Result<RootComponentSubtreeRemovalView, InternalError> {
    let action = component_draining_removal_action(&removal)?;
    let response = match action {
        ComponentDrainingRemovalAction::Advance(request) => {
            advance_subtree_removal(request).await?
        }
        ComponentDrainingRemovalAction::PrepareStop(request) => {
            prepare_subtree_leaf_stop(request).await?
        }
        ComponentDrainingRemovalAction::Stop(request) => stop_subtree_leaf(request).await?,
        ComponentDrainingRemovalAction::PrepareDelete(request) => {
            prepare_subtree_leaf_delete(request).await?
        }
        ComponentDrainingRemovalAction::Delete(request) => delete_subtree_leaf(request).await?,
        ComponentDrainingRemovalAction::RemoveMembership(request) => {
            remove_subtree_leaf_membership(request).await?
        }
        ComponentDrainingRemovalAction::SynchronizeDirectory(request) => {
            synchronize_subtree_leaf_directory(request).await?
        }
        ComponentDrainingRemovalAction::FinalizeLeaf(request) => {
            finalize_subtree_leaf(request).await?
        }
    };
    ComponentRegistryOps::subtree_removal(response.component, response.operation_id)?.ok_or_else(
        || {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Component draining phase removed its durable subtree cursor",
            )
        },
    )
}

fn component_draining_removal_action(
    removal: &RootComponentSubtreeRemovalView,
) -> Result<ComponentDrainingRemovalAction, InternalError> {
    let action = match &removal.progress {
        RootComponentSubtreeRemovalProgressView::Fenced
        | RootComponentSubtreeRemovalProgressView::Traversing { .. } => {
            ComponentDrainingRemovalAction::Advance(RootComponentSubtreeRemovalAdvanceRequest {
                operation_id: removal.operation_id,
                component: removal.component,
                expected_traversal_steps: removal.traversal_steps,
            })
        }
        RootComponentSubtreeRemovalProgressView::LeafSelected { leaf } => {
            ComponentDrainingRemovalAction::PrepareStop(
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
            ComponentDrainingRemovalAction::Stop(RootComponentSubtreeRemovalStopRequest {
                operation_id: removal.operation_id,
                component: removal.component,
                expected_traversal_steps: removal.traversal_steps,
                expected_leaf_canister_id: stop.leaf.canister_id,
                expected_leaf_parent_canister_id: stop.leaf.parent_canister_id,
            })
        }
        RootComponentSubtreeRemovalProgressView::Stopped(stopped) => {
            ComponentDrainingRemovalAction::PrepareDelete(
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
            ComponentDrainingRemovalAction::Delete(RootComponentSubtreeRemovalDeleteRequest {
                operation_id: removal.operation_id,
                component: removal.component,
                expected_traversal_steps: removal.traversal_steps,
                expected_leaf_canister_id: deletion.stopped.stop.leaf.canister_id,
                expected_leaf_parent_canister_id: deletion.stopped.stop.leaf.parent_canister_id,
            })
        }
        RootComponentSubtreeRemovalProgressView::Deleted(deleted) => {
            let leaf = &deleted.deletion.stopped.stop.leaf;
            ComponentDrainingRemovalAction::RemoveMembership(
                component_draining_membership_removal_request(removal, leaf),
            )
        }
        RootComponentSubtreeRemovalProgressView::MembershipRemoved(membership) => {
            let leaf = &membership.deleted.deletion.stopped.stop.leaf;
            ComponentDrainingRemovalAction::SynchronizeDirectory(
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
            ComponentDrainingRemovalAction::FinalizeLeaf(
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
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Component draining cursor retained a completed subtree target",
            ));
        }
    };
    Ok(action)
}

const fn component_draining_membership_removal_request(
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

fn component_draining_advance_removal_response(
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

/// Durably fence one registered child subtree before quiescence and post-order removal.
pub async fn begin_subtree_removal(
    request: RootComponentSubtreeRemovalRequest,
) -> Result<RootComponentSubtreeRemovalResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component subtree removal requires an Active Fleet Subnet Root runtime",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    if let Some(existing) =
        ComponentRegistryOps::subtree_removal(request.component, request.operation_id)?
    {
        validate_subtree_removal(
            &authority.binding,
            authority.initial_release_set,
            &topology,
            &existing,
            Some(&request),
        )?;
        return Ok(subtree_removal_response(existing));
    }
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "removal target Component Spec is absent from the protected topology",
            )
        })?
        .limits
        .maximum_registry_bytes;
    let removal = ComponentRegistryOps::begin_subtree_removal(
        request.component,
        request.operation_id,
        request.target_canister_id,
        request.expected_registry,
        maximum_registry_bytes,
    )?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &removal,
        None,
    )?;
    Ok(subtree_removal_response(removal))
}

/// Advance at most one bounded canonical descent batch toward the next post-order leaf.
pub async fn advance_subtree_removal(
    request: RootComponentSubtreeRemovalAdvanceRequest,
) -> Result<RootComponentSubtreeRemovalResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component subtree traversal requires an Active Fleet Subnet Root runtime",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "removal target Component Spec is absent from the protected topology",
            )
        })?
        .limits
        .maximum_registry_bytes;
    let removal = ComponentRegistryOps::advance_subtree_removal(
        request.component,
        request.operation_id,
        request.expected_traversal_steps,
        maximum_registry_bytes,
    )?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &removal,
        None,
    )?;
    Ok(subtree_removal_response(removal))
}

/// Freeze the exact selected leaf and sole root controller before any stop call.
pub async fn prepare_subtree_leaf_stop(
    request: RootComponentSubtreeRemovalStopPreparationRequest,
) -> Result<RootComponentSubtreeRemovalResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component subtree stop preparation requires an Active Fleet Subnet Root runtime",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "removal target Component Spec is absent from the protected topology",
            )
        })?
        .limits
        .maximum_registry_bytes;
    let removal = ComponentRegistryOps::prepare_subtree_leaf_stop(
        request.component,
        request.operation_id,
        request.expected_traversal_steps,
        request.expected_leaf_canister_id,
        request.expected_leaf_parent_canister_id,
        maximum_registry_bytes,
    )?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &removal,
        None,
    )?;
    Ok(subtree_removal_response(removal))
}

/// Reconcile one prepared leaf stop and commit only an independently observed stopped receipt.
pub async fn stop_subtree_leaf(
    request: RootComponentSubtreeRemovalStopRequest,
) -> Result<RootComponentSubtreeRemovalResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component subtree stop requires an Active Fleet Subnet Root runtime",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_component_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "removal target Component Spec is absent from the protected topology",
            )
        })?
        .limits
        .maximum_registry_bytes;
    let removal = ComponentRegistryOps::subtree_removal(request.component, request.operation_id)?
        .ok_or_else(|| {
        InternalError::unavailable(
            "Component subtree-removal operation has not been durably fenced",
        )
    })?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &removal,
        None,
    )?;
    if ComponentRegistryOps::subtree_removal_completed_leaf_matches(
        request.component,
        request.operation_id,
        request.expected_traversal_steps,
        request.expected_leaf_canister_id,
        request.expected_leaf_parent_canister_id,
    )? {
        return Ok(subtree_removal_response(removal));
    }
    let plan = prepared_subtree_leaf_stop_plan(
        &authority.binding,
        &store,
        &removal,
        &request,
        maximum_component_registry_bytes,
    )?;
    if plan.progressed_beyond_stopped {
        return Ok(subtree_removal_response(removal));
    }
    observe_or_stop_subtree_leaf(&plan).await?;
    let stopped = ComponentRegistryOps::mark_subtree_leaf_stopped(
        plan.component,
        plan.operation_id,
        plan.traversal_steps,
        plan.stop.leaf.canister_id,
        plan.stop.leaf.parent_canister_id,
        plan.expected_status_module_hash,
        plan.maximum_component_registry_bytes,
    )?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &stopped,
        None,
    )?;
    Ok(subtree_removal_response(stopped))
}

/// Freeze one exact stopped receipt before a destructive management call.
pub async fn prepare_subtree_leaf_delete(
    request: RootComponentSubtreeRemovalDeletePreparationRequest,
) -> Result<RootComponentSubtreeRemovalResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component subtree deletion preparation requires an Active Fleet Subnet Root runtime",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "removal target Component Spec is absent from the protected topology",
            )
        })?
        .limits
        .maximum_registry_bytes;
    let removal = ComponentRegistryOps::prepare_subtree_leaf_delete(
        request.component,
        request.operation_id,
        request.expected_traversal_steps,
        request.expected_leaf_canister_id,
        request.expected_leaf_parent_canister_id,
        maximum_registry_bytes,
    )?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &removal,
        None,
    )?;
    Ok(subtree_removal_response(removal))
}

/// Reconcile one prepared deletion and commit only independently observed absence.
pub async fn delete_subtree_leaf(
    request: RootComponentSubtreeRemovalDeleteRequest,
) -> Result<RootComponentSubtreeRemovalResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component subtree deletion requires an Active Fleet Subnet Root runtime",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_component_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "removal target Component Spec is absent from the protected topology",
            )
        })?
        .limits
        .maximum_registry_bytes;
    let removal = ComponentRegistryOps::subtree_removal(request.component, request.operation_id)?
        .ok_or_else(|| {
        InternalError::unavailable(
            "Component subtree-removal operation has not been durably fenced",
        )
    })?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &removal,
        None,
    )?;
    if ComponentRegistryOps::subtree_removal_completed_leaf_matches(
        request.component,
        request.operation_id,
        request.expected_traversal_steps,
        request.expected_leaf_canister_id,
        request.expected_leaf_parent_canister_id,
    )? {
        return Ok(subtree_removal_response(removal));
    }
    let plan = prepared_subtree_leaf_delete_plan(
        &authority.binding,
        &store,
        &removal,
        &request,
        maximum_component_registry_bytes,
    )?;
    if plan.already_deleted {
        return Ok(subtree_removal_response(removal));
    }
    observe_or_delete_subtree_leaf(&plan).await?;
    let deleted = ComponentRegistryOps::mark_subtree_leaf_deleted(
        plan.component,
        plan.operation_id,
        plan.traversal_steps,
        plan.deletion.stopped.stop.leaf.canister_id,
        plan.deletion.stopped.stop.leaf.parent_canister_id,
        plan.maximum_component_registry_bytes,
    )?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &deleted,
        None,
    )?;
    Ok(subtree_removal_response(deleted))
}

/// Atomically remove one independently deleted leaf from Registry membership and indexes.
pub async fn remove_subtree_leaf_membership(
    request: RootComponentSubtreeRemovalMembershipRemovalRequest,
) -> Result<RootComponentSubtreeRemovalResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    let fleet_directory =
        validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component subtree membership removal requires an Active Fleet Subnet Root runtime",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "removal target Component Spec is absent from the protected topology",
            )
        })?
        .limits
        .maximum_registry_bytes;
    let removal = ComponentRegistryOps::remove_subtree_leaf_membership(
        request.component,
        request.operation_id,
        request.expected_traversal_steps,
        request.expected_leaf_canister_id,
        request.expected_leaf_parent_canister_id,
        IcOps::now_nanos(),
        maximum_registry_bytes,
        fleet_directory,
    )?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &removal,
        None,
    )?;
    Ok(subtree_removal_response(removal))
}

/// Converge the post-removal Directory on the surviving owner and distinct parent.
#[expect(
    clippy::too_many_lines,
    reason = "one workflow reverifies root authority and independently converges both bounded recipients"
)]
pub async fn synchronize_subtree_leaf_directory(
    request: RootComponentSubtreeRemovalDirectorySynchronizationRequest,
) -> Result<RootComponentSubtreeRemovalResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    let fleet_directory =
        validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component subtree Directory synchronization requires an Active Fleet Subnet Root runtime",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "removal target Component Spec is absent from the protected topology",
            )
        })?
        .limits
        .maximum_registry_bytes;
    let removal = ComponentRegistryOps::subtree_removal(request.component, request.operation_id)?
        .ok_or_else(|| {
        InternalError::unavailable(
            "Component subtree-removal operation has not been durably fenced",
        )
    })?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &removal,
        None,
    )?;
    if ComponentRegistryOps::subtree_removal_completed_leaf_matches(
        request.component,
        request.operation_id,
        request.expected_traversal_steps,
        request.expected_leaf_canister_id,
        request.expected_leaf_parent_canister_id,
    )? {
        return Ok(subtree_removal_response(removal));
    }
    let membership_removed = match &removal.progress {
        RootComponentSubtreeRemovalProgressView::MembershipRemoved(receipt) => receipt,
        RootComponentSubtreeRemovalProgressView::DirectorySynchronized(receipt) => {
            validate_subtree_directory_request(&removal, &receipt.membership_removed, &request)?;
            return Ok(subtree_removal_response(removal));
        }
        RootComponentSubtreeRemovalProgressView::Fenced
        | RootComponentSubtreeRemovalProgressView::Traversing { .. }
        | RootComponentSubtreeRemovalProgressView::LeafSelected { .. }
        | RootComponentSubtreeRemovalProgressView::StopIntent(_)
        | RootComponentSubtreeRemovalProgressView::Stopped(_)
        | RootComponentSubtreeRemovalProgressView::DeleteIntent(_)
        | RootComponentSubtreeRemovalProgressView::Deleted(_)
        | RootComponentSubtreeRemovalProgressView::Completed(_) => {
            return Err(InternalError::unavailable(
                "Component subtree leaf membership has not been removed",
            ));
        }
    };
    validate_subtree_directory_request(&removal, membership_removed, &request)?;

    let directory_authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory,
        component: component_directory_head(&partition),
    };
    let directory_authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&directory_authority)?;
    let (owning_component, parent) = converge_subtree_directory_recipients(
        &partition,
        membership_removed,
        &directory_authority,
        directory_authority_hash,
    )
    .await?;
    let synchronized = ComponentRegistryOps::mark_subtree_leaf_directory_synchronized(
        request.component,
        request.operation_id,
        request.expected_traversal_steps,
        request.expected_leaf_canister_id,
        request.expected_leaf_parent_canister_id,
        directory_authority,
        directory_authority_hash,
        owning_component,
        parent,
        maximum_registry_bytes,
    )?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &synchronized,
        None,
    )?;
    Ok(subtree_removal_response(synchronized))
}

/// Archive one synchronized leaf and resume from its retained parent.
pub async fn finalize_subtree_leaf(
    request: RootComponentSubtreeRemovalLeafFinalizationRequest,
) -> Result<RootComponentSubtreeRemovalResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component subtree leaf finalization requires an Active Fleet Subnet Root runtime",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "removal target Component Spec is absent from the protected topology",
            )
        })?
        .limits
        .maximum_registry_bytes;
    let removal = ComponentRegistryOps::finalize_subtree_leaf(
        request.component,
        request.operation_id,
        request.expected_traversal_steps,
        request.expected_leaf_canister_id,
        request.expected_leaf_parent_canister_id,
        maximum_registry_bytes,
    )?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &removal,
        None,
    )?;
    Ok(subtree_removal_response(removal))
}

/// Read one durable child-subtree removal operation without mutation.
pub fn subtree_removal_status(
    request: RootComponentSubtreeRemovalStatusRequest,
) -> Result<RootComponentSubtreeRemovalResponse, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let removal = ComponentRegistryOps::subtree_removal(request.component, request.operation_id)?
        .ok_or_else(|| {
        InternalError::unavailable(
            "Component subtree-removal operation has not been durably fenced",
        )
    })?;
    validate_subtree_removal(
        &authority.binding,
        authority.initial_release_set,
        &ConfigOps::component_topology()?,
        &removal,
        None,
    )?;
    Ok(subtree_removal_response(removal))
}

/// Advance one reserved direct child through a root-owned creation effect.
pub async fn create_child_allocation(
    request: RootComponentChildCreationRequest,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component Child creation requires an Active Fleet Subnet Root runtime",
        ));
    }

    let caller = IcOps::msg_caller();
    let parent =
        ComponentRegistryOps::registered_parent(request.component, caller)?.ok_or_else(|| {
            InternalError::public(Error::forbidden(format!(
                "caller {caller} is not a registered member of Component {}",
                request.component
            )))
        })?;
    let allocation =
        ComponentRegistryOps::child_allocation(request.component, request.operation_id)?
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
    validate_child_allocation(
        &authority.binding,
        authority.initial_release_set,
        &ConfigOps::component_topology()?,
        &parent.0,
        &allocation,
        None,
    )?;
    let plan = child_creation_plan(root, &store, &allocation)?;
    advance_child_creation(request.component, request.operation_id, allocation, plan).await
}

/// Advance one reserved top-level Component through a durable creation effect.
pub async fn create_allocation(
    request: RootComponentCreationRequest,
) -> Result<RootComponentAllocationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;

    let topology = ConfigOps::component_topology()?;
    let allocation = ComponentRegistryOps::allocation(request.operation_id).ok_or_else(|| {
        InternalError::unavailable("Component allocation operation has not been reserved")
    })?;
    validate_allocation_caller(&allocation)?;
    validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        request.operation_id,
    )?;
    let plan = creation_plan(root, &store, &allocation)?;

    advance_creation(request.operation_id, allocation, plan).await
}

/// Advance one created top-level Component through exact installation and verification.
pub async fn install_allocation(
    request: RootComponentInstallRequest,
) -> Result<RootComponentAllocationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;

    let topology = ConfigOps::component_topology()?;
    let allocation = ComponentRegistryOps::allocation(request.operation_id).ok_or_else(|| {
        InternalError::unavailable("Component allocation operation has not been reserved")
    })?;
    validate_allocation_caller(&allocation)?;
    validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        request.operation_id,
    )?;
    let plan = component_install_plan(&authority.binding, &store, &allocation).await?;

    advance_install(request.operation_id, allocation, plan).await
}

/// Install and independently verify one exactly created direct child through its root.
pub async fn install_child_allocation(
    request: RootComponentChildInstallRequest,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component Child installation requires an Active Fleet Subnet Root runtime",
        ));
    }

    let caller = IcOps::msg_caller();
    let parent =
        ComponentRegistryOps::registered_parent(request.component, caller)?.ok_or_else(|| {
            InternalError::public(Error::forbidden(format!(
                "caller {caller} is not a registered member of Component {}",
                request.component
            )))
        })?;
    let allocation =
        ComponentRegistryOps::child_allocation(request.component, request.operation_id)?
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
    validate_child_allocation(
        &authority.binding,
        authority.initial_release_set,
        &ConfigOps::component_topology()?,
        &parent.0,
        &allocation,
        None,
    )?;
    let plan =
        child_component_install_plan(&authority.binding, &store, &parent.0, &allocation).await?;
    advance_child_install(request.component, request.operation_id, allocation, plan).await
}

/// Atomically commit one verified direct child and derive the next Component Directory authority.
pub async fn commit_child_allocation(
    request: RootComponentChildCommitRequest,
) -> Result<RootComponentChildCommitResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    let fleet_directory =
        validate_current_mirror_authority(&authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component Child commitment requires an Active Fleet Subnet Root runtime",
        ));
    }

    let caller = IcOps::msg_caller();
    let parent =
        ComponentRegistryOps::registered_parent(request.component, caller)?.ok_or_else(|| {
            InternalError::public(Error::forbidden(format!(
                "caller {caller} is not a registered member of Component {}",
                request.component
            )))
        })?;
    let allocation =
        ComponentRegistryOps::child_allocation(request.component, request.operation_id)?
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
    let topology = ConfigOps::component_topology()?;
    validate_child_allocation(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &parent.0,
        &allocation,
        None,
    )?;
    let plan =
        child_component_install_plan(&authority.binding, &store, &parent.0, &allocation).await?;
    let installation = committed_or_verified_child_installation(&allocation)?;
    validate_child_install_effect(installation, &plan.durable)?;
    verify_installed_child(&plan).await?;

    let (committed, partition) = ComponentRegistryOps::commit_verified_child(
        request.component,
        request.operation_id,
        IcOps::now_nanos(),
        fleet_directory,
    )?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    child_commit_response(committed, partition)
}

/// Prepare one committed child and converge its owning Component plus distinct direct parent.
pub async fn prepare_child_directories(
    request: RootComponentChildDirectoryPreparationRequest,
) -> Result<RootComponentChildDirectoryPreparationResponse, InternalError> {
    let plan = prepared_child_runtime_plan(request.component, request.operation_id).await?;
    let observed = query_component_runtime_status(plan.child_canister).await?;
    let prepared_child = match validate_target_directory_status(
        &observed,
        &plan.child_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )? {
        ComponentRuntimePhase::AwaitingDirectory => {
            prepare_target_component_directories(
                plan.child_canister,
                plan.directory_request.clone(),
            )
            .await?
        }
        ComponentRuntimePhase::DirectoryPrepared | ComponentRuntimePhase::Active => observed,
    };
    let _ = prepared_target_directory_status(
        &prepared_child,
        &plan.child_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )?;

    let independently_observed = query_component_runtime_status(plan.child_canister).await?;
    let child = prepared_target_directory_status(
        &independently_observed,
        &plan.child_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )?;
    let owning_component = converge_active_member_directory(
        &plan.owning_component_binding,
        &plan.directory_request.authority,
        plan.directory_authority_hash,
    )
    .await?;
    let parent = match &plan.parent_binding {
        Some(parent_binding) => Some(
            converge_active_member_directory(
                parent_binding,
                &plan.directory_request.authority,
                plan.directory_authority_hash,
            )
            .await?,
        ),
        None => None,
    };
    validate_requesting_parent_still_active(request.component, &plan.requesting_parent_binding)?;
    let allocation = ComponentRegistryOps::mark_child_directory_prepared(
        request.component,
        request.operation_id,
        plan.directory_authority_hash,
    )?;
    if !committed_child_directory_receipt(&allocation)?.directory_prepared {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component Child Directory preparation did not commit its terminal root receipt",
        ));
    }

    Ok(RootComponentChildDirectoryPreparationResponse {
        committed: child_commit_response(allocation, plan.committed_partition)?,
        child,
        owning_component,
        parent,
    })
}

/// Activate and independently verify one exact Directory-prepared direct-child runtime.
pub async fn activate_child_runtime(
    request: RootComponentChildRuntimeActivationRequest,
) -> Result<RootComponentChildRuntimeActivationResponse, InternalError> {
    let plan = prepared_child_runtime_plan(request.component, request.operation_id).await?;
    if !committed_child_directory_receipt(&plan.allocation)?.directory_prepared {
        return Err(InternalError::unavailable(
            "Component Child runtime activation requires its terminal Directory preparation receipt",
        ));
    }

    let child = activate_directory_prepared_runtime(
        plan.child_canister,
        &plan.child_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )
    .await?;
    validate_requesting_parent_still_active(request.component, &plan.requesting_parent_binding)?;
    let allocation = ComponentRegistryOps::mark_child_runtime_activated(
        request.component,
        request.operation_id,
        plan.directory_authority_hash,
    )?;
    if !committed_child_directory_receipt(&allocation)?.runtime_activated {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component Child runtime activation did not commit its terminal root receipt",
        ));
    }

    Ok(RootComponentChildRuntimeActivationResponse {
        committed: child_commit_response(allocation, plan.committed_partition)?,
        child,
    })
}

/// Activate Registry membership and converge one runtime-active direct child.
pub async fn activate_child_membership(
    request: RootComponentChildMembershipActivationRequest,
) -> Result<RootComponentChildMembershipActivationResponse, InternalError> {
    let plan = prepared_child_runtime_plan(request.component, request.operation_id).await?;
    if !committed_child_directory_receipt(&plan.allocation)?.runtime_activated {
        return Err(InternalError::unavailable(
            "Component Child membership activation requires its terminal runtime receipt",
        ));
    }
    let observed = query_component_runtime_status(plan.child_canister).await?;
    validate_active_target_runtime_status(
        &observed,
        &plan.child_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )?;

    let (activated_allocation, active_partition) = ComponentRegistryOps::activate_child_membership(
        request.component,
        request.operation_id,
        IcOps::now_nanos(),
        plan.directory_request.authority.fleet.clone(),
    )?;
    validate_partition(
        &plan.root_binding,
        activated_allocation.release_set,
        &ConfigOps::component_topology()?,
        &active_partition,
    )?;
    let registered =
        ComponentRegistryOps::registered_parent(request.component, plan.child_canister)?
            .ok_or_else(|| {
                InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    "active Component Child membership has no registered principal",
                )
            })?;
    if registered != (plan.child_binding.clone(), ComponentLifecycleStatus::Active) {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "active Component Child Registry row differs from its protected binding",
        ));
    }

    let synchronization_request = ComponentRuntimeDirectorySynchronizationRequest {
        operation_id: request.operation_id,
        authority: ComponentRuntimeDirectoryAuthority {
            fleet: plan.directory_request.authority.fleet.clone(),
            component: component_directory_head(&active_partition),
        },
    };
    let active_authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&synchronization_request.authority)?;
    let membership = committed_child_directory_receipt(&activated_allocation)?
        .membership
        .as_ref()
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "active Component Child partition has no membership receipt",
            )
        })?;
    let membership_authority =
        ComponentPartitionSnapshotAuthority::from_child_membership(membership);
    let partition_authority =
        ComponentPartitionSnapshotAuthority::from_partition(&active_partition);
    if membership_authority.state != partition_authority.state
        || membership.directory_authority_hash != active_authority_hash
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "active child membership receipt differs from its derived current Directory",
        ));
    }

    let child = converge_active_membership_directory(
        plan.child_canister,
        &plan.child_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
        &synchronization_request,
        active_authority_hash,
    )
    .await?;
    validate_requesting_parent_still_active(request.component, &plan.requesting_parent_binding)?;
    let allocation = ComponentRegistryOps::mark_child_membership_synchronized(
        request.component,
        request.operation_id,
        active_authority_hash,
    )?;
    child_membership_response(
        allocation,
        plan.committed_partition,
        active_partition,
        child,
    )
}

/// Atomically commit one verified top-level Component and its first Directory authority.
pub async fn commit_allocation(
    request: RootComponentCommitRequest,
) -> Result<RootComponentCommitResponse, InternalError> {
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
    let allocation = ComponentRegistryOps::allocation(request.operation_id).ok_or_else(|| {
        InternalError::unavailable("Component allocation operation has not been reserved")
    })?;
    validate_allocation_caller(&allocation)?;
    validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        request.operation_id,
    )?;
    let plan = component_install_plan(&authority.binding, &store, &allocation).await?;
    let installation = committed_or_verified_installation(&allocation)?;
    validate_install_effect(installation, &plan.durable)?;
    verify_installed_component(&plan).await?;

    let (committed, partition) = ComponentRegistryOps::commit_verified(
        request.operation_id,
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
    commit_response(committed, partition)
}

/// Distribute and independently verify exact Directories for one committed Component.
pub async fn prepare_component_directories(
    request: RootComponentDirectoryPreparationRequest,
) -> Result<RootComponentDirectoryPreparationResponse, InternalError> {
    let plan = prepared_component_runtime_plan(request.operation_id).await?;
    let observed = query_component_runtime_status(plan.target_canister).await?;
    let prepared_target = match validate_target_directory_status(
        &observed,
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )? {
        ComponentRuntimePhase::AwaitingDirectory => {
            prepare_target_component_directories(
                plan.target_canister,
                plan.directory_request.clone(),
            )
            .await?
        }
        ComponentRuntimePhase::DirectoryPrepared | ComponentRuntimePhase::Active => observed,
    };
    let _ = prepared_target_directory_status(
        &prepared_target,
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )?;

    let independently_observed = query_component_runtime_status(plan.target_canister).await?;
    let response_target = prepared_target_directory_status(
        &independently_observed,
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )?;
    let allocation = ComponentRegistryOps::mark_directory_prepared(
        request.operation_id,
        plan.directory_authority_hash,
    )?;
    if !committed_directory_receipt(&allocation)?.directory_prepared {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component Directory preparation did not commit its terminal root receipt",
        ));
    }

    Ok(RootComponentDirectoryPreparationResponse {
        committed: commit_response(allocation, plan.partition)?,
        target: response_target,
    })
}

/// Activate and independently verify one exact Directory-prepared Component runtime.
pub async fn activate_component_runtime(
    request: RootComponentRuntimeActivationRequest,
) -> Result<RootComponentRuntimeActivationResponse, InternalError> {
    let plan = prepared_component_runtime_plan(request.operation_id).await?;
    if !committed_directory_receipt(&plan.allocation)?.directory_prepared {
        return Err(InternalError::unavailable(
            "Component runtime activation requires its terminal Directory preparation receipt",
        ));
    }

    let response_target = activate_directory_prepared_runtime(
        plan.target_canister,
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )
    .await?;
    let allocation = ComponentRegistryOps::mark_runtime_activated(
        request.operation_id,
        plan.directory_authority_hash,
    )?;
    if !committed_directory_receipt(&allocation)?.runtime_activated {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component runtime activation did not commit its terminal root receipt",
        ));
    }

    Ok(RootComponentRuntimeActivationResponse {
        committed: commit_response(allocation, plan.partition)?,
        target: response_target,
    })
}

/// Activate Registry membership and converge one runtime-active Component on its current Directory.
pub async fn activate_component_membership(
    request: RootComponentMembershipActivationRequest,
) -> Result<RootComponentMembershipActivationResponse, InternalError> {
    let plan = prepared_component_runtime_plan(request.operation_id).await?;
    if !committed_directory_receipt(&plan.allocation)?.runtime_activated {
        return Err(InternalError::unavailable(
            "Component membership activation requires its terminal runtime receipt",
        ));
    }
    let observed = query_component_runtime_status(plan.target_canister).await?;
    validate_active_target_runtime_status(
        &observed,
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )?;

    let (activated_allocation, active_partition) = ComponentRegistryOps::activate_membership(
        request.operation_id,
        IcOps::now_nanos(),
        plan.maximum_component_registry_bytes,
        plan.directory_request.authority.fleet.clone(),
    )?;
    validate_partition(
        &plan.root_binding,
        activated_allocation.release_set,
        &ConfigOps::component_topology()?,
        &active_partition,
    )?;
    if active_partition.status != ComponentLifecycleStatus::Active {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "membership activation did not produce an Active Component partition",
        ));
    }
    let synchronization_request = ComponentRuntimeDirectorySynchronizationRequest {
        operation_id: request.operation_id,
        authority: ComponentRuntimeDirectoryAuthority {
            fleet: plan.directory_request.authority.fleet.clone(),
            component: component_directory_head(&active_partition),
        },
    };
    let active_authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&synchronization_request.authority)?;
    let membership = committed_directory_receipt(&activated_allocation)?
        .membership
        .as_ref()
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Active Component partition has no membership receipt",
            )
        })?;
    if membership.directory_authority_hash != active_authority_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "active membership receipt differs from its derived current Directory",
        ));
    }

    synchronize_active_membership(
        &plan,
        active_partition,
        synchronization_request,
        active_authority_hash,
    )
    .await
}

async fn synchronize_active_membership(
    plan: &PreparedComponentRuntimePlan,
    active_partition: ComponentRegistryPartitionView,
    synchronization_request: ComponentRuntimeDirectorySynchronizationRequest,
    active_authority_hash: [u8; 32],
) -> Result<RootComponentMembershipActivationResponse, InternalError> {
    let target = converge_active_membership_directory(
        plan.target_canister,
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
        &synchronization_request,
        active_authority_hash,
    )
    .await?;
    let allocation = ComponentRegistryOps::mark_membership_synchronized(
        synchronization_request.operation_id,
        active_authority_hash,
    )?;
    membership_response(allocation, active_partition, target)
}

/// Seal the complete current Component inventory before root activation preparation.
pub async fn seal_root_activation_inventory(
    fleet_activation_operation_id: [u8; 32],
) -> Result<RootComponentInitialInventoryView, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;
    let plan = ComponentRegistryOps::seal_initial_inventory(
        fleet_activation_operation_id,
        IcOps::now_nanos(),
    )?;
    Ok(plan.receipt)
}

/// Re-observe every sealed initial Component on its exact active current Directory.
pub async fn converge_root_activation_inventory(
    fleet_activation_operation_id: [u8; 32],
) -> Result<RootComponentInitialInventoryView, InternalError> {
    let sealed =
        ComponentRegistryOps::validate_sealed_initial_inventory(fleet_activation_operation_id)?;
    for operation_id in &sealed.operation_ids {
        verify_initial_component_convergence(*operation_id).await?;
    }
    let unchanged =
        ComponentRegistryOps::validate_sealed_initial_inventory(fleet_activation_operation_id)?;
    if unchanged.receipt.inventory_hash != sealed.receipt.inventory_hash
        || unchanged.operation_ids != sealed.operation_ids
    {
        return Err(InternalError::conflict(
            "initial Component inventory changed during root activation verification",
        ));
    }
    let receipt = ComponentRegistryOps::mark_initial_inventory_directories_converged(
        fleet_activation_operation_id,
        sealed.receipt.inventory_hash,
    )?;
    if !receipt.directories_converged {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "initial Component Directory convergence did not commit its root receipt",
        ));
    }
    Ok(receipt)
}

/// Record the terminal root-runtime receipt after Fleet activation commits.
pub fn mark_root_runtime_activated(
    fleet_activation_operation_id: [u8; 32],
) -> Result<RootComponentInitialInventoryView, InternalError> {
    let receipt = ComponentRegistryOps::initial_inventory(fleet_activation_operation_id)?;
    if !receipt.directories_converged {
        return Err(InternalError::unavailable(
            "root runtime activation requires terminal initial Directory convergence",
        ));
    }
    let terminal = ComponentRegistryOps::mark_initial_inventory_root_runtime_activated(
        fleet_activation_operation_id,
        receipt.inventory_hash,
    )?;
    if !terminal.root_runtime_activated {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root runtime activation did not commit its terminal Component inventory receipt",
        ));
    }
    Ok(terminal)
}

#[must_use]
pub fn root_runtime_activation_receipt_complete() -> bool {
    ComponentRegistryOps::current()
        .and_then(|registry| registry.initial_inventory)
        .is_some_and(|receipt| receipt.directories_converged && receipt.root_runtime_activated)
}

/// Resolve one active top-level Component from current protected Registry authority.
pub fn active_component_binding(
    canister: candid::Principal,
) -> Result<ComponentBinding, InternalError> {
    let (authority, _) = root_authority()?;
    prepared_registry(&authority.binding, authority.initial_release_set)?;
    let component = ComponentRegistryOps::component_for_principal(canister).ok_or_else(|| {
        InternalError::public(Error::forbidden(format!(
            "caller {canister} has no Component Registry identity"
        )))
    })?;
    let partition = ComponentRegistryOps::partition(component)?.ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component principal index has no Registry partition",
        )
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &ConfigOps::component_topology()?,
        &partition,
    )?;
    if partition.status != ComponentLifecycleStatus::Active
        || partition.binding.canister_id != canister
    {
        return Err(InternalError::public(Error::forbidden(format!(
            "caller {canister} is not an active Component Registry member"
        ))));
    }
    Ok(partition.binding)
}

async fn verify_initial_component_convergence(operation_id: [u8; 32]) -> Result<(), InternalError> {
    let plan = prepared_component_runtime_plan(operation_id).await?;
    let membership = committed_directory_receipt(&plan.allocation)?
        .membership
        .as_ref()
        .ok_or_else(|| {
            InternalError::unavailable(
                "initial Component has no active Registry membership receipt",
            )
        })?;
    if !membership.directory_synchronized {
        return Err(InternalError::unavailable(
            "initial Component has no terminal current-Directory receipt",
        ));
    }
    let active_partition =
        ComponentRegistryOps::partition(plan.allocation.component)?.ok_or_else(|| {
            InternalError::unavailable("initial Component has no current Registry partition")
        })?;
    validate_partition(
        &plan.root_binding,
        plan.allocation.release_set,
        &ConfigOps::component_topology()?,
        &active_partition,
    )?;
    if active_partition.status != ComponentLifecycleStatus::Active {
        return Err(InternalError::unavailable(
            "initial Component Registry partition is not Active",
        ));
    }
    let active_request = ComponentRuntimeDirectorySynchronizationRequest {
        operation_id,
        authority: ComponentRuntimeDirectoryAuthority {
            fleet: plan.directory_request.authority.fleet.clone(),
            component: component_directory_head(&active_partition),
        },
    };
    let active_authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&active_request.authority)?;
    if membership.directory_authority_hash != active_authority_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "initial Component membership receipt differs from active Directory authority",
        ));
    }
    let observed = query_component_runtime_status(plan.target_canister).await?;
    if !validate_target_membership_status(
        &observed,
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
        &active_request,
        active_authority_hash,
    )? {
        return Err(InternalError::unavailable(
            "initial Component has not converged on its active current Directory",
        ));
    }
    Ok(())
}

/// Read one committed Component Registry partition without mutation.
pub fn registry_partition(
    request: ComponentRegistryPartitionRequest,
) -> Result<ComponentRegistryPartitionResponse, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    Ok(partition_response(partition))
}

/// Derive one compact Component Directory head from committed Registry authority.
pub fn directory_head(
    request: ComponentDirectoryHeadRequest,
) -> Result<ComponentDirectoryHead, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    Ok(component_directory_head(&partition))
}

/// Read one bounded, revision-stable Component Directory page for a registered member.
pub fn directory_page(
    request: ComponentDirectoryPageRequest,
) -> Result<ComponentDirectoryPageResponse, InternalError> {
    if request.limit == 0 || request.limit > MAX_COMPONENT_DIRECTORY_PAGE_ENTRIES {
        return Err(InternalError::invalid_input(format!(
            "Component Directory page limit must be between 1 and {MAX_COMPONENT_DIRECTORY_PAGE_ENTRIES}",
        )));
    }

    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let topology = ConfigOps::component_topology()?;
    let component = request.directory.provenance.component.component;
    let partition = ComponentRegistryOps::partition(component)?.ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let caller = IcOps::msg_caller();
    let (member, status) =
        ComponentRegistryOps::registered_parent(component, caller)?.ok_or_else(|| {
            InternalError::public(Error::forbidden(format!(
                "caller {caller} is not a registered member of Component {component}"
            )))
        })?;
    validate_directory_member(&authority.binding, &topology, &partition, &member)?;
    if !component_directory_member_can_read(status) {
        return Err(InternalError::unavailable(
            "Component Directory pages require a live registered member",
        ));
    }

    let directory = component_directory_head(&partition);
    if request.directory != directory {
        return Err(InternalError::conflict(
            "requested Component Directory head is not the exact current authority",
        ));
    }

    if let Some(parent_canister_id) = request.parent_canister_id
        && ComponentRegistryOps::registered_parent(component, parent_canister_id)?.is_none()
    {
        return Err(InternalError::invalid_input(
            "Component Directory parent filter is not a registered member of this Component",
        ));
    }
    if let Some(role) = request.role.as_ref() {
        let spec = topology
            .get(&partition.binding.component_spec)
            .ok_or_else(|| {
                InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    "Component Directory Spec is absent from protected topology",
                )
            })?;
        if spec.child(role).is_none() {
            return Err(InternalError::invalid_input(
                "Component Directory role filter is absent from the Component Spec",
            ));
        }
    }

    let start_after = decode_component_directory_cursor(&request)?;
    let selection = ComponentDirectoryPageSelection {
        parent_canister_id: request.parent_canister_id,
        role: request.role.clone(),
        status: request.status,
        start_after,
    };
    let page =
        ComponentRegistryOps::directory_page(component, &selection, usize::from(request.limit))?;
    let next_cursor = page
        .next_cursor
        .map(|cursor| encode_component_directory_cursor(&request, cursor))
        .transpose()?;

    Ok(ComponentDirectoryPageResponse {
        directory,
        entries: page
            .entries
            .into_iter()
            .map(|entry| ComponentDirectoryChildEntry {
                binding: entry.binding,
                kind: entry.kind,
                installed_artifact_hash: entry.installed_artifact_hash,
                status: entry.status,
            })
            .collect(),
        next_cursor,
    })
}

async fn advance_creation(
    operation_id: [u8; 32],
    allocation: RootComponentAllocationView,
    plan: RootComponentCreationPlan,
) -> Result<RootComponentAllocationResponse, InternalError> {
    if reconcile_existing_creation(&allocation, &plan)? {
        return allocation_response(allocation);
    }

    ComponentRegistryOps::validate_creation_capacity(operation_id, &plan)?;
    let cost_permit = deployment::reserve_component_creation_cost_guard(&plan.initial_cycles)?;
    let intent = match ComponentRegistryOps::begin_creation(
        operation_id,
        plan.clone(),
        cost_permit.replay_settlement(),
    ) {
        Ok(intent) => intent,
        Err(err) => {
            return Err(CostGuardWorkflow::recover_after_failure(
                &cost_permit,
                IcOps::now_secs(),
                err,
            ));
        }
    };
    let effect = match &intent.progress {
        RootComponentAllocationProgressView::CreationIntent(effect) => effect,
        RootComponentAllocationProgressView::Reserved
        | RootComponentAllocationProgressView::Created { .. }
        | RootComponentAllocationProgressView::InstallIntent { .. }
        | RootComponentAllocationProgressView::Installed { .. }
        | RootComponentAllocationProgressView::Verified { .. }
        | RootComponentAllocationProgressView::Committed { .. }
        | RootComponentAllocationProgressView::Removed { .. } => {
            return Err(CostGuardWorkflow::recover_after_failure(
                &cost_permit,
                IcOps::now_secs(),
                InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    "Component creation intent commit returned an invalid phase",
                ),
            ));
        }
    };
    if let Err(err) = validate_creation_effect(effect, &plan) {
        return Err(CostGuardWorkflow::recover_after_failure(
            &cost_permit,
            IcOps::now_secs(),
            err,
        ));
    }

    let canister = match MgmtOps::create_canister_with_permit(
        &cost_permit,
        vec![plan.controller],
        plan.initial_cycles.clone(),
    )
    .await
    {
        Ok(canister) => canister,
        Err(err) => {
            return Err(CostGuardWorkflow::recover_after_failure(
                &cost_permit,
                IcOps::now_secs(),
                err,
            ));
        }
    };

    let created = match ComponentRegistryOps::mark_created(operation_id, canister) {
        Ok(created) => created,
        Err(err) => {
            return Err(CostGuardWorkflow::complete_after_failure(
                &cost_permit,
                IcOps::now_secs(),
                err,
            ));
        }
    };
    CostGuardWorkflow::complete(&cost_permit, IcOps::now_secs())?;
    allocation_response(created)
}

async fn advance_child_creation(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    allocation: RootComponentChildAllocationView,
    plan: RootComponentCreationPlan,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    if reconcile_existing_child_creation(&allocation, &plan)? {
        return Ok(child_allocation_response(allocation));
    }

    ComponentRegistryOps::validate_child_creation_capacity(component, operation_id, &plan)?;
    let cost_permit =
        deployment::reserve_component_child_creation_cost_guard(&plan.initial_cycles)?;
    let intent = match ComponentRegistryOps::begin_child_creation(
        component,
        operation_id,
        plan.clone(),
        cost_permit.replay_settlement(),
    ) {
        Ok(intent) => intent,
        Err(error) => {
            return Err(CostGuardWorkflow::recover_after_failure(
                &cost_permit,
                IcOps::now_secs(),
                error,
            ));
        }
    };
    let effect = match &intent.progress {
        RootComponentChildAllocationProgressView::CreationIntent(effect) => effect,
        RootComponentChildAllocationProgressView::Reserved
        | RootComponentChildAllocationProgressView::Created { .. }
        | RootComponentChildAllocationProgressView::InstallIntent { .. }
        | RootComponentChildAllocationProgressView::Installed { .. }
        | RootComponentChildAllocationProgressView::Verified { .. }
        | RootComponentChildAllocationProgressView::Committed { .. } => {
            return Err(CostGuardWorkflow::recover_after_failure(
                &cost_permit,
                IcOps::now_secs(),
                InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    "Component Child creation intent commit returned an invalid phase",
                ),
            ));
        }
    };
    if let Err(error) = validate_creation_effect(effect, &plan) {
        return Err(CostGuardWorkflow::recover_after_failure(
            &cost_permit,
            IcOps::now_secs(),
            error,
        ));
    }

    let canister = match MgmtOps::create_canister_with_permit(
        &cost_permit,
        vec![plan.controller],
        plan.initial_cycles.clone(),
    )
    .await
    {
        Ok(canister) => canister,
        Err(error) => {
            return Err(CostGuardWorkflow::recover_after_failure(
                &cost_permit,
                IcOps::now_secs(),
                error,
            ));
        }
    };
    let created = match ComponentRegistryOps::mark_child_created(component, operation_id, canister)
    {
        Ok(created) => created,
        Err(error) => {
            return Err(CostGuardWorkflow::complete_after_failure(
                &cost_permit,
                IcOps::now_secs(),
                error,
            ));
        }
    };
    CostGuardWorkflow::complete(&cost_permit, IcOps::now_secs())?;
    Ok(child_allocation_response(created))
}

fn reconcile_existing_creation(
    allocation: &RootComponentAllocationView,
    plan: &RootComponentCreationPlan,
) -> Result<bool, InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Created { effect, .. }
        | RootComponentAllocationProgressView::InstallIntent {
            creation: effect, ..
        }
        | RootComponentAllocationProgressView::Installed {
            creation: effect, ..
        }
        | RootComponentAllocationProgressView::Verified {
            creation: effect, ..
        }
        | RootComponentAllocationProgressView::Committed {
            creation: effect, ..
        }
        | RootComponentAllocationProgressView::Removed {
            creation: effect, ..
        } => {
            validate_creation_effect(effect, plan)?;
            CostGuardWorkflow::complete_replay_settlement(
                &effect.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            Ok(true)
        }
        RootComponentAllocationProgressView::CreationIntent(effect) => {
            validate_creation_effect(effect, plan)?;
            CostGuardWorkflow::recover_replay_settlement(
                &effect.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            Ok(true)
        }
        RootComponentAllocationProgressView::Reserved => Ok(false),
    }
}

fn reconcile_existing_child_creation(
    allocation: &RootComponentChildAllocationView,
    plan: &RootComponentCreationPlan,
) -> Result<bool, InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::Created { effect, .. }
        | RootComponentChildAllocationProgressView::InstallIntent {
            creation: effect, ..
        }
        | RootComponentChildAllocationProgressView::Installed {
            creation: effect, ..
        }
        | RootComponentChildAllocationProgressView::Verified {
            creation: effect, ..
        }
        | RootComponentChildAllocationProgressView::Committed {
            creation: effect, ..
        } => {
            validate_creation_effect(effect, plan)?;
            CostGuardWorkflow::complete_replay_settlement(
                &effect.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            Ok(true)
        }
        RootComponentChildAllocationProgressView::CreationIntent(effect) => {
            validate_creation_effect(effect, plan)?;
            CostGuardWorkflow::recover_replay_settlement(
                &effect.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            Ok(true)
        }
        RootComponentChildAllocationProgressView::Reserved => Ok(false),
    }
}

#[derive(Clone, Debug)]
struct ComponentInstallPlan {
    durable: RootComponentInstallPlan,
    source: ApprovedModuleSource,
    payload: CanisterInitPayload,
    canister: candid::Principal,
    expected_status_module_hash: [u8; 32],
}

#[derive(Clone, Debug)]
struct ComponentChildInstallPlan {
    durable: RootComponentChildInstallPlan,
    source: ApprovedModuleSource,
    payload: CanisterInitPayload,
    canister: candid::Principal,
    expected_status_module_hash: [u8; 32],
}

async fn component_install_plan(
    root: &canic_core::ids::FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    allocation: &RootComponentAllocationView,
) -> Result<ComponentInstallPlan, InternalError> {
    let (creation, canister) = allocation_creation_and_canister(allocation)?;
    let expected_creation = creation_plan(root.fleet_subnet_root, store, allocation)?;
    validate_creation_effect(creation, &expected_creation)?;

    let artifact = exact_store_artifact(store, &allocation.role)?;
    let source = resolved_root_store_module_source(
        store.wasm_store,
        allocation.release_set.release_build_id,
        &allocation.role,
        artifact.payload_hash,
        artifact.payload_size_bytes,
    )
    .await?;
    let chunk_hashes = match source.payload() {
        ApprovedModulePayload::Chunked {
            source_canister,
            chunk_hashes,
        } if source_canister == &store.wasm_store => chunk_hashes.clone(),
        ApprovedModulePayload::Chunked { .. } | ApprovedModulePayload::Embedded { .. } => {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "resolved Component module source differs from the verified root Store",
            ));
        }
    };
    if source.module_hash() != artifact.payload_hash
        || source.payload_size_bytes() != artifact.payload_size_bytes
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "resolved Component module source differs from verified Store artifact evidence",
        ));
    }

    let binding = ComponentBinding {
        authority: root.authority.clone(),
        component: allocation.component,
        component_spec: allocation.component_spec.clone(),
        spec_hash: allocation.spec_hash,
        role: allocation.role.clone(),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        canister_id: canister,
    };
    let topology = ConfigOps::component_topology()?;
    topology
        .validate_component_binding(root, &binding)
        .map_err(|error| {
            InternalError::invalid_input(format!(
                "derived Component install binding is invalid: {error}"
            ))
        })?;
    let maximum_registry_bytes = topology
        .get(&allocation.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Config,
                "installed Component Spec is absent from the protected topology",
            )
        })?
        .limits
        .maximum_registry_bytes;
    let durable = RootComponentInstallPlan {
        raw_module_hash: artifact.raw_module_hash,
        chunk_hashes,
        binding: binding.clone(),
        maximum_registry_bytes,
    };
    let payload = CanisterInitPayload {
        install_id: allocation.operation_id,
        release_build_id: allocation.release_set.release_build_id,
        authority: CanisterInitAuthority::Component {
            root: root.clone(),
            binding,
        },
    };

    Ok(ComponentInstallPlan {
        durable,
        source,
        payload,
        canister,
        expected_status_module_hash: artifact.payload_hash,
    })
}

async fn child_component_install_plan(
    root: &canic_core::ids::FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    parent: &ManagedCanisterBinding,
    allocation: &RootComponentChildAllocationView,
) -> Result<ComponentChildInstallPlan, InternalError> {
    let (creation, canister) = child_allocation_creation_and_canister(allocation)?;
    let expected_creation = child_creation_plan(root.fleet_subnet_root, store, allocation)?;
    validate_creation_effect(creation, &expected_creation)?;

    let artifact = exact_store_artifact(store, &allocation.child_role)?;
    let source = resolved_root_store_module_source(
        store.wasm_store,
        allocation.release_set.release_build_id,
        &allocation.child_role,
        artifact.payload_hash,
        artifact.payload_size_bytes,
    )
    .await?;
    let chunk_hashes = match source.payload() {
        ApprovedModulePayload::Chunked {
            source_canister,
            chunk_hashes,
        } if source_canister == &store.wasm_store => chunk_hashes.clone(),
        ApprovedModulePayload::Chunked { .. } | ApprovedModulePayload::Embedded { .. } => {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "resolved Component Child module source differs from the verified root Store",
            ));
        }
    };
    if source.module_hash() != artifact.payload_hash
        || source.payload_size_bytes() != artifact.payload_size_bytes
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "resolved Component Child module source differs from verified Store artifact evidence",
        ));
    }

    let component = match parent {
        ManagedCanisterBinding::Component(binding) => binding.clone(),
        ManagedCanisterBinding::ComponentChild(binding) => binding.component.clone(),
    };
    let binding = canic_core::ids::ComponentChildBinding {
        component,
        parent_canister_id: allocation.parent_canister_id,
        role: allocation.child_role.clone(),
        canister_id: canister,
    };
    ConfigOps::component_topology()?
        .validate_component_child_binding(root, &binding)
        .map_err(|error| {
            InternalError::invalid_input(format!(
                "derived Component Child install binding is invalid: {error}"
            ))
        })?;
    let durable = RootComponentChildInstallPlan {
        raw_module_hash: artifact.raw_module_hash,
        chunk_hashes,
        binding: binding.clone(),
        maximum_registry_bytes: allocation.maximum_registry_bytes,
    };
    let payload = CanisterInitPayload {
        install_id: allocation.operation_id,
        release_build_id: allocation.release_set.release_build_id,
        authority: CanisterInitAuthority::ComponentChild {
            root: root.clone(),
            binding,
        },
    };

    Ok(ComponentChildInstallPlan {
        durable,
        source,
        payload,
        canister,
        expected_status_module_hash: artifact.payload_hash,
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "one workflow keeps every durable install and uncertain-outcome phase explicit"
)]
async fn advance_child_install(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    allocation: RootComponentChildAllocationView,
    plan: ComponentChildInstallPlan,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::Reserved
        | RootComponentChildAllocationProgressView::CreationIntent(_) => {
            Err(InternalError::conflict(
                "Component Child allocation must be created before installation",
            ))
        }
        RootComponentChildAllocationProgressView::Created { .. } => {
            if observed_child_install_state(&plan).await? {
                return Err(InternalError::conflict(
                    "created Component Child has unjournalled installed code",
                ));
            }
            ComponentRegistryOps::validate_child_install_capacity(
                component,
                operation_id,
                &plan.durable,
            )?;
            let permit = deployment::reserve_component_child_install_cost_guard()?;
            let intent = match ComponentRegistryOps::begin_child_install(
                component,
                operation_id,
                plan.durable.clone(),
                permit.replay_settlement(),
            ) {
                Ok(intent) => intent,
                Err(error) => {
                    return Err(CostGuardWorkflow::recover_after_failure(
                        &permit,
                        IcOps::now_secs(),
                        error,
                    ));
                }
            };
            let installation = child_install_effect(&intent)?;
            if let Err(error) = validate_child_install_effect(installation, &plan.durable) {
                return Err(CostGuardWorkflow::recover_after_failure(
                    &permit,
                    IcOps::now_secs(),
                    error,
                ));
            }
            perform_child_install(component, operation_id, &plan, &permit).await
        }
        RootComponentChildAllocationProgressView::InstallIntent { installation, .. } => {
            validate_child_install_effect(installation, &plan.durable)?;
            if observed_child_install_state(&plan).await? {
                CostGuardWorkflow::recover_replay_settlement(
                    &installation.cost_guard_settlement,
                    IcOps::now_secs(),
                )?;
                let installed =
                    ComponentRegistryOps::mark_child_installed(component, operation_id)?;
                return verify_and_mark_child_installed(component, operation_id, installed, &plan)
                    .await;
            }

            CostGuardWorkflow::recover_replay_settlement(
                &installation.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            let permit = deployment::reserve_component_child_install_cost_guard()?;
            let renewed = match ComponentRegistryOps::renew_child_install_intent(
                component,
                operation_id,
                &plan.durable,
                permit.replay_settlement(),
            ) {
                Ok(renewed) => renewed,
                Err(error) => {
                    return Err(CostGuardWorkflow::recover_after_failure(
                        &permit,
                        IcOps::now_secs(),
                        error,
                    ));
                }
            };
            let installation = child_install_effect(&renewed)?;
            if let Err(error) = validate_child_install_effect(installation, &plan.durable) {
                return Err(CostGuardWorkflow::recover_after_failure(
                    &permit,
                    IcOps::now_secs(),
                    error,
                ));
            }
            perform_child_install(component, operation_id, &plan, &permit).await
        }
        RootComponentChildAllocationProgressView::Installed { installation, .. } => {
            validate_child_install_effect(installation, &plan.durable)?;
            CostGuardWorkflow::recover_replay_settlement(
                &installation.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            verify_and_mark_child_installed(component, operation_id, allocation, &plan).await
        }
        RootComponentChildAllocationProgressView::Verified { installation, .. }
        | RootComponentChildAllocationProgressView::Committed { installation, .. } => {
            validate_child_install_effect(installation, &plan.durable)?;
            CostGuardWorkflow::recover_replay_settlement(
                &installation.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            verify_installed_child(&plan).await?;
            Ok(child_allocation_response(allocation))
        }
    }
}

async fn perform_child_install(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    plan: &ComponentChildInstallPlan,
    permit: &canic_core::control_plane_support::ops::cost_guard::CostGuardPermit,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    if let Err(error) = ModuleInstallWorkflow::install_with_payload_with_permit(
        permit,
        CanisterInstallMode::Install,
        plan.canister,
        &plan.source,
        plan.payload.clone(),
        None,
    )
    .await
    {
        return Err(CostGuardWorkflow::recover_after_failure(
            permit,
            IcOps::now_secs(),
            error,
        ));
    }

    let installed = match ComponentRegistryOps::mark_child_installed(component, operation_id) {
        Ok(installed) => installed,
        Err(error) => {
            return Err(CostGuardWorkflow::recover_after_failure(
                permit,
                IcOps::now_secs(),
                error,
            ));
        }
    };
    CostGuardWorkflow::recover(permit, IcOps::now_secs())?;
    verify_and_mark_child_installed(component, operation_id, installed, plan).await
}

async fn verify_and_mark_child_installed(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    _installed: RootComponentChildAllocationView,
    plan: &ComponentChildInstallPlan,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    verify_installed_child(plan).await?;
    let verified = ComponentRegistryOps::mark_child_verified(component, operation_id)?;
    if !matches!(
        verified.progress,
        RootComponentChildAllocationProgressView::Verified { .. }
    ) {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component Child verification commit returned an invalid phase",
        ));
    }
    Ok(child_allocation_response(verified))
}

async fn observed_child_install_state(
    plan: &ComponentChildInstallPlan,
) -> Result<bool, InternalError> {
    let status = MgmtOps::canister_status(plan.canister).await?;
    if status.settings.controllers != vec![plan.durable.binding.component.fleet_subnet_root] {
        return Err(InternalError::conflict(
            "Component Child Canister controllers differ from its sole root authority",
        ));
    }
    match status.module_hash {
        None => Ok(false),
        Some(module_hash) if module_hash == plan.expected_status_module_hash => Ok(true),
        Some(_) => Err(InternalError::conflict(
            "Component Child Canister module hash differs from its install intent",
        )),
    }
}

async fn verify_installed_child(plan: &ComponentChildInstallPlan) -> Result<(), InternalError> {
    if !observed_child_install_state(plan).await? {
        return Err(InternalError::unavailable(
            "Component Child Canister has no installed module after installation",
        ));
    }
    let observed = query_managed_binding(plan.canister).await?;
    let expected = ManagedCanisterBinding::ComponentChild(plan.durable.binding.clone());
    if observed != expected {
        return Err(InternalError::conflict(
            "installed Component Child retained binding differs from root install authority",
        ));
    }
    Ok(())
}

fn removed_allocation_response(
    allocation: RootComponentAllocationView,
    plan: &ComponentInstallPlan,
) -> Result<RootComponentAllocationResponse, InternalError> {
    let RootComponentAllocationProgressView::Removed { installation, .. } = &allocation.progress
    else {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "removed Component response requires removed allocation authority",
        ));
    };
    validate_install_effect(installation, &plan.durable)?;
    CostGuardWorkflow::recover_replay_settlement(
        &installation.cost_guard_settlement,
        IcOps::now_secs(),
    )?;
    allocation_response(allocation)
}

async fn advance_install(
    operation_id: [u8; 32],
    allocation: RootComponentAllocationView,
    plan: ComponentInstallPlan,
) -> Result<RootComponentAllocationResponse, InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Reserved
        | RootComponentAllocationProgressView::CreationIntent(_) => Err(InternalError::conflict(
            "Component allocation must be created before installation",
        )),
        RootComponentAllocationProgressView::Created { .. } => {
            if observed_install_state(&plan).await? {
                return Err(InternalError::conflict(
                    "created Component has unjournalled installed code",
                ));
            }
            ComponentRegistryOps::validate_install_capacity(operation_id, &plan.durable)?;
            let permit = deployment::reserve_component_install_cost_guard()?;
            let intent = match ComponentRegistryOps::begin_install(
                operation_id,
                plan.durable.clone(),
                permit.replay_settlement(),
            ) {
                Ok(intent) => intent,
                Err(error) => {
                    return Err(CostGuardWorkflow::recover_after_failure(
                        &permit,
                        IcOps::now_secs(),
                        error,
                    ));
                }
            };
            let installation = install_effect(&intent)?;
            if let Err(error) = validate_install_effect(installation, &plan.durable) {
                return Err(CostGuardWorkflow::recover_after_failure(
                    &permit,
                    IcOps::now_secs(),
                    error,
                ));
            }
            perform_install(operation_id, &plan, &permit).await
        }
        RootComponentAllocationProgressView::InstallIntent { installation, .. } => {
            validate_install_effect(installation, &plan.durable)?;
            if observed_install_state(&plan).await? {
                CostGuardWorkflow::recover_replay_settlement(
                    &installation.cost_guard_settlement,
                    IcOps::now_secs(),
                )?;
                let installed = ComponentRegistryOps::mark_installed(operation_id)?;
                return verify_and_mark_installed(operation_id, installed, &plan).await;
            }

            CostGuardWorkflow::recover_replay_settlement(
                &installation.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            let permit = deployment::reserve_component_install_cost_guard()?;
            let renewed = match ComponentRegistryOps::renew_install_intent(
                operation_id,
                &plan.durable,
                permit.replay_settlement(),
            ) {
                Ok(renewed) => renewed,
                Err(error) => {
                    return Err(CostGuardWorkflow::recover_after_failure(
                        &permit,
                        IcOps::now_secs(),
                        error,
                    ));
                }
            };
            let installation = install_effect(&renewed)?;
            if let Err(error) = validate_install_effect(installation, &plan.durable) {
                return Err(CostGuardWorkflow::recover_after_failure(
                    &permit,
                    IcOps::now_secs(),
                    error,
                ));
            }
            perform_install(operation_id, &plan, &permit).await
        }
        RootComponentAllocationProgressView::Installed { installation, .. } => {
            validate_install_effect(installation, &plan.durable)?;
            CostGuardWorkflow::recover_replay_settlement(
                &installation.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            verify_and_mark_installed(operation_id, allocation, &plan).await
        }
        RootComponentAllocationProgressView::Verified { installation, .. }
        | RootComponentAllocationProgressView::Committed { installation, .. } => {
            validate_install_effect(installation, &plan.durable)?;
            CostGuardWorkflow::recover_replay_settlement(
                &installation.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            verify_installed_component(&plan).await?;
            allocation_response(allocation)
        }
        RootComponentAllocationProgressView::Removed { .. } => {
            removed_allocation_response(allocation, &plan)
        }
    }
}

async fn perform_install(
    operation_id: [u8; 32],
    plan: &ComponentInstallPlan,
    permit: &canic_core::control_plane_support::ops::cost_guard::CostGuardPermit,
) -> Result<RootComponentAllocationResponse, InternalError> {
    if let Err(error) = ModuleInstallWorkflow::install_with_payload_with_permit(
        permit,
        CanisterInstallMode::Install,
        plan.canister,
        &plan.source,
        plan.payload.clone(),
        None,
    )
    .await
    {
        return Err(CostGuardWorkflow::recover_after_failure(
            permit,
            IcOps::now_secs(),
            error,
        ));
    }

    let installed = match ComponentRegistryOps::mark_installed(operation_id) {
        Ok(installed) => installed,
        Err(error) => {
            return Err(CostGuardWorkflow::recover_after_failure(
                permit,
                IcOps::now_secs(),
                error,
            ));
        }
    };
    CostGuardWorkflow::recover(permit, IcOps::now_secs())?;
    verify_and_mark_installed(operation_id, installed, plan).await
}

async fn verify_and_mark_installed(
    operation_id: [u8; 32],
    _installed: RootComponentAllocationView,
    plan: &ComponentInstallPlan,
) -> Result<RootComponentAllocationResponse, InternalError> {
    verify_installed_component(plan).await?;
    let verified = ComponentRegistryOps::mark_verified(operation_id)?;
    if !matches!(
        verified.progress,
        RootComponentAllocationProgressView::Verified { .. }
    ) {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component verification commit returned an invalid phase",
        ));
    }
    allocation_response(verified)
}

async fn observed_install_state(plan: &ComponentInstallPlan) -> Result<bool, InternalError> {
    let status = MgmtOps::canister_status(plan.canister).await?;
    if status.settings.controllers != vec![plan.durable.binding.fleet_subnet_root] {
        return Err(InternalError::conflict(
            "Component Canister controllers differ from its sole root authority",
        ));
    }
    match status.module_hash {
        None => Ok(false),
        Some(module_hash) if module_hash == plan.expected_status_module_hash => Ok(true),
        Some(_) => Err(InternalError::conflict(
            "Component Canister module hash differs from its install intent",
        )),
    }
}

async fn verify_installed_component(plan: &ComponentInstallPlan) -> Result<(), InternalError> {
    if !observed_install_state(plan).await? {
        return Err(InternalError::unavailable(
            "Component Canister has no installed module after installation",
        ));
    }
    let observed = query_managed_binding(plan.canister).await?;
    let expected = ManagedCanisterBinding::Component(plan.durable.binding.clone());
    if observed != expected {
        return Err(InternalError::conflict(
            "installed Component retained binding differs from root install authority",
        ));
    }
    Ok(())
}

async fn query_managed_binding(
    canister: candid::Principal,
) -> Result<ManagedCanisterBinding, InternalError> {
    let call = CallOps::bounded_wait(canister, protocol::CANIC_MANAGED_CANISTER_BINDING)
        .execute()
        .await
        .map_err(|error| InternalError::public(Error::unavailable(error.to_string())))?;
    let result: Result<ManagedCanisterBinding, Error> = call
        .candid()
        .map_err(|error| InternalError::public(Error::invariant(error.to_string())))?;
    result.map_err(InternalError::public)
}

async fn prepared_component_runtime_plan(
    operation_id: [u8; 32],
) -> Result<PreparedComponentRuntimePlan, InternalError> {
    let (root_authority, root) = root_authority()?;
    let prepared = prepared_registry(&root_authority.binding, root_authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    let fleet_directory =
        validate_current_mirror_authority(&root_authority, root, &preparation_request)?;
    let topology = ConfigOps::component_topology()?;
    let allocation = ComponentRegistryOps::allocation(operation_id).ok_or_else(|| {
        InternalError::unavailable("Component allocation operation has not been reserved")
    })?;
    validate_allocation_caller(&allocation)?;
    validate_allocation_record(
        &root_authority.binding,
        root_authority.initial_release_set,
        &topology,
        &allocation,
        operation_id,
    )?;
    let install = component_install_plan(&root_authority.binding, &store, &allocation).await?;
    let installation = committed_installation(&allocation)?;
    validate_install_effect(installation, &install.durable)?;
    verify_installed_component(&install).await?;
    let partition = ComponentRegistryOps::prepared_partition(operation_id)?;
    validate_partition(
        &root_authority.binding,
        root_authority.initial_release_set,
        &topology,
        &partition,
    )?;
    if partition.status != ComponentLifecycleStatus::Prepared {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "prepared Component receipt did not reconstruct a Prepared Registry partition",
        ));
    }
    let authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory,
        component: component_directory_head(&partition),
    };
    let directory_authority_hash = ComponentRuntimeOps::directory_authority_hash(&authority)?;
    if committed_directory_receipt(&allocation)?.directory_authority_hash
        != directory_authority_hash
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "committed root Directory receipt differs from current Registry authority",
        ));
    }
    Ok(PreparedComponentRuntimePlan {
        root_binding: root_authority.binding,
        allocation,
        partition,
        target_canister: install.canister,
        target_binding: ManagedCanisterBinding::Component(install.durable.binding),
        directory_request: ComponentRuntimeDirectoryPreparationRequest {
            operation_id,
            authority,
        },
        directory_authority_hash,
        maximum_component_registry_bytes: install.durable.maximum_registry_bytes,
    })
}

async fn prepared_child_runtime_plan(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
) -> Result<PreparedChildRuntimePlan, InternalError> {
    let (root_authority, root) = root_authority()?;
    let prepared = prepared_registry(&root_authority.binding, root_authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    let fleet_directory =
        validate_current_mirror_authority(&root_authority, root, &preparation_request)?;
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(
            "Component Child lifecycle requires an Active Fleet Subnet Root runtime",
        ));
    }

    let caller = IcOps::msg_caller();
    let (parent_binding, parent_status) =
        ComponentRegistryOps::registered_parent(component, caller)?.ok_or_else(|| {
            InternalError::public(Error::forbidden(format!(
                "caller {caller} is not a registered member of Component {component}"
            )))
        })?;
    if parent_status != ComponentLifecycleStatus::Active {
        return Err(InternalError::unavailable(
            "Component Child lifecycle requires its exact parent to remain Active",
        ));
    }
    let allocation =
        ComponentRegistryOps::child_allocation(component, operation_id)?.ok_or_else(|| {
            InternalError::unavailable("Component Child allocation operation has not been reserved")
        })?;
    let topology = ConfigOps::component_topology()?;
    validate_child_allocation(
        &root_authority.binding,
        root_authority.initial_release_set,
        &topology,
        &parent_binding,
        &allocation,
        None,
    )?;
    let install = child_component_install_plan(
        &root_authority.binding,
        &store,
        &parent_binding,
        &allocation,
    )
    .await?;
    let installation = committed_child_installation(&allocation)?;
    validate_child_install_effect(installation, &install.durable)?;
    verify_installed_child(&install).await?;
    let child_binding = ManagedCanisterBinding::ComponentChild(installation.binding.clone());

    let (allocation, committed_partition) =
        ComponentRegistryOps::committed_child_authority(component, operation_id, &fleet_directory)?;
    validate_partition(
        &root_authority.binding,
        root_authority.initial_release_set,
        &topology,
        &committed_partition,
    )?;
    let current_partition = current_child_partition(
        &root_authority.binding,
        root_authority.initial_release_set,
        &topology,
        component,
        &committed_partition,
    )?;
    let (directory_request, directory_authority_hash) = child_directory_request(
        operation_id,
        fleet_directory,
        &committed_partition,
        &allocation,
    )?;

    let owning_component_binding = ManagedCanisterBinding::Component(current_partition.binding);
    let requesting_parent_binding = parent_binding;
    let parent_binding = (managed_canister_principal(&requesting_parent_binding)
        != managed_canister_principal(&owning_component_binding))
    .then_some(requesting_parent_binding.clone());
    Ok(PreparedChildRuntimePlan {
        root_binding: root_authority.binding,
        allocation,
        committed_partition,
        child_canister: install.canister,
        child_binding,
        owning_component_binding,
        requesting_parent_binding,
        parent_binding,
        directory_request,
        directory_authority_hash,
    })
}

fn validate_requesting_parent_still_active(
    component: canic_core::ids::ComponentInstanceId,
    expected: &ManagedCanisterBinding,
) -> Result<(), InternalError> {
    let caller = IcOps::msg_caller();
    let (current, status) = ComponentRegistryOps::registered_parent(component, caller)?
        .ok_or_else(|| {
            InternalError::public(Error::forbidden(format!(
                "caller {caller} is no longer a registered member of Component {component}"
            )))
        })?;
    if current != *expected || status != ComponentLifecycleStatus::Active {
        return Err(InternalError::conflict(
            "Component Child parent changed before terminal lifecycle commitment",
        ));
    }
    Ok(())
}

fn current_child_partition(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    component: canic_core::ids::ComponentInstanceId,
    committed: &ComponentRegistryPartitionView,
) -> Result<ComponentRegistryPartitionView, InternalError> {
    let current = ComponentRegistryOps::partition(component)?.ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "committed Component Child has no current owning Registry partition",
        )
    })?;
    validate_partition(root, release_set, topology, &current)?;
    if current.status != ComponentLifecycleStatus::Active
        || current.binding != committed.binding
        || current.revision < committed.revision
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "committed Component Child differs from its current owning Component authority",
        ));
    }
    Ok(current)
}

fn child_directory_request(
    operation_id: [u8; 32],
    fleet: FleetDirectorySnapshot,
    partition: &ComponentRegistryPartitionView,
    allocation: &RootComponentChildAllocationView,
) -> Result<(ComponentRuntimeDirectoryPreparationRequest, [u8; 32]), InternalError> {
    let request = ComponentRuntimeDirectoryPreparationRequest {
        operation_id,
        authority: ComponentRuntimeDirectoryAuthority {
            fleet,
            component: component_directory_head(partition),
        },
    };
    let authority_hash = ComponentRuntimeOps::directory_authority_hash(&request.authority)?;
    if committed_child_directory_receipt(allocation)?.directory_authority_hash != authority_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "committed Component Child Directory receipt differs from its Registry authority",
        ));
    }
    Ok((request, authority_hash))
}

async fn query_component_runtime_status(
    canister: candid::Principal,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let call = CallOps::bounded_wait(canister, protocol::CANIC_COMPONENT_RUNTIME_STATUS)
        .execute()
        .await
        .map_err(|error| InternalError::public(Error::unavailable(error.to_string())))?;
    let result: Result<ComponentRuntimeStatusResponse, Error> = call
        .candid()
        .map_err(|error| InternalError::public(Error::invariant(error.to_string())))?;
    result.map_err(InternalError::public)
}

async fn activate_target_component_runtime(
    canister: candid::Principal,
    request: ComponentRuntimeActivationRequest,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let call = CallOps::bounded_wait(canister, protocol::CANIC_COMPONENT_RUNTIME_ACTIVATE)
        .with_arg(request)?
        .execute()
        .await
        .map_err(|error| InternalError::public(Error::unavailable(error.to_string())))?;
    let result: Result<ComponentRuntimeStatusResponse, Error> = call
        .candid()
        .map_err(|error| InternalError::public(Error::invariant(error.to_string())))?;
    result.map_err(InternalError::public)
}

async fn activate_directory_prepared_runtime(
    canister: candid::Principal,
    binding: &ManagedCanisterBinding,
    directory_request: &ComponentRuntimeDirectoryPreparationRequest,
    directory_authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let observed = query_component_runtime_status(canister).await?;
    let activated = match validate_target_directory_status(
        &observed,
        binding,
        directory_request,
        directory_authority_hash,
    )? {
        ComponentRuntimePhase::AwaitingDirectory => {
            return Err(InternalError::unavailable(
                "Component runtime has not retained its Directory authority",
            ));
        }
        ComponentRuntimePhase::DirectoryPrepared => {
            let request = ComponentRuntimeActivationRequest {
                operation_id: directory_request.operation_id,
                directory_authority_hash,
            };
            match activate_target_component_runtime(canister, request).await {
                Ok(status) => status,
                Err(call_error) => {
                    let reconciled = query_component_runtime_status(canister).await?;
                    if validate_active_target_runtime_status(
                        &reconciled,
                        binding,
                        directory_request,
                        directory_authority_hash,
                    )
                    .is_ok()
                    {
                        reconciled
                    } else {
                        return Err(call_error);
                    }
                }
            }
        }
        ComponentRuntimePhase::Active => observed,
    };
    validate_active_target_runtime_status(
        &activated,
        binding,
        directory_request,
        directory_authority_hash,
    )?;

    let independently_observed = query_component_runtime_status(canister).await?;
    active_target_runtime_status(
        &independently_observed,
        binding,
        directory_request,
        directory_authority_hash,
    )
}

async fn converge_active_membership_directory(
    canister: candid::Principal,
    binding: &ManagedCanisterBinding,
    prepared_request: &ComponentRuntimeDirectoryPreparationRequest,
    prepared_authority_hash: [u8; 32],
    active_request: &ComponentRuntimeDirectorySynchronizationRequest,
    active_authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let observed = query_component_runtime_status(canister).await?;
    let synchronized = if validate_target_membership_status(
        &observed,
        binding,
        prepared_request,
        prepared_authority_hash,
        active_request,
        active_authority_hash,
    )? {
        observed
    } else {
        match synchronize_target_component_directory(canister, active_request.clone()).await {
            Ok(status) => status,
            Err(call_error) => {
                let reconciled = query_component_runtime_status(canister).await?;
                if matches!(
                    validate_target_membership_status(
                        &reconciled,
                        binding,
                        prepared_request,
                        prepared_authority_hash,
                        active_request,
                        active_authority_hash,
                    ),
                    Ok(true)
                ) {
                    reconciled
                } else {
                    return Err(call_error);
                }
            }
        }
    };
    if !validate_target_membership_status(
        &synchronized,
        binding,
        prepared_request,
        prepared_authority_hash,
        active_request,
        active_authority_hash,
    )? {
        return Err(InternalError::unavailable(
            "Component runtime has not retained its active membership Directory",
        ));
    }

    let independently_observed = query_component_runtime_status(canister).await?;
    active_membership_target_status(
        &independently_observed,
        binding,
        prepared_request,
        prepared_authority_hash,
        active_request,
        active_authority_hash,
    )
}

async fn synchronize_target_component_directory(
    canister: candid::Principal,
    request: ComponentRuntimeDirectorySynchronizationRequest,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let call = CallOps::bounded_wait(
        canister,
        protocol::CANIC_COMPONENT_RUNTIME_DIRECTORY_SYNCHRONIZE,
    )
    .with_arg(request)?
    .execute()
    .await
    .map_err(|error| InternalError::public(Error::unavailable(error.to_string())))?;
    let result: Result<ComponentRuntimeStatusResponse, Error> = call
        .candid()
        .map_err(|error| InternalError::public(Error::invariant(error.to_string())))?;
    result.map_err(InternalError::public)
}

async fn prepare_target_component_directories(
    canister: candid::Principal,
    request: ComponentRuntimeDirectoryPreparationRequest,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let call = CallOps::bounded_wait(
        canister,
        protocol::CANIC_COMPONENT_RUNTIME_DIRECTORY_PREPARE,
    )
    .with_arg(request)?
    .execute()
    .await
    .map_err(|error| InternalError::public(Error::unavailable(error.to_string())))?;
    let result: Result<ComponentRuntimeStatusResponse, Error> = call
        .candid()
        .map_err(|error| InternalError::public(Error::invariant(error.to_string())))?;
    result.map_err(InternalError::public)
}

async fn converge_subtree_directory_recipients(
    partition: &ComponentRegistryPartitionView,
    membership: &RootComponentSubtreeMembershipRemovedView,
    authority: &ComponentRuntimeDirectoryAuthority,
    authority_hash: [u8; 32],
) -> Result<
    (
        Option<ComponentRuntimeDirectoryConvergenceEvidence>,
        Option<ComponentRuntimeDirectoryConvergenceEvidence>,
    ),
    InternalError,
> {
    let owning_binding = ManagedCanisterBinding::Component(partition.binding.clone());
    let owning_component = match partition.status {
        ComponentLifecycleStatus::Active => Some(
            converge_active_member_directory(&owning_binding, authority, authority_hash).await?,
        ),
        ComponentLifecycleStatus::Draining => {
            let draining = ComponentRegistryOps::component_draining(partition.binding.component)?
                .ok_or_else(|| {
                InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    "Draining Component has no durable draining authority",
                )
            })?;
            if !matches!(
                draining.quiescence,
                Some(RootComponentQuiescenceProgressView::Quiescent(_))
            ) {
                return Err(InternalError::unavailable(
                    "Draining Component is not terminally quiescent",
                ));
            }
            None
        }
        ComponentLifecycleStatus::Prepared | ComponentLifecycleStatus::Removed => {
            return Err(InternalError::conflict(
                "Component subtree Directory convergence requires Active or Draining authority",
            ));
        }
    };
    let leaf = &membership.deleted.deletion.stopped.stop.leaf;
    if leaf.parent_canister_id == partition.binding.canister_id {
        return Ok((owning_component, None));
    }
    let (parent_binding, status) = ComponentRegistryOps::registered_parent(
        partition.binding.component,
        leaf.parent_canister_id,
    )?
    .ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "removed Component subtree leaf has no retained registered parent",
        )
    })?;
    if status != ComponentLifecycleStatus::Active {
        return Err(InternalError::conflict(
            "removed Component subtree leaf parent is not Active",
        ));
    }
    let parent =
        converge_active_member_directory(&parent_binding, authority, authority_hash).await?;
    Ok((owning_component, Some(parent)))
}

async fn converge_active_member_directory(
    binding: &ManagedCanisterBinding,
    authority: &ComponentRuntimeDirectoryAuthority,
    authority_hash: [u8; 32],
) -> Result<ComponentRuntimeDirectoryConvergenceEvidence, InternalError> {
    let canister = managed_canister_principal(binding);
    let observed = query_component_runtime_status(canister).await?;
    let converged =
        if active_member_directory_is_converged(&observed, binding, authority, authority_hash)? {
            observed
        } else {
            let request = ComponentRuntimeDirectorySynchronizationRequest {
                operation_id: observed.operation_id,
                authority: authority.clone(),
            };
            match synchronize_target_component_directory(canister, request).await {
                Ok(status) => status,
                Err(call_error) => {
                    let reconciled = query_component_runtime_status(canister).await?;
                    if active_member_directory_is_converged(
                        &reconciled,
                        binding,
                        authority,
                        authority_hash,
                    )
                    .is_ok_and(|converged| converged)
                    {
                        reconciled
                    } else {
                        return Err(call_error);
                    }
                }
            }
        };
    if !active_member_directory_is_converged(&converged, binding, authority, authority_hash)? {
        return Err(InternalError::unavailable(
            "active Component-tree member has not retained the committed Directory authority",
        ));
    }

    let independently_observed = query_component_runtime_status(canister).await?;
    exact_active_member_directory_receipt(
        &independently_observed,
        binding,
        authority,
        authority_hash,
    )
}

fn active_member_directory_is_converged(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    expected: &ComponentRuntimeDirectoryAuthority,
    expected_hash: [u8; 32],
) -> Result<bool, InternalError> {
    let current = status.authority.as_ref().ok_or_else(|| {
        InternalError::conflict("Active Component-tree member has no current Directory authority")
    })?;
    let current_hash = status.authority_hash.ok_or_else(|| {
        InternalError::conflict("Active Component-tree member has no current Directory hash")
    })?;
    let activation = status.activation.ok_or_else(|| {
        InternalError::conflict("Active Component-tree member has no immutable activation receipt")
    })?;
    if status.binding != *binding
        || status.phase != ComponentRuntimePhase::Active
        || activation.directory_authority_hash == [0; 32]
        || activation.activated_at_ns == 0
        || ComponentRuntimeOps::directory_authority_hash(current)? != current_hash
    {
        return Err(InternalError::conflict(
            "Component-tree member status differs from its active protected authority",
        ));
    }

    let current_component = &current.component.provenance;
    let expected_component = &expected.component.provenance;
    if current_component.component != expected_component.component
        || current_component.source_fleet_subnet_root != expected_component.source_fleet_subnet_root
        || current_component.component != *owning_component(binding)
    {
        return Err(InternalError::conflict(
            "Component-tree member belongs to a different Component Directory",
        ));
    }
    match current_component
        .component_registry_revision
        .cmp(&expected_component.component_registry_revision)
    {
        std::cmp::Ordering::Equal => {
            if current == expected && current_hash == expected_hash {
                Ok(true)
            } else {
                Err(InternalError::conflict(
                    "Component-tree member retained conflicting authority at the committed revision",
                ))
            }
        }
        std::cmp::Ordering::Less => {
            if expected_component.component_registry_content_hash
                == current_component.component_registry_content_hash
                || expected_component.synchronized_at_ns <= current_component.synchronized_at_ns
                || !fleet_directory_non_regressing(&current.fleet, &expected.fleet)
            {
                return Err(InternalError::conflict(
                    "committed Component Directory cannot advance the active member safely",
                ));
            }
            Ok(false)
        }
        std::cmp::Ordering::Greater => {
            if current_component.component_registry_content_hash
                == expected_component.component_registry_content_hash
                || current_component.synchronized_at_ns <= expected_component.synchronized_at_ns
                || !fleet_directory_non_regressing(&expected.fleet, &current.fleet)
            {
                return Err(InternalError::conflict(
                    "active Component-tree member progressed through conflicting Directory authority",
                ));
            }
            Ok(true)
        }
    }
}

fn exact_active_member_directory_receipt(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    authority: &ComponentRuntimeDirectoryAuthority,
    authority_hash: [u8; 32],
) -> Result<ComponentRuntimeDirectoryConvergenceEvidence, InternalError> {
    if !active_member_directory_is_converged(status, binding, authority, authority_hash)? {
        return Err(InternalError::unavailable(
            "active Component-tree member has not converged on the committed Directory authority",
        ));
    }
    let activation = status.activation.ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "converged Component-tree member lost its immutable activation receipt",
        )
    })?;
    Ok(ComponentRuntimeDirectoryConvergenceEvidence {
        operation_id: status.operation_id,
        binding: binding.clone(),
        covered_authority: authority.clone(),
        covered_authority_hash: authority_hash,
        activation,
    })
}

fn fleet_directory_non_regressing(
    current: &FleetDirectorySnapshot,
    next: &FleetDirectorySnapshot,
) -> bool {
    let current_revision = current.provenance.registry.revision;
    let next_revision = next.provenance.registry.revision;
    next_revision > current_revision || (next_revision == current_revision && next == current)
}

const fn managed_canister_principal(binding: &ManagedCanisterBinding) -> candid::Principal {
    match binding {
        ManagedCanisterBinding::Component(component) => component.canister_id,
        ManagedCanisterBinding::ComponentChild(child) => child.canister_id,
    }
}

const fn owning_component(binding: &ManagedCanisterBinding) -> &ComponentBinding {
    match binding {
        ManagedCanisterBinding::Component(component) => component,
        ManagedCanisterBinding::ComponentChild(child) => &child.component,
    }
}

fn validate_target_directory_status(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    request: &ComponentRuntimeDirectoryPreparationRequest,
    authority_hash: [u8; 32],
) -> Result<ComponentRuntimePhase, InternalError> {
    if status.operation_id != request.operation_id || status.binding != *binding {
        return Err(InternalError::conflict(
            "Component runtime Directory status differs from root installation authority",
        ));
    }
    match status.phase {
        ComponentRuntimePhase::AwaitingDirectory
            if status.authority.is_none()
                && status.authority_hash.is_none()
                && status.activation.is_none() =>
        {
            Ok(ComponentRuntimePhase::AwaitingDirectory)
        }
        ComponentRuntimePhase::DirectoryPrepared
            if status.authority.as_ref() == Some(&request.authority)
                && status.authority_hash == Some(authority_hash)
                && status.activation.is_none() =>
        {
            Ok(ComponentRuntimePhase::DirectoryPrepared)
        }
        ComponentRuntimePhase::Active
            if status.authority.is_some()
                && status.authority_hash.is_some()
                && status.activation.as_ref().is_some_and(|activation| {
                    activation.directory_authority_hash == authority_hash
                        && activation.activated_at_ns != 0
                }) =>
        {
            Ok(ComponentRuntimePhase::Active)
        }
        ComponentRuntimePhase::AwaitingDirectory
        | ComponentRuntimePhase::DirectoryPrepared
        | ComponentRuntimePhase::Active => Err(InternalError::conflict(
            "Component runtime retained conflicting or incomplete Directory authority",
        )),
    }
}

fn prepared_target_directory_status(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    request: &ComponentRuntimeDirectoryPreparationRequest,
    authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    match validate_target_directory_status(status, binding, request, authority_hash)? {
        ComponentRuntimePhase::DirectoryPrepared => Ok(status.clone()),
        ComponentRuntimePhase::Active => Ok(ComponentRuntimeStatusResponse {
            operation_id: request.operation_id,
            binding: binding.clone(),
            phase: ComponentRuntimePhase::DirectoryPrepared,
            authority: Some(request.authority.clone()),
            authority_hash: Some(authority_hash),
            activation: None,
        }),
        ComponentRuntimePhase::AwaitingDirectory => Err(InternalError::unavailable(
            "Component runtime has not retained the complete Directory authority",
        )),
    }
}

fn validate_active_target_runtime_status(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    request: &ComponentRuntimeDirectoryPreparationRequest,
    authority_hash: [u8; 32],
) -> Result<(), InternalError> {
    if validate_target_directory_status(status, binding, request, authority_hash)?
        != ComponentRuntimePhase::Active
    {
        return Err(InternalError::unavailable(
            "Component runtime has not completed exact Directory-bound activation",
        ));
    }
    Ok(())
}

fn active_target_runtime_status(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    request: &ComponentRuntimeDirectoryPreparationRequest,
    authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    validate_active_target_runtime_status(status, binding, request, authority_hash)?;
    let activation = status.activation.ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Active Component runtime has no immutable activation receipt",
        )
    })?;
    Ok(ComponentRuntimeStatusResponse {
        operation_id: request.operation_id,
        binding: binding.clone(),
        phase: ComponentRuntimePhase::Active,
        authority: Some(request.authority.clone()),
        authority_hash: Some(authority_hash),
        activation: Some(activation),
    })
}

fn validate_target_membership_status(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    prepared_request: &ComponentRuntimeDirectoryPreparationRequest,
    prepared_authority_hash: [u8; 32],
    active_request: &ComponentRuntimeDirectorySynchronizationRequest,
    active_authority_hash: [u8; 32],
) -> Result<bool, InternalError> {
    validate_active_target_runtime_status(
        status,
        binding,
        prepared_request,
        prepared_authority_hash,
    )?;
    active_member_directory_is_converged(
        status,
        binding,
        &active_request.authority,
        active_authority_hash,
    )
}

fn active_membership_target_status(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    prepared_request: &ComponentRuntimeDirectoryPreparationRequest,
    prepared_authority_hash: [u8; 32],
    active_request: &ComponentRuntimeDirectorySynchronizationRequest,
    active_authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    if !validate_target_membership_status(
        status,
        binding,
        prepared_request,
        prepared_authority_hash,
        active_request,
        active_authority_hash,
    )? {
        return Err(InternalError::unavailable(
            "Component runtime did not converge on its active membership Directory",
        ));
    }
    let activation = status.activation.ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "active membership target lost its immutable activation receipt",
        )
    })?;
    Ok(ComponentRuntimeStatusResponse {
        operation_id: prepared_request.operation_id,
        binding: binding.clone(),
        phase: ComponentRuntimePhase::Active,
        authority: Some(active_request.authority.clone()),
        authority_hash: Some(active_authority_hash),
        activation: Some(activation),
    })
}

fn allocation_creation_and_canister(
    allocation: &RootComponentAllocationView,
) -> Result<(&RootComponentCreationEffectView, candid::Principal), InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Created { effect, canister } => {
            Ok((effect, *canister))
        }
        RootComponentAllocationProgressView::InstallIntent {
            creation, canister, ..
        }
        | RootComponentAllocationProgressView::Installed {
            creation, canister, ..
        }
        | RootComponentAllocationProgressView::Verified {
            creation, canister, ..
        }
        | RootComponentAllocationProgressView::Committed {
            creation, canister, ..
        }
        | RootComponentAllocationProgressView::Removed {
            creation, canister, ..
        } => Ok((creation, *canister)),
        RootComponentAllocationProgressView::Reserved
        | RootComponentAllocationProgressView::CreationIntent(_) => Err(InternalError::conflict(
            "Component allocation must be created before installation",
        )),
    }
}

fn child_allocation_creation_and_canister(
    allocation: &RootComponentChildAllocationView,
) -> Result<(&RootComponentCreationEffectView, candid::Principal), InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::Created { effect, canister } => {
            Ok((effect, *canister))
        }
        RootComponentChildAllocationProgressView::InstallIntent {
            creation, canister, ..
        }
        | RootComponentChildAllocationProgressView::Installed {
            creation, canister, ..
        }
        | RootComponentChildAllocationProgressView::Verified {
            creation, canister, ..
        }
        | RootComponentChildAllocationProgressView::Committed {
            creation, canister, ..
        } => Ok((creation, *canister)),
        RootComponentChildAllocationProgressView::Reserved
        | RootComponentChildAllocationProgressView::CreationIntent(_) => {
            Err(InternalError::conflict(
                "Component Child allocation must be created before installation",
            ))
        }
    }
}

fn install_effect(
    allocation: &RootComponentAllocationView,
) -> Result<&RootComponentInstallEffectView, InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::InstallIntent { installation, .. } => Ok(installation),
        _ => Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component install intent commit returned an invalid phase",
        )),
    }
}

fn child_install_effect(
    allocation: &RootComponentChildAllocationView,
) -> Result<&RootComponentChildInstallEffectView, InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::InstallIntent { installation, .. } => {
            Ok(installation)
        }
        _ => Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component Child install intent commit returned an invalid phase",
        )),
    }
}

fn committed_or_verified_installation(
    allocation: &RootComponentAllocationView,
) -> Result<&RootComponentInstallEffectView, InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Verified { installation, .. }
        | RootComponentAllocationProgressView::Committed { installation, .. } => Ok(installation),
        _ => Err(InternalError::conflict(
            "Component allocation must be verified before Registry commitment",
        )),
    }
}

fn committed_or_verified_child_installation(
    allocation: &RootComponentChildAllocationView,
) -> Result<&RootComponentChildInstallEffectView, InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::Verified { installation, .. }
        | RootComponentChildAllocationProgressView::Committed { installation, .. } => {
            Ok(installation)
        }
        _ => Err(InternalError::conflict(
            "Component Child allocation must be verified before Registry commitment",
        )),
    }
}

fn committed_installation(
    allocation: &RootComponentAllocationView,
) -> Result<&RootComponentInstallEffectView, InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Committed { installation, .. } => Ok(installation),
        _ => Err(InternalError::conflict(
            "Component allocation must be committed before Directory preparation",
        )),
    }
}

fn committed_child_installation(
    allocation: &RootComponentChildAllocationView,
) -> Result<&RootComponentChildInstallEffectView, InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::Committed { installation, .. } => {
            Ok(installation)
        }
        _ => Err(InternalError::conflict(
            "Component Child allocation must be committed before Directory preparation",
        )),
    }
}

fn committed_directory_receipt(
    allocation: &RootComponentAllocationView,
) -> Result<&crate::view::component_registry::RootComponentCommitmentView, InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Committed { commitment, .. } => Ok(commitment),
        _ => Err(InternalError::conflict(
            "Component allocation has no committed Directory authority",
        )),
    }
}

fn committed_child_directory_receipt(
    allocation: &RootComponentChildAllocationView,
) -> Result<&crate::view::component_registry::RootComponentChildCommitmentView, InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::Committed { commitment, .. } => Ok(commitment),
        _ => Err(InternalError::conflict(
            "Component Child allocation has no committed Directory authority",
        )),
    }
}

fn prepared_registry(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
) -> Result<RootComponentRegistryView, InternalError> {
    let prepared = ComponentRegistryOps::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    if &prepared.root != root || prepared.release_set != release_set {
        return Err(InternalError::conflict(
            "durable Component Registry authority differs from the protected root",
        ));
    }
    Ok(prepared)
}

fn validate_preparation_authority(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    root: candid::Principal,
    request: &RootComponentRegistryPreparationRequest,
) -> Result<FleetDirectorySnapshot, InternalError> {
    let mirror = FleetRegistryMirrorOps::validated_current(authority, root)?;
    let target_is_exact = mirror.active.snapshot.version == request.expected_fleet_registry;
    let root_is_active = mirror.root_entry.status == FleetSubnetRootStatus::Active;
    if !target_is_exact || !root_is_active {
        return Err(InternalError::conflict(
            "Component Registry preparation requires this root's exact Active mirror authority",
        ));
    }
    Ok(mirror.active.directory)
}

fn validate_current_mirror_authority(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    root: candid::Principal,
    request: &RootComponentRegistryPreparationRequest,
) -> Result<FleetDirectorySnapshot, InternalError> {
    let mirror = FleetRegistryMirrorOps::validated_current(authority, root)?;
    let prepared = &request.expected_fleet_registry;
    let current = &mirror.active.snapshot.version;
    let preparation_is_covered =
        ComponentRegistryOps::registry_covers_preparation(prepared, current);
    if !preparation_is_covered {
        return Err(InternalError::conflict(
            "current root Registry Mirror does not cover Component Registry preparation authority",
        ));
    }
    if mirror.root_entry.status == FleetSubnetRootStatus::Draining {
        ComponentRegistryOps::validate_published_root_draining(current)?;
    }
    Ok(mirror.active.directory)
}

fn root_authority() -> Result<
    (
        canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
        candid::Principal,
    ),
    InternalError,
> {
    let authority = FleetActivationApi::root_authority().map_err(InternalError::public)?;
    let root = IcOps::canister_self();
    if authority.binding.fleet_subnet_root != root {
        return Err(InternalError::invalid_input(
            "protected Fleet Subnet Root authority does not name this Canister",
        ));
    }
    Ok((authority, root))
}

fn require_active_root_runtime(unavailable_message: &'static str) -> Result<(), InternalError> {
    if FleetActivationApi::status()
        .map_err(InternalError::public)?
        .phase
        != FleetActivationPhase::Active
    {
        return Err(InternalError::unavailable(unavailable_message));
    }
    Ok(())
}

fn response(
    root: candid::Principal,
    prepared: &RootComponentRegistryView,
) -> Result<RootComponentRegistryStatusResponse, InternalError> {
    if prepared.root.fleet_subnet_root != root {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component Registry authority does not name this root",
        ));
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

fn allocation_response(
    allocation: RootComponentAllocationView,
) -> Result<RootComponentAllocationResponse, InternalError> {
    if allocation.allocation_sequence == 0 {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation sequence is zero",
        ));
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

fn child_allocation_response(
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

const fn component_draining_response(
    draining: RootComponentDrainingView,
) -> RootComponentDrainingResponse {
    RootComponentDrainingResponse {
        operation_id: draining.operation_id,
        component: draining.component,
        previous_registry: draining.previous_registry,
        registry: draining.registry,
        descendant_count: draining.descendant_count,
        descendant_content_hash: draining.descendant_content_hash,
        directory_authority_hash: draining.directory_authority_hash,
        started_at_ns: draining.started_at_ns,
    }
}

const fn component_final_inventory_response(
    operation_id: [u8; 32],
    component: ComponentInstanceId,
    inventory: RootComponentFinalInventoryView,
) -> RootComponentFinalInventoryResponse {
    RootComponentFinalInventoryResponse {
        operation_id,
        component,
        inventory: component_final_inventory(inventory),
    }
}

fn component_deletion_response(
    draining: RootComponentDrainingView,
) -> Result<RootComponentDeletionResponse, InternalError> {
    let progress = draining.deletion.ok_or_else(|| {
        InternalError::unavailable("Component deletion intent has not been durably prepared")
    })?;
    let phase = match progress {
        RootComponentDeletionProgressView::DeleteIntent(intent) => {
            RootComponentDeletionPhase::DeleteIntent(component_deletion_intent(intent))
        }
        RootComponentDeletionProgressView::Deleted(receipt) => {
            RootComponentDeletionPhase::Deleted(RootComponentDeletedReceipt {
                deletion: component_deletion_intent(receipt.deletion),
                deleted_at_ns: receipt.deleted_at_ns,
            })
        }
        RootComponentDeletionProgressView::MembershipRemoved(receipt) => {
            RootComponentDeletionPhase::MembershipRemoved(component_membership_removed_receipt(
                receipt,
            ))
        }
    };
    Ok(RootComponentDeletionResponse {
        operation_id: draining.operation_id,
        component: draining.component,
        phase,
    })
}

const fn component_membership_removed_receipt(
    receipt: RootComponentMembershipRemovedView,
) -> RootComponentMembershipRemovedReceipt {
    RootComponentMembershipRemovedReceipt {
        deleted: RootComponentDeletedReceipt {
            deletion: component_deletion_intent(receipt.deleted.deletion),
            deleted_at_ns: receipt.deleted.deleted_at_ns,
        },
        allocation_operation_id: receipt.allocation_operation_id,
        remaining_spec_committed_instances: receipt.remaining_spec_committed_instances,
        root_committed_component_instances: receipt.root_committed_component_instances,
        root_known_created_component_canisters: receipt.root_known_created_component_canisters,
        root_registry_encoded_bytes: receipt.root_registry_encoded_bytes,
        removed_at_ns: receipt.removed_at_ns,
        removal_hash: receipt.removal_hash,
    }
}

const fn component_deletion_intent(
    intent: RootComponentDeletionIntentView,
) -> RootComponentDeletionIntent {
    RootComponentDeletionIntent {
        final_inventory: component_final_inventory(intent.final_inventory),
        quiescence: RootComponentQuiescentReceipt {
            stop: component_quiescence_stop_intent(intent.quiescence.stop),
            observed_module_hash: intent.quiescence.observed_module_hash,
            quiesced_at_ns: intent.quiescence.quiesced_at_ns,
        },
        prepared_at_ns: intent.prepared_at_ns,
    }
}

const fn component_final_inventory(
    inventory: RootComponentFinalInventoryView,
) -> RootComponentFinalInventory {
    RootComponentFinalInventory {
        registry: inventory.registry,
        descendant_content_hash: inventory.descendant_content_hash,
        registry_encoded_bytes: inventory.registry_encoded_bytes,
        directory_synchronized_at_ns: inventory.directory_synchronized_at_ns,
        covered_fleet_registry_revision: inventory.covered_fleet_registry_revision,
        covered_fleet_registry_content_hash: inventory.covered_fleet_registry_content_hash,
        directory_authority_hash: inventory.directory_authority_hash,
        inventory_hash: inventory.inventory_hash,
        finalized_at_ns: inventory.finalized_at_ns,
    }
}

fn component_quiescence_response(
    draining: RootComponentDrainingView,
) -> Result<RootComponentQuiescenceResponse, InternalError> {
    let phase = draining.quiescence.ok_or_else(|| {
        InternalError::unavailable("Component quiescence stop intent has not been prepared")
    })?;
    Ok(RootComponentQuiescenceResponse {
        operation_id: draining.operation_id,
        component: draining.component,
        phase: match phase {
            RootComponentQuiescenceProgressView::StopIntent(intent) => {
                RootComponentQuiescencePhase::StopIntent(component_quiescence_stop_intent(intent))
            }
            RootComponentQuiescenceProgressView::Quiescent(receipt) => {
                RootComponentQuiescencePhase::Quiescent(RootComponentQuiescentReceipt {
                    stop: component_quiescence_stop_intent(receipt.stop),
                    observed_module_hash: receipt.observed_module_hash,
                    quiesced_at_ns: receipt.quiesced_at_ns,
                })
            }
        },
    })
}

const fn component_quiescence_stop_intent(
    intent: RootComponentQuiescenceStopIntentView,
) -> RootComponentQuiescenceStopIntent {
    RootComponentQuiescenceStopIntent {
        registry: intent.registry,
        descendant_count: intent.descendant_count,
        descendant_content_hash: intent.descendant_content_hash,
        canister_id: intent.canister_id,
        controller: intent.controller,
        expected_module_hash: intent.expected_module_hash,
        covered_fleet_registry_revision: intent.covered_fleet_registry_revision,
        covered_fleet_registry_content_hash: intent.covered_fleet_registry_content_hash,
        covered_authority_hash: intent.covered_authority_hash,
        runtime_operation_id: intent.runtime_operation_id,
        activation: intent.activation,
        prepared_at_ns: intent.prepared_at_ns,
    }
}

fn subtree_removal_response(
    removal: RootComponentSubtreeRemovalView,
) -> RootComponentSubtreeRemovalResponse {
    RootComponentSubtreeRemovalResponse {
        operation_id: removal.operation_id,
        component: removal.component,
        target_canister_id: removal.target_canister_id,
        target_parent_canister_id: removal.target_parent_canister_id,
        target_role: removal.target_role,
        target_status: removal.target_status,
        reserved_against_registry: removal.reserved_against_registry,
        maximum_completed_leaves: removal.maximum_completed_leaves,
        completed_leaves: removal.completed_leaves,
        traversal_steps: removal.traversal_steps,
        phase: match removal.progress {
            RootComponentSubtreeRemovalProgressView::Fenced => {
                RootComponentSubtreeRemovalPhase::Fenced
            }
            RootComponentSubtreeRemovalProgressView::Traversing { cursor } => {
                RootComponentSubtreeRemovalPhase::Traversing(RootComponentSubtreeRemovalNode {
                    canister_id: cursor.canister_id,
                    parent_canister_id: cursor.parent_canister_id,
                    role: cursor.role,
                    kind: cursor.kind,
                    installed_artifact_hash: cursor.installed_artifact_hash,
                    status: cursor.status,
                })
            }
            RootComponentSubtreeRemovalProgressView::LeafSelected { leaf } => {
                RootComponentSubtreeRemovalPhase::LeafSelected(RootComponentSubtreeRemovalNode {
                    canister_id: leaf.canister_id,
                    parent_canister_id: leaf.parent_canister_id,
                    role: leaf.role,
                    kind: leaf.kind,
                    installed_artifact_hash: leaf.installed_artifact_hash,
                    status: leaf.status,
                })
            }
            RootComponentSubtreeRemovalProgressView::StopIntent(effect) => {
                RootComponentSubtreeRemovalPhase::StopIntent(subtree_stop_intent_response(effect))
            }
            RootComponentSubtreeRemovalProgressView::Stopped(receipt) => {
                RootComponentSubtreeRemovalPhase::Stopped(subtree_stopped_receipt_response(receipt))
            }
            RootComponentSubtreeRemovalProgressView::DeleteIntent(deletion) => {
                RootComponentSubtreeRemovalPhase::DeleteIntent(
                    RootComponentSubtreeRemovalDeleteIntent {
                        stopped: subtree_stopped_receipt_response(deletion.stopped),
                    },
                )
            }
            RootComponentSubtreeRemovalProgressView::Deleted(receipt) => {
                RootComponentSubtreeRemovalPhase::Deleted(
                    RootComponentSubtreeRemovalDeletedReceipt {
                        deletion: RootComponentSubtreeRemovalDeleteIntent {
                            stopped: subtree_stopped_receipt_response(receipt.deletion.stopped),
                        },
                    },
                )
            }
            RootComponentSubtreeRemovalProgressView::MembershipRemoved(receipt) => {
                RootComponentSubtreeRemovalPhase::MembershipRemoved(
                    subtree_membership_removed_receipt_response(receipt),
                )
            }
            RootComponentSubtreeRemovalProgressView::DirectorySynchronized(receipt) => {
                RootComponentSubtreeRemovalPhase::DirectorySynchronized(
                    subtree_directory_synchronized_receipt_response(receipt),
                )
            }
            RootComponentSubtreeRemovalProgressView::Completed(completed) => {
                RootComponentSubtreeRemovalPhase::Completed(
                    RootComponentSubtreeRemovalCompletedReceipt {
                        registry: completed.registry,
                        directory_authority_hash: completed.directory_authority_hash,
                    },
                )
            }
        },
    }
}

fn subtree_directory_synchronized_receipt_response(
    receipt: RootComponentSubtreeDirectorySynchronizedView,
) -> RootComponentSubtreeRemovalDirectorySynchronizedReceipt {
    RootComponentSubtreeRemovalDirectorySynchronizedReceipt {
        membership_removed: subtree_membership_removed_receipt_response(receipt.membership_removed),
        covered_fleet_registry_revision: receipt.covered_fleet_registry_revision,
        covered_fleet_registry_content_hash: receipt.covered_fleet_registry_content_hash,
        covered_component_registry: receipt.covered_component_registry,
        covered_authority_hash: receipt.covered_authority_hash,
        owning_component: receipt
            .owning_component
            .map(subtree_directory_convergence_evidence_response),
        parent: receipt
            .parent
            .map(subtree_directory_convergence_evidence_response),
    }
}

const fn subtree_directory_convergence_evidence_response(
    evidence: RootComponentSubtreeDirectoryConvergenceView,
) -> RootComponentSubtreeRemovalDirectoryConvergenceEvidence {
    RootComponentSubtreeRemovalDirectoryConvergenceEvidence {
        operation_id: evidence.operation_id,
        canister_id: evidence.canister_id,
        activation: evidence.activation,
    }
}

fn subtree_membership_removed_receipt_response(
    receipt: RootComponentSubtreeMembershipRemovedView,
) -> RootComponentSubtreeRemovalMembershipRemovedReceipt {
    RootComponentSubtreeRemovalMembershipRemovedReceipt {
        deleted: RootComponentSubtreeRemovalDeletedReceipt {
            deletion: RootComponentSubtreeRemovalDeleteIntent {
                stopped: subtree_stopped_receipt_response(receipt.deleted.deletion.stopped),
            },
        },
        removed_from_registry: receipt.removed_from_registry,
        previous_descendant_content_hash: receipt.previous_descendant_content_hash,
        previous_committed_descendants: receipt.previous_committed_descendants,
        registry: receipt.registry,
        descendant_content_hash: receipt.descendant_content_hash,
        registry_encoded_bytes: receipt.registry_encoded_bytes,
        reserved_descendants: receipt.reserved_descendants,
        committed_descendants: receipt.committed_descendants,
        directory_synchronized_at_ns: receipt.directory_synchronized_at_ns,
        directory_authority_hash: receipt.directory_authority_hash,
        parent_role_instances: receipt.parent_role_instances,
        root_managed_descendants: receipt.root_managed_descendants,
        root_known_created_component_canisters: receipt.root_known_created_component_canisters,
    }
}

fn subtree_stop_intent_response(
    effect: RootComponentSubtreeStopEffectView,
) -> RootComponentSubtreeRemovalStopIntent {
    RootComponentSubtreeRemovalStopIntent {
        leaf: RootComponentSubtreeRemovalNode {
            canister_id: effect.leaf.canister_id,
            parent_canister_id: effect.leaf.parent_canister_id,
            role: effect.leaf.role,
            kind: effect.leaf.kind,
            installed_artifact_hash: effect.leaf.installed_artifact_hash,
            status: effect.leaf.status,
        },
        controller: effect.controller,
    }
}

fn subtree_stopped_receipt_response(
    receipt: RootComponentSubtreeStoppedEffectView,
) -> RootComponentSubtreeRemovalStoppedReceipt {
    RootComponentSubtreeRemovalStoppedReceipt {
        stop: subtree_stop_intent_response(receipt.stop),
        observed_module_hash: receipt.observed_module_hash,
    }
}

const fn registry_evidence(head: &ComponentRegistryHead) -> ComponentRegistryVersionEvidence {
    ComponentRegistryVersionEvidence {
        component: head.component,
        revision: head.revision,
        content_hash: head.content_hash,
    }
}

fn child_commit_response(
    allocation: RootComponentChildAllocationView,
    partition: ComponentRegistryPartitionView,
) -> Result<RootComponentChildCommitResponse, InternalError> {
    let RootComponentChildAllocationProgressView::Committed { commitment, .. } =
        &allocation.progress
    else {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component Child Registry commit returned a non-committed allocation",
        ));
    };
    if ComponentPartitionSnapshotAuthority::from_child_commitment(commitment)
        != ComponentPartitionSnapshotAuthority::from_partition(&partition)
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component Child receipt differs from its Registry or Directory authority",
        ));
    }
    let registry = partition_response(partition.clone());
    let directory = component_directory_head(&partition);
    Ok(RootComponentChildCommitResponse {
        allocation: child_allocation_response(allocation),
        registry,
        directory,
    })
}

fn commit_response(
    allocation: RootComponentAllocationView,
    partition: ComponentRegistryPartitionView,
) -> Result<RootComponentCommitResponse, InternalError> {
    let RootComponentAllocationProgressView::Committed { commitment, .. } = &allocation.progress
    else {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component Registry commit returned a non-committed allocation",
        ));
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
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component allocation receipt differs from its Registry or Directory authority",
        ));
    }
    let registry = partition_response(partition.clone());
    let directory = component_directory_head(&partition);
    Ok(RootComponentCommitResponse {
        allocation: allocation_response(allocation)?,
        registry,
        directory,
    })
}

fn membership_response(
    allocation: RootComponentAllocationView,
    partition: ComponentRegistryPartitionView,
    target: ComponentRuntimeStatusResponse,
) -> Result<RootComponentMembershipActivationResponse, InternalError> {
    let membership = committed_directory_receipt(&allocation)?
        .membership
        .as_ref()
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Component allocation has no active membership receipt",
            )
        })?;
    let encoded_bytes_covered = membership.registry_encoded_bytes <= partition.encoded_bytes;
    if !membership.directory_synchronized
        || !encoded_bytes_covered
        || membership.directory_synchronized_at_ns != partition.directory_synchronized_at_ns
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component membership receipt differs from active Registry authority",
        ));
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

fn child_membership_response(
    allocation: RootComponentChildAllocationView,
    committed_partition: ComponentRegistryPartitionView,
    active_partition: ComponentRegistryPartitionView,
    child: ComponentRuntimeStatusResponse,
) -> Result<RootComponentChildMembershipActivationResponse, InternalError> {
    let membership = committed_child_directory_receipt(&allocation)?
        .membership
        .as_ref()
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Component Child allocation has no active membership receipt",
            )
        })?;
    let membership_matches_partition =
        ComponentPartitionSnapshotAuthority::from_child_membership(membership)
            == ComponentPartitionSnapshotAuthority::from_partition(&active_partition);
    if !membership.directory_synchronized || !membership_matches_partition {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component Child membership receipt differs from active Registry authority",
        ));
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

fn partition_response(
    partition: ComponentRegistryPartitionView,
) -> ComponentRegistryPartitionResponse {
    ComponentRegistryPartitionResponse {
        head: ComponentRegistryHead {
            component: partition.binding.component,
            revision: partition.revision,
            content_hash: partition.content_hash,
        },
        binding: partition.binding,
        provisioning_origin: partition.provisioning_origin,
        release_set: partition.release_set,
        status: partition.status,
        reserved_descendants: partition.reserved_descendants,
        committed_descendants: partition.committed_descendants,
        encoded_bytes: partition.encoded_bytes,
    }
}

fn component_directory_head(partition: &ComponentRegistryPartitionView) -> ComponentDirectoryHead {
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

fn decode_component_directory_cursor(
    request: &ComponentDirectoryPageRequest,
) -> Result<Option<ComponentDirectoryCanonicalCursor>, InternalError> {
    let Some(cursor) = request.cursor.as_ref() else {
        return Ok(None);
    };
    if cursor.0.is_empty() || cursor.0.len() > MAX_COMPONENT_DIRECTORY_CURSOR_BYTES {
        return Err(InternalError::invalid_input(
            "Component Directory cursor has an invalid encoded size",
        ));
    }
    let payload =
        candid::decode_one::<ComponentDirectoryCursorPayload>(&cursor.0).map_err(|_| {
            InternalError::invalid_input("Component Directory cursor is malformed or unsupported")
        })?;
    if ComponentDirectoryCursorBinding::from_payload(&payload)
        != ComponentDirectoryCursorBinding::from_request(request)
    {
        return Err(InternalError::conflict(
            "Component Directory cursor is bound to a different head or filter",
        ));
    }
    Ok(Some(ComponentDirectoryCanonicalCursor {
        parent_canister_id: payload.last_parent_canister_id,
        role: payload.last_role,
        canister_id: payload.last_canister_id,
    }))
}

fn encode_component_directory_cursor(
    request: &ComponentDirectoryPageRequest,
    cursor: ComponentDirectoryCanonicalCursor,
) -> Result<ComponentDirectoryPageCursor, InternalError> {
    let payload = ComponentDirectoryCursorPayload {
        directory: request.directory.clone(),
        parent_canister_id: request.parent_canister_id,
        role: request.role.clone(),
        status: request.status,
        last_parent_canister_id: cursor.parent_canister_id,
        last_role: cursor.role,
        last_canister_id: cursor.canister_id,
    };
    let bytes = candid::encode_one(payload).map_err(|error| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("Component Directory cursor encoding failed: {error}"),
        )
    })?;
    if bytes.len() > MAX_COMPONENT_DIRECTORY_CURSOR_BYTES {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "Component Directory cursor exceeds its protocol byte bound",
        ));
    }
    Ok(ComponentDirectoryPageCursor(bytes))
}

fn creation_plan(
    root: candid::Principal,
    store: &RootStoreBootstrapResponse,
    allocation: &RootComponentAllocationView,
) -> Result<RootComponentCreationPlan, InternalError> {
    if store.fleet_subnet_root != root || store.release_set != allocation.release_set {
        return Err(InternalError::conflict(
            "verified Store evidence differs from the reserved Component authority",
        ));
    }
    let artifact = exact_store_artifact(store, &allocation.role)?;
    let config = ConfigOps::try_get_canister_by_role(&allocation.role)?;

    Ok(RootComponentCreationPlan {
        wasm_store: store.wasm_store,
        payload_hash: artifact.payload_hash,
        payload_size_bytes: artifact.payload_size_bytes,
        initial_cycles: config.initial_cycles,
        controller: root,
    })
}

fn child_creation_plan(
    root: candid::Principal,
    store: &RootStoreBootstrapResponse,
    allocation: &RootComponentChildAllocationView,
) -> Result<RootComponentCreationPlan, InternalError> {
    if store.fleet_subnet_root != root || store.release_set != allocation.release_set {
        return Err(InternalError::conflict(
            "verified Store evidence differs from the reserved Component Child authority",
        ));
    }
    let artifact = exact_store_artifact(store, &allocation.child_role)?;
    let config = ConfigOps::try_get_canister_by_role(&allocation.child_role)?;

    Ok(RootComponentCreationPlan {
        wasm_store: store.wasm_store,
        payload_hash: artifact.payload_hash,
        payload_size_bytes: artifact.payload_size_bytes,
        initial_cycles: config.initial_cycles,
        controller: root,
    })
}

fn exact_store_artifact<'a>(
    store: &'a RootStoreBootstrapResponse,
    role: &canic_core::ids::CanisterRole,
) -> Result<&'a canic_core::dto::root_store::RootStoreCatalogEntry, InternalError> {
    let mut matching = store.catalog.iter().filter(|entry| &entry.role == role);
    let artifact = matching.next().ok_or_else(|| {
        InternalError::unavailable(format!(
            "verified Store has no artifact for reserved Component role '{role}'"
        ))
    })?;
    if matching.next().is_some() {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "verified Store contains duplicate artifacts for one Component role",
        ));
    }
    Ok(artifact)
}

fn prepared_component_quiescence_plan(
    root: &FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    partition: &ComponentRegistryPartitionView,
    draining: &RootComponentDrainingView,
) -> Result<PreparedComponentQuiescencePlan, InternalError> {
    let (stop, observed_module_hash, already_quiescent) = match &draining.quiescence {
        Some(RootComponentQuiescenceProgressView::StopIntent(stop)) => (stop.clone(), None, false),
        Some(RootComponentQuiescenceProgressView::Quiescent(receipt)) => (
            receipt.stop.clone(),
            Some(receipt.observed_module_hash),
            true,
        ),
        None => {
            return Err(InternalError::unavailable(
                "Component quiescence stop intent has not been durably prepared",
            ));
        }
    };
    let durable_authority = (
        draining.operation_id,
        draining.component,
        &draining.registry,
        partition.binding.canister_id,
        partition.binding.fleet_subnet_root,
    );
    let stop_authority = (
        draining.operation_id,
        partition.binding.component,
        &stop.registry,
        stop.canister_id,
        stop.controller,
    );
    if durable_authority != stop_authority {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component quiescence stop intent differs from protected draining authority",
        ));
    }
    if store.fleet_subnet_root != root.fleet_subnet_root
        || store.release_set != partition.release_set
    {
        return Err(InternalError::conflict(
            "verified Store differs from Component quiescence root authority",
        ));
    }
    let artifact = exact_store_artifact(store, &partition.binding.role)?;
    if stop.expected_module_hash != artifact.payload_hash
        || observed_module_hash.is_some_and(|hash| hash != artifact.payload_hash)
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component quiescence module authority differs from the verified Store artifact",
        ));
    }
    Ok(PreparedComponentQuiescencePlan {
        component: draining.component,
        operation_id: draining.operation_id,
        stop,
        expected_status_module_hash: artifact.payload_hash,
        already_quiescent,
    })
}

fn prepared_component_deletion_plan(
    root: &FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    partition: &ComponentRegistryPartitionView,
    draining: &RootComponentDrainingView,
    request: &RootComponentDeletionRequest,
) -> Result<PreparedComponentDeletionPlan, InternalError> {
    let (deletion, already_deleted) = match &draining.deletion {
        Some(RootComponentDeletionProgressView::DeleteIntent(deletion)) => {
            (deletion.clone(), false)
        }
        Some(RootComponentDeletionProgressView::Deleted(receipt)) => {
            (receipt.deletion.clone(), true)
        }
        Some(RootComponentDeletionProgressView::MembershipRemoved(receipt)) => {
            (receipt.deleted.deletion.clone(), true)
        }
        None => {
            return Err(InternalError::unavailable(
                "Component deletion intent has not been durably prepared",
            ));
        }
    };
    let request_authority = ComponentDeletionRequestAuthority::from_request(request);
    let durable_authority = ComponentDeletionRequestAuthority::from_durable(draining, &deletion);
    if request_authority != durable_authority {
        return Err(InternalError::conflict(
            "Component deletion request differs from durable final authority",
        ));
    }
    validate_component_deletion_binding(root, partition, &deletion)?;
    let expected_status_module_hash =
        component_deletion_store_module(store, root, partition, &deletion)?;
    Ok(PreparedComponentDeletionPlan {
        component: draining.component,
        operation_id: draining.operation_id,
        deletion,
        expected_status_module_hash,
        already_deleted,
    })
}

fn validate_component_deletion_binding(
    root: &FleetSubnetRootBinding,
    partition: &ComponentRegistryPartitionView,
    deletion: &RootComponentDeletionIntentView,
) -> Result<(), InternalError> {
    if deletion.quiescence.stop.canister_id != partition.binding.canister_id {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component deletion Canister differs from protected Component binding",
        ));
    }
    if deletion.quiescence.stop.controller != root.fleet_subnet_root {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component deletion controller differs from protected root authority",
        ));
    }
    Ok(())
}

fn component_deletion_store_module(
    store: &RootStoreBootstrapResponse,
    root: &FleetSubnetRootBinding,
    partition: &ComponentRegistryPartitionView,
    deletion: &RootComponentDeletionIntentView,
) -> Result<[u8; 32], InternalError> {
    if store.fleet_subnet_root != root.fleet_subnet_root {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "verified Store differs from Component deletion root authority",
        ));
    }
    if store.release_set != partition.release_set {
        return Err(InternalError::conflict(
            "verified Store release set differs from Component deletion authority",
        ));
    }
    let artifact = exact_store_artifact(store, &partition.binding.role)?;
    if deletion.quiescence.stop.expected_module_hash != artifact.payload_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component deletion intent differs from verified Store module authority",
        ));
    }
    if deletion.quiescence.observed_module_hash != artifact.payload_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component deletion quiescence differs from verified Store module authority",
        ));
    }
    Ok(artifact.payload_hash)
}

fn prepared_subtree_leaf_stop_plan(
    root: &canic_core::ids::FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    removal: &RootComponentSubtreeRemovalView,
    request: &RootComponentSubtreeRemovalStopRequest,
    maximum_component_registry_bytes: u64,
) -> Result<PreparedSubtreeLeafStopPlan, InternalError> {
    let (stop, durable_module_hash, progressed_beyond_stopped) = match &removal.progress {
        RootComponentSubtreeRemovalProgressView::StopIntent(effect) => {
            (effect.clone(), None, false)
        }
        RootComponentSubtreeRemovalProgressView::Stopped(receipt) => (
            receipt.stop.clone(),
            Some(receipt.observed_module_hash),
            false,
        ),
        RootComponentSubtreeRemovalProgressView::DeleteIntent(deletion) => (
            deletion.stopped.stop.clone(),
            Some(deletion.stopped.observed_module_hash),
            true,
        ),
        RootComponentSubtreeRemovalProgressView::Deleted(receipt) => (
            receipt.deletion.stopped.stop.clone(),
            Some(receipt.deletion.stopped.observed_module_hash),
            true,
        ),
        RootComponentSubtreeRemovalProgressView::MembershipRemoved(receipt) => (
            receipt.deleted.deletion.stopped.stop.clone(),
            Some(receipt.deleted.deletion.stopped.observed_module_hash),
            true,
        ),
        RootComponentSubtreeRemovalProgressView::DirectorySynchronized(receipt) => (
            receipt
                .membership_removed
                .deleted
                .deletion
                .stopped
                .stop
                .clone(),
            Some(
                receipt
                    .membership_removed
                    .deleted
                    .deletion
                    .stopped
                    .observed_module_hash,
            ),
            true,
        ),
        RootComponentSubtreeRemovalProgressView::Fenced
        | RootComponentSubtreeRemovalProgressView::Traversing { .. }
        | RootComponentSubtreeRemovalProgressView::LeafSelected { .. }
        | RootComponentSubtreeRemovalProgressView::Completed(_) => {
            return Err(InternalError::unavailable(
                "Component subtree leaf has not prepared its stop intent",
            ));
        }
    };
    let requested_leaf_authority = (
        request.operation_id,
        request.component,
        request.expected_traversal_steps,
        request.expected_leaf_canister_id,
        request.expected_leaf_parent_canister_id,
    );
    let durable_leaf_authority = (
        removal.operation_id,
        removal.component,
        removal.traversal_steps,
        stop.leaf.canister_id,
        stop.leaf.parent_canister_id,
    );
    if requested_leaf_authority != durable_leaf_authority {
        return Err(InternalError::conflict(
            "Component subtree stop request differs from durable leaf authority",
        ));
    }
    if stop.controller != root.fleet_subnet_root {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "durable Component subtree stop controller differs from protected root authority",
        ));
    }
    if store.fleet_subnet_root != root.fleet_subnet_root {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "verified Store differs from Component subtree stop root authority",
        ));
    }
    let artifact = exact_store_artifact(store, &stop.leaf.role)?;
    if artifact.raw_module_hash != stop.leaf.installed_artifact_hash
        || durable_module_hash.is_some_and(|hash| hash != artifact.payload_hash)
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component subtree stop authority differs from verified Store artifact",
        ));
    }
    Ok(PreparedSubtreeLeafStopPlan {
        component: removal.component,
        operation_id: removal.operation_id,
        traversal_steps: removal.traversal_steps,
        stop,
        expected_status_module_hash: artifact.payload_hash,
        maximum_component_registry_bytes,
        progressed_beyond_stopped,
    })
}

fn validate_subtree_directory_request(
    removal: &RootComponentSubtreeRemovalView,
    membership: &RootComponentSubtreeMembershipRemovedView,
    request: &RootComponentSubtreeRemovalDirectorySynchronizationRequest,
) -> Result<(), InternalError> {
    let leaf = &membership.deleted.deletion.stopped.stop.leaf;
    let requested_leaf_authority = (
        request.operation_id,
        request.component,
        request.expected_traversal_steps,
        request.expected_leaf_canister_id,
        request.expected_leaf_parent_canister_id,
    );
    let durable_leaf_authority = (
        removal.operation_id,
        removal.component,
        removal.traversal_steps,
        leaf.canister_id,
        leaf.parent_canister_id,
    );
    if requested_leaf_authority != durable_leaf_authority {
        return Err(InternalError::conflict(
            "Component subtree Directory request differs from durable leaf authority",
        ));
    }
    Ok(())
}

fn prepared_subtree_leaf_delete_plan(
    root: &canic_core::ids::FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    removal: &RootComponentSubtreeRemovalView,
    request: &RootComponentSubtreeRemovalDeleteRequest,
    maximum_component_registry_bytes: u64,
) -> Result<PreparedSubtreeLeafDeletePlan, InternalError> {
    let (deletion, already_deleted) = match &removal.progress {
        RootComponentSubtreeRemovalProgressView::DeleteIntent(deletion) => {
            (deletion.clone(), false)
        }
        RootComponentSubtreeRemovalProgressView::Deleted(receipt) => {
            (receipt.deletion.clone(), true)
        }
        RootComponentSubtreeRemovalProgressView::MembershipRemoved(receipt) => {
            (receipt.deleted.deletion.clone(), true)
        }
        RootComponentSubtreeRemovalProgressView::DirectorySynchronized(receipt) => {
            (receipt.membership_removed.deleted.deletion.clone(), true)
        }
        RootComponentSubtreeRemovalProgressView::Fenced
        | RootComponentSubtreeRemovalProgressView::Traversing { .. }
        | RootComponentSubtreeRemovalProgressView::LeafSelected { .. }
        | RootComponentSubtreeRemovalProgressView::StopIntent(_)
        | RootComponentSubtreeRemovalProgressView::Stopped(_)
        | RootComponentSubtreeRemovalProgressView::Completed(_) => {
            return Err(InternalError::unavailable(
                "Component subtree leaf has not prepared its deletion intent",
            ));
        }
    };
    let requested_leaf_authority = (
        request.operation_id,
        request.component,
        request.expected_traversal_steps,
        request.expected_leaf_canister_id,
        request.expected_leaf_parent_canister_id,
    );
    let durable_leaf_authority = (
        removal.operation_id,
        removal.component,
        removal.traversal_steps,
        deletion.stopped.stop.leaf.canister_id,
        deletion.stopped.stop.leaf.parent_canister_id,
    );
    if requested_leaf_authority != durable_leaf_authority {
        return Err(InternalError::conflict(
            "Component subtree deletion request differs from durable leaf authority",
        ));
    }
    if deletion.stopped.stop.controller != root.fleet_subnet_root {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "durable Component subtree deletion controller differs from protected root authority",
        ));
    }
    if store.fleet_subnet_root != root.fleet_subnet_root {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "verified Store differs from Component subtree deletion root authority",
        ));
    }
    let artifact = exact_store_artifact(store, &deletion.stopped.stop.leaf.role)?;
    if artifact.raw_module_hash != deletion.stopped.stop.leaf.installed_artifact_hash
        || artifact.payload_hash != deletion.stopped.observed_module_hash
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component subtree deletion authority differs from verified Store artifact",
        ));
    }
    Ok(PreparedSubtreeLeafDeletePlan {
        component: removal.component,
        operation_id: removal.operation_id,
        traversal_steps: removal.traversal_steps,
        deletion,
        expected_status_module_hash: artifact.payload_hash,
        maximum_component_registry_bytes,
        already_deleted,
    })
}

async fn observe_or_stop_component(
    plan: &PreparedComponentQuiescencePlan,
) -> Result<(), InternalError> {
    match observed_component_quiescence_status(plan).await? {
        CanisterStatusType::Stopped => return Ok(()),
        CanisterStatusType::Stopping => {
            return Err(InternalError::unavailable(
                "Component quiescence stop is still in progress",
            ));
        }
        CanisterStatusType::Running => {}
    }

    let stop_error = MgmtOps::stop_canister(plan.stop.canister_id).await.err();
    match observed_component_quiescence_status(plan).await? {
        CanisterStatusType::Stopped => Ok(()),
        CanisterStatusType::Stopping => Err(InternalError::unavailable(
            "Component quiescence stop is still in progress",
        )),
        CanisterStatusType::Running => match stop_error {
            Some(error) => Err(error),
            None => Err(InternalError::unavailable(
                "Component remains running after its quiescence stop call completed",
            )),
        },
    }
}

async fn observed_component_quiescence_status(
    plan: &PreparedComponentQuiescencePlan,
) -> Result<CanisterStatusType, InternalError> {
    let status = MgmtOps::canister_status(plan.stop.canister_id).await?;
    if status.settings.controllers != vec![plan.stop.controller] {
        return Err(InternalError::conflict(
            "Component controllers differ from its sole root quiescence authority",
        ));
    }
    if status.module_hash.as_deref() != Some(plan.expected_status_module_hash.as_slice()) {
        return Err(InternalError::conflict(
            "Component module differs from its verified Store quiescence authority",
        ));
    }
    Ok(status.status)
}

async fn observe_or_stop_subtree_leaf(
    plan: &PreparedSubtreeLeafStopPlan,
) -> Result<(), InternalError> {
    match observed_subtree_leaf_status(plan).await? {
        CanisterStatusType::Stopped => return Ok(()),
        CanisterStatusType::Stopping => {
            return Err(InternalError::unavailable(
                "Component subtree leaf stop is still in progress",
            ));
        }
        CanisterStatusType::Running => {}
    }

    let stop_error = MgmtOps::stop_canister(plan.stop.leaf.canister_id)
        .await
        .err();
    match observed_subtree_leaf_status(plan).await? {
        CanisterStatusType::Stopped => Ok(()),
        CanisterStatusType::Stopping => Err(InternalError::unavailable(
            "Component subtree leaf stop is still in progress",
        )),
        CanisterStatusType::Running => match stop_error {
            Some(error) => Err(error),
            None => Err(InternalError::unavailable(
                "Component subtree leaf remains running after its stop call completed",
            )),
        },
    }
}

async fn observe_or_delete_subtree_leaf(
    plan: &PreparedSubtreeLeafDeletePlan,
) -> Result<(), InternalError> {
    match observed_subtree_leaf_for_deletion(plan).await? {
        CanisterStatusObservation::Absent => return Ok(()),
        CanisterStatusObservation::Present(_) => {}
    }

    let delete_error = MgmtOps::delete_canister(plan.deletion.stopped.stop.leaf.canister_id)
        .await
        .err();
    match observed_subtree_leaf_for_deletion(plan).await? {
        CanisterStatusObservation::Absent => Ok(()),
        CanisterStatusObservation::Present(_) => match delete_error {
            Some(error) => Err(error),
            None => Err(InternalError::unavailable(
                "Component subtree leaf remains present after its deletion call completed",
            )),
        },
    }
}

async fn observe_or_delete_component(
    plan: &PreparedComponentDeletionPlan,
) -> Result<(), InternalError> {
    match observed_component_for_deletion(plan).await? {
        CanisterStatusObservation::Absent => return Ok(()),
        CanisterStatusObservation::Present(_) => {}
    }

    let delete_error = MgmtOps::delete_canister(plan.deletion.quiescence.stop.canister_id)
        .await
        .err();
    match observed_component_for_deletion(plan).await? {
        CanisterStatusObservation::Absent => Ok(()),
        CanisterStatusObservation::Present(_) => match delete_error {
            Some(error) => Err(error),
            None => Err(InternalError::unavailable(
                "Component remains present after its deletion call completed",
            )),
        },
    }
}

async fn observed_component_for_deletion(
    plan: &PreparedComponentDeletionPlan,
) -> Result<CanisterStatusObservation, InternalError> {
    let stop = &plan.deletion.quiescence.stop;
    let observation = MgmtOps::observe_canister_status(stop.canister_id).await?;
    let CanisterStatusObservation::Present(status) = &observation else {
        return Ok(observation);
    };
    validate_component_deletion_live_status(status, stop, plan.expected_status_module_hash)?;
    Ok(observation)
}

fn validate_component_deletion_live_status(
    status: &CanisterStatus,
    stop: &RootComponentQuiescenceStopIntentView,
    expected_status_module_hash: [u8; 32],
) -> Result<(), InternalError> {
    if status.settings.controllers != vec![stop.controller] {
        return Err(InternalError::conflict(
            "Component controllers differ from its sole root deletion authority",
        ));
    }
    if status.module_hash.as_deref() != Some(expected_status_module_hash.as_slice()) {
        return Err(InternalError::conflict(
            "Component module differs from its verified Store deletion authority",
        ));
    }
    if status.status != CanisterStatusType::Stopped {
        return Err(InternalError::conflict(
            "Component is no longer stopped under its deletion authority",
        ));
    }
    Ok(())
}

async fn observed_subtree_leaf_status(
    plan: &PreparedSubtreeLeafStopPlan,
) -> Result<CanisterStatusType, InternalError> {
    let status = MgmtOps::canister_status(plan.stop.leaf.canister_id).await?;
    validate_subtree_leaf_live_status(&status, &plan.stop, plan.expected_status_module_hash)
}

async fn observed_subtree_leaf_for_deletion(
    plan: &PreparedSubtreeLeafDeletePlan,
) -> Result<CanisterStatusObservation, InternalError> {
    let observation =
        MgmtOps::observe_canister_status(plan.deletion.stopped.stop.leaf.canister_id).await?;
    let CanisterStatusObservation::Present(status) = &observation else {
        return Ok(observation);
    };
    let status_type = validate_subtree_leaf_live_status(
        status,
        &plan.deletion.stopped.stop,
        plan.expected_status_module_hash,
    )?;
    if status_type != CanisterStatusType::Stopped {
        return Err(InternalError::conflict(
            "Component subtree leaf is no longer stopped under its deletion authority",
        ));
    }
    Ok(observation)
}

fn validate_subtree_leaf_live_status(
    status: &CanisterStatus,
    stop: &RootComponentSubtreeStopEffectView,
    expected_status_module_hash: [u8; 32],
) -> Result<CanisterStatusType, InternalError> {
    if status.settings.controllers != vec![stop.controller] {
        return Err(InternalError::conflict(
            "Component subtree leaf controllers differ from its sole root authority",
        ));
    }
    if status.module_hash.as_deref() != Some(expected_status_module_hash.as_slice()) {
        return Err(InternalError::conflict(
            "Component subtree leaf module differs from its verified Store authority",
        ));
    }
    Ok(status.status)
}

fn validate_allocation_caller(
    allocation: &RootComponentAllocationView,
) -> Result<(), InternalError> {
    let expected = ComponentProvisioningOrigin::FleetAdministrator {
        caller: IcOps::msg_caller(),
    };
    if allocation.provisioning_origin != expected {
        return Err(InternalError::conflict(
            "Component creation caller differs from its reserved administrator origin",
        ));
    }
    Ok(())
}

fn validate_creation_effect(
    effect: &RootComponentCreationEffectView,
    expected: &RootComponentCreationPlan,
) -> Result<(), InternalError> {
    if !expected.matches_effect(effect) {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "durable Component creation intent differs from verified Store or root settings",
        ));
    }
    Ok(())
}

fn validate_install_effect(
    effect: &RootComponentInstallEffectView,
    expected: &RootComponentInstallPlan,
) -> Result<(), InternalError> {
    if !expected.matches_effect(effect) {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "durable Component install intent differs from verified module or binding authority",
        ));
    }
    Ok(())
}

fn validate_child_install_effect(
    effect: &RootComponentChildInstallEffectView,
    expected: &RootComponentChildInstallPlan,
) -> Result<(), InternalError> {
    if !expected.matches_effect(effect) {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "durable Component Child install intent differs from verified module or binding authority",
        ));
    }
    Ok(())
}

const fn creation_evidence(
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

fn install_evidence(effect: RootComponentInstallEffectView) -> RootComponentInstallEvidence {
    RootComponentInstallEvidence {
        raw_module_hash: effect.raw_module_hash,
        chunk_hashes: effect.chunk_hashes,
        binding: effect.binding,
    }
}

fn child_install_evidence(
    effect: RootComponentChildInstallEffectView,
) -> RootComponentChildInstallEvidence {
    RootComponentChildInstallEvidence {
        raw_module_hash: effect.raw_module_hash,
        chunk_hashes: effect.chunk_hashes,
        binding: effect.binding,
    }
}

fn validate_allocation_record(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    allocation: &RootComponentAllocationView,
    expected_operation_id: [u8; 32],
) -> Result<(), InternalError> {
    if allocation.operation_id == [0; 32] || allocation.operation_id != expected_operation_id {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation operation identity is invalid",
        ));
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
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation identity differs from its root-local sequence",
        ));
    }
    if allocation.release_set != release_set {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation release set differs from protected root authority",
        ));
    }
    let admission = root
        .component_admissions
        .binary_search_by(|candidate| candidate.component_spec.cmp(&allocation.component_spec))
        .ok()
        .map(|index| &root.component_admissions[index])
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "stored Component allocation Spec is not admitted by its protected root",
            )
        })?;
    let spec = topology.get(&allocation.component_spec).ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation Spec is absent from the protected topology",
        )
    })?;
    if allocation.spec_hash != admission.spec_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation hash differs from its protected root admission",
        ));
    }
    if allocation.spec_hash != spec.spec_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation hash differs from its protected Spec",
        ));
    }
    if allocation.role != spec.component_role {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation role differs from its protected Spec",
        ));
    }
    Ok(())
}

const fn retained_subtree_stop_controller(
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

fn validate_subtree_removal(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    removal: &RootComponentSubtreeRemovalView,
    request: Option<&RootComponentSubtreeRemovalRequest>,
) -> Result<(), InternalError> {
    let partition = ComponentRegistryOps::partition(removal.component)?.ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "subtree-removal fence has no owning Component partition",
        )
    })?;
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
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "subtree-removal fence differs from protected Component authority",
        ));
    }
    let stop_controller = retained_subtree_stop_controller(&removal.progress);
    if stop_controller.is_some_and(|controller| controller != root.fleet_subnet_root) {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "subtree-removal stop intent differs from protected root authority",
        ));
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
            return Err(InternalError::conflict(
                "Component subtree-removal operation is already bound to different intent",
            ));
        }
    }
    Ok(())
}

fn validate_subtree_removal_target(
    root: &canic_core::ids::FleetSubnetRootBinding,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    removal: &RootComponentSubtreeRemovalView,
) -> Result<(), InternalError> {
    let registered_target =
        ComponentRegistryOps::registered_parent(removal.component, removal.target_canister_id)?;
    if matches!(
        &removal.progress,
        RootComponentSubtreeRemovalProgressView::Completed(_)
    ) {
        if registered_target.is_some() {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "completed subtree-removal target remains registered",
            ));
        }
    } else {
        let (target, _current_status) = registered_target.ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "subtree-removal fence target is no longer registered",
            )
        })?;
        let ManagedCanisterBinding::ComponentChild(target) = target else {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "subtree-removal fence targets a top-level Component",
            ));
        };
        topology
            .validate_component_child_binding(root, &target)
            .map_err(|error| {
                InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    format!("subtree-removal target binding is invalid: {error}"),
                )
            })?;
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
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "subtree-removal fence differs from registered target authority",
            ));
        }
    }
    Ok(())
}

fn validate_child_allocation(
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
                .map_err(|error| {
                    InternalError::invariant(
                        InternalErrorOrigin::Storage,
                        format!("registered Component parent binding is invalid: {error}"),
                    )
                })?;
            (binding, binding.canister_id, &binding.role)
        }
        ManagedCanisterBinding::ComponentChild(binding) => {
            topology
                .validate_component_child_binding(root, binding)
                .map_err(|error| {
                    InternalError::invariant(
                        InternalErrorOrigin::Storage,
                        format!("registered Component Child parent binding is invalid: {error}"),
                    )
                })?;
            (&binding.component, binding.canister_id, &binding.role)
        }
    };
    if parent_canister_id != allocation.parent_canister_id {
        return Err(InternalError::public(Error::forbidden(
            "Component Child allocation belongs to a different registered parent",
        )));
    }
    let spec = topology
        .get(&parent_component.component_spec)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "registered Component parent Spec is absent from protected topology",
            )
        })?;
    let child = spec.child(&allocation.child_role).ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "reserved Component Child role is absent from protected Component Spec",
        )
    })?;
    let grant = spec
        .spawn_grant(parent_role, &allocation.child_role)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "reserved Component Child has no protected parent spawn grant",
            )
        })?;
    let expected_authority = ComponentChildAllocationAuthority {
        component: parent_component.component,
        parent_role,
        child_kind: child.kind,
        maximum_instances_per_parent: grant.maximum_instances_per_parent,
        maximum_descendants: spec.limits.maximum_descendants,
        maximum_registry_bytes: spec.limits.maximum_registry_bytes,
        release_set,
        reserved_component: parent_component.component,
    };
    let reservation_is_versioned = allocation.reserved_against_registry.revision > 0;
    if ComponentChildAllocationAuthority::from_allocation(allocation) != expected_authority
        || !reservation_is_versioned
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "durable Component Child allocation differs from protected tree authority",
        ));
    }
    if request.is_some_and(|request| !child_allocation_request_matches(request, allocation)) {
        return Err(InternalError::conflict(
            "Component Child allocation operation is already bound to different intent",
        ));
    }
    Ok(())
}

fn child_allocation_request_matches(
    request: &RootComponentChildAllocationRequest,
    allocation: &RootComponentChildAllocationView,
) -> bool {
    let registry_matches = request.expected_registry == allocation.reserved_against_registry;
    request.operation_id == allocation.operation_id
        && request.component == allocation.component
        && registry_matches
        && request.child_role == allocation.child_role
}

fn validate_partition(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    partition: &ComponentRegistryPartitionView,
) -> Result<(), InternalError> {
    topology
        .validate_component_binding(root, &partition.binding)
        .map_err(|error| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                format!("committed Component binding is invalid: {error}"),
            )
        })?;
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
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "committed Component partition differs from protected root or principal authority",
        ));
    }
    Ok(())
}

fn validate_component_draining(
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
        return Err(InternalError::conflict(
            "Component draining receipt differs from protected intent or Registry authority",
        ));
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
        };
        if ComponentRuntimeOps::directory_authority_hash(&authority)?
            != draining.directory_authority_hash
        {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Component draining Directory authority hash is invalid",
            ));
        }
    }
    Ok(())
}

fn validate_directory_member(
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
                .map_err(|error| {
                    InternalError::invariant(
                        InternalErrorOrigin::Storage,
                        format!("Component Directory caller binding is invalid: {error}"),
                    )
                })
        }
        ManagedCanisterBinding::Component(_) | ManagedCanisterBinding::ComponentChild(_) => {
            Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Component Directory caller differs from its protected partition",
            ))
        }
    }
}

const fn component_directory_member_can_read(status: ComponentLifecycleStatus) -> bool {
    matches!(
        status,
        ComponentLifecycleStatus::Active | ComponentLifecycleStatus::Draining
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::ids::{
        AppId, CanonicalNetworkId, ComponentInstanceId, FleetBinding, FleetCoordinatorBinding,
        FleetId, FleetKey, FleetRegistryAuthority, SubnetId,
    };

    #[test]
    fn draining_component_members_retain_directory_lookup() {
        assert!(component_directory_member_can_read(
            ComponentLifecycleStatus::Active
        ));
        assert!(component_directory_member_can_read(
            ComponentLifecycleStatus::Draining
        ));
        assert!(!component_directory_member_can_read(
            ComponentLifecycleStatus::Prepared
        ));
        assert!(!component_directory_member_can_read(
            ComponentLifecycleStatus::Removed
        ));
    }

    #[test]
    fn component_directory_cursor_is_opaque_and_bound_to_head_and_filters() {
        let root = candid::Principal::from_slice(&[1; 29]);
        let component = ComponentInstanceId::from_generated_bytes([2; 32]);
        let binding = ComponentBinding {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                            fleet_id: FleetId::from_generated_bytes([3; 32]),
                        },
                        app: AppId::from("toko"),
                    },
                    coordinator_subnet: SubnetId::from_principal(candid::Principal::from_slice(
                        &[4; 29],
                    )),
                    coordinator: candid::Principal::from_slice(&[5; 29]),
                },
                epoch: 1,
            },
            component,
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
            placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[7; 29])),
            fleet_subnet_root: root,
            canister_id: candid::Principal::from_slice(&[8; 29]),
        };
        let mut request = ComponentDirectoryPageRequest {
            directory: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: binding.clone(),
                    source_fleet_subnet_root: root,
                    component_registry_revision: 9,
                    component_registry_content_hash: [10; 32],
                    synchronized_at_ns: 11,
                },
                descendant_count: 12,
            },
            parent_canister_id: Some(binding.canister_id),
            role: Some(CanisterRole::new("project_instance")),
            status: None,
            cursor: None,
            limit: 50,
        };
        let canonical = ComponentDirectoryCanonicalCursor {
            parent_canister_id: binding.canister_id,
            role: CanisterRole::new("project_instance"),
            canister_id: candid::Principal::from_slice(&[13; 29]),
        };
        let encoded = encode_component_directory_cursor(&request, canonical.clone())
            .expect("encode Directory cursor");
        request.cursor = Some(encoded);

        assert_eq!(
            decode_component_directory_cursor(&request).expect("decode Directory cursor"),
            Some(canonical)
        );
        let mut conflicting = request.clone();
        conflicting.status = Some(ComponentLifecycleStatus::Active);
        assert!(decode_component_directory_cursor(&conflicting).is_err());
        let mut malformed = request;
        malformed.cursor = Some(ComponentDirectoryPageCursor(vec![1, 2, 3]));
        assert!(decode_component_directory_cursor(&malformed).is_err());
    }
}
