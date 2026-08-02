//! Module: workflow::component_rpc::lifecycle
//!
//! Responsibility: resume root-owned Component Child lifecycle for one capability request.
//! Does not own: endpoint admission, core replay receipts, or Component Registry mutation rules.
//! Boundary: implements the core lifecycle driver with existing control-plane phase owners.

use async_trait::async_trait;
use canic_core::{
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::ic::IcOps,
        workflow::rpc::{RootCapabilityLifecycleExecutor, RootComponentChildProvisionRequest},
    },
    dto::component_registry::{
        RootComponentChildAllocationRequest, RootComponentChildCommitRequest,
        RootComponentChildCreationRequest, RootComponentChildDirectoryPreparationRequest,
        RootComponentChildInstallRequest, RootComponentChildMembershipActivationRequest,
        RootComponentChildRuntimeActivationRequest,
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
}

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
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "Component Child lifecycle returned top-level Component authority",
        ));
    };
    if ProvisionedChildIdentity::from_binding(&binding) != expected_identity {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "Component Child lifecycle returned different protected identity",
        ));
    }
    Ok(binding.canister_id)
}
