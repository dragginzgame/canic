//! Module: workflow::component_rpc::lifecycle
//!
//! Responsibility: resume root-owned Component Child lifecycle for one capability request.
//! Does not own: endpoint admission, core replay receipts, or Component Registry mutation rules.
//! Boundary: implements the core lifecycle driver with existing control-plane phase owners.

use async_trait::async_trait;
use canic_core::{
    control_plane_support::{
        error::InternalError,
        ops::ic::IcOps,
        workflow::rpc::{
            RootCapabilityLifecycleExecutor, RootComponentChildProvisionRequest,
            RootComponentChildRecycleOutcome, RootComponentChildRecycleRequest,
        },
    },
    dto::component_registry::{
        RootComponentChildAllocationRequest, RootComponentChildCommitRequest,
        RootComponentChildCreationRequest, RootComponentChildDirectoryPreparationRequest,
        RootComponentChildInstallRequest, RootComponentChildMembershipActivationRequest,
        RootComponentChildRuntimeActivationRequest, RootComponentSubtreeRemovalPhase,
        RootComponentSubtreeRemovalRequest, RootComponentSubtreeRemovalResponse,
        RootComponentSubtreeRemovalStatusRequest,
    },
    ids::{CanisterRole, ComponentChildBinding, ComponentInstanceId, ManagedCanisterBinding},
};

use crate::workflow::component_registry;

///
/// ComponentChildLifecycleExecutor
///
/// Root capability driver backed by the protected Component Registry workflow.
///

pub(super) struct ComponentChildLifecycleExecutor;

pub(super) static COMPONENT_CHILD_LIFECYCLE_EXECUTOR: ComponentChildLifecycleExecutor =
    ComponentChildLifecycleExecutor;

#[async_trait]
impl RootCapabilityLifecycleExecutor for ComponentChildLifecycleExecutor {
    async fn provision_component_child(
        &self,
        request: RootComponentChildProvisionRequest,
    ) -> Result<candid::Principal, InternalError> {
        Box::pin(provision_component_child(request)).await
    }

    async fn recycle_component_child(
        &self,
        request: RootComponentChildRecycleRequest,
    ) -> Result<RootComponentChildRecycleOutcome, InternalError> {
        Box::pin(recycle_component_child(request)).await
    }
}

const MAX_RECYCLE_PHASES_PER_INVOCATION: usize = 16;

#[derive(Debug, Eq, PartialEq)]
struct ProvisionedChildIdentity {
    component: ComponentInstanceId,
    parent_canister_id: candid::Principal,
    role: CanisterRole,
}

impl ProvisionedChildIdentity {
    fn from_binding(binding: &ComponentChildBinding) -> Self {
        Self {
            component: binding.component.component,
            parent_canister_id: binding.parent_canister_id,
            role: binding.role.clone(),
        }
    }
}

async fn provision_component_child(
    request: RootComponentChildProvisionRequest,
) -> Result<candid::Principal, InternalError> {
    let operation_id = request.operation_id;
    let component = request.component;
    let expected_identity = ProvisionedChildIdentity {
        component,
        parent_canister_id: IcOps::msg_caller(),
        role: request.child_role.clone(),
    };
    component_registry::reserve_child_allocation(RootComponentChildAllocationRequest {
        operation_id,
        component,
        expected_registry: request.expected_registry,
        child_role: request.child_role,
        application_init_args: request.application_init_args,
    })
    .await?;
    component_registry::create_child_allocation(RootComponentChildCreationRequest {
        operation_id,
        component,
    })
    .await?;
    Box::pin(component_registry::install_child_allocation(
        RootComponentChildInstallRequest {
            operation_id,
            component,
        },
    ))
    .await?;
    component_registry::commit_child_allocation(RootComponentChildCommitRequest {
        operation_id,
        component,
    })
    .await?;
    Box::pin(component_registry::prepare_child_directories(
        RootComponentChildDirectoryPreparationRequest {
            operation_id,
            component,
        },
    ))
    .await?;
    component_registry::activate_child_runtime(RootComponentChildRuntimeActivationRequest {
        operation_id,
        component,
    })
    .await?;
    let active = Box::pin(component_registry::activate_child_membership(
        RootComponentChildMembershipActivationRequest {
            operation_id,
            component,
        },
    ))
    .await?;
    let ManagedCanisterBinding::ComponentChild(binding) = active.child.binding else {
        return Err(InternalError::invariant());
    };
    if ProvisionedChildIdentity::from_binding(&binding) != expected_identity {
        return Err(InternalError::invariant());
    }
    Ok(binding.canister_id)
}

async fn recycle_component_child(
    request: RootComponentChildRecycleRequest,
) -> Result<RootComponentChildRecycleOutcome, InternalError> {
    let status_request = RootComponentSubtreeRemovalStatusRequest {
        operation_id: request.operation_id,
        component: request.component,
    };
    let mut removal = match component_registry::existing_subtree_removal(status_request)? {
        Some(removal) => removal,
        None => {
            component_registry::begin_subtree_removal(RootComponentSubtreeRemovalRequest {
                operation_id: request.operation_id,
                component: request.component,
                target_canister_id: request.target_canister_id,
                expected_registry: request.expected_registry.clone(),
            })
            .await?
        }
    };
    require_expected_recycle_identity(&request, &removal)?;
    let starting_completed_leaves = removal.completed_leaves;

    for _ in 0..MAX_RECYCLE_PHASES_PER_INVOCATION {
        if matches!(
            &removal.phase,
            RootComponentSubtreeRemovalPhase::Completed(_)
        ) {
            return Ok(RootComponentChildRecycleOutcome::Completed);
        }
        if removal.completed_leaves > starting_completed_leaves {
            return Ok(RootComponentChildRecycleOutcome::InProgress);
        }
        removal = component_registry::advance_existing_subtree_removal(status_request).await?;
        require_expected_recycle_identity(&request, &removal)?;
    }

    Ok(RootComponentChildRecycleOutcome::InProgress)
}

fn require_expected_recycle_identity(
    request: &RootComponentChildRecycleRequest,
    removal: &RootComponentSubtreeRemovalResponse,
) -> Result<(), InternalError> {
    if removal.component != request.component {
        return Err(InternalError::invariant());
    }
    if removal.target_canister_id != request.target_canister_id {
        return Err(InternalError::invariant());
    }
    if removal.target_parent_canister_id != IcOps::msg_caller() {
        return Err(InternalError::invariant());
    }
    Ok(())
}
