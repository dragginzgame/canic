//! Module: workflow::component_registry
//!
//! Responsibility: prepare Component Registry authority and advance root-executed Component-tree lifecycle.
//! Does not own: topology compilation, Store publication, or root runtime activation.
//! Boundary: every mutation follows exact Store and active Registry Mirror/Directory verification.

use crate::{
    dto::root::RootComponentOperationStatus,
    ops::{
        canister_pool::{CanisterPoolClaimKey, CanisterPoolOps},
        component_provisioning::RootComponentProvisioningOps,
        component_registry::{
            ComponentRegistryOps, RootComponentChildInstallPlan, RootComponentCreationPlan,
            RootComponentInstallPlan,
        },
        fleet_registry_mirror::FleetRegistryMirrorOps,
        fleet_service_peer::FleetServicePeerOps,
    },
    view::component_registry::{
        ActiveComponentMemberView, ComponentDirectoryCanonicalCursor,
        ComponentDirectoryPageSelection, ComponentRegistryPartitionView,
        RootComponentAllocationProgressView, RootComponentAllocationView,
        RootComponentChildAllocationProgressView, RootComponentChildAllocationView,
        RootComponentChildCommitmentView, RootComponentChildInstallEffectView,
        RootComponentChildMembershipView, RootComponentCreationEffectView,
        RootComponentDeletionIntentView, RootComponentDeletionProgressView,
        RootComponentDrainingAdvanceView, RootComponentDrainingView,
        RootComponentFinalInventoryView, RootComponentInitialInventoryView,
        RootComponentInstallEffectView, RootComponentMembershipRemovedView,
        RootComponentQuiescenceProgressView, RootComponentQuiescenceStopIntentView,
        RootComponentRegistryView, RootComponentSubtreeDeleteEffectView,
        RootComponentSubtreeDirectoryConvergenceView,
        RootComponentSubtreeDirectorySynchronizedView, RootComponentSubtreeMembershipRemovedView,
        RootComponentSubtreeRemovalProgressView, RootComponentSubtreeRemovalView,
        RootComponentSubtreeStopEffectView, RootComponentSubtreeStoppedEffectView,
    },
    workflow::{
        bootstrap::root_store, deployment,
        root_authority::validated_root_authority as root_authority,
        runtime::template::resolved_root_store_module_source,
    },
};
use candid::CandidType;
use canic_core::api::{runtime::install::ApprovedModuleSource, timer::TimerApi};
use canic_core::{
    control_plane_support::{
        config::schema::ComponentChildKind,
        error::InternalError,
        ops::{
            component_runtime::ComponentRuntimeOps,
            config::ConfigOps,
            ic::{
                IcOps,
                call::CallOps,
                mgmt::{CanisterStatus, CanisterStatusObservation, CanisterStatusType, MgmtOps},
            },
        },
        policy::{
            component_allocation::{
                PeerComponentProvisioningInput, PeerComponentProvisioningReadiness,
                TopLevelComponentAllocationDecision, TopLevelComponentAllocationInput,
                authorize_peer_component_provisioning, reserve_top_level_component,
            },
            component_child_allocation::{
                ComponentChildAllocationInput, ComponentChildAllocationReadiness,
                ComponentRegistryVersionEvidence, reserve_component_child,
            },
        },
        workflow::{
            cost_guard::CostGuardWorkflow,
            runtime::{fleet_activation::FleetActivationWorkflow, install::ModuleInstallWorkflow},
        },
    },
    dto::{
        abi::v1::{CanisterInitAuthority, CanisterInitPayload},
        component_deployment::{ComponentDeploymentLimits, ProtectedComponentDeployment},
        component_provisioning::ComponentGroupDirectory,
        component_registry::{
            ComponentDirectoryChildEntry, ComponentDirectoryHead, ComponentDirectoryHeadRequest,
            ComponentDirectoryPageCursor, ComponentDirectoryPageRequest,
            ComponentDirectoryPageResponse, ComponentDirectoryProvenance, ComponentLifecycleStatus,
            ComponentProvisioningOrigin, ComponentRegistryHead, ComponentRegistryPartitionRequest,
            ComponentRegistryPartitionResponse, ComponentRuntimeActivationEvidence,
            ComponentRuntimeActivationRequest, ComponentRuntimeDirectChild,
            ComponentRuntimeDirectoryAuthority, ComponentRuntimeDirectoryConvergenceEvidence,
            ComponentRuntimeDirectoryPreparationRequest,
            ComponentRuntimeDirectorySynchronizationRequest, ComponentRuntimePhase,
            ComponentRuntimeStatusResponse, FleetServiceComponentRequester, PeerComponentRequester,
            RootComponentAllocationPhase, RootComponentAllocationRequest,
            RootComponentAllocationResponse, RootComponentAllocationStatusRequest,
            RootComponentChildAllocationRequest, RootComponentChildAllocationResponse,
            RootComponentChildAllocationStatusRequest, RootComponentChildCommitRequest,
            RootComponentChildCommitResponse, RootComponentChildCreationRequest,
            RootComponentChildDirectoryPreparationRequest,
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
            RootPeerComponentAllocationRequest,
        },
        error::Error,
        fleet_activation::FleetActivationPhase,
        fleet_registry::{FleetDirectorySnapshot, FleetRegistryVersion, FleetSubnetRootStatus},
        role::{ComponentRuntimeOperationStatus, OperationReceipt, OperationStatusRequest},
        root_store::{RootStoreBootstrapRequest, RootStoreBootstrapResponse},
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentInstanceId, FleetSubnetRootBinding,
        FleetSubnetRootReleaseSet, ManagedCanisterBinding,
    },
    protocol,
};
use serde::Deserialize;
use std::time::Duration;

const MAX_COMPONENT_DIRECTORY_PAGE_ENTRIES: u16 = 100;
const MAX_COMPONENT_DIRECTORY_CURSOR_BYTES: usize = 2_048;

#[derive(CandidType)]
enum CanisterCommandFragment {
    ConfigureRuntime(ComponentRuntimeDirectoryPreparationRequest),
}

#[derive(CandidType, Deserialize)]
enum CanisterCommandResponseFragment {
    OperationAccepted(OperationReceipt),
}

#[derive(CandidType)]
enum CanisterStatusRequestFragment {
    Binding,
    Operation(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum CanisterStatusResponseFragment {
    Binding(Box<ManagedCanisterBinding>),
    Operation(Box<CanisterOperationStatusFragment>),
}

#[derive(CandidType, Deserialize)]
enum CanisterOperationStatusFragment {
    ConfigureRuntime(ComponentRuntimeOperationStatus),
}

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

#[derive(Debug, Eq, PartialEq)]
struct ComponentRuntimeDirectoryStatusIdentity<'a> {
    authority: Option<&'a ComponentRuntimeDirectoryAuthority>,
    authority_hash: Option<[u8; 32]>,
    direct_children_hash: Option<[u8; 32]>,
}

impl<'a> ComponentRuntimeDirectoryStatusIdentity<'a> {
    const fn from_status(status: &'a ComponentRuntimeStatusResponse) -> Self {
        Self {
            authority: status.authority.as_ref(),
            authority_hash: status.authority_hash,
            direct_children_hash: status.direct_children_hash,
        }
    }

    const fn exact(
        authority: &'a ComponentRuntimeDirectoryAuthority,
        authority_hash: [u8; 32],
        direct_children_hash: [u8; 32],
    ) -> Self {
        Self {
            authority: Some(authority),
            authority_hash: Some(authority_hash),
            direct_children_hash: Some(direct_children_hash),
        }
    }

    const fn empty() -> Self {
        Self {
            authority: None,
            authority_hash: None,
            direct_children_hash: None,
        }
    }

    const fn is_complete(&self) -> bool {
        matches!(
            (
                self.authority,
                self.authority_hash,
                self.direct_children_hash
            ),
            (Some(_), Some(_), Some(_))
        )
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ComponentDirectoryOwnership<'a> {
    component: &'a ComponentBinding,
    source_fleet_subnet_root: &'a candid::Principal,
}

impl<'a> ComponentDirectoryOwnership<'a> {
    const fn from_binding(component: &'a ComponentBinding) -> Self {
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
    deployment: ProtectedComponentDeployment,
    directory_request: ComponentRuntimeDirectoryPreparationRequest,
    directory_authority_hash: [u8; 32],
    maximum_component_registry_bytes: u64,
}

#[derive(Clone, Copy)]
struct GroupComponentRuntimeAuthority<'a> {
    provisioning_origin: &'a ComponentProvisioningOrigin,
    deployment: &'a ProtectedComponentDeployment,
    component_group: &'a ComponentGroupDirectory,
}

#[derive(Clone, Copy)]
enum ComponentRuntimePlanAuthority<'a> {
    Caller,
    Reconciler,
    Group(GroupComponentRuntimeAuthority<'a>),
}

struct PreparedChildRuntimePlan {
    root_binding: canic_core::ids::FleetSubnetRootBinding,
    allocation: RootComponentChildAllocationView,
    committed_partition: ComponentRegistryPartitionView,
    child_canister: candid::Principal,
    child_binding: ManagedCanisterBinding,
    deployment: ProtectedComponentDeployment,
    owning_component_binding: ManagedCanisterBinding,
    requesting_parent_binding: ManagedCanisterBinding,
    parent_binding: Option<ManagedCanisterBinding>,
    directory_request: ComponentRuntimeDirectoryPreparationRequest,
    directory_authority_hash: [u8; 32],
}

struct ActivatedChildMembership {
    partition: ComponentRegistryPartitionView,
    synchronization_request: ComponentRuntimeDirectorySynchronizationRequest,
    authority_hash: [u8; 32],
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
        return Err(InternalError::conflict());
    }
    CanisterPoolOps::complete_recycling(
        receipt.deleted.deletion.quiescence.stop.canister_id,
        request.component,
        IcOps::now_nanos(),
    )?;
    component_deletion_response(draining).map(Some)
}

enum ComponentSubtreeRemovalAction {
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

    let prepared = ComponentRegistryOps::current().ok_or_else(InternalError::unavailable)?;
    let expected = ComponentRegistryPreparationAuthority::new(
        &authority.binding,
        &request.expected_fleet_registry,
        authority.initial_release_set,
        &request.store_bootstrap,
    );
    if ComponentRegistryPreparationAuthority::from_registry(&prepared) != expected {
        return Err(InternalError::conflict());
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
            return Err(InternalError::conflict());
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

    crate::ops::component_provisioning::RootComponentProvisioningOps::
        require_ordinary_allocation_open()?;
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
        FleetActivationWorkflow::status()?.phase == FleetActivationPhase::Active,
    )?;
    allocation_response(reserved)
}

/// Durably reserve one peer Component for an exact local or remote requester caller.
pub async fn reserve_peer_allocation(
    request: RootPeerComponentAllocationRequest,
) -> Result<RootComponentAllocationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;

    let topology = ConfigOps::component_topology()?;
    if let Some(existing) = ComponentRegistryOps::allocation(request.operation_id) {
        revalidate_peer_provisioning_origin(
            &authority,
            &topology,
            &request.requester,
            &existing.provisioning_origin,
            IcOps::msg_caller(),
        )?;
        return replay_peer_allocation(
            &authority.binding,
            authority.initial_release_set,
            &topology,
            &request,
            existing,
        );
    }

    let requester = peer_requester_authority(
        &authority,
        authority.initial_release_set,
        &topology,
        &request.requester,
        IcOps::msg_caller(),
    )?;
    crate::ops::component_provisioning::RootComponentProvisioningOps::
        require_ordinary_allocation_open()?;
    ComponentRegistryOps::require_top_level_allocation_open()?;
    let provisioning_origin = authorize_new_peer_allocation(
        &authority.binding,
        &topology,
        &requester,
        &request.component_spec,
    )?;
    let allocation_request = RootComponentAllocationRequest {
        operation_id: request.operation_id,
        component_spec: request.component_spec,
    };
    let decision = top_level_allocation_decision(
        &authority.binding,
        &topology,
        &prepared,
        &allocation_request,
    )?;
    let reserved = ComponentRegistryOps::reserve_allocation(
        decision,
        allocation_request.operation_id,
        provisioning_origin,
        true,
    )?;
    allocation_response(reserved)
}

/// Authorize the exact local or Fleet-service peer caller before endpoint dispatch.
pub fn authorize_peer_allocation_caller(
    request: &RootPeerComponentAllocationRequest,
    caller: candid::Principal,
) -> Result<(), InternalError> {
    let (authority, _) = root_authority()?;
    let topology = ConfigOps::component_topology()?;
    peer_requester_authority(
        &authority,
        authority.initial_release_set,
        &topology,
        &request.requester,
        caller,
    )?;
    Ok(())
}

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
            advance_install(operation_id, allocation, plan).await?;
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
                activate_component_runtime_with_plan(
                    RootComponentRuntimeActivationRequest { operation_id },
                    plan,
                )
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

fn component_allocation_reconciliation_complete(allocation: &RootComponentAllocationView) -> bool {
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
                Ok(true) => {}
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

enum PeerRequesterAuthority {
    SameRoot {
        partition: ComponentRegistryPartitionView,
        root: FleetSubnetRootBinding,
    },
    FleetService {
        requester: FleetServiceComponentRequester,
        registry: FleetRegistryVersion,
        root: FleetSubnetRootBinding,
    },
}

impl PeerRequesterAuthority {
    const fn binding(&self) -> &ComponentBinding {
        match self {
            Self::SameRoot { partition, .. } => &partition.binding,
            Self::FleetService { requester, .. } => &requester.component,
        }
    }

    const fn root(&self) -> &FleetSubnetRootBinding {
        match self {
            Self::SameRoot { root, .. } | Self::FleetService { root, .. } => root,
        }
    }

    const fn requester_is_active(&self) -> bool {
        match self {
            Self::SameRoot { partition, .. } => {
                matches!(partition.status, ComponentLifecycleStatus::Active)
            }
            Self::FleetService { .. } => true,
        }
    }

    fn origin(
        &self,
        grant: canic_core::control_plane_support::config::ComponentProvisioningGrant,
    ) -> ComponentProvisioningOrigin {
        match self {
            Self::SameRoot { partition, .. } => ComponentProvisioningOrigin::Component {
                requester: Box::new(partition.binding.clone()),
                grant: Box::new(grant),
            },
            Self::FleetService {
                requester,
                registry,
                ..
            } => ComponentProvisioningOrigin::FleetServiceComponent {
                requester: Box::new(requester.clone()),
                registry: Box::new(registry.clone()),
                grant: Box::new(grant),
            },
        }
    }
}

fn peer_requester_authority(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    release_set: FleetSubnetRootReleaseSet,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    requester: &PeerComponentRequester,
    caller: candid::Principal,
) -> Result<PeerRequesterAuthority, InternalError> {
    match requester {
        PeerComponentRequester::SameRoot => Ok(PeerRequesterAuthority::SameRoot {
            partition: peer_component_requester(&authority.binding, release_set, topology, caller)?,
            root: authority.binding.clone(),
        }),
        PeerComponentRequester::FleetService {
            service,
            expected_registry,
        } => {
            let mirror = FleetRegistryMirrorOps::validated_current(
                authority,
                authority.binding.fleet_subnet_root,
            )?;
            let resolved = FleetServicePeerOps::resolve(
                &authority.binding,
                topology,
                &mirror,
                caller,
                service,
            )?;
            if &mirror.active.snapshot.version != expected_registry.as_ref() {
                return Err(InternalError::conflict());
            }
            Ok(PeerRequesterAuthority::FleetService {
                requester: resolved.requester,
                registry: expected_registry.as_ref().clone(),
                root: resolved.root,
            })
        }
    }
}

fn peer_component_requester(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    caller: candid::Principal,
) -> Result<ComponentRegistryPartitionView, InternalError> {
    let requester_component =
        ComponentRegistryOps::component_for_principal(caller).ok_or_else(|| {
            InternalError::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
        })?;
    let requester = ComponentRegistryOps::partition(requester_component)?
        .ok_or_else(InternalError::invariant)?;
    if requester.binding.canister_id != caller {
        return Err(InternalError::public(
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    validate_partition(root, release_set, topology, &requester)?;
    Ok(requester)
}

fn replay_peer_allocation(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    request: &RootPeerComponentAllocationRequest,
    existing: RootComponentAllocationView,
) -> Result<RootComponentAllocationResponse, InternalError> {
    if existing.component_spec != request.component_spec {
        return Err(InternalError::conflict());
    }
    validate_allocation_record(root, release_set, topology, &existing, request.operation_id)?;
    allocation_response(existing)
}

fn authorize_new_peer_allocation(
    target_root: &canic_core::ids::FleetSubnetRootBinding,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    requester: &PeerRequesterAuthority,
    target_component_spec: &canic_core::ids::ComponentSpecId,
) -> Result<ComponentProvisioningOrigin, InternalError> {
    let readiness = match (
        FleetActivationWorkflow::status()?.phase,
        requester.requester_is_active(),
    ) {
        (FleetActivationPhase::Active, true) => PeerComponentProvisioningReadiness::Ready,
        (FleetActivationPhase::Active, false) => {
            PeerComponentProvisioningReadiness::RequesterRegistryMemberInactive
        }
        _ => PeerComponentProvisioningReadiness::RootRuntimeInactive,
    };
    let counts =
        ComponentRegistryOps::peer_component_counts(requester.binding(), target_component_spec)?;
    let grant = authorize_peer_component_provisioning(PeerComponentProvisioningInput {
        requester: requester.binding(),
        requester_root: requester.root(),
        target_component_spec,
        target_root,
        topology,
        readiness,
        reserved_peer_instances: counts.reserved,
        committed_peer_instances: counts.committed,
    })
    .map_err(InternalError::from)?;
    Ok(requester.origin(grant))
}

pub(super) fn top_level_allocation_decision(
    root: &canic_core::ids::FleetSubnetRootBinding,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    prepared: &RootComponentRegistryView,
    request: &RootComponentAllocationRequest,
) -> Result<TopLevelComponentAllocationDecision, InternalError> {
    let counts = ComponentRegistryOps::component_spec_counts(&request.component_spec)?;
    reserve_top_level_component(TopLevelComponentAllocationInput {
        operation_id: request.operation_id,
        component_spec: &request.component_spec,
        root,
        topology,
        next_allocation_sequence: prepared.next_allocation_sequence,
        reserved_component_instances: prepared.reserved_component_instances,
        committed_component_instances: prepared.committed_component_instances,
        managed_descendants: prepared.managed_descendants,
        reserved_spec_instances: counts.reserved,
        committed_spec_instances: counts.committed,
    })
    .map_err(InternalError::from)
}

/// Read one durable top-level Component allocation reservation without mutation.
pub fn allocation_status(
    request: RootComponentAllocationStatusRequest,
) -> Result<RootComponentAllocationResponse, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let allocation = ComponentRegistryOps::allocation(request.operation_id)
        .ok_or_else(InternalError::unavailable)?;
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

/// Resolve one top-level Component allocation for the role-owned operation status lane.
pub fn allocation_operation_status(
    operation_id: [u8; 32],
    caller: candid::Principal,
    caller_is_controller: bool,
) -> Result<Option<RootComponentOperationStatus>, InternalError> {
    let Some(allocation) = ComponentRegistryOps::allocation(operation_id) else {
        return Ok(None);
    };

    match &allocation.provisioning_origin {
        ComponentProvisioningOrigin::FleetAdministrator { .. } => {
            if !caller_is_controller {
                return Err(InternalError::forbidden());
            }
        }
        ComponentProvisioningOrigin::Component { .. }
        | ComponentProvisioningOrigin::FleetServiceComponent { .. } => {
            let (authority, _) = root_authority()?;
            require_active_root_runtime(
                "peer Component lifecycle requires an Active Fleet Subnet Root runtime",
            )?;
            revalidate_retained_peer_origin(
                &authority,
                &ConfigOps::component_topology()?,
                &allocation.provisioning_origin,
                caller,
            )?;
        }
        ComponentProvisioningOrigin::ComponentGroup { .. } => return Ok(None),
    }

    let complete = component_allocation_reconciliation_complete(&allocation);
    Ok(Some(RootComponentOperationStatus {
        allocation: allocation_response(allocation)?,
        complete,
    }))
}

/// Read one peer allocation for its exact active requester caller.
pub fn peer_allocation_status(
    request: RootComponentAllocationStatusRequest,
) -> Result<RootComponentAllocationResponse, InternalError> {
    require_active_peer_allocation_caller(request.operation_id)?;
    allocation_status(request)
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
            InternalError::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
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

    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
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
    let fleet_activation = FleetActivationWorkflow::status()?;
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
        .ok_or_else(InternalError::resource_exhausted)?;
    let parent_role_instances = ComponentRegistryOps::parent_role_instances(
        request.component,
        caller,
        &request.child_role,
    )?;
    let deployment_limits = component_deployment_limits(&partition, &topology)?;
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
        deployment_limits: &deployment_limits,
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
        request.application_init_args,
        request.expected_registry,
    )?;
    Ok(child_allocation_response(reserved))
}

/// Authorize the exact registered parent before endpoint dispatch.
pub fn authorize_child_allocation_caller(
    request: &RootComponentChildAllocationRequest,
    caller: candid::Principal,
) -> Result<(), InternalError> {
    ComponentRegistryOps::registered_parent(request.component, caller)?.ok_or_else(|| {
        InternalError::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
    })?;
    Ok(())
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
            InternalError::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
        })?;
    let allocation =
        ComponentRegistryOps::child_allocation(request.component, request.operation_id)?
            .ok_or_else(InternalError::unavailable)?;
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

/// Resolve one direct-child allocation for its controller or exact registered parent.
pub fn child_allocation_operation_status(
    operation_id: [u8; 32],
    caller: candid::Principal,
    caller_is_controller: bool,
) -> Result<Option<RootComponentChildAllocationResponse>, InternalError> {
    let Some(allocation) = ComponentRegistryOps::child_allocation_by_operation(operation_id)?
    else {
        return Ok(None);
    };
    if !caller_is_controller {
        ComponentRegistryOps::registered_parent(allocation.component, caller)?
            .ok_or_else(InternalError::forbidden)?;
        if caller != allocation.parent_canister_id {
            return Err(InternalError::forbidden());
        }
    }
    Ok(Some(child_allocation_response(allocation)))
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
    require_active_root_runtime("Component draining requires an Active Fleet Subnet Root runtime")?;

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(InternalError::invariant)?
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
    let current =
        ComponentRegistryOps::partition(request.component)?.ok_or_else(InternalError::invariant)?;
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
    let draining = ComponentRegistryOps::component_draining(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    if draining.operation_id != request.operation_id {
        return Err(InternalError::conflict());
    }
    let partition =
        ComponentRegistryOps::partition(request.component)?.ok_or_else(InternalError::invariant)?;
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
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
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
    let draining = ComponentRegistryOps::component_draining(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    validate_component_draining(&partition, &draining, None, None)?;
    let operation_matches = request.operation_id == draining.operation_id;
    let registry_matches = request.expected_registry == draining.registry;
    if !operation_matches || !registry_matches {
        return Err(InternalError::conflict());
    }

    let draining = if draining.quiescence.is_none() {
        let component_authority =
            current_component_directory_authority(&partition, fleet_directory)?;
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
    let current =
        ComponentRegistryOps::partition(request.component)?.ok_or_else(InternalError::invariant)?;
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
    let draining = ComponentRegistryOps::component_draining(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    if request.operation_id != draining.operation_id {
        return Err(InternalError::conflict());
    }
    let partition =
        ComponentRegistryOps::partition(request.component)?.ok_or_else(InternalError::invariant)?;
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
            let removal = Box::pin(advance_subtree_removal_phase(*removal)).await?;
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

/// Delete one qualified top-level workload and retain its physical Canister in the local pool.
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
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
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

    observe_or_recycle_component(&plan).await?;
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
    let recycling_canister = component_recycling_canister(&request)?;
    CanisterPoolOps::validate_complete_recycling(recycling_canister, request.component)?;
    let removed = ComponentRegistryOps::remove_component_membership(
        request.component,
        request.operation_id,
        request.expected_inventory_hash,
        IcOps::now_nanos(),
    )?;
    let canister_id = removed
        .deletion
        .as_ref()
        .and_then(|progress| match progress {
            RootComponentDeletionProgressView::MembershipRemoved(receipt) => {
                Some(receipt.deleted.deletion.quiescence.stop.canister_id)
            }
            RootComponentDeletionProgressView::DeleteIntent(_)
            | RootComponentDeletionProgressView::Deleted(_) => None,
        })
        .ok_or_else(InternalError::invariant)?;
    CanisterPoolOps::complete_recycling(canister_id, request.component, IcOps::now_nanos())?;
    component_deletion_response(removed)
}

fn component_recycling_canister(
    request: &RootComponentDeletionRequest,
) -> Result<candid::Principal, InternalError> {
    let draining = ComponentRegistryOps::component_draining(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    if draining.operation_id != request.operation_id {
        return Err(InternalError::conflict());
    }
    let deletion = draining.deletion.ok_or_else(InternalError::unavailable)?;
    match deletion {
        RootComponentDeletionProgressView::Deleted(receipt) => {
            Ok(receipt.deletion.quiescence.stop.canister_id)
        }
        RootComponentDeletionProgressView::MembershipRemoved(receipt) => {
            Ok(receipt.deleted.deletion.quiescence.stop.canister_id)
        }
        RootComponentDeletionProgressView::DeleteIntent(_) => Err(InternalError::unavailable()),
    }
}

/// Read one finalized Component's durable top-level deletion progress without mutation.
pub fn component_deletion_status(
    request: RootComponentDeletionStatusRequest,
) -> Result<RootComponentDeletionResponse, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let draining = ComponentRegistryOps::component_draining(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    if request.operation_id != draining.operation_id {
        return Err(InternalError::conflict());
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
        return Err(InternalError::invariant());
    }
    component_deletion_response(draining)
}

/// Resolve one top-level Component removal from its first durable draining fence onward.
pub fn component_removal_operation_status(
    operation_id: [u8; 32],
) -> Result<
    Option<(
        RootComponentDrainingResponse,
        Option<RootComponentDeletionResponse>,
    )>,
    InternalError,
> {
    let Some(draining) = ComponentRegistryOps::component_draining_by_operation(operation_id)?
    else {
        return Ok(None);
    };
    let deletion = if draining.deletion.is_some() {
        Some(component_deletion_response(draining.clone())?)
    } else {
        None
    };
    Ok(Some((component_draining_response(draining), deletion)))
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

pub(super) async fn advance_component_removal_once(
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

async fn advance_subtree_removal_phase(
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
    require_active_root_runtime(
        "Component subtree removal requires an Active Fleet Subnet Root runtime",
    )?;

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
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(InternalError::invariant)?
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
    require_active_root_runtime(
        "Component subtree traversal requires an Active Fleet Subnet Root runtime",
    )?;

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(InternalError::invariant)?
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
    require_active_root_runtime(
        "Component subtree stop preparation requires an Active Fleet Subnet Root runtime",
    )?;

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(InternalError::invariant)?
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
    require_active_root_runtime(
        "Component subtree stop requires an Active Fleet Subnet Root runtime",
    )?;

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
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
    let removal = ComponentRegistryOps::subtree_removal(request.component, request.operation_id)?
        .ok_or_else(InternalError::unavailable)?;
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
    require_active_root_runtime(
        "Component subtree deletion preparation requires an Active Fleet Subnet Root runtime",
    )?;

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(InternalError::invariant)?
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

/// Delete one prepared leaf workload and retain its physical Canister in the local pool.
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
    require_active_root_runtime(
        "Component subtree deletion requires an Active Fleet Subnet Root runtime",
    )?;

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
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
    let removal = ComponentRegistryOps::subtree_removal(request.component, request.operation_id)?
        .ok_or_else(InternalError::unavailable)?;
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
    observe_or_recycle_subtree_leaf(&plan).await?;
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
    require_active_root_runtime(
        "Component subtree membership removal requires an Active Fleet Subnet Root runtime",
    )?;

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(InternalError::invariant)?
        .limits
        .maximum_registry_bytes;
    CanisterPoolOps::validate_complete_recycling(
        request.expected_leaf_canister_id,
        request.component,
    )?;
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
    CanisterPoolOps::complete_recycling(
        request.expected_leaf_canister_id,
        request.component,
        IcOps::now_nanos(),
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
    require_active_root_runtime(
        "Component subtree Directory synchronization requires an Active Fleet Subnet Root runtime",
    )?;

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(InternalError::invariant)?
        .limits
        .maximum_registry_bytes;
    let removal = ComponentRegistryOps::subtree_removal(request.component, request.operation_id)?
        .ok_or_else(InternalError::unavailable)?;
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
            return Err(InternalError::unavailable());
        }
    };
    validate_subtree_directory_request(&removal, membership_removed, &request)?;

    let directory_authority = current_component_directory_authority(&partition, fleet_directory)?;
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
    require_active_root_runtime(
        "Component subtree leaf finalization requires an Active Fleet Subnet Root runtime",
    )?;

    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let maximum_registry_bytes = topology
        .get(&partition.binding.component_spec)
        .ok_or_else(InternalError::invariant)?
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
    existing_subtree_removal(request)?.ok_or_else(InternalError::unavailable)
}

/// Resolve one subtree removal through its domain-owned operation identity.
pub fn subtree_removal_operation_status(
    operation_id: [u8; 32],
) -> Result<Option<RootComponentSubtreeRemovalResponse>, InternalError> {
    ComponentRegistryOps::subtree_removal_by_operation(operation_id)
        .map(|removal| removal.map(subtree_removal_response))
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
pub(super) fn existing_subtree_removal(
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
pub(super) async fn advance_existing_subtree_removal(
    request: RootComponentSubtreeRemovalStatusRequest,
) -> Result<RootComponentSubtreeRemovalResponse, InternalError> {
    let removal = ComponentRegistryOps::subtree_removal(request.component, request.operation_id)?
        .ok_or_else(InternalError::unavailable)?;
    let removal = Box::pin(advance_subtree_removal_phase(removal)).await?;
    Ok(subtree_removal_response(removal))
}

/// Advance one reserved direct child through a root-owned creation effect.
pub async fn create_child_allocation(
    request: RootComponentChildCreationRequest,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    create_child_allocation_for_parent(request, IcOps::msg_caller()).await
}

async fn create_child_allocation_for_parent(
    request: RootComponentChildCreationRequest,
    parent_canister_id: candid::Principal,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;
    require_active_root_runtime(
        "Component Child creation requires an Active Fleet Subnet Root runtime",
    )?;

    let parent = ComponentRegistryOps::registered_parent(request.component, parent_canister_id)?
        .ok_or_else(|| {
            InternalError::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
        })?;
    let allocation =
        ComponentRegistryOps::child_allocation(request.component, request.operation_id)?
            .ok_or_else(InternalError::unavailable)?;
    validate_child_allocation(
        &authority.binding,
        authority.initial_release_set,
        &ConfigOps::component_topology()?,
        &parent.0,
        &allocation,
        None,
    )?;
    let plan = child_creation_plan(root, &store, &allocation)?;
    advance_child_creation(request.component, request.operation_id, allocation, plan)
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
    let allocation = ComponentRegistryOps::allocation(request.operation_id)
        .ok_or_else(InternalError::unavailable)?;
    validate_allocation_caller(&allocation)?;
    validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        request.operation_id,
    )?;
    let plan = creation_plan(root, &store, &allocation)?;

    allocation_response(advance_creation(request.operation_id, allocation, plan)?)
}

/// Reuse the ordinary top-level Component pool-claim journal for one accepted group member.
pub(super) fn advance_group_member_creation(
    root: candid::Principal,
    store: &RootStoreBootstrapResponse,
    allocation: RootComponentAllocationView,
) -> Result<RootComponentAllocationView, InternalError> {
    let operation_id = allocation.operation_id;
    let plan = creation_plan(root, store, &allocation)?;
    advance_creation(operation_id, allocation, plan)
}

/// Reuse the ordinary top-level Component install journal with plan-derived grouped context.
pub(super) async fn advance_group_member_install(
    root: &canic_core::ids::FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    allocation: RootComponentAllocationView,
    deployment: ProtectedComponentDeployment,
) -> Result<RootComponentAllocationView, InternalError> {
    let operation_id = allocation.operation_id;
    let plan =
        component_install_plan_with_deployment(root, store, &allocation, Some(deployment)).await?;
    let _response = advance_install(operation_id, allocation, plan).await?;
    ComponentRegistryOps::allocation(operation_id).ok_or_else(InternalError::invariant)
}

/// Reuse the ordinary top-level Registry commitment with plan-derived grouped limits.
pub(super) async fn advance_group_member_registry_commit(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    root: &canic_core::ids::FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    allocation: RootComponentAllocationView,
    deployment: ProtectedComponentDeployment,
    fleet_directory: FleetDirectorySnapshot,
) -> Result<(RootComponentAllocationView, ComponentRegistryPartitionView), InternalError> {
    let operation_id = allocation.operation_id;
    let plan =
        component_install_plan_with_deployment(root, store, &allocation, Some(deployment)).await?;
    let installation = committed_or_verified_installation(&allocation)?;
    validate_install_effect(installation, &plan.durable)?;
    verify_committed_or_verified_install(&allocation, &plan).await?;
    let mirror = FleetRegistryMirrorOps::validated_current(authority, root.fleet_subnet_root)?;
    if mirror.root_entry.status != FleetSubnetRootStatus::Active
        || mirror.active.directory != fleet_directory
    {
        return Err(InternalError::conflict());
    }
    let (committed, partition) = ComponentRegistryOps::commit_verified(
        operation_id,
        IcOps::now_nanos(),
        plan.durable.maximum_registry_bytes,
        fleet_directory,
    )?;
    validate_partition(
        root,
        allocation.release_set,
        &ConfigOps::component_topology()?,
        &partition,
    )?;
    let _ = commit_response(committed.clone(), partition.clone())?;
    Ok((committed, partition))
}

/// Advance peer Component creation for its exact active requester caller.
pub async fn create_peer_allocation(
    request: RootComponentCreationRequest,
) -> Result<RootComponentAllocationResponse, InternalError> {
    require_active_peer_allocation_caller(request.operation_id)?;
    create_allocation(request).await
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
    let allocation = ComponentRegistryOps::allocation(request.operation_id)
        .ok_or_else(InternalError::unavailable)?;
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

/// Advance peer Component installation for its exact active requester caller.
pub async fn install_peer_allocation(
    request: RootComponentInstallRequest,
) -> Result<RootComponentAllocationResponse, InternalError> {
    require_active_peer_allocation_caller(request.operation_id)?;
    Box::pin(install_allocation(request)).await
}

/// Install and independently verify one exactly created direct child through its root.
pub async fn install_child_allocation(
    request: RootComponentChildInstallRequest,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    Box::pin(install_child_allocation_for_parent(
        request,
        IcOps::msg_caller(),
    ))
    .await
}

async fn install_child_allocation_for_parent(
    request: RootComponentChildInstallRequest,
    parent_canister_id: candid::Principal,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    let store = root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_current_mirror_authority(&authority, root, &preparation_request)?;
    require_active_root_runtime(
        "Component Child installation requires an Active Fleet Subnet Root runtime",
    )?;

    let parent = ComponentRegistryOps::registered_parent(request.component, parent_canister_id)?
        .ok_or_else(|| {
            InternalError::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
        })?;
    let allocation =
        ComponentRegistryOps::child_allocation(request.component, request.operation_id)?
            .ok_or_else(InternalError::unavailable)?;
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
    Box::pin(advance_child_install(
        request.component,
        request.operation_id,
        allocation,
        plan,
    ))
    .await
}

/// Atomically commit one verified direct child and derive the next Component Directory authority.
pub async fn commit_child_allocation(
    request: RootComponentChildCommitRequest,
) -> Result<RootComponentChildCommitResponse, InternalError> {
    commit_child_allocation_for_parent(request, IcOps::msg_caller()).await
}

async fn commit_child_allocation_for_parent(
    request: RootComponentChildCommitRequest,
    parent_canister_id: candid::Principal,
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
    require_active_root_runtime(
        "Component Child commitment requires an Active Fleet Subnet Root runtime",
    )?;

    let parent = ComponentRegistryOps::registered_parent(request.component, parent_canister_id)?
        .ok_or_else(|| {
            InternalError::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
        })?;
    let allocation =
        ComponentRegistryOps::child_allocation(request.component, request.operation_id)?
            .ok_or_else(InternalError::unavailable)?;
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
        plan.component_group.as_ref(),
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
    Box::pin(prepare_child_directories_for_parent(
        request,
        IcOps::msg_caller(),
    ))
    .await
}

async fn prepare_child_directories_for_parent(
    request: RootComponentChildDirectoryPreparationRequest,
    parent_canister_id: candid::Principal,
) -> Result<RootComponentChildDirectoryPreparationResponse, InternalError> {
    let plan =
        prepared_child_runtime_plan(request.component, request.operation_id, parent_canister_id)
            .await?;
    let observed =
        query_component_runtime_status(plan.child_canister, plan.directory_request.operation_id)
            .await?;
    let prepared_child = match validate_target_directory_status_for_deployment(
        &observed,
        &plan.child_binding,
        &plan.deployment,
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
    let _ = prepared_target_directory_status_for_deployment(
        &prepared_child,
        &plan.child_binding,
        &plan.deployment,
        &plan.directory_request,
        plan.directory_authority_hash,
    )?;

    let independently_observed =
        query_component_runtime_status(plan.child_canister, plan.directory_request.operation_id)
            .await?;
    let child = prepared_target_directory_status_for_deployment(
        &independently_observed,
        &plan.child_binding,
        &plan.deployment,
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
    validate_requesting_parent_still_active(
        request.component,
        parent_canister_id,
        &plan.requesting_parent_binding,
    )?;
    let allocation = ComponentRegistryOps::mark_child_directory_prepared(
        request.component,
        request.operation_id,
        plan.directory_authority_hash,
    )?;
    if !committed_child_directory_receipt(&allocation)?.directory_prepared {
        return Err(InternalError::invariant());
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
    activate_child_runtime_for_parent(request, IcOps::msg_caller()).await
}

async fn activate_child_runtime_for_parent(
    request: RootComponentChildRuntimeActivationRequest,
    parent_canister_id: candid::Principal,
) -> Result<RootComponentChildRuntimeActivationResponse, InternalError> {
    let plan =
        prepared_child_runtime_plan(request.component, request.operation_id, parent_canister_id)
            .await?;
    if !committed_child_directory_receipt(&plan.allocation)?.directory_prepared {
        return Err(InternalError::unavailable());
    }

    let child = activate_directory_prepared_runtime_for_deployment(
        plan.child_canister,
        &plan.child_binding,
        &plan.deployment,
        &plan.directory_request,
        plan.directory_authority_hash,
    )
    .await?;
    validate_requesting_parent_still_active(
        request.component,
        parent_canister_id,
        &plan.requesting_parent_binding,
    )?;
    let allocation = ComponentRegistryOps::mark_child_runtime_activated(
        request.component,
        request.operation_id,
        plan.directory_authority_hash,
    )?;
    if !committed_child_directory_receipt(&allocation)?.runtime_activated {
        return Err(InternalError::invariant());
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
    Box::pin(activate_child_membership_for_parent(
        request,
        IcOps::msg_caller(),
    ))
    .await
}

async fn activate_child_membership_for_parent(
    request: RootComponentChildMembershipActivationRequest,
    parent_canister_id: candid::Principal,
) -> Result<RootComponentChildMembershipActivationResponse, InternalError> {
    let plan =
        prepared_child_runtime_plan(request.component, request.operation_id, parent_canister_id)
            .await?;
    if !committed_child_directory_receipt(&plan.allocation)?.runtime_activated {
        return Err(InternalError::unavailable());
    }
    let observed =
        query_component_runtime_status(plan.child_canister, request.operation_id).await?;
    validate_active_target_runtime_status_for_deployment(
        &observed,
        &plan.child_binding,
        &plan.deployment,
        &plan.directory_request,
        plan.directory_authority_hash,
    )?;

    let active = activate_and_validate_child_membership(&plan, request.operation_id)?;

    let child = converge_active_membership_directory_for_deployment(
        plan.child_canister,
        &plan.child_binding,
        &plan.deployment,
        &plan.directory_request,
        plan.directory_authority_hash,
        &active.synchronization_request,
        active.authority_hash,
    )
    .await?;
    converge_active_child_parent_directories(
        &plan,
        &active.synchronization_request.authority,
        active.authority_hash,
    )
    .await?;
    validate_requesting_parent_still_active(
        request.component,
        parent_canister_id,
        &plan.requesting_parent_binding,
    )?;
    let allocation = ComponentRegistryOps::mark_child_membership_synchronized(
        request.component,
        request.operation_id,
        active.authority_hash,
    )?;
    child_membership_response(
        allocation,
        plan.committed_partition,
        active.partition,
        child,
    )
}

fn activate_and_validate_child_membership(
    plan: &PreparedChildRuntimePlan,
    operation_id: [u8; 32],
) -> Result<ActivatedChildMembership, InternalError> {
    let (allocation, partition) = ComponentRegistryOps::activate_child_membership(
        plan.allocation.component,
        operation_id,
        IcOps::now_nanos(),
        plan.directory_request.authority.fleet.clone(),
        plan.directory_request.authority.component_group.as_ref(),
    )?;
    validate_partition(
        &plan.root_binding,
        allocation.release_set,
        &ConfigOps::component_topology()?,
        &partition,
    )?;
    let registered =
        ComponentRegistryOps::registered_parent(plan.allocation.component, plan.child_canister)?
            .ok_or_else(InternalError::invariant)?;
    if registered != (plan.child_binding.clone(), ComponentLifecycleStatus::Active) {
        return Err(InternalError::invariant());
    }

    let synchronization_request = ComponentRuntimeDirectorySynchronizationRequest {
        operation_id,
        authority: ComponentRuntimeDirectoryAuthority {
            fleet: plan.directory_request.authority.fleet.clone(),
            component: component_directory_head(&partition),
            component_group: plan.directory_request.authority.component_group.clone(),
        },
        direct_children: active_component_direct_children(&partition, plan.child_canister)?,
    };
    let authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&synchronization_request.authority)?;
    validate_child_membership_receipt(&allocation, &partition, authority_hash)?;
    Ok(ActivatedChildMembership {
        partition,
        synchronization_request,
        authority_hash,
    })
}

fn validate_child_membership_receipt(
    allocation: &RootComponentChildAllocationView,
    partition: &ComponentRegistryPartitionView,
    authority_hash: [u8; 32],
) -> Result<(), InternalError> {
    let membership = committed_child_directory_receipt(allocation)?
        .membership
        .as_ref()
        .ok_or_else(InternalError::invariant)?;
    let membership_authority =
        ComponentPartitionSnapshotAuthority::from_child_membership(membership);
    let partition_authority = ComponentPartitionSnapshotAuthority::from_partition(partition);
    if membership_authority.state != partition_authority.state
        || membership.directory_authority_hash != authority_hash
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

async fn converge_active_child_parent_directories(
    plan: &PreparedChildRuntimePlan,
    authority: &ComponentRuntimeDirectoryAuthority,
    authority_hash: [u8; 32],
) -> Result<(), InternalError> {
    converge_active_member_directory(&plan.owning_component_binding, authority, authority_hash)
        .await?;
    if let Some(parent_binding) = &plan.parent_binding {
        converge_active_member_directory(parent_binding, authority, authority_hash).await?;
    }
    Ok(())
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
    let allocation = ComponentRegistryOps::allocation(request.operation_id)
        .ok_or_else(InternalError::unavailable)?;
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
    verify_committed_or_verified_install(&allocation, &plan).await?;

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

/// Commit one verified peer Component for its exact active requester caller.
pub async fn commit_peer_allocation(
    request: RootComponentCommitRequest,
) -> Result<RootComponentCommitResponse, InternalError> {
    require_active_peer_allocation_caller(request.operation_id)?;
    commit_allocation(request).await
}

/// Distribute and independently verify exact Directories for one committed Component.
pub async fn prepare_component_directories(
    request: RootComponentDirectoryPreparationRequest,
) -> Result<RootComponentDirectoryPreparationResponse, InternalError> {
    let plan = prepared_component_runtime_plan(request.operation_id).await?;
    prepare_component_directories_with_plan(request, plan).await
}

async fn prepare_component_directories_with_plan(
    request: RootComponentDirectoryPreparationRequest,
    plan: PreparedComponentRuntimePlan,
) -> Result<RootComponentDirectoryPreparationResponse, InternalError> {
    let observed =
        query_component_runtime_status(plan.target_canister, plan.directory_request.operation_id)
            .await?;
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

    let independently_observed =
        query_component_runtime_status(plan.target_canister, plan.directory_request.operation_id)
            .await?;
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
        return Err(InternalError::invariant());
    }

    Ok(RootComponentDirectoryPreparationResponse {
        committed: commit_response(allocation, plan.partition)?,
        target: response_target,
    })
}

/// Deliver and independently verify one grouped runtime configuration intent.
pub(super) async fn prepare_grouped_component_directories(
    canister: candid::Principal,
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
    request: &ComponentRuntimeDirectoryPreparationRequest,
    directory_authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let observed = query_component_runtime_status(canister, request.operation_id).await?;
    let prepared = match validate_target_directory_status_for_deployment(
        &observed,
        binding,
        deployment,
        request,
        directory_authority_hash,
    )? {
        ComponentRuntimePhase::AwaitingDirectory => {
            prepare_target_component_directories(canister, request.clone()).await?
        }
        ComponentRuntimePhase::DirectoryPrepared | ComponentRuntimePhase::Active => observed,
    };
    let _ = prepared_target_directory_status_for_deployment(
        &prepared,
        binding,
        deployment,
        request,
        directory_authority_hash,
    )?;
    let independently_observed =
        query_component_runtime_status(canister, request.operation_id).await?;
    let independently_prepared = prepared_target_directory_status_for_deployment(
        &independently_observed,
        binding,
        deployment,
        request,
        directory_authority_hash,
    )?;
    Ok(independently_prepared)
}

/// Synchronize one already-Active grouped Component to a current Directory covering the request.
pub(super) async fn synchronize_grouped_component_directory(
    canister: candid::Principal,
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
    request: &ComponentRuntimeDirectorySynchronizationRequest,
    directory_authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let observed = query_component_runtime_status(canister, request.operation_id).await?;
    let activation = validate_active_directory_refresh_identity(
        &observed,
        binding,
        deployment,
        request.operation_id,
    )?;
    let synchronized =
        if active_directory_refresh_covers(&observed, request, directory_authority_hash)? {
            observed
        } else {
            match synchronize_target_component_directory(canister, request.clone()).await {
                Ok(status) => status,
                Err(call_error) => {
                    let reconciled =
                        query_component_runtime_status(canister, request.operation_id).await?;
                    if active_directory_refresh_covers(
                        &reconciled,
                        request,
                        directory_authority_hash,
                    )? {
                        reconciled
                    } else {
                        return Err(call_error);
                    }
                }
            }
        };
    let synchronized_activation = validate_active_directory_refresh_identity(
        &synchronized,
        binding,
        deployment,
        request.operation_id,
    )?;
    if synchronized_activation != activation
        || !active_directory_refresh_covers(&synchronized, request, directory_authority_hash)?
    {
        return Err(InternalError::conflict());
    }
    let independently_observed =
        query_component_runtime_status(canister, request.operation_id).await?;
    let independently_observed_activation = validate_active_directory_refresh_identity(
        &independently_observed,
        binding,
        deployment,
        request.operation_id,
    )?;
    if independently_observed_activation != activation
        || !active_directory_refresh_covers(
            &independently_observed,
            request,
            directory_authority_hash,
        )?
    {
        return Err(InternalError::unavailable());
    }
    Ok(independently_observed)
}

/// Prepare one peer Component's Directories for its exact active requester caller.
pub async fn prepare_peer_component_directories(
    request: RootComponentDirectoryPreparationRequest,
) -> Result<RootComponentDirectoryPreparationResponse, InternalError> {
    require_active_peer_allocation_caller(request.operation_id)?;
    Box::pin(prepare_component_directories(request)).await
}

/// Activate and independently verify one exact Directory-prepared Component runtime.
pub async fn activate_component_runtime(
    request: RootComponentRuntimeActivationRequest,
) -> Result<RootComponentRuntimeActivationResponse, InternalError> {
    let plan = prepared_component_runtime_plan(request.operation_id).await?;
    activate_component_runtime_with_plan(request, plan).await
}

/// Activate one grouped Component only through its exact aggregate authority.
pub(super) async fn activate_group_member_runtime(
    request: RootComponentRuntimeActivationRequest,
    provisioning_origin: &ComponentProvisioningOrigin,
    deployment: &ProtectedComponentDeployment,
    component_group: &ComponentGroupDirectory,
) -> Result<RootComponentRuntimeActivationResponse, InternalError> {
    let plan = prepared_group_component_runtime_plan(
        request.operation_id,
        GroupComponentRuntimeAuthority {
            provisioning_origin,
            deployment,
            component_group,
        },
    )
    .await?;
    activate_component_runtime_with_plan(request, plan).await
}

async fn activate_component_runtime_with_plan(
    request: RootComponentRuntimeActivationRequest,
    plan: PreparedComponentRuntimePlan,
) -> Result<RootComponentRuntimeActivationResponse, InternalError> {
    if !committed_directory_receipt(&plan.allocation)?.directory_prepared {
        return Err(InternalError::unavailable());
    }

    let response_target = activate_directory_prepared_runtime_for_deployment(
        plan.target_canister,
        &plan.target_binding,
        &plan.deployment,
        &plan.directory_request,
        plan.directory_authority_hash,
    )
    .await?;
    let allocation = ComponentRegistryOps::mark_runtime_activated(
        request.operation_id,
        plan.directory_authority_hash,
    )?;
    if !committed_directory_receipt(&allocation)?.runtime_activated {
        return Err(InternalError::invariant());
    }

    Ok(RootComponentRuntimeActivationResponse {
        committed: commit_response(allocation, plan.partition)?,
        target: response_target,
    })
}

/// Activate one peer Component runtime for its exact active requester caller.
pub async fn activate_peer_component_runtime(
    request: RootComponentRuntimeActivationRequest,
) -> Result<RootComponentRuntimeActivationResponse, InternalError> {
    require_active_peer_allocation_caller(request.operation_id)?;
    activate_component_runtime(request).await
}

/// Activate Registry membership and converge one runtime-active Component on its current Directory.
pub async fn activate_component_membership(
    request: RootComponentMembershipActivationRequest,
) -> Result<RootComponentMembershipActivationResponse, InternalError> {
    let plan = prepared_component_runtime_plan(request.operation_id).await?;
    Box::pin(activate_component_membership_with_plan(request, plan)).await
}

/// Activate one grouped Component membership only through its exact aggregate authority.
pub(super) async fn activate_group_member_membership(
    request: RootComponentMembershipActivationRequest,
    provisioning_origin: &ComponentProvisioningOrigin,
    deployment: &ProtectedComponentDeployment,
    component_group: &ComponentGroupDirectory,
) -> Result<RootComponentMembershipActivationResponse, InternalError> {
    let plan = prepared_group_component_runtime_plan(
        request.operation_id,
        GroupComponentRuntimeAuthority {
            provisioning_origin,
            deployment,
            component_group,
        },
    )
    .await?;
    Box::pin(activate_component_membership_with_plan(request, plan)).await
}

async fn activate_component_membership_with_plan(
    request: RootComponentMembershipActivationRequest,
    plan: PreparedComponentRuntimePlan,
) -> Result<RootComponentMembershipActivationResponse, InternalError> {
    if !committed_directory_receipt(&plan.allocation)?.runtime_activated {
        return Err(InternalError::unavailable());
    }
    let observed =
        query_component_runtime_status(plan.target_canister, plan.directory_request.operation_id)
            .await?;
    validate_active_target_runtime_status_for_deployment(
        &observed,
        &plan.target_binding,
        &plan.deployment,
        &plan.directory_request,
        plan.directory_authority_hash,
    )?;

    let fleet_directory = plan.directory_request.authority.fleet.clone();
    let activated = match &plan.directory_request.authority.component_group {
        Some(component_group) => ComponentRegistryOps::activate_group_membership(
            request.operation_id,
            IcOps::now_nanos(),
            plan.maximum_component_registry_bytes,
            fleet_directory,
            component_group,
        ),
        None => ComponentRegistryOps::activate_membership(
            request.operation_id,
            IcOps::now_nanos(),
            plan.maximum_component_registry_bytes,
            fleet_directory,
        ),
    }?;
    let (activated_allocation, active_partition) = activated;
    validate_partition(
        &plan.root_binding,
        activated_allocation.release_set,
        &ConfigOps::component_topology()?,
        &active_partition,
    )?;
    if active_partition.status != ComponentLifecycleStatus::Active {
        return Err(InternalError::invariant());
    }
    let synchronization_request = ComponentRuntimeDirectorySynchronizationRequest {
        operation_id: request.operation_id,
        authority: ComponentRuntimeDirectoryAuthority {
            fleet: plan.directory_request.authority.fleet.clone(),
            component: component_directory_head(&active_partition),
            component_group: plan.directory_request.authority.component_group.clone(),
        },
        direct_children: active_component_direct_children(&active_partition, plan.target_canister)?,
    };
    let active_authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&synchronization_request.authority)?;
    let membership = committed_directory_receipt(&activated_allocation)?
        .membership
        .as_ref()
        .ok_or_else(InternalError::invariant)?;
    if membership.directory_authority_hash != active_authority_hash {
        return Err(InternalError::invariant());
    }

    synchronize_active_membership(
        &plan,
        active_partition,
        synchronization_request,
        active_authority_hash,
    )
    .await
}

/// Activate one peer Component's membership for its exact active requester caller.
pub async fn activate_peer_component_membership(
    request: RootComponentMembershipActivationRequest,
) -> Result<RootComponentMembershipActivationResponse, InternalError> {
    require_active_peer_allocation_caller(request.operation_id)?;
    Box::pin(activate_component_membership(request)).await
}

async fn synchronize_active_membership(
    plan: &PreparedComponentRuntimePlan,
    active_partition: ComponentRegistryPartitionView,
    synchronization_request: ComponentRuntimeDirectorySynchronizationRequest,
    active_authority_hash: [u8; 32],
) -> Result<RootComponentMembershipActivationResponse, InternalError> {
    let target = converge_active_membership_directory_for_deployment(
        plan.target_canister,
        &plan.target_binding,
        &plan.deployment,
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
        return Err(InternalError::conflict());
    }
    let receipt = ComponentRegistryOps::mark_initial_inventory_directories_converged(
        fleet_activation_operation_id,
        sealed.receipt.inventory_hash,
    )?;
    if !receipt.directories_converged {
        return Err(InternalError::invariant());
    }
    Ok(receipt)
}

/// Record the terminal root-runtime receipt after Fleet activation commits.
pub fn mark_root_runtime_activated(
    fleet_activation_operation_id: [u8; 32],
) -> Result<RootComponentInitialInventoryView, InternalError> {
    let receipt = ComponentRegistryOps::initial_inventory(fleet_activation_operation_id)?;
    if !receipt.directories_converged {
        return Err(InternalError::unavailable());
    }
    let terminal = ComponentRegistryOps::mark_initial_inventory_root_runtime_activated(
        fleet_activation_operation_id,
        receipt.inventory_hash,
    )?;
    if !terminal.root_runtime_activated {
        return Err(InternalError::invariant());
    }
    Ok(terminal)
}

#[must_use]
pub fn root_runtime_activation_receipt_complete() -> bool {
    ComponentRegistryOps::current()
        .and_then(|registry| registry.initial_inventory)
        .is_some_and(|receipt| receipt.directories_converged && receipt.root_runtime_activated)
}

/// Resolve one active member from current protected Component Registry authority.
pub fn active_component_member(
    canister: candid::Principal,
) -> Result<ManagedCanisterBinding, ActiveComponentMemberError> {
    Ok(active_component_member_authority(canister)?.binding)
}

///
/// ActiveComponentMemberError
///
/// Distinguishes an ordinary negative membership predicate from a protected
/// Registry or runtime failure that must retain its typed cause.
///

#[derive(Debug)]
pub enum ActiveComponentMemberError {
    Internal(InternalError),
    NotActive,
}

impl From<InternalError> for ActiveComponentMemberError {
    fn from(error: InternalError) -> Self {
        Self::Internal(error)
    }
}

impl From<ActiveComponentMemberError> for InternalError {
    fn from(error: ActiveComponentMemberError) -> Self {
        match error {
            ActiveComponentMemberError::Internal(error) => error,
            ActiveComponentMemberError::NotActive => {
                Self::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
            }
        }
    }
}

#[cfg(test)]
mod active_component_member_error_tests {
    use super::*;

    #[test]
    fn negative_membership_and_registry_failures_remain_distinct() {
        let inactive = InternalError::from(ActiveComponentMemberError::NotActive);
        assert_eq!(
            inactive.public_error().code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );

        let internal = InternalError::platform_failure();
        let recovered = InternalError::from(ActiveComponentMemberError::Internal(internal));
        assert_eq!(
            recovered.code(),
            canic_core::diagnostics::codes::PLATFORM_FAILED
        );
        assert_eq!(
            recovered.public_error().code(),
            canic_core::diagnostics::codes::STATE_FAILED.raw_code()
        );
    }
}

/// Resolve one active member together with its current owning Registry head.
pub fn active_component_member_authority(
    canister: candid::Principal,
) -> Result<ActiveComponentMemberView, ActiveComponentMemberError> {
    let (authority, _) = root_authority()?;
    prepared_registry(&authority.binding, authority.initial_release_set)?;
    let component = ComponentRegistryOps::component_for_principal(canister)
        .ok_or(ActiveComponentMemberError::NotActive)?;
    let partition =
        ComponentRegistryOps::partition(component)?.ok_or_else(InternalError::invariant)?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &ConfigOps::component_topology()?,
        &partition,
    )?;
    let (member, member_status) = ComponentRegistryOps::registered_parent(component, canister)?
        .ok_or_else(InternalError::invariant)?;
    if partition.status != ComponentLifecycleStatus::Active
        || member_status != ComponentLifecycleStatus::Active
    {
        return Err(ActiveComponentMemberError::NotActive);
    }
    Ok(ActiveComponentMemberView {
        binding: member,
        registry: ComponentRegistryHead {
            component: partition.binding.component,
            revision: partition.revision,
            content_hash: partition.content_hash,
        },
    })
}

async fn verify_initial_component_convergence(operation_id: [u8; 32]) -> Result<(), InternalError> {
    let plan = prepared_initial_component_runtime_plan(operation_id).await?;
    let membership = committed_directory_receipt(&plan.allocation)?
        .membership
        .as_ref()
        .ok_or_else(InternalError::unavailable)?;
    if !membership.directory_synchronized {
        return Err(InternalError::unavailable());
    }
    let active_partition = ComponentRegistryOps::partition(plan.allocation.component)?
        .ok_or_else(InternalError::unavailable)?;
    validate_partition(
        &plan.root_binding,
        plan.allocation.release_set,
        &ConfigOps::component_topology()?,
        &active_partition,
    )?;
    if active_partition.status != ComponentLifecycleStatus::Active {
        return Err(InternalError::unavailable());
    }
    let active_request = ComponentRuntimeDirectorySynchronizationRequest {
        operation_id,
        authority: ComponentRuntimeDirectoryAuthority {
            fleet: plan.directory_request.authority.fleet.clone(),
            component: component_directory_head(&active_partition),
            component_group: plan.directory_request.authority.component_group.clone(),
        },
        direct_children: active_component_direct_children(&active_partition, plan.target_canister)?,
    };
    let active_authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&active_request.authority)?;
    if membership.directory_authority_hash != active_authority_hash {
        return Err(InternalError::invariant());
    }
    let observed = query_component_runtime_status(plan.target_canister, operation_id).await?;
    if !validate_target_membership_status_for_deployment(
        &observed,
        &plan.target_binding,
        &plan.deployment,
        &plan.directory_request,
        plan.directory_authority_hash,
        &active_request,
        active_authority_hash,
    )? {
        return Err(InternalError::unavailable());
    }
    Ok(())
}

async fn prepared_initial_component_runtime_plan(
    operation_id: [u8; 32],
) -> Result<PreparedComponentRuntimePlan, InternalError> {
    let allocation =
        ComponentRegistryOps::allocation(operation_id).ok_or_else(InternalError::unavailable)?;
    if !matches!(
        &allocation.provisioning_origin,
        ComponentProvisioningOrigin::ComponentGroup { .. }
    ) {
        return prepared_component_runtime_plan(operation_id).await;
    }
    let retained = RootComponentProvisioningOps::component_group_runtime_authority(&allocation)?;
    prepared_group_component_runtime_plan(
        operation_id,
        GroupComponentRuntimeAuthority {
            provisioning_origin: &allocation.provisioning_origin,
            deployment: &retained.deployment,
            component_group: &retained.component_group,
        },
    )
    .await
}

/// Read one committed Component Registry partition without mutation.
pub fn registry_partition(
    request: ComponentRegistryPartitionRequest,
) -> Result<ComponentRegistryPartitionResponse, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let topology = ConfigOps::component_topology()?;
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
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
    let partition = ComponentRegistryOps::partition(request.component)?
        .ok_or_else(InternalError::unavailable)?;
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
        return Err(InternalError::invalid_input());
    }

    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let topology = ConfigOps::component_topology()?;
    let component = request.directory.provenance.component.component;
    let partition =
        ComponentRegistryOps::partition(component)?.ok_or_else(InternalError::unavailable)?;
    validate_partition(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &partition,
    )?;
    let caller = IcOps::msg_caller();
    let (member, status) =
        ComponentRegistryOps::registered_parent(component, caller)?.ok_or_else(|| {
            InternalError::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
        })?;
    validate_directory_member(&authority.binding, &topology, &partition, &member)?;
    if !component_directory_member_can_read(status) {
        return Err(InternalError::unavailable());
    }

    let directory = component_directory_head(&partition);
    if request.directory != directory {
        return Err(InternalError::conflict());
    }

    if let Some(parent_canister_id) = request.parent_canister_id
        && ComponentRegistryOps::registered_parent(component, parent_canister_id)?.is_none()
    {
        return Err(InternalError::invalid_input());
    }
    if let Some(role) = request.role.as_ref() {
        let spec = topology
            .get(&partition.binding.component_spec)
            .ok_or_else(InternalError::invariant)?;
        if spec.child(role).is_none() {
            return Err(InternalError::invalid_input());
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
                protocol_profile_digest: entry.protocol_profile_digest,
                status: entry.status,
            })
            .collect(),
        next_cursor,
    })
}

fn advance_creation(
    operation_id: [u8; 32],
    allocation: RootComponentAllocationView,
    plan: RootComponentCreationPlan,
) -> Result<RootComponentAllocationView, InternalError> {
    let allocation = reconcile_component_pool_claim(operation_id, allocation)?;
    if reconcile_existing_creation(&allocation, &plan)? {
        return Ok(allocation);
    }

    ComponentRegistryOps::validate_creation_capacity(operation_id, &plan)?;
    let pool_claim = CanisterPoolClaimKey {
        component: allocation.component,
        operation_id,
    };
    if let Some(canister) =
        CanisterPoolOps::claim_oldest_ready(&pool_claim, &plan.initial_cycles, IcOps::now_nanos())?
    {
        return claim_component_pool_asset(operation_id, plan, pool_claim, canister);
    }
    Err(InternalError::resource_exhausted())
}

fn advance_child_creation(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    allocation: RootComponentChildAllocationView,
    plan: RootComponentCreationPlan,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    let allocation = reconcile_component_child_pool_claim(component, operation_id, allocation)?;
    if reconcile_existing_child_creation(&allocation, &plan)? {
        return Ok(child_allocation_response(allocation));
    }

    ComponentRegistryOps::validate_child_creation_capacity(component, operation_id, &plan)?;
    let pool_claim = CanisterPoolClaimKey {
        component,
        operation_id,
    };
    if let Some(canister) =
        CanisterPoolOps::claim_oldest_ready(&pool_claim, &plan.initial_cycles, IcOps::now_nanos())?
    {
        return claim_component_child_pool_asset(
            component,
            operation_id,
            plan,
            pool_claim,
            canister,
        );
    }
    Err(InternalError::resource_exhausted())
}

fn claim_component_pool_asset(
    operation_id: [u8; 32],
    plan: RootComponentCreationPlan,
    claim: CanisterPoolClaimKey,
    canister: candid::Principal,
) -> Result<RootComponentAllocationView, InternalError> {
    let permit = deployment::reserve_component_pool_claim_guard()?;
    let intent = ComponentRegistryOps::begin_creation(
        operation_id,
        plan.clone(),
        permit.replay_settlement(),
    )
    .map_err(|error| CostGuardWorkflow::recover_after_failure(&permit, IcOps::now_secs(), error))?;
    let RootComponentAllocationProgressView::CreationIntent(effect) = &intent.progress else {
        return Err(CostGuardWorkflow::recover_after_failure(
            &permit,
            IcOps::now_secs(),
            InternalError::invariant(),
        ));
    };
    validate_creation_effect(effect, &plan).map_err(|error| {
        CostGuardWorkflow::recover_after_failure(&permit, IcOps::now_secs(), error)
    })?;
    let created = ComponentRegistryOps::mark_created(operation_id, canister).map_err(|error| {
        CostGuardWorkflow::complete_after_failure(&permit, IcOps::now_secs(), error)
    })?;
    CostGuardWorkflow::complete(&permit, IcOps::now_secs())?;
    CanisterPoolOps::finalize_claim(&claim, canister, IcOps::now_nanos())?;
    Ok(created)
}

fn claim_component_child_pool_asset(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    plan: RootComponentCreationPlan,
    claim: CanisterPoolClaimKey,
    canister: candid::Principal,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    let permit = deployment::reserve_component_child_pool_claim_guard()?;
    let intent = ComponentRegistryOps::begin_child_creation(
        component,
        operation_id,
        plan.clone(),
        permit.replay_settlement(),
    )
    .map_err(|error| CostGuardWorkflow::recover_after_failure(&permit, IcOps::now_secs(), error))?;
    let RootComponentChildAllocationProgressView::CreationIntent(effect) = &intent.progress else {
        return Err(CostGuardWorkflow::recover_after_failure(
            &permit,
            IcOps::now_secs(),
            InternalError::invariant(),
        ));
    };
    validate_creation_effect(effect, &plan).map_err(|error| {
        CostGuardWorkflow::recover_after_failure(&permit, IcOps::now_secs(), error)
    })?;
    let created = ComponentRegistryOps::mark_child_created(component, operation_id, canister)
        .map_err(|error| {
            CostGuardWorkflow::complete_after_failure(&permit, IcOps::now_secs(), error)
        })?;
    CostGuardWorkflow::complete(&permit, IcOps::now_secs())?;
    CanisterPoolOps::finalize_claim(&claim, canister, IcOps::now_nanos())?;
    Ok(child_allocation_response(created))
}

fn reconcile_component_pool_claim(
    operation_id: [u8; 32],
    allocation: RootComponentAllocationView,
) -> Result<RootComponentAllocationView, InternalError> {
    let claim = CanisterPoolClaimKey {
        component: allocation.component,
        operation_id,
    };
    let Some(canister) = CanisterPoolOps::claimed_canister(&claim)? else {
        return Ok(allocation);
    };
    let reconciled = match &allocation.progress {
        RootComponentAllocationProgressView::Reserved => return Ok(allocation),
        RootComponentAllocationProgressView::CreationIntent(_) => {
            ComponentRegistryOps::mark_created(operation_id, canister)?
        }
        progress => {
            require_component_progress_canister(progress, canister)?;
            allocation
        }
    };
    CanisterPoolOps::finalize_claim(&claim, canister, IcOps::now_nanos())?;
    Ok(reconciled)
}

fn reconcile_component_child_pool_claim(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    allocation: RootComponentChildAllocationView,
) -> Result<RootComponentChildAllocationView, InternalError> {
    let claim = CanisterPoolClaimKey {
        component,
        operation_id,
    };
    let Some(canister) = CanisterPoolOps::claimed_canister(&claim)? else {
        return Ok(allocation);
    };
    let reconciled = match &allocation.progress {
        RootComponentChildAllocationProgressView::Reserved => return Ok(allocation),
        RootComponentChildAllocationProgressView::CreationIntent(_) => {
            ComponentRegistryOps::mark_child_created(component, operation_id, canister)?
        }
        progress => {
            require_component_child_progress_canister(progress, canister)?;
            allocation
        }
    };
    CanisterPoolOps::finalize_claim(&claim, canister, IcOps::now_nanos())?;
    Ok(reconciled)
}

fn require_component_progress_canister(
    progress: &RootComponentAllocationProgressView,
    expected: candid::Principal,
) -> Result<(), InternalError> {
    let actual = match progress {
        RootComponentAllocationProgressView::Created { canister, .. }
        | RootComponentAllocationProgressView::InstallIntent { canister, .. }
        | RootComponentAllocationProgressView::Installed { canister, .. }
        | RootComponentAllocationProgressView::Verified { canister, .. }
        | RootComponentAllocationProgressView::Committed { canister, .. }
        | RootComponentAllocationProgressView::Removed { canister, .. } => *canister,
        RootComponentAllocationProgressView::Reserved
        | RootComponentAllocationProgressView::CreationIntent(_) => {
            return Err(InternalError::invariant());
        }
    };
    if actual != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn require_component_child_progress_canister(
    progress: &RootComponentChildAllocationProgressView,
    expected: candid::Principal,
) -> Result<(), InternalError> {
    let actual = match progress {
        RootComponentChildAllocationProgressView::Created { canister, .. }
        | RootComponentChildAllocationProgressView::InstallIntent { canister, .. }
        | RootComponentChildAllocationProgressView::Installed { canister, .. }
        | RootComponentChildAllocationProgressView::Verified { canister, .. }
        | RootComponentChildAllocationProgressView::Committed { canister, .. } => *canister,
        RootComponentChildAllocationProgressView::Reserved
        | RootComponentChildAllocationProgressView::CreationIntent(_) => {
            return Err(InternalError::invariant());
        }
    };
    if actual != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
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
    deployment: ProtectedComponentDeployment,
    canister: candid::Principal,
    expected_status_module_hash: [u8; 32],
}

#[derive(Clone, Debug)]
struct ComponentChildInstallPlan {
    durable: RootComponentChildInstallPlan,
    source: ApprovedModuleSource,
    payload: CanisterInitPayload,
    deployment: ProtectedComponentDeployment,
    component_group: Option<ComponentGroupDirectory>,
    canister: candid::Principal,
    expected_status_module_hash: [u8; 32],
    application_init_args: Option<Vec<u8>>,
}

async fn component_install_plan(
    root: &canic_core::ids::FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    allocation: &RootComponentAllocationView,
) -> Result<ComponentInstallPlan, InternalError> {
    component_install_plan_with_deployment(root, store, allocation, None).await
}

async fn component_install_plan_with_deployment(
    root: &canic_core::ids::FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    allocation: &RootComponentAllocationView,
    deployment: Option<ProtectedComponentDeployment>,
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
    if source.source_canister() != &store.wasm_store {
        return Err(InternalError::invariant());
    }
    let chunk_hashes = source.chunk_hashes().to_vec();
    if source.module_hash() != artifact.payload_hash
        || source.payload_size_bytes() != artifact.payload_size_bytes
    {
        return Err(InternalError::invariant());
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
        .map_err(|_error| InternalError::invalid_input())?;
    let spec_maximum_registry_bytes = topology
        .get(&allocation.component_spec)
        .ok_or_else(InternalError::invariant)?
        .limits
        .maximum_registry_bytes;
    let deployment =
        deployment.unwrap_or_else(|| ProtectedComponentDeployment::UngroupedOrdinary {
            binding: binding.clone(),
        });
    ConfigOps::validate_protected_component_deployment(&deployment, &binding)?;
    let maximum_registry_bytes = match &deployment {
        ProtectedComponentDeployment::UngroupedOrdinary { .. } => spec_maximum_registry_bytes,
        ProtectedComponentDeployment::GroupMember { limits, .. } => limits.maximum_registry_bytes,
    };
    let durable = RootComponentInstallPlan {
        raw_module_hash: artifact.raw_module_hash,
        protocol_profile_digest: artifact.protocol_profile_digest,
        chunk_hashes,
        binding: binding.clone(),
        maximum_registry_bytes,
    };
    let payload = CanisterInitPayload {
        install_id: allocation.operation_id,
        release_build_id: allocation.release_set.release_build_id,
        component_deployment: Box::new(deployment.clone()),
        authority: CanisterInitAuthority::Component {
            root: root.clone(),
            binding,
        },
    };

    Ok(ComponentInstallPlan {
        durable,
        source,
        payload,
        deployment,
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
    if source.source_canister() != &store.wasm_store {
        return Err(InternalError::invariant());
    }
    let chunk_hashes = source.chunk_hashes().to_vec();
    if source.module_hash() != artifact.payload_hash
        || source.payload_size_bytes() != artifact.payload_size_bytes
    {
        return Err(InternalError::invariant());
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
        .map_err(|_error| InternalError::invalid_input())?;
    let partition = ComponentRegistryOps::partition(allocation.component)?
        .ok_or_else(InternalError::invariant)?;
    let deployment_authority = RootComponentProvisioningOps::component_deployment_authority(
        &partition.provisioning_origin,
        &binding.component,
    )?;
    let deployment = deployment_authority.deployment;
    ConfigOps::validate_protected_component_deployment(&deployment, &binding.component)?;
    let durable = RootComponentChildInstallPlan {
        raw_module_hash: artifact.raw_module_hash,
        protocol_profile_digest: artifact.protocol_profile_digest,
        chunk_hashes,
        binding: binding.clone(),
        maximum_registry_bytes: allocation.maximum_registry_bytes,
    };
    let payload = CanisterInitPayload {
        install_id: allocation.operation_id,
        release_build_id: allocation.release_set.release_build_id,
        component_deployment: Box::new(deployment.clone()),
        authority: CanisterInitAuthority::ComponentChild {
            root: root.clone(),
            binding,
        },
    };

    Ok(ComponentChildInstallPlan {
        durable,
        source,
        payload,
        deployment,
        component_group: deployment_authority.component_group,
        canister,
        expected_status_module_hash: artifact.payload_hash,
        application_init_args: allocation.application_init_args.clone(),
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
            Err(InternalError::conflict())
        }
        RootComponentChildAllocationProgressView::Created { .. } => {
            if observed_child_install_state(&plan).await? {
                return Err(InternalError::conflict());
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
        plan.canister,
        &plan.source,
        plan.payload.clone(),
        plan.application_init_args.clone(),
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
        return Err(InternalError::invariant());
    }
    Ok(child_allocation_response(verified))
}

async fn observed_child_install_state(
    plan: &ComponentChildInstallPlan,
) -> Result<bool, InternalError> {
    let status = MgmtOps::canister_status(plan.canister).await?;
    if status.settings.controllers != vec![plan.durable.binding.component.fleet_subnet_root] {
        return Err(InternalError::conflict());
    }
    match status.module_hash {
        None => Ok(false),
        Some(module_hash) if module_hash == plan.expected_status_module_hash => Ok(true),
        Some(_) => Err(InternalError::conflict()),
    }
}

async fn verify_installed_child(plan: &ComponentChildInstallPlan) -> Result<(), InternalError> {
    if !observed_child_install_state(plan).await? {
        return Err(InternalError::unavailable());
    }
    let observed = query_managed_binding(plan.canister).await?;
    let expected = ManagedCanisterBinding::ComponentChild(plan.durable.binding.clone());
    if observed != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn removed_allocation_response(
    allocation: RootComponentAllocationView,
    plan: &ComponentInstallPlan,
) -> Result<RootComponentAllocationResponse, InternalError> {
    let RootComponentAllocationProgressView::Removed { installation, .. } = &allocation.progress
    else {
        return Err(InternalError::invariant());
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
        | RootComponentAllocationProgressView::CreationIntent(_) => Err(InternalError::conflict()),
        RootComponentAllocationProgressView::Created { .. } => {
            if observed_install_state(&plan).await? {
                return Err(InternalError::conflict());
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
            verify_committed_or_verified_install(&allocation, &plan).await?;
            allocation_response(allocation)
        }
        RootComponentAllocationProgressView::Removed { .. } => {
            removed_allocation_response(allocation, &plan)
        }
    }
}

async fn verify_committed_or_verified_install(
    allocation: &RootComponentAllocationView,
    plan: &ComponentInstallPlan,
) -> Result<(), InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Verified { .. } => {
            verify_prepared_installed_component(plan).await
        }
        RootComponentAllocationProgressView::Committed { .. } => {
            verify_installed_component(plan).await
        }
        _ => Err(InternalError::invariant()),
    }
}

async fn perform_install(
    operation_id: [u8; 32],
    plan: &ComponentInstallPlan,
    permit: &canic_core::control_plane_support::ops::cost_guard::CostGuardPermit,
) -> Result<RootComponentAllocationResponse, InternalError> {
    if let Err(error) = ModuleInstallWorkflow::install_with_payload_with_permit(
        permit,
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
    verify_prepared_installed_component(plan).await?;
    let verified = ComponentRegistryOps::mark_verified(operation_id)?;
    if !matches!(
        verified.progress,
        RootComponentAllocationProgressView::Verified { .. }
    ) {
        return Err(InternalError::invariant());
    }
    allocation_response(verified)
}

async fn observed_install_state(plan: &ComponentInstallPlan) -> Result<bool, InternalError> {
    let status = MgmtOps::canister_status(plan.canister).await?;
    if status.settings.controllers != vec![plan.durable.binding.fleet_subnet_root] {
        return Err(InternalError::conflict());
    }
    match status.module_hash {
        None => Ok(false),
        Some(module_hash) if module_hash == plan.expected_status_module_hash => Ok(true),
        Some(_) => Err(InternalError::conflict()),
    }
}

async fn installed_component_status(
    plan: &ComponentInstallPlan,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    if !observed_install_state(plan).await? {
        return Err(InternalError::unavailable());
    }
    let observed = query_managed_binding(plan.canister).await?;
    let expected = ManagedCanisterBinding::Component(plan.durable.binding.clone());
    if observed != expected {
        return Err(InternalError::conflict());
    }
    query_component_runtime_status(plan.canister, plan.payload.install_id).await
}

async fn verify_installed_component(plan: &ComponentInstallPlan) -> Result<(), InternalError> {
    let status = installed_component_status(plan).await?;
    validate_installed_component_status(
        &status,
        plan.payload.install_id,
        &ManagedCanisterBinding::Component(plan.durable.binding.clone()),
        &plan.deployment,
    )
}

async fn verify_prepared_installed_component(
    plan: &ComponentInstallPlan,
) -> Result<(), InternalError> {
    let status = installed_component_status(plan).await?;
    validate_prepared_install_status(
        &status,
        plan.payload.install_id,
        &ManagedCanisterBinding::Component(plan.durable.binding.clone()),
        &plan.deployment,
    )
}

fn validate_prepared_install_status(
    status: &ComponentRuntimeStatusResponse,
    operation_id: [u8; 32],
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
) -> Result<(), InternalError> {
    validate_installed_component_status(status, operation_id, binding, deployment)?;
    let directory_is_empty = ComponentRuntimeDirectoryStatusIdentity::from_status(status)
        == ComponentRuntimeDirectoryStatusIdentity::empty();
    let runtime_is_prepared = status.phase == ComponentRuntimePhase::AwaitingDirectory
        && directory_is_empty
        && status.activation.is_none();
    if !runtime_is_prepared {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_installed_component_status(
    status: &ComponentRuntimeStatusResponse,
    operation_id: [u8; 32],
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
) -> Result<(), InternalError> {
    if status.operation_id != operation_id {
        return Err(InternalError::conflict());
    }
    if &status.binding != binding {
        return Err(InternalError::conflict());
    }
    if status.deployment.as_ref() != deployment {
        return Err(InternalError::conflict());
    }
    Ok(())
}

async fn query_managed_binding(
    canister: candid::Principal,
) -> Result<ManagedCanisterBinding, InternalError> {
    let call = CallOps::bounded_wait(canister, protocol::CANIC_STATUS)
        .with_arg(CanisterStatusRequestFragment::Binding)?
        .execute()
        .await
        .map_err(|_error| {
            InternalError::public(canic_core::diagnostics::codes::STATE_UNAVAILABLE)
        })?;
    let result: Result<CanisterStatusResponseFragment, Error> = call
        .candid()
        .map_err(|_error| InternalError::public(canic_core::diagnostics::codes::STATE_INVALID))?;
    match result.map_err(InternalError::observed_public)? {
        CanisterStatusResponseFragment::Binding(binding) => Ok(*binding),
        CanisterStatusResponseFragment::Operation(_) => Err(InternalError::conflict()),
    }
}

async fn prepared_component_runtime_plan(
    operation_id: [u8; 32],
) -> Result<PreparedComponentRuntimePlan, InternalError> {
    prepared_component_runtime_plan_with_authority(
        operation_id,
        ComponentRuntimePlanAuthority::Caller,
    )
    .await
}

async fn prepared_component_runtime_plan_for_reconciliation(
    operation_id: [u8; 32],
) -> Result<PreparedComponentRuntimePlan, InternalError> {
    prepared_component_runtime_plan_with_authority(
        operation_id,
        ComponentRuntimePlanAuthority::Reconciler,
    )
    .await
}

async fn prepared_group_component_runtime_plan(
    operation_id: [u8; 32],
    authority: GroupComponentRuntimeAuthority<'_>,
) -> Result<PreparedComponentRuntimePlan, InternalError> {
    prepared_component_runtime_plan_with_authority(
        operation_id,
        ComponentRuntimePlanAuthority::Group(authority),
    )
    .await
}

async fn prepared_component_runtime_plan_with_authority(
    operation_id: [u8; 32],
    plan_authority: ComponentRuntimePlanAuthority<'_>,
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
    let allocation =
        ComponentRegistryOps::allocation(operation_id).ok_or_else(InternalError::unavailable)?;
    let retained_group_authority = match plan_authority {
        ComponentRuntimePlanAuthority::Caller => {
            validate_allocation_caller(&allocation)?;
            None
        }
        ComponentRuntimePlanAuthority::Reconciler => {
            if matches!(
                allocation.provisioning_origin,
                ComponentProvisioningOrigin::ComponentGroup { .. }
            ) {
                return Err(InternalError::invariant());
            }
            None
        }
        ComponentRuntimePlanAuthority::Group(group_authority) => Some(
            validated_group_component_runtime_authority(&allocation, group_authority)?,
        ),
    };
    validate_allocation_record(
        &root_authority.binding,
        root_authority.initial_release_set,
        &topology,
        &allocation,
        operation_id,
    )?;
    let install = component_install_plan_with_deployment(
        &root_authority.binding,
        &store,
        &allocation,
        retained_group_authority
            .as_ref()
            .map(|authority| authority.deployment.clone()),
    )
    .await?;
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
        return Err(InternalError::invariant());
    }
    let authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory,
        component: component_directory_head(&partition),
        component_group: retained_group_authority.map(|authority| authority.component_group),
    };
    let directory_authority_hash = ComponentRuntimeOps::directory_authority_hash(&authority)?;
    if committed_directory_receipt(&allocation)?.directory_authority_hash
        != directory_authority_hash
    {
        return Err(InternalError::invariant());
    }
    Ok(PreparedComponentRuntimePlan {
        root_binding: root_authority.binding,
        allocation,
        partition: partition.clone(),
        target_canister: install.canister,
        target_binding: ManagedCanisterBinding::Component(install.durable.binding),
        deployment: install.deployment,
        directory_request: ComponentRuntimeDirectoryPreparationRequest {
            operation_id,
            authority,
            direct_children: active_component_direct_children(&partition, install.canister)?,
        },
        directory_authority_hash,
        maximum_component_registry_bytes: install.durable.maximum_registry_bytes,
    })
}

fn validated_group_component_runtime_authority(
    allocation: &RootComponentAllocationView,
    group_authority: GroupComponentRuntimeAuthority<'_>,
) -> Result<
    crate::view::component_provisioning::RootComponentGroupRuntimeAuthorityView,
    InternalError,
> {
    if &allocation.provisioning_origin != group_authority.provisioning_origin {
        return Err(InternalError::conflict());
    }
    let retained = RootComponentProvisioningOps::component_group_runtime_authority(allocation)?;
    let authority_is_exact = retained.deployment == *group_authority.deployment
        && retained.component_group == *group_authority.component_group;
    if !authority_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(retained)
}

async fn prepared_child_runtime_plan(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    parent_canister_id: candid::Principal,
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
    require_active_root_runtime(
        "Component Child lifecycle requires an Active Fleet Subnet Root runtime",
    )?;

    let (parent_binding, parent_status) =
        ComponentRegistryOps::registered_parent(component, parent_canister_id)?.ok_or_else(
            || InternalError::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED),
        )?;
    if parent_status != ComponentLifecycleStatus::Active {
        return Err(InternalError::unavailable());
    }
    let allocation = ComponentRegistryOps::child_allocation(component, operation_id)?
        .ok_or_else(InternalError::unavailable)?;
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

    let (allocation, committed_partition) = ComponentRegistryOps::committed_child_authority(
        component,
        operation_id,
        &fleet_directory,
        install.component_group.as_ref(),
    )?;
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
        install.canister,
        install.component_group.clone(),
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
        deployment: install.deployment,
        owning_component_binding,
        requesting_parent_binding,
        parent_binding,
        directory_request,
        directory_authority_hash,
    })
}

fn validate_requesting_parent_still_active(
    component: canic_core::ids::ComponentInstanceId,
    parent_canister_id: candid::Principal,
    expected: &ManagedCanisterBinding,
) -> Result<(), InternalError> {
    let (current, status) = ComponentRegistryOps::registered_parent(component, parent_canister_id)?
        .ok_or_else(|| {
            InternalError::public(canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
        })?;
    if current != *expected || status != ComponentLifecycleStatus::Active {
        return Err(InternalError::conflict());
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
    let current =
        ComponentRegistryOps::partition(component)?.ok_or_else(InternalError::invariant)?;
    validate_partition(root, release_set, topology, &current)?;
    if current.status != ComponentLifecycleStatus::Active
        || current.binding != committed.binding
        || current.revision < committed.revision
    {
        return Err(InternalError::invariant());
    }
    Ok(current)
}

fn child_directory_request(
    operation_id: [u8; 32],
    fleet: FleetDirectorySnapshot,
    partition: &ComponentRegistryPartitionView,
    allocation: &RootComponentChildAllocationView,
    child_canister: candid::Principal,
    component_group: Option<ComponentGroupDirectory>,
) -> Result<(ComponentRuntimeDirectoryPreparationRequest, [u8; 32]), InternalError> {
    let request = ComponentRuntimeDirectoryPreparationRequest {
        operation_id,
        authority: ComponentRuntimeDirectoryAuthority {
            fleet,
            component: component_directory_head(partition),
            component_group,
        },
        direct_children: active_component_direct_children(partition, child_canister)?,
    };
    let authority_hash = ComponentRuntimeOps::directory_authority_hash(&request.authority)?;
    if committed_child_directory_receipt(allocation)?.directory_authority_hash != authority_hash {
        return Err(InternalError::invariant());
    }
    Ok((request, authority_hash))
}

fn active_component_direct_children_for_authority(
    authority: &ComponentRuntimeDirectoryAuthority,
    parent_canister_id: candid::Principal,
) -> Result<Vec<ComponentRuntimeDirectChild>, InternalError> {
    let component = authority.component.provenance.component.component;
    let partition =
        ComponentRegistryOps::partition(component)?.ok_or_else(InternalError::unavailable)?;
    if component_directory_head(&partition) != authority.component {
        return Err(InternalError::conflict());
    }
    active_component_direct_children(&partition, parent_canister_id)
}

pub(super) fn active_component_direct_children(
    partition: &ComponentRegistryPartitionView,
    parent_canister_id: candid::Principal,
) -> Result<Vec<ComponentRuntimeDirectChild>, InternalError> {
    let scan_limit = usize::try_from(partition.committed_descendants)
        .unwrap_or(usize::MAX)
        .saturating_add(1);
    let page = ComponentRegistryOps::directory_page(
        partition.binding.component,
        &ComponentDirectoryPageSelection {
            parent_canister_id: Some(parent_canister_id),
            role: None,
            status: Some(ComponentLifecycleStatus::Active),
            start_after: None,
        },
        scan_limit,
    )?;
    if page.next_cursor.is_some() {
        return Err(InternalError::invariant());
    }
    let mut direct_children = page
        .entries
        .into_iter()
        .map(|entry| ComponentRuntimeDirectChild {
            canister_id: entry.binding.canister_id,
            role: entry.binding.role,
            protocol_profile_digest: entry.protocol_profile_digest,
        })
        .collect::<Vec<_>>();
    direct_children.sort();
    if direct_children
        .windows(2)
        .any(|pair| pair[0].canister_id == pair[1].canister_id)
    {
        return Err(InternalError::invariant());
    }
    Ok(direct_children)
}

async fn query_component_runtime_status(
    canister: candid::Principal,
    operation_id: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let call = CallOps::bounded_wait(canister, protocol::CANIC_STATUS)
        .with_arg(CanisterStatusRequestFragment::Operation(
            OperationStatusRequest { operation_id },
        ))?
        .execute()
        .await
        .map_err(|_error| {
            InternalError::public(canic_core::diagnostics::codes::STATE_UNAVAILABLE)
        })?;
    let result: Result<CanisterStatusResponseFragment, Error> = call
        .candid()
        .map_err(|_error| InternalError::public(canic_core::diagnostics::codes::STATE_INVALID))?;
    match result.map_err(InternalError::observed_public)? {
        CanisterStatusResponseFragment::Operation(operation) => {
            let CanisterOperationStatusFragment::ConfigureRuntime(status) = *operation;
            if status.operation_id == operation_id {
                Ok(status.runtime)
            } else {
                Err(InternalError::conflict())
            }
        }
        CanisterStatusResponseFragment::Binding(_) => Err(InternalError::conflict()),
    }
}

async fn activate_target_component_runtime(
    canister: candid::Principal,
    request: ComponentRuntimeActivationRequest,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let status = query_component_runtime_status(canister, request.operation_id).await?;
    if status.authority_hash != Some(request.directory_authority_hash) {
        return Err(InternalError::conflict());
    }
    Ok(status)
}

async fn activate_directory_prepared_runtime_for_deployment(
    canister: candid::Principal,
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
    directory_request: &ComponentRuntimeDirectoryPreparationRequest,
    directory_authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let observed = query_component_runtime_status(canister, directory_request.operation_id).await?;
    let activated = match validate_target_directory_status_for_deployment(
        &observed,
        binding,
        deployment,
        directory_request,
        directory_authority_hash,
    )? {
        ComponentRuntimePhase::AwaitingDirectory => {
            return Err(InternalError::unavailable());
        }
        ComponentRuntimePhase::DirectoryPrepared => {
            let request = ComponentRuntimeActivationRequest {
                operation_id: directory_request.operation_id,
                directory_authority_hash,
            };
            match activate_target_component_runtime(canister, request).await {
                Ok(status) => status,
                Err(call_error) => {
                    let reconciled =
                        query_component_runtime_status(canister, directory_request.operation_id)
                            .await?;
                    if validate_active_target_runtime_status_for_deployment(
                        &reconciled,
                        binding,
                        deployment,
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
    validate_active_target_runtime_status_for_deployment(
        &activated,
        binding,
        deployment,
        directory_request,
        directory_authority_hash,
    )?;

    let independently_observed =
        query_component_runtime_status(canister, directory_request.operation_id).await?;
    active_target_runtime_status_for_deployment(
        &independently_observed,
        binding,
        deployment,
        directory_request,
        directory_authority_hash,
    )
}

async fn converge_active_membership_directory_for_deployment(
    canister: candid::Principal,
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
    prepared_request: &ComponentRuntimeDirectoryPreparationRequest,
    prepared_authority_hash: [u8; 32],
    active_request: &ComponentRuntimeDirectorySynchronizationRequest,
    active_authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let observed = query_component_runtime_status(canister, prepared_request.operation_id).await?;
    let synchronized = if validate_target_membership_status_for_deployment(
        &observed,
        binding,
        deployment,
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
                let reconciled =
                    query_component_runtime_status(canister, prepared_request.operation_id).await?;
                if matches!(
                    validate_target_membership_status_for_deployment(
                        &reconciled,
                        binding,
                        deployment,
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
    if !validate_target_membership_status_for_deployment(
        &synchronized,
        binding,
        deployment,
        prepared_request,
        prepared_authority_hash,
        active_request,
        active_authority_hash,
    )? {
        return Err(InternalError::unavailable());
    }

    let independently_observed =
        query_component_runtime_status(canister, prepared_request.operation_id).await?;
    active_membership_target_status_for_deployment(
        &independently_observed,
        binding,
        deployment,
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
    configure_target_component_runtime(
        canister,
        ComponentRuntimeDirectoryPreparationRequest {
            operation_id: request.operation_id,
            authority: request.authority,
            direct_children: request.direct_children,
        },
    )
    .await
}

async fn prepare_target_component_directories(
    canister: candid::Principal,
    request: ComponentRuntimeDirectoryPreparationRequest,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    configure_target_component_runtime(canister, request).await
}

async fn configure_target_component_runtime(
    canister: candid::Principal,
    request: ComponentRuntimeDirectoryPreparationRequest,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let operation_id = request.operation_id;
    let call = CallOps::bounded_wait(canister, protocol::CANIC_COMMAND)
        .with_arg(CanisterCommandFragment::ConfigureRuntime(request))?
        .execute()
        .await
        .map_err(|_error| {
            InternalError::public(canic_core::diagnostics::codes::STATE_UNAVAILABLE)
        })?;
    let result: Result<CanisterCommandResponseFragment, Error> = call
        .candid()
        .map_err(|_error| InternalError::public(canic_core::diagnostics::codes::STATE_INVALID))?;
    let CanisterCommandResponseFragment::OperationAccepted(receipt) =
        result.map_err(InternalError::observed_public)?;
    if receipt.operation_id != operation_id {
        return Err(InternalError::conflict());
    }
    query_component_runtime_status(canister, operation_id).await
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
                .ok_or_else(InternalError::invariant)?;
            if !matches!(
                draining.quiescence,
                Some(RootComponentQuiescenceProgressView::Quiescent(_))
            ) {
                return Err(InternalError::unavailable());
            }
            None
        }
        ComponentLifecycleStatus::Prepared | ComponentLifecycleStatus::Removed => {
            return Err(InternalError::conflict());
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
    .ok_or_else(InternalError::invariant)?;
    if status != ComponentLifecycleStatus::Active {
        return Err(InternalError::conflict());
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
    let operation_id = ComponentRegistryOps::managed_runtime_operation_id(binding)?;
    let direct_children = active_component_direct_children_for_authority(authority, canister)?;
    let direct_children_hash = ComponentRuntimeOps::direct_children_hash(&direct_children)?;
    let observed = query_component_runtime_status(canister, operation_id).await?;
    let converged = if active_member_directory_is_converged(
        &observed,
        binding,
        authority,
        authority_hash,
        direct_children_hash,
    )? {
        observed
    } else {
        let request = ComponentRuntimeDirectorySynchronizationRequest {
            operation_id: observed.operation_id,
            authority: authority.clone(),
            direct_children,
        };
        match synchronize_target_component_directory(canister, request).await {
            Ok(status) => status,
            Err(call_error) => {
                let reconciled = query_component_runtime_status(canister, operation_id).await?;
                if active_member_directory_is_converged(
                    &reconciled,
                    binding,
                    authority,
                    authority_hash,
                    direct_children_hash,
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
    if !active_member_directory_is_converged(
        &converged,
        binding,
        authority,
        authority_hash,
        direct_children_hash,
    )? {
        return Err(InternalError::unavailable());
    }

    let independently_observed = query_component_runtime_status(canister, operation_id).await?;
    exact_active_member_directory_receipt(
        &independently_observed,
        binding,
        authority,
        authority_hash,
        direct_children_hash,
    )
}

fn active_member_directory_is_converged(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    expected: &ComponentRuntimeDirectoryAuthority,
    expected_hash: [u8; 32],
    expected_direct_children_hash: [u8; 32],
) -> Result<bool, InternalError> {
    let current = status
        .authority
        .as_ref()
        .ok_or_else(InternalError::conflict)?;
    let current_hash = status.authority_hash.ok_or_else(InternalError::conflict)?;
    let current_direct_children_hash = status
        .direct_children_hash
        .ok_or_else(InternalError::conflict)?;
    let activation = status.activation.ok_or_else(InternalError::conflict)?;
    validate_active_member_protected_status(status, binding, current, current_hash, activation)?;

    let current_component = &current.component.provenance;
    let expected_component = &expected.component.provenance;
    let current_ownership = ComponentDirectoryOwnership::from_provenance(current_component);
    let expected_ownership = ComponentDirectoryOwnership::from_provenance(expected_component);
    let binding_ownership = ComponentDirectoryOwnership::from_binding(owning_component(binding));
    if current_ownership != expected_ownership {
        return Err(InternalError::conflict());
    }
    if current_ownership != binding_ownership {
        return Err(InternalError::conflict());
    }
    match current_component
        .component_registry_revision
        .cmp(&expected_component.component_registry_revision)
    {
        std::cmp::Ordering::Equal => {
            let current_identity = ComponentRuntimeDirectoryStatusIdentity::exact(
                current,
                current_hash,
                current_direct_children_hash,
            );
            let expected_identity = ComponentRuntimeDirectoryStatusIdentity::exact(
                expected,
                expected_hash,
                expected_direct_children_hash,
            );
            if current_identity == expected_identity {
                Ok(true)
            } else {
                Err(InternalError::conflict())
            }
        }
        std::cmp::Ordering::Less => {
            if !component_directory_progresses(current, expected) {
                return Err(InternalError::conflict());
            }
            Ok(false)
        }
        std::cmp::Ordering::Greater => {
            if !component_directory_progresses(expected, current) {
                return Err(InternalError::conflict());
            }
            Ok(true)
        }
    }
}

fn validate_active_member_protected_status(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    current: &ComponentRuntimeDirectoryAuthority,
    current_hash: [u8; 32],
    activation: ComponentRuntimeActivationEvidence,
) -> Result<(), InternalError> {
    if status.binding != *binding {
        return Err(InternalError::conflict());
    }
    if status.phase != ComponentRuntimePhase::Active {
        return Err(InternalError::conflict());
    }
    if activation.directory_authority_hash == [0; 32] {
        return Err(InternalError::conflict());
    }
    if activation.activated_at_ns == 0 {
        return Err(InternalError::conflict());
    }
    if ComponentRuntimeOps::directory_authority_hash(current)? != current_hash {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn component_directory_progresses(
    current: &ComponentRuntimeDirectoryAuthority,
    next: &ComponentRuntimeDirectoryAuthority,
) -> bool {
    let current_component = &current.component.provenance;
    let next_component = &next.component.provenance;
    if next_component.component_registry_content_hash
        == current_component.component_registry_content_hash
    {
        return false;
    }
    if next_component.synchronized_at_ns <= current_component.synchronized_at_ns {
        return false;
    }
    fleet_directory_non_regressing(&current.fleet, &next.fleet)
}

fn exact_active_member_directory_receipt(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    authority: &ComponentRuntimeDirectoryAuthority,
    authority_hash: [u8; 32],
    direct_children_hash: [u8; 32],
) -> Result<ComponentRuntimeDirectoryConvergenceEvidence, InternalError> {
    if !active_member_directory_is_converged(
        status,
        binding,
        authority,
        authority_hash,
        direct_children_hash,
    )? {
        return Err(InternalError::unavailable());
    }
    let activation = status.activation.ok_or_else(InternalError::invariant)?;
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
    let deployment = ungrouped_component_deployment(binding);
    validate_target_directory_status_for_deployment(
        status,
        binding,
        &deployment,
        request,
        authority_hash,
    )
}

fn validate_target_directory_status_for_deployment(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
    request: &ComponentRuntimeDirectoryPreparationRequest,
    authority_hash: [u8; 32],
) -> Result<ComponentRuntimePhase, InternalError> {
    let direct_children_hash = ComponentRuntimeOps::direct_children_hash(&request.direct_children)?;
    if status.operation_id != request.operation_id {
        return Err(InternalError::conflict());
    }
    if status.binding != *binding {
        return Err(InternalError::conflict());
    }
    if status.deployment.as_ref() != deployment {
        return Err(InternalError::conflict());
    }
    let identity = ComponentRuntimeDirectoryStatusIdentity::from_status(status);
    let prepared_identity = ComponentRuntimeDirectoryStatusIdentity::exact(
        &request.authority,
        authority_hash,
        direct_children_hash,
    );
    match status.phase {
        ComponentRuntimePhase::AwaitingDirectory
            if identity == ComponentRuntimeDirectoryStatusIdentity::empty()
                && status.activation.is_none() =>
        {
            Ok(ComponentRuntimePhase::AwaitingDirectory)
        }
        ComponentRuntimePhase::DirectoryPrepared
            if identity == prepared_identity && status.activation.is_none() =>
        {
            Ok(ComponentRuntimePhase::DirectoryPrepared)
        }
        ComponentRuntimePhase::Active
            if identity.is_complete()
                && target_activation_matches(status.activation, authority_hash) =>
        {
            Ok(ComponentRuntimePhase::Active)
        }
        ComponentRuntimePhase::AwaitingDirectory
        | ComponentRuntimePhase::DirectoryPrepared
        | ComponentRuntimePhase::Active => Err(InternalError::conflict()),
    }
}

#[cfg(test)]
fn target_deployment_matches(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
) -> bool {
    status.deployment.as_ref() == &ungrouped_component_deployment(binding)
}

fn ungrouped_component_deployment(
    binding: &ManagedCanisterBinding,
) -> ProtectedComponentDeployment {
    let component = match binding {
        ManagedCanisterBinding::Component(component) => component,
        ManagedCanisterBinding::ComponentChild(child) => &child.component,
    };
    ProtectedComponentDeployment::UngroupedOrdinary {
        binding: component.clone(),
    }
}

fn target_activation_matches(
    activation: Option<ComponentRuntimeActivationEvidence>,
    expected_directory_authority_hash: [u8; 32],
) -> bool {
    let Some(activation) = activation else {
        return false;
    };
    if activation.directory_authority_hash != expected_directory_authority_hash {
        return false;
    }
    activation.activated_at_ns != 0
}

fn validate_active_directory_refresh_identity(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
    operation_id: [u8; 32],
) -> Result<ComponentRuntimeActivationEvidence, InternalError> {
    let identity_is_exact = [
        status.operation_id == operation_id,
        status.binding == *binding,
        status.deployment.as_ref() == deployment,
        status.phase == ComponentRuntimePhase::Active,
        status.authority.is_some(),
        status.authority_hash.is_some(),
        status.direct_children_hash.is_some(),
    ]
    .into_iter()
    .all(|matches| matches);
    if !identity_is_exact {
        return Err(InternalError::conflict());
    }
    status.activation.ok_or_else(InternalError::invariant)
}

fn active_directory_refresh_covers(
    status: &ComponentRuntimeStatusResponse,
    request: &ComponentRuntimeDirectorySynchronizationRequest,
    directory_authority_hash: [u8; 32],
) -> Result<bool, InternalError> {
    let Some(current) = status.authority.as_ref() else {
        return Ok(false);
    };
    let Some(current_authority_hash) = status.authority_hash else {
        return Ok(false);
    };
    if ComponentRuntimeOps::directory_authority_hash(current)? != current_authority_hash {
        return Ok(false);
    }
    if current == &request.authority {
        return Ok([
            current_authority_hash == directory_authority_hash,
            status.direct_children_hash
                == Some(ComponentRuntimeOps::direct_children_hash(
                    &request.direct_children,
                )?),
        ]
        .into_iter()
        .all(|matches| matches));
    }

    let current_component = &current.component.provenance;
    let required_component = &request.authority.component.provenance;
    let later_component_authority = [
        current_component.component == required_component.component,
        current_component.source_fleet_subnet_root == required_component.source_fleet_subnet_root,
        current_component.component_registry_revision
            > required_component.component_registry_revision,
        current_component.component_registry_content_hash
            != required_component.component_registry_content_hash,
        current_component.synchronized_at_ns > required_component.synchronized_at_ns,
    ]
    .into_iter()
    .all(|matches| matches);
    Ok([
        current.fleet == request.authority.fleet,
        current.component_group == request.authority.component_group,
        later_component_authority,
        current_authority_hash != directory_authority_hash,
        status.direct_children_hash.is_some(),
    ]
    .into_iter()
    .all(|matches| matches))
}

fn prepared_target_directory_status(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    request: &ComponentRuntimeDirectoryPreparationRequest,
    authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let deployment = ungrouped_component_deployment(binding);
    prepared_target_directory_status_for_deployment(
        status,
        binding,
        &deployment,
        request,
        authority_hash,
    )
}

fn prepared_target_directory_status_for_deployment(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
    request: &ComponentRuntimeDirectoryPreparationRequest,
    authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    match validate_target_directory_status_for_deployment(
        status,
        binding,
        deployment,
        request,
        authority_hash,
    )? {
        ComponentRuntimePhase::DirectoryPrepared => Ok(status.clone()),
        ComponentRuntimePhase::Active => Ok(ComponentRuntimeStatusResponse {
            operation_id: request.operation_id,
            binding: binding.clone(),
            deployment: status.deployment.clone(),
            phase: ComponentRuntimePhase::DirectoryPrepared,
            authority: Some(request.authority.clone()),
            authority_hash: Some(authority_hash),
            direct_children_hash: Some(ComponentRuntimeOps::direct_children_hash(
                &request.direct_children,
            )?),
            activation: None,
        }),
        ComponentRuntimePhase::AwaitingDirectory => Err(InternalError::unavailable()),
    }
}

fn validate_active_target_runtime_status_for_deployment(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
    request: &ComponentRuntimeDirectoryPreparationRequest,
    authority_hash: [u8; 32],
) -> Result<(), InternalError> {
    if validate_target_directory_status_for_deployment(
        status,
        binding,
        deployment,
        request,
        authority_hash,
    )? != ComponentRuntimePhase::Active
    {
        return Err(InternalError::unavailable());
    }
    Ok(())
}

fn active_target_runtime_status_for_deployment(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
    request: &ComponentRuntimeDirectoryPreparationRequest,
    authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    validate_active_target_runtime_status_for_deployment(
        status,
        binding,
        deployment,
        request,
        authority_hash,
    )?;
    let activation = status.activation.ok_or_else(InternalError::invariant)?;
    Ok(ComponentRuntimeStatusResponse {
        operation_id: request.operation_id,
        binding: binding.clone(),
        deployment: status.deployment.clone(),
        phase: ComponentRuntimePhase::Active,
        authority: Some(request.authority.clone()),
        authority_hash: Some(authority_hash),
        direct_children_hash: Some(ComponentRuntimeOps::direct_children_hash(
            &request.direct_children,
        )?),
        activation: Some(activation),
    })
}

fn validate_target_membership_status_for_deployment(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
    prepared_request: &ComponentRuntimeDirectoryPreparationRequest,
    prepared_authority_hash: [u8; 32],
    active_request: &ComponentRuntimeDirectorySynchronizationRequest,
    active_authority_hash: [u8; 32],
) -> Result<bool, InternalError> {
    validate_active_target_runtime_status_for_deployment(
        status,
        binding,
        deployment,
        prepared_request,
        prepared_authority_hash,
    )?;
    active_member_directory_is_converged(
        status,
        binding,
        &active_request.authority,
        active_authority_hash,
        ComponentRuntimeOps::direct_children_hash(&active_request.direct_children)?,
    )
}

fn active_membership_target_status_for_deployment(
    status: &ComponentRuntimeStatusResponse,
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
    prepared_request: &ComponentRuntimeDirectoryPreparationRequest,
    prepared_authority_hash: [u8; 32],
    active_request: &ComponentRuntimeDirectorySynchronizationRequest,
    active_authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    if !validate_target_membership_status_for_deployment(
        status,
        binding,
        deployment,
        prepared_request,
        prepared_authority_hash,
        active_request,
        active_authority_hash,
    )? {
        return Err(InternalError::unavailable());
    }
    let activation = status.activation.ok_or_else(InternalError::invariant)?;
    Ok(ComponentRuntimeStatusResponse {
        operation_id: prepared_request.operation_id,
        binding: binding.clone(),
        deployment: status.deployment.clone(),
        phase: ComponentRuntimePhase::Active,
        authority: Some(active_request.authority.clone()),
        authority_hash: Some(active_authority_hash),
        direct_children_hash: Some(ComponentRuntimeOps::direct_children_hash(
            &active_request.direct_children,
        )?),
        activation: Some(activation),
    })
}

const fn allocation_creation_and_canister(
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
        | RootComponentAllocationProgressView::CreationIntent(_) => Err(InternalError::conflict()),
    }
}

const fn child_allocation_creation_and_canister(
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
            Err(InternalError::conflict())
        }
    }
}

const fn install_effect(
    allocation: &RootComponentAllocationView,
) -> Result<&RootComponentInstallEffectView, InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::InstallIntent { installation, .. } => Ok(installation),
        _ => Err(InternalError::invariant()),
    }
}

const fn child_install_effect(
    allocation: &RootComponentChildAllocationView,
) -> Result<&RootComponentChildInstallEffectView, InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::InstallIntent { installation, .. } => {
            Ok(installation)
        }
        _ => Err(InternalError::invariant()),
    }
}

const fn committed_or_verified_installation(
    allocation: &RootComponentAllocationView,
) -> Result<&RootComponentInstallEffectView, InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Verified { installation, .. }
        | RootComponentAllocationProgressView::Committed { installation, .. } => Ok(installation),
        _ => Err(InternalError::conflict()),
    }
}

const fn committed_or_verified_child_installation(
    allocation: &RootComponentChildAllocationView,
) -> Result<&RootComponentChildInstallEffectView, InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::Verified { installation, .. }
        | RootComponentChildAllocationProgressView::Committed { installation, .. } => {
            Ok(installation)
        }
        _ => Err(InternalError::conflict()),
    }
}

const fn committed_installation(
    allocation: &RootComponentAllocationView,
) -> Result<&RootComponentInstallEffectView, InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Committed { installation, .. } => Ok(installation),
        _ => Err(InternalError::conflict()),
    }
}

const fn committed_child_installation(
    allocation: &RootComponentChildAllocationView,
) -> Result<&RootComponentChildInstallEffectView, InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::Committed { installation, .. } => {
            Ok(installation)
        }
        _ => Err(InternalError::conflict()),
    }
}

pub(super) const fn committed_directory_receipt(
    allocation: &RootComponentAllocationView,
) -> Result<&crate::view::component_registry::RootComponentCommitmentView, InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Committed { commitment, .. } => Ok(commitment),
        _ => Err(InternalError::conflict()),
    }
}

const fn committed_child_directory_receipt(
    allocation: &RootComponentChildAllocationView,
) -> Result<&crate::view::component_registry::RootComponentChildCommitmentView, InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::Committed { commitment, .. } => Ok(commitment),
        _ => Err(InternalError::conflict()),
    }
}

fn prepared_registry(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
) -> Result<RootComponentRegistryView, InternalError> {
    let prepared = ComponentRegistryOps::current().ok_or_else(InternalError::unavailable)?;
    if &prepared.root != root || prepared.release_set != release_set {
        return Err(InternalError::conflict());
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
        return Err(InternalError::conflict());
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
        return Err(InternalError::conflict());
    }
    if mirror.root_entry.status == FleetSubnetRootStatus::Draining {
        ComponentRegistryOps::validate_published_root_draining(current)?;
    }
    Ok(mirror.active.directory)
}

fn require_active_root_runtime(_unavailable_message: &'static str) -> Result<(), InternalError> {
    if FleetActivationWorkflow::status()?.phase != FleetActivationPhase::Active {
        return Err(InternalError::unavailable());
    }
    Ok(())
}

fn response(
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

fn allocation_response(
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
    let progress = draining.deletion.ok_or_else(InternalError::unavailable)?;
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
    let phase = draining.quiescence.ok_or_else(InternalError::unavailable)?;
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

fn commit_response(
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

fn membership_response(
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

fn child_membership_response(
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
        protocol_profile_digest: partition.protocol_profile_digest,
        provisioning_origin: partition.provisioning_origin,
        release_set: partition.release_set,
        status: partition.status,
        reserved_descendants: partition.reserved_descendants,
        committed_descendants: partition.committed_descendants,
        encoded_bytes: partition.encoded_bytes,
    }
}

pub(super) fn component_directory_head(
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

fn current_component_directory_authority(
    partition: &ComponentRegistryPartitionView,
    fleet: FleetDirectorySnapshot,
) -> Result<ComponentRuntimeDirectoryAuthority, InternalError> {
    let deployment_authority = RootComponentProvisioningOps::component_deployment_authority(
        &partition.provisioning_origin,
        &partition.binding,
    )?;
    Ok(ComponentRuntimeDirectoryAuthority {
        fleet,
        component: component_directory_head(partition),
        component_group: deployment_authority.component_group,
    })
}

fn component_deployment_limits(
    partition: &ComponentRegistryPartitionView,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
) -> Result<ComponentDeploymentLimits, InternalError> {
    let authority = RootComponentProvisioningOps::component_deployment_authority(
        &partition.provisioning_origin,
        &partition.binding,
    )?;
    ConfigOps::validate_protected_component_deployment(&authority.deployment, &partition.binding)?;
    match authority.deployment {
        ProtectedComponentDeployment::UngroupedOrdinary { .. } => {
            let spec = topology
                .get(&partition.binding.component_spec)
                .ok_or_else(InternalError::invariant)?;
            Ok(ComponentDeploymentLimits {
                maximum_descendants: spec.limits.maximum_descendants,
                maximum_registry_bytes: spec.limits.maximum_registry_bytes,
                spawn_grant_reductions: Vec::new(),
            })
        }
        ProtectedComponentDeployment::GroupMember { limits, .. } => Ok(limits),
    }
}

fn decode_component_directory_cursor(
    request: &ComponentDirectoryPageRequest,
) -> Result<Option<ComponentDirectoryCanonicalCursor>, InternalError> {
    let Some(cursor) = request.cursor.as_ref() else {
        return Ok(None);
    };
    if cursor.0.is_empty() || cursor.0.len() > MAX_COMPONENT_DIRECTORY_CURSOR_BYTES {
        return Err(InternalError::invalid_input());
    }
    let payload = candid::decode_one::<ComponentDirectoryCursorPayload>(&cursor.0)
        .map_err(|_| InternalError::invalid_input())?;
    if ComponentDirectoryCursorBinding::from_payload(&payload)
        != ComponentDirectoryCursorBinding::from_request(request)
    {
        return Err(InternalError::conflict());
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
    let bytes = candid::encode_one(payload).map_err(|_error| InternalError::invariant())?;
    if bytes.len() > MAX_COMPONENT_DIRECTORY_CURSOR_BYTES {
        return Err(InternalError::invariant());
    }
    Ok(ComponentDirectoryPageCursor(bytes))
}

fn creation_plan(
    root: candid::Principal,
    store: &RootStoreBootstrapResponse,
    allocation: &RootComponentAllocationView,
) -> Result<RootComponentCreationPlan, InternalError> {
    if store.fleet_subnet_root != root || store.release_set != allocation.release_set {
        return Err(InternalError::conflict());
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
        return Err(InternalError::conflict());
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
    let artifact = matching.next().ok_or_else(InternalError::unavailable)?;
    if matching.next().is_some() {
        return Err(InternalError::invariant());
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
            return Err(InternalError::unavailable());
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
        return Err(InternalError::invariant());
    }
    if store.fleet_subnet_root != root.fleet_subnet_root
        || store.release_set != partition.release_set
    {
        return Err(InternalError::conflict());
    }
    let artifact = exact_store_artifact(store, &partition.binding.role)?;
    if stop.expected_module_hash != artifact.payload_hash
        || observed_module_hash.is_some_and(|hash| hash != artifact.payload_hash)
    {
        return Err(InternalError::invariant());
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
            return Err(InternalError::unavailable());
        }
    };
    let request_authority = ComponentDeletionRequestAuthority::from_request(request);
    let durable_authority = ComponentDeletionRequestAuthority::from_durable(draining, &deletion);
    if request_authority != durable_authority {
        return Err(InternalError::conflict());
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
        return Err(InternalError::invariant());
    }
    if deletion.quiescence.stop.controller != root.fleet_subnet_root {
        return Err(InternalError::invariant());
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
        return Err(InternalError::invariant());
    }
    if store.release_set != partition.release_set {
        return Err(InternalError::conflict());
    }
    let artifact = exact_store_artifact(store, &partition.binding.role)?;
    if deletion.quiescence.stop.expected_module_hash != artifact.payload_hash {
        return Err(InternalError::invariant());
    }
    if deletion.quiescence.observed_module_hash != artifact.payload_hash {
        return Err(InternalError::invariant());
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
            return Err(InternalError::unavailable());
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
        return Err(InternalError::conflict());
    }
    if stop.controller != root.fleet_subnet_root {
        return Err(InternalError::invariant());
    }
    if store.fleet_subnet_root != root.fleet_subnet_root {
        return Err(InternalError::invariant());
    }
    let artifact = exact_store_artifact(store, &stop.leaf.role)?;
    if artifact.raw_module_hash != stop.leaf.installed_artifact_hash
        || durable_module_hash.is_some_and(|hash| hash != artifact.payload_hash)
    {
        return Err(InternalError::invariant());
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
        return Err(InternalError::conflict());
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
            return Err(InternalError::unavailable());
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
        return Err(InternalError::conflict());
    }
    if deletion.stopped.stop.controller != root.fleet_subnet_root {
        return Err(InternalError::invariant());
    }
    if store.fleet_subnet_root != root.fleet_subnet_root {
        return Err(InternalError::invariant());
    }
    let artifact = exact_store_artifact(store, &deletion.stopped.stop.leaf.role)?;
    if artifact.raw_module_hash != deletion.stopped.stop.leaf.installed_artifact_hash
        || artifact.payload_hash != deletion.stopped.observed_module_hash
    {
        return Err(InternalError::invariant());
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
            return Err(InternalError::unavailable());
        }
        CanisterStatusType::Running => {}
    }

    let stop_error = MgmtOps::stop_canister(plan.stop.canister_id).await.err();
    match observed_component_quiescence_status(plan).await? {
        CanisterStatusType::Stopped => Ok(()),
        CanisterStatusType::Stopping => Err(InternalError::unavailable()),
        CanisterStatusType::Running => match stop_error {
            Some(error) => Err(error),
            None => Err(InternalError::unavailable()),
        },
    }
}

async fn observed_component_quiescence_status(
    plan: &PreparedComponentQuiescencePlan,
) -> Result<CanisterStatusType, InternalError> {
    let status = MgmtOps::canister_status(plan.stop.canister_id).await?;
    if status.settings.controllers != vec![plan.stop.controller] {
        return Err(InternalError::conflict());
    }
    if status.module_hash.as_deref() != Some(plan.expected_status_module_hash.as_slice()) {
        return Err(InternalError::conflict());
    }
    Ok(status.status)
}

async fn observe_or_stop_subtree_leaf(
    plan: &PreparedSubtreeLeafStopPlan,
) -> Result<(), InternalError> {
    match observed_subtree_leaf_status(plan).await? {
        CanisterStatusType::Stopped => return Ok(()),
        CanisterStatusType::Stopping => {
            return Err(InternalError::unavailable());
        }
        CanisterStatusType::Running => {}
    }

    let stop_error = MgmtOps::stop_canister(plan.stop.leaf.canister_id)
        .await
        .err();
    match observed_subtree_leaf_status(plan).await? {
        CanisterStatusType::Stopped => Ok(()),
        CanisterStatusType::Stopping => Err(InternalError::unavailable()),
        CanisterStatusType::Running => match stop_error {
            Some(error) => Err(error),
            None => Err(InternalError::unavailable()),
        },
    }
}

async fn observe_or_recycle_subtree_leaf(
    plan: &PreparedSubtreeLeafDeletePlan,
) -> Result<(), InternalError> {
    let canister_id = plan.deletion.stopped.stop.leaf.canister_id;
    if CanisterPoolOps::contains_asset(canister_id) {
        return crate::workflow::canister_pool::recycle(canister_id).await;
    }
    match observed_subtree_leaf_for_deletion(plan).await? {
        CanisterStatusObservation::Absent => {
            return Err(InternalError::unavailable());
        }
        CanisterStatusObservation::Present(_) => {}
    }

    crate::workflow::canister_pool::recycle(canister_id).await
}

async fn observe_or_recycle_component(
    plan: &PreparedComponentDeletionPlan,
) -> Result<(), InternalError> {
    let canister_id = plan.deletion.quiescence.stop.canister_id;
    if CanisterPoolOps::contains_asset(canister_id) {
        return crate::workflow::canister_pool::recycle(canister_id).await;
    }
    match observed_component_for_deletion(plan).await? {
        CanisterStatusObservation::Absent => {
            return Err(InternalError::unavailable());
        }
        CanisterStatusObservation::Present(_) => {}
    }

    crate::workflow::canister_pool::recycle(canister_id).await
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
        return Err(InternalError::conflict());
    }
    if status.module_hash.as_deref() != Some(expected_status_module_hash.as_slice()) {
        return Err(InternalError::conflict());
    }
    if status.status != CanisterStatusType::Stopped {
        return Err(InternalError::conflict());
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
        return Err(InternalError::conflict());
    }
    Ok(observation)
}

fn validate_subtree_leaf_live_status(
    status: &CanisterStatus,
    stop: &RootComponentSubtreeStopEffectView,
    expected_status_module_hash: [u8; 32],
) -> Result<CanisterStatusType, InternalError> {
    if status.settings.controllers != vec![stop.controller] {
        return Err(InternalError::conflict());
    }
    if status.module_hash.as_deref() != Some(expected_status_module_hash.as_slice()) {
        return Err(InternalError::conflict());
    }
    Ok(status.status)
}

fn validate_allocation_caller(
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
struct PeerRequesterAccessEvidence<'a> {
    caller: candid::Principal,
    indexed_component: Option<canic_core::ids::ComponentInstanceId>,
    retained: &'a canic_core::ids::ComponentBinding,
    current: &'a canic_core::ids::ComponentBinding,
    current_status: ComponentLifecycleStatus,
}

impl PeerRequesterAccessEvidence<'_> {
    fn is_exact_active(&self) -> bool {
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

fn require_active_peer_allocation_caller(operation_id: [u8; 32]) -> Result<(), InternalError> {
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

fn revalidate_peer_provisioning_origin(
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

fn revalidate_retained_peer_origin(
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

fn revalidate_same_root_peer_origin(
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

fn revalidate_fleet_service_peer_origin(
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

fn validate_retained_peer_grant(
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

fn validate_creation_effect(
    effect: &RootComponentCreationEffectView,
    expected: &RootComponentCreationPlan,
) -> Result<(), InternalError> {
    if !expected.matches_effect(effect) {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_install_effect(
    effect: &RootComponentInstallEffectView,
    expected: &RootComponentInstallPlan,
) -> Result<(), InternalError> {
    if !expected.matches_effect(effect) {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_child_install_effect(
    effect: &RootComponentChildInstallEffectView,
    expected: &RootComponentChildInstallPlan,
) -> Result<(), InternalError> {
    if !expected.matches_effect(effect) {
        return Err(InternalError::invariant());
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

pub(super) fn validate_allocation_record(
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

fn validate_provisioning_origin(
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

fn validate_subtree_removal_target(
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

const fn subtree_target_membership_is_removed(
    progress: &RootComponentSubtreeRemovalProgressView,
) -> bool {
    matches!(
        progress,
        RootComponentSubtreeRemovalProgressView::MembershipRemoved(_)
            | RootComponentSubtreeRemovalProgressView::DirectorySynchronized(_)
            | RootComponentSubtreeRemovalProgressView::Completed(_)
    )
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

fn deployment_spawn_grant_maximum(
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

fn child_allocation_request_matches(
    request: &RootComponentChildAllocationRequest,
    allocation: &RootComponentChildAllocationView,
) -> bool {
    ComponentChildRequestIdentity::from(request) == ComponentChildRequestIdentity::from(allocation)
}

#[derive(Eq, PartialEq)]
struct ComponentChildRequestIdentity<'a> {
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

fn validate_partition(
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
                .map_err(|_error| InternalError::invariant())
        }
        ManagedCanisterBinding::Component(_) | ManagedCanisterBinding::ComponentChild(_) => {
            Err(InternalError::invariant())
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
        FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootReleaseSet, ReleaseBuildId,
        ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    };

    #[test]
    fn grouped_allocation_cannot_advance_through_ordinary_lifecycle() {
        let allocation = RootComponentAllocationView {
            operation_id: [1; 32],
            allocation_sequence: 1,
            component: ComponentInstanceId::from_generated_bytes([2; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [3; 32],
            role: CanisterRole::new("project_hub"),
            provisioning_origin: ComponentProvisioningOrigin::ComponentGroup {
                operation_id: [4; 32],
                plan_hash: [5; 32],
                group_placement: canic_core::ids::ComponentGroupPlacementId {
                    deployment: "cells".parse().expect("deployment ID"),
                    ordinal: 0,
                },
                member_path: canic_core::ids::ComponentGroupMemberPath::try_from(vec![
                    "hub".parse().expect("member ID"),
                ])
                .expect("member path"),
            },
            release_set: FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [6; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([7; 32]),
            },
            progress: RootComponentAllocationProgressView::Reserved,
        };

        assert!(validate_allocation_caller(&allocation).is_err());
    }

    #[test]
    fn grouped_install_status_requires_exact_context_and_empty_prepared_fence() {
        let binding = component_binding();
        let managed = ManagedCanisterBinding::Component(binding.clone());
        let deployment = ProtectedComponentDeployment::GroupMember {
            binding,
            configuration_digest:
                canic_core::ids::ComponentDeploymentConfigurationDigest::from_bytes([9; 32]),
            group_placement: canic_core::ids::ComponentGroupPlacementId {
                deployment: "cells".parse().expect("deployment ID"),
                ordinal: 2,
            },
            component_group: "cell".parse().expect("Component Group ID"),
            member_path: canic_core::ids::ComponentGroupMemberPath::try_from(vec![
                "hub".parse().expect("member ID"),
            ])
            .expect("member path"),
            purpose: canic_core::dto::component_deployment::ComponentDeploymentPurpose::Ordinary,
            labels: Vec::new(),
            limits: canic_core::dto::component_deployment::ComponentDeploymentLimits {
                maximum_descendants: 10_000,
                maximum_registry_bytes: 16_777_216,
                spawn_grant_reductions: Vec::new(),
            },
        };
        let operation_id = [10; 32];
        let mut status = ComponentRuntimeStatusResponse {
            operation_id,
            binding: managed.clone(),
            deployment: Box::new(deployment.clone()),
            phase: ComponentRuntimePhase::AwaitingDirectory,
            authority: None,
            authority_hash: None,
            direct_children_hash: None,
            activation: None,
        };

        validate_prepared_install_status(&status, operation_id, &managed, &deployment)
            .expect("exact grouped Prepared status");
        *status.deployment = ProtectedComponentDeployment::UngroupedOrdinary {
            binding: component_binding(),
        };
        assert!(
            validate_prepared_install_status(&status, operation_id, &managed, &deployment).is_err()
        );
        *status.deployment = deployment.clone();
        status.phase = ComponentRuntimePhase::DirectoryPrepared;
        assert!(
            validate_prepared_install_status(&status, operation_id, &managed, &deployment).is_err()
        );
        validate_installed_component_status(&status, operation_id, &managed, &deployment)
            .expect("advanced runtime retains exact installed identity");
    }

    #[test]
    fn active_directory_refresh_accepts_only_valid_later_component_coverage() {
        let binding = component_binding();
        let fleet = FleetDirectorySnapshot {
            provenance: canic_core::dto::fleet_registry::FleetDirectoryProvenance {
                registry: FleetRegistryVersion {
                    authority: binding.authority.clone(),
                    revision: 7,
                    content_hash: [21; 32],
                },
                source_fleet_subnet_root: binding.fleet_subnet_root,
            },
            fleet_subnet_roots: Vec::new(),
            services: Vec::new(),
        };
        let operation_id = [22; 32];
        let required_authority = ComponentRuntimeDirectoryAuthority {
            fleet,
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: binding.clone(),
                    source_fleet_subnet_root: binding.fleet_subnet_root,
                    component_registry_revision: 3,
                    component_registry_content_hash: [23; 32],
                    synchronized_at_ns: 24,
                },
                descendant_count: 0,
            },
            component_group: None,
        };
        let required_hash = ComponentRuntimeOps::directory_authority_hash(&required_authority)
            .expect("required Directory authority hash");
        let request = ComponentRuntimeDirectorySynchronizationRequest {
            operation_id,
            authority: required_authority.clone(),
            direct_children: Vec::new(),
        };
        let mut current_authority = required_authority;
        current_authority
            .component
            .provenance
            .component_registry_revision = 4;
        current_authority
            .component
            .provenance
            .component_registry_content_hash = [25; 32];
        current_authority.component.provenance.synchronized_at_ns = 26;
        let current_hash = ComponentRuntimeOps::directory_authority_hash(&current_authority)
            .expect("current Directory authority hash");
        let direct_children_hash =
            ComponentRuntimeOps::direct_children_hash(&[]).expect("empty direct-child hash");
        let mut status = ComponentRuntimeStatusResponse {
            operation_id,
            binding: ManagedCanisterBinding::Component(binding.clone()),
            deployment: Box::new(ProtectedComponentDeployment::UngroupedOrdinary { binding }),
            phase: ComponentRuntimePhase::Active,
            authority: Some(current_authority),
            authority_hash: Some(current_hash),
            direct_children_hash: Some(direct_children_hash),
            activation: Some(ComponentRuntimeActivationEvidence {
                directory_authority_hash: required_hash,
                activated_at_ns: 27,
            }),
        };

        assert!(
            active_directory_refresh_covers(&status, &request, required_hash)
                .expect("later coverage")
        );
        status
            .authority
            .as_mut()
            .expect("current authority")
            .fleet
            .provenance
            .registry
            .revision = 8;
        status.authority_hash = Some(
            ComponentRuntimeOps::directory_authority_hash(
                status.authority.as_ref().expect("current authority"),
            )
            .expect("changed authority hash"),
        );
        assert!(
            !active_directory_refresh_covers(&status, &request, required_hash)
                .expect("foreign Fleet coverage rejects")
        );
    }

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
    fn peer_requester_access_requires_exact_active_top_level_component() {
        let requester = component_binding();
        let caller = requester.canister_id;
        let exact = PeerRequesterAccessEvidence {
            caller,
            indexed_component: Some(requester.component),
            retained: &requester,
            current: &requester,
            current_status: ComponentLifecycleStatus::Active,
        };
        assert!(exact.is_exact_active());

        let foreign_caller = PeerRequesterAccessEvidence {
            caller: candid::Principal::from_slice(&[9; 29]),
            ..exact
        };
        assert!(!foreign_caller.is_exact_active());
        let missing_index = PeerRequesterAccessEvidence {
            indexed_component: None,
            ..exact
        };
        assert!(!missing_index.is_exact_active());
        let drifted_binding = ComponentBinding {
            canister_id: candid::Principal::from_slice(&[10; 29]),
            ..requester.clone()
        };
        let drifted = PeerRequesterAccessEvidence {
            current: &drifted_binding,
            ..exact
        };
        assert!(!drifted.is_exact_active());
        let draining = PeerRequesterAccessEvidence {
            current_status: ComponentLifecycleStatus::Draining,
            ..exact
        };
        assert!(!draining.is_exact_active());
    }

    #[test]
    fn component_directory_cursor_is_opaque_and_bound_to_head_and_filters() {
        let binding = component_binding();
        let root = binding.fleet_subnet_root;
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

    #[test]
    fn ordinary_root_observation_rejects_deployment_context_substitution() {
        let component = component_binding();
        let managed = ManagedCanisterBinding::Component(component.clone());
        let mut status = ComponentRuntimeStatusResponse {
            operation_id: [11; 32],
            binding: managed.clone(),
            deployment: Box::new(ProtectedComponentDeployment::UngroupedOrdinary {
                binding: component,
            }),
            phase: ComponentRuntimePhase::AwaitingDirectory,
            authority: None,
            authority_hash: None,
            direct_children_hash: None,
            activation: None,
        };
        assert!(target_deployment_matches(&status, &managed));

        let ProtectedComponentDeployment::UngroupedOrdinary { binding } =
            status.deployment.as_mut()
        else {
            unreachable!()
        };
        binding.component = ComponentInstanceId::from_generated_bytes([12; 32]);
        assert!(!target_deployment_matches(&status, &managed));
    }

    fn component_binding() -> ComponentBinding {
        let root = candid::Principal::from_slice(&[1; 29]);
        ComponentBinding {
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
            component: ComponentInstanceId::from_generated_bytes([2; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
            placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[7; 29])),
            fleet_subnet_root: root,
            canister_id: candid::Principal::from_slice(&[8; 29]),
        }
    }
}
