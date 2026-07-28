//! Module: workflow::component_registry
//!
//! Responsibility: prepare Component Registry authority and advance top-level creation, install and commitment.
//! Does not own: descendant lifecycle, Directory distribution, or runtime activation.
//! Boundary: every mutation follows exact Store and active Registry Mirror/Directory verification.

use crate::{
    ops::{
        component_registry::{
            ComponentRegistryOps, RootComponentCreationPlan, RootComponentInstallPlan,
        },
        fleet_registry_mirror::FleetRegistryMirrorOps,
    },
    view::component_registry::{
        ComponentRegistryPartitionView, RootComponentAllocationProgressView,
        RootComponentAllocationView, RootComponentCreationEffectView,
        RootComponentInstallEffectView, RootComponentRegistryView,
    },
    workflow::{
        bootstrap::root_store, deployment, runtime::template::resolved_root_store_module_source,
    },
};
use canic_core::api::runtime::install::{ApprovedModulePayload, ApprovedModuleSource};
use canic_core::{
    api::fleet_activation::FleetActivationApi,
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::{
            component_runtime::ComponentRuntimeOps,
            config::ConfigOps,
            fleet_registry::FleetRegistryOps,
            ic::{
                IcOps,
                call::CallOps,
                mgmt::{CanisterInstallMode, MgmtOps},
            },
        },
        policy::component_allocation::{
            TopLevelComponentAllocationInput, reserve_top_level_component,
        },
        workflow::{cost_guard::CostGuardWorkflow, runtime::install::ModuleInstallWorkflow},
    },
    dto::{
        abi::v1::{CanisterInitAuthority, CanisterInitPayload},
        component_registry::{
            ComponentDirectoryHead, ComponentDirectoryHeadRequest, ComponentDirectoryProvenance,
            ComponentLifecycleStatus, ComponentProvisioningOrigin, ComponentRegistryHead,
            ComponentRegistryPartitionRequest, ComponentRegistryPartitionResponse,
            ComponentRuntimeActivationRequest, ComponentRuntimeDirectoryAuthority,
            ComponentRuntimeDirectoryPreparationRequest,
            ComponentRuntimeDirectorySynchronizationRequest, ComponentRuntimePhase,
            ComponentRuntimeStatusResponse, RootComponentAllocationPhase,
            RootComponentAllocationRequest, RootComponentAllocationResponse,
            RootComponentAllocationStatusRequest, RootComponentCommitRequest,
            RootComponentCommitResponse, RootComponentCreationEvidence,
            RootComponentCreationRequest, RootComponentDirectoryPreparationRequest,
            RootComponentDirectoryPreparationResponse, RootComponentInstallEvidence,
            RootComponentInstallRequest, RootComponentMembershipActivationRequest,
            RootComponentMembershipActivationResponse, RootComponentRegistryPreparationRequest,
            RootComponentRegistryStatusResponse, RootComponentRuntimeActivationRequest,
            RootComponentRuntimeActivationResponse,
        },
        error::Error,
        fleet_registry::{FleetDirectorySnapshot, FleetSubnetRootEntry, FleetSubnetRootStatus},
        root_store::RootStoreBootstrapResponse,
    },
    ids::{ComponentBinding, ManagedCanisterBinding},
    protocol,
};

struct PreparedComponentRuntimePlan {
    root_binding: canic_core::ids::FleetSubnetRootBinding,
    allocation: RootComponentAllocationView,
    partition: ComponentRegistryPartitionView,
    target_canister: candid::Principal,
    target_binding: ComponentBinding,
    directory_request: ComponentRuntimeDirectoryPreparationRequest,
    directory_authority_hash: [u8; 32],
    maximum_component_registry_bytes: u64,
}

/// Prepare the one empty Component Registry meta record under exact active root authority.
pub async fn prepare(
    request: RootComponentRegistryPreparationRequest,
) -> Result<RootComponentRegistryStatusResponse, InternalError> {
    let (authority, root) = root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;
    validate_active_authority(&authority, root, &request)?;

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
    validate_active_authority(&authority, root, &request)?;

    let prepared = ComponentRegistryOps::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    if prepared.root != authority.binding
        || prepared.prepared_against_registry != request.expected_fleet_registry
        || prepared.release_set != authority.initial_release_set
        || prepared.store_bootstrap != request.store_bootstrap
    {
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
    validate_active_authority(&authority, root, &preparation_request)?;

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
    validate_active_authority(&authority, root, &preparation_request)?;

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
    validate_active_authority(&authority, root, &preparation_request)?;

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
    let fleet_directory = validate_active_authority(&authority, root, &preparation_request)?;

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

    let target_request = ComponentRuntimeActivationRequest {
        operation_id: request.operation_id,
        directory_authority_hash: plan.directory_authority_hash,
    };
    let observed = query_component_runtime_status(plan.target_canister).await?;
    let activated = match validate_target_directory_status(
        &observed,
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )? {
        ComponentRuntimePhase::AwaitingDirectory => {
            return Err(InternalError::unavailable(
                "Component runtime has not retained its Directory authority",
            ));
        }
        ComponentRuntimePhase::DirectoryPrepared => {
            match activate_target_component_runtime(plan.target_canister, target_request).await {
                Ok(status) => status,
                Err(call_error) => {
                    let reconciled = query_component_runtime_status(plan.target_canister).await?;
                    if validate_active_target_runtime_status(
                        &reconciled,
                        &plan.target_binding,
                        &plan.directory_request,
                        plan.directory_authority_hash,
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
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )?;

    let independently_observed = query_component_runtime_status(plan.target_canister).await?;
    let response_target = active_target_runtime_status(
        &independently_observed,
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
    )?;
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
    let observed = query_component_runtime_status(plan.target_canister).await?;
    let synchronized = if validate_target_membership_status(
        &observed,
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
        &synchronization_request,
        active_authority_hash,
    )? {
        observed
    } else {
        match synchronize_target_component_directory(
            plan.target_canister,
            synchronization_request.clone(),
        )
        .await
        {
            Ok(status) => status,
            Err(call_error) => {
                let reconciled = query_component_runtime_status(plan.target_canister).await?;
                if matches!(
                    validate_target_membership_status(
                        &reconciled,
                        &plan.target_binding,
                        &plan.directory_request,
                        plan.directory_authority_hash,
                        &synchronization_request,
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
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
        &synchronization_request,
        active_authority_hash,
    )? {
        return Err(InternalError::unavailable(
            "Component runtime has not retained its active membership Directory",
        ));
    }
    let independently_observed = query_component_runtime_status(plan.target_canister).await?;
    if !validate_target_membership_status(
        &independently_observed,
        &plan.target_binding,
        &plan.directory_request,
        plan.directory_authority_hash,
        &synchronization_request,
        active_authority_hash,
    )? {
        return Err(InternalError::unavailable(
            "Component runtime did not converge on its active membership Directory",
        ));
    }
    let allocation = ComponentRegistryOps::mark_membership_synchronized(
        synchronization_request.operation_id,
        active_authority_hash,
    )?;
    membership_response(allocation, active_partition, independently_observed)
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
        | RootComponentAllocationProgressView::Committed { .. } => {
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

#[derive(Clone, Debug)]
struct ComponentInstallPlan {
    durable: RootComponentInstallPlan,
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
    let fleet_directory = validate_active_authority(&root_authority, root, &preparation_request)?;
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
        target_binding: install.durable.binding,
        directory_request: ComponentRuntimeDirectoryPreparationRequest {
            operation_id,
            authority,
        },
        directory_authority_hash,
        maximum_component_registry_bytes: install.durable.maximum_registry_bytes,
    })
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

fn validate_target_directory_status(
    status: &ComponentRuntimeStatusResponse,
    binding: &ComponentBinding,
    request: &ComponentRuntimeDirectoryPreparationRequest,
    authority_hash: [u8; 32],
) -> Result<ComponentRuntimePhase, InternalError> {
    if status.operation_id != request.operation_id
        || status.binding != ManagedCanisterBinding::Component(binding.clone())
    {
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
    binding: &ComponentBinding,
    request: &ComponentRuntimeDirectoryPreparationRequest,
    authority_hash: [u8; 32],
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    match validate_target_directory_status(status, binding, request, authority_hash)? {
        ComponentRuntimePhase::DirectoryPrepared => Ok(status.clone()),
        ComponentRuntimePhase::Active => Ok(ComponentRuntimeStatusResponse {
            operation_id: request.operation_id,
            binding: ManagedCanisterBinding::Component(binding.clone()),
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
    binding: &ComponentBinding,
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
    binding: &ComponentBinding,
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
        binding: ManagedCanisterBinding::Component(binding.clone()),
        phase: ComponentRuntimePhase::Active,
        authority: Some(request.authority.clone()),
        authority_hash: Some(authority_hash),
        activation: Some(activation),
    })
}

fn validate_target_membership_status(
    status: &ComponentRuntimeStatusResponse,
    binding: &ComponentBinding,
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
    if status.authority.as_ref() == Some(&active_request.authority)
        && status.authority_hash == Some(active_authority_hash)
    {
        return Ok(true);
    }
    if status.authority.as_ref() == Some(&prepared_request.authority)
        && status.authority_hash == Some(prepared_authority_hash)
    {
        return Ok(false);
    }
    Err(InternalError::conflict(
        "Component runtime current Directory differs from prepared and active membership authority",
    ))
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
        } => Ok((creation, *canister)),
        RootComponentAllocationProgressView::Reserved
        | RootComponentAllocationProgressView::CreationIntent(_) => Err(InternalError::conflict(
            "Component allocation must be created before installation",
        )),
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

fn validate_active_authority(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    root: candid::Principal,
    request: &RootComponentRegistryPreparationRequest,
) -> Result<FleetDirectorySnapshot, InternalError> {
    let active = FleetRegistryMirrorOps::current().active.ok_or_else(|| {
        InternalError::unavailable("root has no active Fleet Registry Mirror and Directory")
    })?;
    if active.snapshot.version != request.expected_fleet_registry {
        return Err(InternalError::conflict(
            "active root Registry Mirror differs from Component Registry preparation authority",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    FleetRegistryOps::validate(
        &authority.binding.authority,
        &topology,
        &active.snapshot.registry,
    )?;
    let manifest = FleetRegistryOps::manifest(
        &authority.binding.authority,
        &topology,
        &active.snapshot.registry,
    )?;
    let version = FleetRegistryOps::version(
        &authority.binding.authority,
        &topology,
        &active.snapshot.registry,
    )?;
    let expected_entry = FleetSubnetRootEntry {
        placement_subnet: authority.binding.placement_subnet,
        fleet_subnet_root: root,
        component_admissions: authority.binding.component_admissions.clone(),
        component_topology_digest: authority.binding.component_topology_digest,
        active_release_set: authority.initial_release_set,
        limits: authority.binding.limits.clone(),
        status: FleetSubnetRootStatus::Active,
    };
    let directory = FleetRegistryOps::active_directory_for_root(
        &authority.binding.authority,
        &topology,
        &active.snapshot.registry,
        root,
    )?;
    if active.snapshot.manifest != manifest
        || active.snapshot.version != version
        || !active
            .snapshot
            .registry
            .fleet_subnet_roots
            .iter()
            .any(|entry| entry == &expected_entry)
        || active.directory != directory
    {
        return Err(InternalError::invalid_input(
            "active root Registry Mirror or Fleet Directory differs from protected authority",
        ));
    }
    Ok(directory)
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
        encoded_bytes: prepared.encoded_bytes,
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
    let encoded_bytes_match = membership.registry_encoded_bytes == partition.encoded_bytes;
    if !membership.directory_synchronized
        || !encoded_bytes_match
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
        descendant_count: 0,
    }
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
    if effect.wasm_store != expected.wasm_store
        || effect.payload_hash != expected.payload_hash
        || effect.payload_size_bytes != expected.payload_size_bytes
        || effect.initial_cycles != expected.initial_cycles
        || effect.controller != expected.controller
    {
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
    if effect.raw_module_hash != expected.raw_module_hash
        || effect.chunk_hashes != expected.chunk_hashes
        || effect.binding != expected.binding
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "durable Component install intent differs from verified module or binding authority",
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
    if partition.release_set != release_set
        || !matches!(
            partition.status,
            ComponentLifecycleStatus::Prepared | ComponentLifecycleStatus::Active
        )
        || partition.binding.fleet_subnet_root != root.fleet_subnet_root
        || partition.binding.placement_subnet != root.placement_subnet
        || partition.revision == 0
        || partition.directory_synchronized_at_ns == 0
        || ComponentRegistryOps::component_for_principal(partition.binding.canister_id)
            != Some(partition.binding.component)
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "committed Component partition differs from protected root or principal authority",
        ));
    }
    Ok(())
}
