//! Module: workflow::component_directory_synchronization
//!
//! Responsibility: converge affected existing service members on one published scale-out Registry.
//! Does not own: Coordinator topology, root mirror storage, Component Registry records, or auth policy.
//! Boundary: authenticates the Coordinator, persists intent before each Component call, and records
//! only independently observed exact current-Directory evidence.

use crate::{
    ops::{
        component_directory_synchronization::RootComponentDirectorySynchronizationOps,
        component_provisioning::RootComponentProvisioningOps,
        component_registry::ComponentRegistryOps,
    },
    view::component_directory_synchronization::{
        RootComponentDirectorySynchronizationDisposition,
        RootComponentDirectorySynchronizationIntentView,
    },
    workflow::{component_registry, root_authority::validated_root_authority},
};
use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::{
            component_provisioning_receipt::RootComponentProvisioningReceiptOps,
            component_runtime::ComponentRuntimeOps, fleet_registry::FleetRegistryOps, ic::IcOps,
        },
        workflow::runtime::fleet_activation::FleetActivationWorkflow,
    },
    dto::{
        component_deployment::ProtectedComponentDeployment,
        component_provisioning::{
            RootComponentDirectorySynchronizationRequest,
            RootComponentDirectorySynchronizationResponse,
        },
        component_registry::ComponentRuntimeDirectorySynchronizationRequest,
        fleet_activation::FleetActivationPhase,
    },
    ids::ManagedCanisterBinding,
};

/// Advance one exact affected-root Directory synchronization step.
pub async fn synchronize(
    caller: Principal,
    request: RootComponentDirectorySynchronizationRequest,
) -> Result<RootComponentDirectorySynchronizationResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    require_coordinator(caller, authority.binding.authority.binding.coordinator)?;
    if FleetActivationWorkflow::status()?.phase != FleetActivationPhase::Active {
        return Err(InternalError::conflict(
            "scale-out Directory synchronization requires an Active root runtime",
        ));
    }
    let registry = ComponentRegistryOps::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    RootComponentDirectorySynchronizationOps::validate_command(&request)?;

    let prepared = RootComponentDirectorySynchronizationOps::is_prepared(request.operation_id)
        .then(|| RootComponentDirectorySynchronizationOps::status(&request))
        .transpose()?;
    let mirror = if prepared.is_some() {
        super::fleet_registry_mirror::advance_for_component_publication(
            request.source_fleet_registry.clone(),
            request.published_fleet_registry.clone(),
            registry.store_bootstrap.clone(),
        )
        .await?
    } else {
        prepare_and_commit(&request, root, registry.store_bootstrap.clone()).await?
    };
    if mirror.fleet_subnet_root != root || mirror.version != request.published_fleet_registry {
        return Err(InternalError::conflict(
            "root Fleet Registry mirror differs from Directory synchronization authority",
        ));
    }
    let view = prepared.map_or_else(
        || RootComponentDirectorySynchronizationOps::status(&request),
        Ok,
    )?;
    if view.fleet_directory_content_hash
        != RootComponentProvisioningReceiptOps::fleet_directory_content_hash(&mirror.directory)?
    {
        return Err(InternalError::conflict(
            "root Fleet Directory differs from the durable synchronization authority",
        ));
    }
    let started_at_ns = IcOps::now_nanos();
    let proposed = if view.in_flight.is_none() && !view.complete {
        next_intent(&view, &mirror.directory, started_at_ns)?
    } else {
        None
    };
    let disposition =
        RootComponentDirectorySynchronizationOps::advance(&request, proposed, started_at_ns)?;
    let intent = match disposition {
        RootComponentDirectorySynchronizationDisposition::Current(response) => {
            return Ok(*response);
        }
        RootComponentDirectorySynchronizationDisposition::Invoke(intent)
        | RootComponentDirectorySynchronizationDisposition::Reconcile(intent) => intent,
    };
    synchronize_target(&request, &mirror.directory, &intent).await
}

async fn prepare_and_commit(
    request: &RootComponentDirectorySynchronizationRequest,
    root: Principal,
    store_bootstrap: canic_core::dto::root_store::RootStoreBootstrapRequest,
) -> Result<
    canic_core::dto::fleet_registry::FleetSubnetRootRegistryMirrorActivationResponse,
    InternalError,
> {
    let prepared = super::fleet_registry_mirror::prepare_component_publication_transition(
        request.source_fleet_registry.clone(),
        request.published_fleet_registry.clone(),
        store_bootstrap,
    )
    .await?;
    let affected = FleetRegistryOps::affected_existing_service_components(
        &prepared.source.registry,
        &prepared.target.registry,
        root,
    )?;
    let targets = ComponentRegistryOps::directory_synchronization_targets(&affected)?;
    let fleet_directory_content_hash =
        RootComponentProvisioningReceiptOps::fleet_directory_content_hash(&prepared.directory)?;
    RootComponentDirectorySynchronizationOps::accept(
        request,
        root,
        fleet_directory_content_hash,
        targets,
        IcOps::now_nanos(),
    )?;
    super::fleet_registry_mirror::commit_component_publication_transition(&prepared)
}

fn next_intent(
    view: &crate::view::component_directory_synchronization::RootComponentDirectorySynchronizationView,
    fleet_directory: &canic_core::dto::fleet_registry::FleetDirectorySnapshot,
    started_at_ns: u64,
) -> Result<Option<RootComponentDirectorySynchronizationIntentView>, InternalError> {
    let index = usize::try_from(view.synchronized_component_count).map_err(|_| {
        InternalError::resource_exhausted("Component Directory cursor exceeds usize")
    })?;
    let Some(target) = view.targets.get(index) else {
        return Ok(None);
    };
    let allocation =
        ComponentRegistryOps::allocation(target.allocation_operation_id).ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "affected service Component allocation disappeared",
            )
        })?;
    let retained = RootComponentProvisioningOps::component_group_runtime_authority(&allocation)?;
    let plan = ComponentRegistryOps::prepare_directory_refresh(
        target,
        fleet_directory.clone(),
        Some(retained.component_group),
        started_at_ns,
    )?;
    Ok(Some(RootComponentDirectorySynchronizationIntentView {
        component_index: view.synchronized_component_count,
        component: target.component,
        canister_id: target.canister_id,
        allocation_operation_id: target.allocation_operation_id,
        previous_registry: plan.previous_registry,
        registry: plan.registry,
        directory_synchronized_at_ns: plan.directory_synchronized_at_ns,
        directory_authority_hash: plan.directory_authority_hash,
        started_at_ns,
    }))
}

async fn synchronize_target(
    request: &RootComponentDirectorySynchronizationRequest,
    fleet_directory: &canic_core::dto::fleet_registry::FleetDirectorySnapshot,
    intent: &RootComponentDirectorySynchronizationIntentView,
) -> Result<RootComponentDirectorySynchronizationResponse, InternalError> {
    let allocation =
        ComponentRegistryOps::allocation(intent.allocation_operation_id).ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "affected service Component allocation disappeared",
            )
        })?;
    let retained = RootComponentProvisioningOps::component_group_runtime_authority(&allocation)?;
    let (binding, maximum_registry_bytes) = group_member_runtime_limits(&retained.deployment)?;
    let plan = ComponentRegistryOps::directory_refresh_plan_for_intent(
        intent,
        fleet_directory.clone(),
        Some(retained.component_group),
    )?;
    let partition = ComponentRegistryOps::commit_directory_refresh(&plan, maximum_registry_bytes)?;
    if component_registry::component_directory_head(&partition) != plan.authority.component {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "committed Component Directory refresh differs from its durable plan",
        ));
    }
    let runtime_request = ComponentRuntimeDirectorySynchronizationRequest {
        operation_id: intent.allocation_operation_id,
        authority: plan.authority,
        direct_children: component_registry::active_component_direct_children(
            &partition,
            intent.canister_id,
        )?,
    };
    let synchronized = component_registry::synchronize_grouped_component_directory(
        intent.canister_id,
        &ManagedCanisterBinding::Component(binding),
        &retained.deployment,
        &runtime_request,
        intent.directory_authority_hash,
    )
    .await?;
    validate_synchronized_target_coverage(intent, fleet_directory, &partition, &synchronized)?;
    RootComponentDirectorySynchronizationOps::record_synchronized(
        request,
        intent,
        IcOps::now_nanos(),
    )
}

fn validate_synchronized_target_coverage(
    intent: &RootComponentDirectorySynchronizationIntentView,
    fleet_directory: &canic_core::dto::fleet_registry::FleetDirectorySnapshot,
    committed_partition: &crate::view::component_registry::ComponentRegistryPartitionView,
    status: &canic_core::dto::component_registry::ComponentRuntimeStatusResponse,
) -> Result<(), InternalError> {
    let current = ComponentRegistryOps::partition(intent.component)?.ok_or_else(|| {
        InternalError::unavailable("synchronized service Component partition is absent")
    })?;
    let authority = status.authority.as_ref().ok_or_else(|| {
        InternalError::conflict("synchronized service Component has no current Directory")
    })?;
    let authority_hash = ComponentRuntimeOps::directory_authority_hash(authority)?;
    let direct_children =
        component_registry::active_component_direct_children(&current, intent.canister_id)?;
    let exact_or_later_registry = current.revision >= committed_partition.revision
        && (current.revision != committed_partition.revision
            || current.content_hash == committed_partition.content_hash);
    let coverage_is_current = [
        authority.fleet == *fleet_directory,
        authority.component == component_registry::component_directory_head(&current),
        exact_or_later_registry,
        status.authority_hash == Some(authority_hash),
        status.direct_children_hash
            == Some(ComponentRuntimeOps::direct_children_hash(&direct_children)?),
    ]
    .into_iter()
    .all(|matches| matches);
    if !coverage_is_current {
        return Err(InternalError::conflict(
            "active service Component Directory does not cover durable synchronization intent",
        ));
    }
    Ok(())
}

fn group_member_runtime_limits(
    deployment: &ProtectedComponentDeployment,
) -> Result<(canic_core::ids::ComponentBinding, u64), InternalError> {
    let ProtectedComponentDeployment::GroupMember {
        binding, limits, ..
    } = deployment
    else {
        return Err(InternalError::conflict(
            "Fleet-service member is not protected by a Component Group deployment",
        ));
    };
    Ok((binding.clone(), limits.maximum_registry_bytes))
}

fn require_coordinator(caller: Principal, coordinator: Principal) -> Result<(), InternalError> {
    if caller == coordinator {
        return Ok(());
    }
    Err(InternalError::forbidden(
        "root Component Directory synchronization requires the protected Fleet Coordinator",
    ))
}
