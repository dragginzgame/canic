//! Module: workflow::component_provisioning
//!
//! Responsibility: authenticate and advance one exact Coordinator-planned root batch.
//! Does not own: stable records, pool state, service publication, or target-local lifecycle state.
//! Boundary: each bounded member step revalidates protected authority before delegating to the
//! existing root-local lifecycle journals.

use crate::{
    ops::{
        canister_pool::CanisterPoolOps, component_provisioning::RootComponentProvisioningOps,
        component_registry::ComponentRegistryOps, fleet_registry_mirror::FleetRegistryMirrorOps,
    },
    view::{
        component_provisioning::{
            RootComponentProvisioningAdvanceDisposition, RootComponentProvisioningMemberView,
            RootComponentProvisioningRuntimeMode, RootComponentProvisioningView,
        },
        component_registry::{
            RootComponentAllocationView, RootComponentInitialInventoryView,
            RootComponentRegistryView,
        },
    },
    workflow::{
        bootstrap::root_store, root_authority::validated_root_authority,
        runtime::fleet_activation as root_fleet_activation,
    },
};
use candid::{CandidType, Principal};
use canic_core::{
    api::timer::TimerApi,
    control_plane_support::{
        error::InternalError,
        ops::{
            component_provisioning_plan::{
                ComponentProvisioningPlanOps, RootComponentProvisioningBatchValidation,
            },
            component_runtime::ComponentRuntimeOps,
            config::ConfigOps,
            ic::{IcOps, call::CallOps},
        },
        workflow::runtime::fleet_activation::FleetActivationWorkflow,
    },
    dto::{
        component_provisioning::{
            RootComponentActivationEvidence, RootComponentActivationRequest,
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningAdvanceRequest,
            RootComponentProvisioningPhase, RootComponentProvisioningStatusRequest,
            RootComponentProvisioningStatusResponse, RootComponentPublicationRequest,
        },
        component_registry::{
            ComponentLifecycleStatus, ComponentProvisioningOrigin,
            ComponentRuntimeDirectoryAuthority, ComponentRuntimeDirectoryPreparationRequest,
            RootComponentAllocationRequest, RootComponentMembershipActivationRequest,
            RootComponentRuntimeActivationRequest,
        },
        error::Error,
        fleet_activation::{
            FleetActivationPhase, FleetActivationResumeRequest, FleetActivationStatusResponse,
        },
        fleet_registry::FleetSubnetRootStatus,
        fleet_subnet_root::FleetSubnetRootAuthority,
        role::{OperationReceipt, OperationStatusRequest},
    },
    ids::ManagedCanisterBinding,
    log::Topic,
    protocol,
};
use serde::Deserialize;
use std::time::Duration;

#[derive(CandidType)]
enum RemoteCoordinatorStatusRequest {
    Operation(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RemoteCoordinatorStatusResponse {
    Operation(RemoteCoordinatorOperationStatusResponse),
}

#[derive(CandidType, Deserialize)]
enum RemoteCoordinatorOperationStatusResponse {
    ComponentProvisioning(
        canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
    ),
}

/// Durably accept one complete root batch under the exact protected Coordinator.
pub async fn accept(
    caller: Principal,
    request: RootComponentProvisioningAcceptanceRequest,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    require_coordinator(caller, authority.binding.authority.binding.coordinator)?;
    if let Some(existing) = RootComponentProvisioningOps::acceptance_replay(&request)? {
        return Ok(crate::ops::component_provisioning::status_response(
            existing,
        ));
    }
    crate::workflow::root_admission::require_catalog_mutation_allowed()?;
    RootComponentProvisioningOps::require_acceptance_open(request.operation_id)?;

    let mirror = FleetRegistryMirrorOps::validated_current(&authority, root)?;
    if mirror.root_entry.status != FleetSubnetRootStatus::Active
        || mirror.active.snapshot.version != request.fleet_registry
    {
        return Err(InternalError::conflict());
    }
    let config = ConfigOps::get()?;
    let validation = ComponentProvisioningPlanOps::validate_root_batch(
        &config,
        &mirror.active.snapshot.registry,
        &request.fleet_registry,
        request.configuration_digest,
        &authority.binding,
        &request.batch,
    )?;
    let _canonical_batch = ComponentProvisioningPlanOps::root_batch_canonical_bytes(
        &config,
        &mirror.active.snapshot.registry,
        &request.fleet_registry,
        request.configuration_digest,
        &authority.binding,
        &request.batch,
    )?;

    let acceptance = current_registry_for_acceptance(&authority, root, &request, &validation)?;

    let store = root_store::status(acceptance.registry.store_bootstrap.clone()).await?;
    validate_store_artifacts(&store, &validation.component_roles)?;
    let revalidated = current_registry_for_acceptance(&authority, root, &request, &validation)?;
    if revalidated != acceptance {
        return Err(InternalError::conflict());
    }
    let cycle_demands =
        ComponentProvisioningPlanOps::root_batch_initial_cycle_demands(&config, &request.batch)?;
    if u32::try_from(cycle_demands.len()).ok() != Some(validation.component_count) {
        return Err(InternalError::invariant());
    }
    require_ready_pool_capacity(&authority.binding.limits.canister_pool, &cycle_demands).await?;
    let final_registry = current_registry_for_acceptance(&authority, root, &request, &validation)?;
    if final_registry != acceptance {
        return Err(InternalError::conflict());
    }
    let accepted = RootComponentProvisioningOps::accept(
        request,
        &validation,
        acceptance.runtime_mode,
        IcOps::now_nanos(),
    )?;
    Ok(crate::ops::component_provisioning::status_response(
        accepted,
    ))
}

/// Authorize the protected Coordinator before a Root provisioning workflow is entered.
pub fn authorize_coordinator_caller(caller: Principal) -> Result<(), InternalError> {
    let (authority, _) = validated_root_authority()?;
    require_coordinator(caller, authority.binding.authority.binding.coordinator)
}

/// Accept one high-level root batch and privately schedule its local provisioning work.
pub async fn accept_and_schedule(
    caller: Principal,
    request: RootComponentProvisioningAcceptanceRequest,
) -> Result<OperationReceipt, InternalError> {
    let operation_id = request.operation_id;
    let plan_hash = request.plan_hash;
    let status = Box::pin(accept(caller, request)).await?;
    if status.operation_id != operation_id || status.plan_hash != plan_hash {
        return Err(InternalError::invariant());
    }
    schedule_provisioning(operation_id, plan_hash, Duration::ZERO);
    Ok(OperationReceipt { operation_id })
}

fn schedule_provisioning(operation_id: [u8; 32], plan_hash: [u8; 32], delay: Duration) {
    TimerApi::defer_lifecycle_required(
        delay,
        "Fleet Subnet Root Component provisioning",
        async move {
            Box::pin(advance_scheduled_provisioning(operation_id, plan_hash)).await;
        },
    );
}

#[expect(
    clippy::too_many_lines,
    reason = "one private phase dispatcher advances the sole durable Root provisioning operation"
)]
async fn advance_scheduled_provisioning(operation_id: [u8; 32], plan_hash: [u8; 32]) {
    let Ok(current) =
        RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
            operation_id,
            plan_hash,
        })
    else {
        return;
    };
    let coordinator = match validated_root_authority() {
        Ok((authority, _root)) => authority.binding.authority.binding.coordinator,
        Err(_) => return,
    };
    let result = match current.phase {
        RootComponentProvisioningPhase::Accepted => {
            advance(
                coordinator,
                RootComponentProvisioningAdvanceRequest {
                    operation_id,
                    plan_hash,
                    expected_reserved_component_count: current
                        .reservation_cursor
                        .reserved_component_count,
                    expected_claimed_component_count: current.claim_cursor.claimed_component_count,
                    expected_installed_component_count: current
                        .install_cursor
                        .installed_component_count,
                    expected_registry_committed_component_count: current
                        .registry_cursor
                        .registry_committed_component_count,
                },
            )
            .await
        }
        RootComponentProvisioningPhase::Provisioned => {
            let coordinator_status =
                query_coordinator_provisioning(coordinator, operation_id, plan_hash).await;
            match coordinator_status {
                Ok(status) => match status.published_fleet_registry {
                    Some(published_fleet_registry) => {
                        Box::pin(publish(
                            coordinator,
                            RootComponentPublicationRequest {
                                operation_id,
                                plan_hash,
                                published_fleet_registry,
                                expected_published_component_count: current
                                    .published_component_count,
                            },
                        ))
                        .await
                    }
                    None => Err(InternalError::unavailable()),
                },
                Err(error) => Err(error),
            }
        }
        RootComponentProvisioningPhase::Published => {
            let coordinator_status =
                query_coordinator_provisioning(coordinator, operation_id, plan_hash).await;
            match coordinator_status {
                Ok(status)
                    if matches!(
                        status.phase,
                        canic_core::dto::component_provisioning::FleetComponentProvisioningPhase::DirectoriesConfirmed
                            | canic_core::dto::component_provisioning::FleetComponentProvisioningPhase::ActivatingRuntimes
                            | canic_core::dto::component_provisioning::FleetComponentProvisioningPhase::RuntimesActivated
                    ) =>
                {
                    activate(
                        coordinator,
                        RootComponentActivationRequest {
                            operation_id,
                            plan_hash,
                            expected_activated_component_count: current.activated_component_count,
                            expected_root_runtime_active: current.root_runtime_active,
                        },
                    )
                    .await
                }
                Ok(_) => Err(InternalError::unavailable()),
                Err(error) => Err(error),
            }
        }
        RootComponentProvisioningPhase::RuntimesActive => return,
    };
    match result {
        Ok(status) if status.phase == RootComponentProvisioningPhase::RuntimesActive => {}
        Ok(_) => schedule_provisioning(operation_id, plan_hash, Duration::ZERO),
        Err(error) => {
            canic_core::log!(
                Topic::Fleet,
                Warn,
                "Root Component provisioning retry: operation_id={operation_id:?} phase={:?} activated_components={}/{} diagnostic={}",
                current.phase,
                current.activated_component_count,
                current.component_count,
                error.code()
            );
            schedule_provisioning(operation_id, plan_hash, Duration::from_secs(1));
        }
    }
}

async fn query_coordinator_provisioning(
    coordinator: Principal,
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
) -> Result<
    canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
    InternalError,
> {
    let call = CallOps::unbounded_wait(coordinator, protocol::CANIC_STATUS)
        .with_arg(RemoteCoordinatorStatusRequest::Operation(
            OperationStatusRequest { operation_id },
        ))?
        .execute()
        .await?;
    let result: Result<RemoteCoordinatorStatusResponse, Error> = call.candid()?;
    match result.map_err(InternalError::observed_public)? {
        RemoteCoordinatorStatusResponse::Operation(
            RemoteCoordinatorOperationStatusResponse::ComponentProvisioning(status),
        ) if status.operation_id == operation_id && status.plan_hash == plan_hash => Ok(status),
        RemoteCoordinatorStatusResponse::Operation(_) => Err(InternalError::conflict()),
    }
}

/// Read one exact durable acceptance receipt under Coordinator authentication.
pub fn status(
    caller: Principal,
    request: RootComponentProvisioningStatusRequest,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let (authority, _root) = validated_root_authority()?;
    require_coordinator(caller, authority.binding.authority.binding.coordinator)?;
    let current = RootComponentProvisioningOps::status(request)?;
    require_next_claim_capacity(&authority, &current)?;
    Ok(crate::ops::component_provisioning::status_response(current))
}

/// Advance one canonical lifecycle step or freeze the complete provisioned result.
pub async fn advance(
    caller: Principal,
    request: RootComponentProvisioningAdvanceRequest,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    require_coordinator(caller, authority.binding.authority.binding.coordinator)?;
    let current = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })?;
    match RootComponentProvisioningOps::advance_disposition(request, &current)? {
        RootComponentProvisioningAdvanceDisposition::Complete
        | RootComponentProvisioningAdvanceDisposition::Replay => {
            return Ok(crate::ops::component_provisioning::status_response(current));
        }
        RootComponentProvisioningAdvanceDisposition::Advance => {}
    }

    let advanced = if current.reservation_cursor.reserved_component_count < current.component_count
    {
        advance_member_reservation(&authority, root, request, &current)?
    } else if current.claim_cursor.claimed_component_count < current.component_count {
        advance_member_claim(&authority, root, request, &current).await?
    } else if current.install_cursor.installed_component_count < current.component_count {
        Box::pin(advance_member_install(&authority, root, request, &current)).await?
    } else if current.registry_cursor.registry_committed_component_count < current.component_count {
        Box::pin(advance_member_registry_commit(
            &authority, root, request, &current,
        ))
        .await?
    } else {
        let _registry = current_registry_for_progress(&authority, root, &current)?;
        RootComponentProvisioningOps::finalize_provisioned(request, IcOps::now_nanos())?
    };
    Ok(crate::ops::component_provisioning::status_response(
        advanced,
    ))
}

/// Advance one exact Fleet/Component/Component Group Directory publication step.
pub async fn publish(
    caller: Principal,
    request: RootComponentPublicationRequest,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    require_coordinator(caller, authority.binding.authority.binding.coordinator)?;
    let runtime = FleetActivationWorkflow::status()?;
    let before = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })?;
    let registry = ComponentRegistryOps::current().ok_or_else(InternalError::unavailable)?;
    let runtime_mode = validate_component_registry_authority(
        &registry,
        &authority.binding,
        authority.initial_release_set,
        &request.published_fleet_registry,
        runtime.phase,
        runtime.identity.operation_id,
    )?;
    require_runtime_mode(before.runtime_mode, runtime_mode)?;
    let mirror = super::fleet_registry_mirror::advance_for_component_publication(
        before.fleet_registry.clone(),
        request.published_fleet_registry.clone(),
        registry.store_bootstrap.clone(),
    )
    .await?;
    if mirror.fleet_subnet_root != root || mirror.version != request.published_fleet_registry {
        return Err(InternalError::conflict());
    }
    let current = RootComponentProvisioningOps::begin_publication(
        &request,
        &mirror.directory,
        IcOps::now_nanos(),
    )?;
    if request.expected_published_component_count < current.published_component_count
        || current.phase
            == canic_core::dto::component_provisioning::RootComponentProvisioningPhase::Published
    {
        return Ok(crate::ops::component_provisioning::status_response(current));
    }
    let Some(member) = RootComponentProvisioningOps::next_publication_member(&current)? else {
        return RootComponentProvisioningOps::finalize_published(&request, IcOps::now_nanos())
            .map(crate::ops::component_provisioning::status_response);
    };
    publish_component_directory(&request, member, mirror.directory).await
}

/// Activate one exact grouped Component step or the root runtime after publication.
pub async fn activate(
    caller: Principal,
    request: RootComponentActivationRequest,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let (authority, _root) = validated_root_authority()?;
    require_coordinator(caller, authority.binding.authority.binding.coordinator)?;
    let before = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })?;
    if before.phase == RootComponentProvisioningPhase::RuntimesActive {
        return RootComponentProvisioningOps::begin_activation(&request, IcOps::now_nanos())
            .map(crate::ops::component_provisioning::status_response);
    }
    validate_activation_runtime_authority(&authority, &before)?;
    let current = RootComponentProvisioningOps::begin_activation(&request, IcOps::now_nanos())?;
    if request.expected_activated_component_count < current.activated_component_count
        || request.expected_root_runtime_active != current.root_runtime_active
        || current.phase == RootComponentProvisioningPhase::RuntimesActive
    {
        return Ok(crate::ops::component_provisioning::status_response(current));
    }
    if let Some(member) = RootComponentProvisioningOps::next_activation_member(&current)? {
        return Box::pin(activate_component_step(&request, member)).await;
    }
    Box::pin(activate_root_runtime(&request)).await
}

async fn activate_component_step(
    request: &RootComponentActivationRequest,
    member: crate::view::component_provisioning::RootComponentPublicationMemberView,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let provisioning_origin = activation_member_origin(request, &member)
        .map_err(|error| activation_member_failure("origin", &member, error))?;
    let allocation = ComponentRegistryOps::allocation(member.member_operation_id)
        .ok_or_else(InternalError::unavailable)
        .map_err(|error| activation_member_failure("allocation", &member, error))?;
    let runtime_active = match &allocation.progress {
        crate::view::component_registry::RootComponentAllocationProgressView::Committed {
            commitment,
            ..
        } => commitment.runtime_activated,
        _ => false,
    };
    if !runtime_active {
        Box::pin(super::component_registry::activate_group_member_runtime(
            RootComponentRuntimeActivationRequest {
                operation_id: member.member_operation_id,
            },
            &provisioning_origin,
            &member.deployment,
            &member.component_group,
        ))
        .await
        .map_err(|error| activation_member_failure("runtime", &member, error))?;
    }
    Box::pin(super::component_registry::activate_group_member_membership(
        RootComponentMembershipActivationRequest {
            operation_id: member.member_operation_id,
        },
        &provisioning_origin,
        &member.deployment,
        &member.component_group,
    ))
    .await
    .map_err(|error| activation_member_failure("membership", &member, error))?;
    RootComponentProvisioningOps::mark_member_activated(request, &member)
        .map_err(|error| activation_member_failure("commit", &member, error))
        .map(crate::ops::component_provisioning::status_response)
}

fn activation_member_failure(
    stage: &'static str,
    member: &crate::view::component_provisioning::RootComponentPublicationMemberView,
    error: InternalError,
) -> InternalError {
    canic_core::log!(
        Topic::Fleet,
        Error,
        "Root Component activation failed stage={stage} component_index={} canister={} operation_id={:?} diagnostic={}",
        member.component_index,
        member.binding.canister_id,
        member.member_operation_id,
        error.code()
    );
    error
}

fn activation_member_origin(
    request: &RootComponentActivationRequest,
    member: &crate::view::component_provisioning::RootComponentPublicationMemberView,
) -> Result<ComponentProvisioningOrigin, InternalError> {
    let canic_core::dto::component_deployment::ProtectedComponentDeployment::GroupMember {
        group_placement,
        member_path,
        ..
    } = &member.deployment
    else {
        return Err(InternalError::invariant());
    };
    Ok(ComponentProvisioningOrigin::ComponentGroup {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
        group_placement: group_placement.clone(),
        member_path: member_path.clone(),
    })
}

async fn activate_root_runtime(
    request: &RootComponentActivationRequest,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let provisioning =
        RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
            operation_id: request.operation_id,
            plan_hash: request.plan_hash,
        })?;
    let observed = FleetActivationWorkflow::status()?;
    match provisioning.runtime_mode {
        RootComponentProvisioningRuntimeMode::FreshRoot => {
            Box::pin(activate_fresh_root_runtime(
                request,
                &provisioning,
                observed,
            ))
            .await
        }
        RootComponentProvisioningRuntimeMode::ActiveRoot => {
            activate_active_root_batch(request, &provisioning, observed)
        }
    }
}

async fn activate_fresh_root_runtime(
    request: &RootComponentActivationRequest,
    provisioning: &RootComponentProvisioningView,
    observed: FleetActivationStatusResponse,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let prepared =
        if observed.phase == FleetActivationPhase::Prepared && observed.credential.is_none() {
            root_fleet_activation::prepare_root().await?
        } else {
            observed
        };
    let active = if prepared.phase == FleetActivationPhase::Prepared {
        let credential = prepared.credential.ok_or_else(InternalError::unavailable)?;
        Box::pin(root_fleet_activation::resume_root(
            FleetActivationResumeRequest {
                operation_id: prepared.identity.operation_id,
                credential,
            },
        ))
        .await?
        .status
    } else {
        prepared
    };
    let activated_at_ns = active
        .activated_at_ns
        .ok_or_else(InternalError::unavailable)?;
    let inventory = ComponentRegistryOps::initial_inventory(active.identity.operation_id)?;
    let activation_is_terminal = active.phase == FleetActivationPhase::Active
        && inventory.directories_converged
        && inventory.root_runtime_activated
        && inventory.component_count == provisioning.component_count;
    if !activation_is_terminal {
        return Err(InternalError::conflict());
    }
    RootComponentProvisioningOps::finalize_runtimes_active(
        request,
        RootComponentActivationEvidence {
            fleet_activation_operation_id: active.identity.operation_id,
            initial_inventory_hash: inventory.inventory_hash,
            component_count: provisioning.component_count,
            root_activated_at_ns: activated_at_ns,
        },
        activated_at_ns,
    )
    .map(crate::ops::component_provisioning::status_response)
}

fn activate_active_root_batch(
    request: &RootComponentActivationRequest,
    provisioning: &RootComponentProvisioningView,
    active: FleetActivationStatusResponse,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let activated_at_ns = active
        .activated_at_ns
        .ok_or_else(InternalError::unavailable)?;
    let inventory = ComponentRegistryOps::initial_inventory(active.identity.operation_id)?;
    let initial_runtime_is_exact = active.phase == FleetActivationPhase::Active
        && activated_at_ns > 0
        && activated_at_ns <= provisioning.accepted_at_ns
        && inventory.directories_converged
        && inventory.root_runtime_activated;
    if !initial_runtime_is_exact {
        return Err(InternalError::conflict());
    }
    let completed_at_ns = IcOps::now_nanos();
    RootComponentProvisioningOps::finalize_runtimes_active(
        request,
        RootComponentActivationEvidence {
            fleet_activation_operation_id: active.identity.operation_id,
            initial_inventory_hash: inventory.inventory_hash,
            component_count: provisioning.component_count,
            root_activated_at_ns: activated_at_ns,
        },
        completed_at_ns,
    )
    .map(crate::ops::component_provisioning::status_response)
}

async fn publish_component_directory(
    request: &RootComponentPublicationRequest,
    member: crate::view::component_provisioning::RootComponentPublicationMemberView,
    fleet_directory: canic_core::dto::fleet_registry::FleetDirectorySnapshot,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let partition = ComponentRegistryOps::partition(member.binding.component)?
        .ok_or_else(InternalError::invariant)?;
    validate_publication_partition(&member, &partition)?;
    let allocation = ComponentRegistryOps::allocation(member.member_operation_id)
        .ok_or_else(InternalError::invariant)?;
    let retained = RootComponentProvisioningOps::component_group_runtime_authority(&allocation)?;
    if retained.deployment != member.deployment
        || retained.component_group != member.component_group
    {
        return Err(InternalError::conflict());
    }
    let previous_directory_authority_hash =
        super::component_registry::committed_directory_receipt(&allocation)?
            .directory_authority_hash;
    let directory_request = ComponentRuntimeDirectoryPreparationRequest {
        operation_id: member.member_operation_id,
        authority: ComponentRuntimeDirectoryAuthority {
            fleet: fleet_directory,
            component: super::component_registry::component_directory_head(&partition),
            component_group: Some(member.component_group.clone()),
        },
        direct_children: super::component_registry::active_component_direct_children(
            &partition,
            member.binding.canister_id,
        )?,
    };
    let directory_authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&directory_request.authority)?;
    let current = RootComponentProvisioningOps::begin_publication_delivery(
        request,
        &member,
        directory_authority_hash,
        IcOps::now_nanos(),
    )?;
    let intent = current
        .publication_in_flight
        .as_ref()
        .ok_or_else(InternalError::invariant)?;
    if intent.component_index != member.component_index
        || intent.canister_id != member.binding.canister_id
        || intent.directory_authority_hash != directory_authority_hash
    {
        return Err(InternalError::conflict());
    }
    let binding = ManagedCanisterBinding::Component(member.binding.clone());
    let _observed = super::component_registry::prepare_grouped_component_directories(
        member.binding.canister_id,
        &binding,
        &member.deployment,
        &directory_request,
        directory_authority_hash,
    )
    .await?;
    let allocation = ComponentRegistryOps::record_group_directory_prepared(
        member.member_operation_id,
        previous_directory_authority_hash,
        directory_authority_hash,
    )?;
    let receipt = super::component_registry::committed_directory_receipt(&allocation)?;
    if !receipt.directory_prepared || receipt.directory_authority_hash != directory_authority_hash {
        return Err(InternalError::invariant());
    }
    RootComponentProvisioningOps::record_publication_delivery(
        request,
        &member,
        directory_authority_hash,
    )
    .map(crate::ops::component_provisioning::status_response)
}

fn validate_publication_partition(
    member: &crate::view::component_provisioning::RootComponentPublicationMemberView,
    partition: &crate::view::component_registry::ComponentRegistryPartitionView,
) -> Result<(), InternalError> {
    if partition.status != ComponentLifecycleStatus::Prepared
        || partition.binding != member.binding
        || partition.revision != member.component_registry_revision
        || partition.content_hash != member.component_registry_content_hash
    {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn advance_member_reservation(
    authority: &FleetSubnetRootAuthority,
    root: Principal,
    request: RootComponentProvisioningAdvanceRequest,
    current: &RootComponentProvisioningView,
) -> Result<RootComponentProvisioningView, InternalError> {
    let registry = current_registry_for_progress(authority, root, current)?;
    let member = RootComponentProvisioningOps::next_member_reservation(current)?;
    let existing = ComponentRegistryOps::allocation(member.member_operation_id);
    validate_reservation_registry_progress(
        registry.reserved_component_instances,
        current.reservation_cursor.reserved_component_count,
        existing.is_some(),
    )?;
    let topology = ConfigOps::component_topology()?;
    let allocation = match existing {
        Some(allocation) => allocation,
        None => reserve_group_member(authority, &registry, current, &member, &topology)?,
    };
    super::component_registry::validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        member.member_operation_id,
    )?;
    RootComponentProvisioningOps::mark_member_reserved(request, &allocation)
}

async fn advance_member_claim(
    authority: &FleetSubnetRootAuthority,
    root: Principal,
    request: RootComponentProvisioningAdvanceRequest,
    current: &RootComponentProvisioningView,
) -> Result<RootComponentProvisioningView, InternalError> {
    let registry = current_registry_for_progress(authority, root, current)?;
    validate_claim_registry_progress(&registry, current.component_count)?;
    let member = RootComponentProvisioningOps::next_member_claim(current)?;
    let allocation = ComponentRegistryOps::allocation(member.member_operation_id)
        .ok_or_else(InternalError::invariant)?;
    let topology = ConfigOps::component_topology()?;
    super::component_registry::validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        member.member_operation_id,
    )?;

    let store = root_store::status(registry.store_bootstrap.clone()).await?;
    let revalidated = current_registry_for_progress(authority, root, current)?;
    if revalidated.store_bootstrap != registry.store_bootstrap {
        return Err(InternalError::conflict());
    }
    let latest = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })?;
    match RootComponentProvisioningOps::advance_disposition(request, &latest)? {
        RootComponentProvisioningAdvanceDisposition::Complete
        | RootComponentProvisioningAdvanceDisposition::Replay => return Ok(latest),
        RootComponentProvisioningAdvanceDisposition::Advance => {}
    }

    let allocation = ComponentRegistryOps::allocation(member.member_operation_id)
        .ok_or_else(InternalError::invariant)?;
    super::component_registry::validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        member.member_operation_id,
    )?;
    let claimed =
        super::component_registry::advance_group_member_creation(root, &store, allocation)?;
    let context =
        RootComponentProvisioningOps::member_deployment_context(&latest, &member, &claimed)?;
    validate_group_member_context(&context)?;
    RootComponentProvisioningOps::mark_member_claimed(request, &claimed)
}

async fn advance_member_install(
    authority: &FleetSubnetRootAuthority,
    root: Principal,
    request: RootComponentProvisioningAdvanceRequest,
    current: &RootComponentProvisioningView,
) -> Result<RootComponentProvisioningView, InternalError> {
    let registry = current_registry_for_progress(authority, root, current)?;
    validate_claim_registry_progress(&registry, current.component_count)?;
    let member = RootComponentProvisioningOps::next_member_install(current)?;
    let allocation = required_member_allocation(member.member_operation_id, "install")?;
    let topology = ConfigOps::component_topology()?;
    super::component_registry::validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        member.member_operation_id,
    )?;
    let deployment =
        RootComponentProvisioningOps::member_deployment_context(current, &member, &allocation)?;
    validate_group_member_context(&deployment)?;

    let store = root_store::status(registry.store_bootstrap.clone()).await?;
    let revalidated = current_registry_for_progress(authority, root, current)?;
    if revalidated.store_bootstrap != registry.store_bootstrap {
        return Err(InternalError::conflict());
    }
    let latest = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })?;
    match RootComponentProvisioningOps::advance_disposition(request, &latest)? {
        RootComponentProvisioningAdvanceDisposition::Complete
        | RootComponentProvisioningAdvanceDisposition::Replay => return Ok(latest),
        RootComponentProvisioningAdvanceDisposition::Advance => {}
    }
    if RootComponentProvisioningOps::next_member_install(&latest)? != member {
        return Err(InternalError::conflict());
    }

    let allocation = required_member_allocation(member.member_operation_id, "install")?;
    super::component_registry::validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        member.member_operation_id,
    )?;
    let deployment =
        RootComponentProvisioningOps::member_deployment_context(&latest, &member, &allocation)?;
    validate_group_member_context(&deployment)?;
    let installed = Box::pin(super::component_registry::advance_group_member_install(
        &authority.binding,
        &store,
        allocation,
        deployment,
    ))
    .await?;

    let _registry = current_registry_for_progress(authority, root, &latest)?;
    let committed = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })?;
    match RootComponentProvisioningOps::advance_disposition(request, &committed)? {
        RootComponentProvisioningAdvanceDisposition::Complete
        | RootComponentProvisioningAdvanceDisposition::Replay => Ok(committed),
        RootComponentProvisioningAdvanceDisposition::Advance => {
            RootComponentProvisioningOps::mark_member_installed(request, &installed)
        }
    }
}

async fn advance_member_registry_commit(
    authority: &FleetSubnetRootAuthority,
    root: Principal,
    request: RootComponentProvisioningAdvanceRequest,
    current: &RootComponentProvisioningView,
) -> Result<RootComponentProvisioningView, InternalError> {
    let registry = current_registry_for_progress(authority, root, current)?;
    let fleet_directory = current_fleet_directory_for_progress(authority, root, current)?;
    let member = RootComponentProvisioningOps::next_member_registry_commit(current)?;
    let allocation = required_member_allocation(member.member_operation_id, "Registry commit")?;
    validate_registry_commit_progress(
        registry.reserved_component_instances,
        current.component_count,
        current.registry_cursor.registry_committed_component_count,
        matches!(
            allocation.progress,
            crate::view::component_registry::RootComponentAllocationProgressView::Committed { .. }
        ),
    )?;
    let topology = ConfigOps::component_topology()?;
    super::component_registry::validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        member.member_operation_id,
    )?;
    let deployment =
        RootComponentProvisioningOps::member_deployment_context(current, &member, &allocation)?;
    validate_group_member_context(&deployment)?;

    let store = root_store::status(registry.store_bootstrap.clone()).await?;
    let revalidated = current_registry_for_progress(authority, root, current)?;
    if revalidated.store_bootstrap != registry.store_bootstrap {
        return Err(InternalError::conflict());
    }
    let revalidated_directory = current_fleet_directory_for_progress(authority, root, current)?;
    if revalidated_directory != fleet_directory {
        return Err(InternalError::conflict());
    }
    let latest = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })?;
    match RootComponentProvisioningOps::advance_disposition(request, &latest)? {
        RootComponentProvisioningAdvanceDisposition::Complete
        | RootComponentProvisioningAdvanceDisposition::Replay => return Ok(latest),
        RootComponentProvisioningAdvanceDisposition::Advance => {}
    }
    if RootComponentProvisioningOps::next_member_registry_commit(&latest)? != member {
        return Err(InternalError::conflict());
    }

    let allocation = required_member_allocation(member.member_operation_id, "Registry commit")?;
    super::component_registry::validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        member.member_operation_id,
    )?;
    let deployment =
        RootComponentProvisioningOps::member_deployment_context(&latest, &member, &allocation)?;
    validate_group_member_context(&deployment)?;
    let (committed, partition) = Box::pin(
        super::component_registry::advance_group_member_registry_commit(
            authority,
            &authority.binding,
            &store,
            allocation,
            deployment,
            fleet_directory,
        ),
    )
    .await?;

    let current_registry = current_registry_for_progress(authority, root, &latest)?;
    validate_registry_commit_progress(
        current_registry.reserved_component_instances,
        latest.component_count,
        latest.registry_cursor.registry_committed_component_count,
        true,
    )?;
    let aggregate = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })?;
    match RootComponentProvisioningOps::advance_disposition(request, &aggregate)? {
        RootComponentProvisioningAdvanceDisposition::Complete
        | RootComponentProvisioningAdvanceDisposition::Replay => Ok(aggregate),
        RootComponentProvisioningAdvanceDisposition::Advance => {
            RootComponentProvisioningOps::mark_member_registry_committed(
                request, &committed, &partition,
            )
        }
    }
}

fn required_member_allocation(
    operation_id: [u8; 32],
    _phase: &str,
) -> Result<RootComponentAllocationView, InternalError> {
    ComponentRegistryOps::allocation(operation_id).ok_or_else(InternalError::invariant)
}

fn require_coordinator(caller: Principal, coordinator: Principal) -> Result<(), InternalError> {
    if caller != coordinator {
        return Err(InternalError::forbidden());
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct RootComponentProvisioningAcceptanceContext {
    registry: RootComponentRegistryView,
    runtime_mode: RootComponentProvisioningRuntimeMode,
}

fn current_registry_for_acceptance(
    authority: &FleetSubnetRootAuthority,
    root: Principal,
    request: &RootComponentProvisioningAcceptanceRequest,
    validation: &RootComponentProvisioningBatchValidation,
) -> Result<RootComponentProvisioningAcceptanceContext, InternalError> {
    let runtime = FleetActivationWorkflow::status()?;
    let mirror = FleetRegistryMirrorOps::validated_current(authority, root)?;
    if mirror.root_entry.status != FleetSubnetRootStatus::Active
        || mirror.active.snapshot.version != request.fleet_registry
    {
        return Err(InternalError::conflict());
    }
    let current = ComponentRegistryOps::current().ok_or_else(InternalError::unavailable)?;
    let runtime_mode = validate_component_registry_authority(
        &current,
        &authority.binding,
        authority.initial_release_set,
        &request.fleet_registry,
        runtime.phase,
        runtime.identity.operation_id,
    )?;
    validate_component_capacity(&current, validation)?;
    validate_group_placement_capacity(
        RootComponentProvisioningOps::tracked_group_placements()?,
        validation.placement_count,
        authority.binding.limits.maximum_group_placements,
    )?;
    Ok(RootComponentProvisioningAcceptanceContext {
        registry: current,
        runtime_mode,
    })
}

fn current_registry_for_progress(
    authority: &FleetSubnetRootAuthority,
    root: Principal,
    provisioning: &RootComponentProvisioningView,
) -> Result<RootComponentRegistryView, InternalError> {
    let runtime = FleetActivationWorkflow::status()?;
    let mirror = FleetRegistryMirrorOps::validated_current(authority, root)?;
    if mirror.root_entry.status != FleetSubnetRootStatus::Active
        || mirror.active.snapshot.version != provisioning.fleet_registry
    {
        return Err(InternalError::conflict());
    }
    let config = ConfigOps::get()?;
    ComponentProvisioningPlanOps::validate_root_batch(
        &config,
        &mirror.active.snapshot.registry,
        &provisioning.fleet_registry,
        provisioning.configuration_digest,
        &authority.binding,
        &provisioning.batch,
    )?;
    let current = ComponentRegistryOps::current().ok_or_else(InternalError::unavailable)?;
    let runtime_mode = validate_component_registry_authority(
        &current,
        &authority.binding,
        authority.initial_release_set,
        &provisioning.fleet_registry,
        runtime.phase,
        runtime.identity.operation_id,
    )?;
    require_runtime_mode(provisioning.runtime_mode, runtime_mode)?;
    Ok(current)
}

fn current_fleet_directory_for_progress(
    authority: &FleetSubnetRootAuthority,
    root: Principal,
    provisioning: &RootComponentProvisioningView,
) -> Result<canic_core::dto::fleet_registry::FleetDirectorySnapshot, InternalError> {
    let mirror = FleetRegistryMirrorOps::validated_current(authority, root)?;
    if mirror.root_entry.status != FleetSubnetRootStatus::Active
        || mirror.active.snapshot.version != provisioning.fleet_registry
    {
        return Err(InternalError::conflict());
    }
    Ok(mirror.active.directory)
}

fn validate_reservation_registry_progress(
    registry_reserved_components: u32,
    aggregate_reserved_components: u32,
    current_member_exists: bool,
) -> Result<(), InternalError> {
    let expected = aggregate_reserved_components
        .checked_add(u32::from(current_member_exists))
        .ok_or_else(InternalError::resource_exhausted)?;
    if registry_reserved_components != expected {
        return Err(InternalError::invariant());
    }
    Ok(())
}

const fn validate_claim_registry_progress(
    registry: &RootComponentRegistryView,
    component_count: u32,
) -> Result<(), InternalError> {
    if registry.reserved_component_instances != component_count {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_registry_commit_progress(
    registry_reserved_components: u32,
    component_count: u32,
    aggregate_registry_committed_components: u32,
    current_member_is_committed: bool,
) -> Result<(), InternalError> {
    let reconciled_committed = aggregate_registry_committed_components
        .checked_add(u32::from(current_member_is_committed))
        .ok_or_else(InternalError::resource_exhausted)?;
    let expected_reserved = component_count
        .checked_sub(reconciled_committed)
        .ok_or_else(InternalError::invariant)?;
    if registry_reserved_components != expected_reserved {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_group_member_context(
    context: &canic_core::dto::component_deployment::ProtectedComponentDeployment,
) -> Result<(), InternalError> {
    let canic_core::dto::component_deployment::ProtectedComponentDeployment::GroupMember {
        binding,
        ..
    } = context
    else {
        return Err(InternalError::invariant());
    };
    ConfigOps::validate_protected_component_deployment(context, binding)
}

fn reserve_group_member(
    authority: &FleetSubnetRootAuthority,
    registry: &RootComponentRegistryView,
    provisioning: &RootComponentProvisioningView,
    member: &RootComponentProvisioningMemberView,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
) -> Result<RootComponentAllocationView, InternalError> {
    let request = RootComponentAllocationRequest {
        operation_id: member.member_operation_id,
        component_spec: member.component_spec.clone(),
    };
    let decision = super::component_registry::top_level_allocation_decision(
        &authority.binding,
        topology,
        registry,
        &request,
    )?;
    let origin = ComponentProvisioningOrigin::ComponentGroup {
        operation_id: provisioning.operation_id,
        plan_hash: provisioning.plan_hash,
        group_placement: member.group_placement.clone(),
        member_path: member.member_path.clone(),
    };
    ComponentRegistryOps::reserve_allocation(
        decision,
        member.member_operation_id,
        origin,
        registry.initial_inventory.is_some(),
    )
}

fn validate_component_registry_authority(
    current: &RootComponentRegistryView,
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    fleet_registry: &canic_core::dto::fleet_registry::FleetRegistryVersion,
    runtime_phase: FleetActivationPhase,
    runtime_operation_id: [u8; 32],
) -> Result<RootComponentProvisioningRuntimeMode, InternalError> {
    let runtime_mode = component_provisioning_runtime_mode(
        current.initial_inventory,
        runtime_phase,
        runtime_operation_id,
    );
    let registry_authority_facts = [
        &current.root == root,
        current.release_set == release_set,
        current.root_draining.is_none(),
        ComponentRegistryOps::registry_covers_preparation(
            &current.prepared_against_registry,
            fleet_registry,
        ),
    ];
    let registry_authority_is_exact = registry_authority_facts.into_iter().all(|fact| fact);
    let Some(runtime_mode) = runtime_mode else {
        return Err(InternalError::conflict());
    };
    if !registry_authority_is_exact {
        return Err(InternalError::conflict());
    }
    ComponentRegistryOps::require_top_level_allocation_open()?;
    Ok(runtime_mode)
}

fn component_provisioning_runtime_mode(
    inventory: Option<RootComponentInitialInventoryView>,
    runtime_phase: FleetActivationPhase,
    runtime_operation_id: [u8; 32],
) -> Option<RootComponentProvisioningRuntimeMode> {
    match (runtime_phase, inventory) {
        (FleetActivationPhase::Prepared, None) => {
            Some(RootComponentProvisioningRuntimeMode::FreshRoot)
        }
        (FleetActivationPhase::Prepared, Some(_)) | (FleetActivationPhase::Active, None) => None,
        (FleetActivationPhase::Active, Some(inventory)) => {
            let runtime_inventory_facts = [
                inventory.fleet_activation_operation_id == runtime_operation_id,
                inventory.directories_converged,
                inventory.root_runtime_activated,
            ];
            runtime_inventory_facts
                .into_iter()
                .all(|fact| fact)
                .then_some(RootComponentProvisioningRuntimeMode::ActiveRoot)
        }
    }
}

fn require_runtime_mode(
    expected: RootComponentProvisioningRuntimeMode,
    actual: RootComponentProvisioningRuntimeMode,
) -> Result<(), InternalError> {
    if expected != actual {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_activation_runtime_authority(
    authority: &FleetSubnetRootAuthority,
    provisioning: &RootComponentProvisioningView,
) -> Result<(), InternalError> {
    let runtime = FleetActivationWorkflow::status()?;
    let registry = ComponentRegistryOps::current().ok_or_else(InternalError::unavailable)?;
    let published_registry = &provisioning
        .publication
        .as_ref()
        .ok_or_else(InternalError::conflict)?
        .fleet_registry;
    let actual = validate_component_registry_authority(
        &registry,
        &authority.binding,
        authority.initial_release_set,
        published_registry,
        runtime.phase,
        runtime.identity.operation_id,
    )?;
    let fresh_root_response_retry = provisioning.runtime_mode
        == RootComponentProvisioningRuntimeMode::FreshRoot
        && provisioning.activated_component_count == provisioning.component_count
        && runtime.phase == FleetActivationPhase::Active;
    if actual != provisioning.runtime_mode && !fresh_root_response_retry {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_component_capacity(
    current: &RootComponentRegistryView,
    validation: &RootComponentProvisioningBatchValidation,
) -> Result<(), InternalError> {
    if current.reserved_component_instances != 0 {
        return Err(InternalError::unavailable());
    }
    let occupied = current
        .reserved_component_instances
        .checked_add(current.committed_component_instances)
        .and_then(|count| count.checked_add(validation.component_count))
        .ok_or_else(InternalError::resource_exhausted)?;
    if occupied > current.root.limits.maximum_component_instances {
        return Err(InternalError::resource_exhausted());
    }
    for (component_spec, requested) in &validation.component_spec_counts {
        let admission = current
            .root
            .component_admissions
            .binary_search_by(|candidate| candidate.component_spec.cmp(component_spec))
            .ok()
            .map(|index| &current.root.component_admissions[index])
            .ok_or_else(InternalError::conflict)?;
        let counts = ComponentRegistryOps::component_spec_counts(component_spec)?;
        let occupied = counts
            .reserved
            .checked_add(counts.committed)
            .and_then(|count| count.checked_add(*requested))
            .ok_or_else(InternalError::resource_exhausted)?;
        if occupied > admission.maximum_root_instances {
            return Err(InternalError::resource_exhausted());
        }
    }
    Ok(())
}

fn validate_group_placement_capacity(
    tracked: u32,
    requested: u32,
    maximum: u32,
) -> Result<(), InternalError> {
    let required = tracked
        .checked_add(requested)
        .ok_or_else(InternalError::resource_exhausted)?;
    if required > maximum {
        return Err(InternalError::resource_exhausted());
    }
    Ok(())
}

async fn require_ready_pool_capacity(
    pool: &canic_core::ids::FleetSubnetCanisterPoolConfig,
    cycle_demands: &[canic_core::cdk::types::Cycles],
) -> Result<(), InternalError> {
    if CanisterPoolOps::ready_assets_cover(cycle_demands) {
        return Ok(());
    }
    if cycle_demands
        .iter()
        .any(|required| required > &pool.canister_cycles)
    {
        return Err(InternalError::public(
            canic_core::diagnostics::codes::CAPACITY_INSUFFICIENT,
        ));
    }
    let ready_target =
        u32::try_from(cycle_demands.len()).map_err(|_error| InternalError::resource_exhausted())?;
    let _maintenance =
        crate::workflow::canister_pool::maintain_ready_capacity_once(ready_target).await?;
    if CanisterPoolOps::ready_assets_cover(cycle_demands) {
        return Ok(());
    }
    if CanisterPoolOps::standby_capacity_is_exhausted(pool) {
        return Err(InternalError::public(
            canic_core::diagnostics::codes::CAPACITY_INSUFFICIENT,
        ));
    }
    Err(InternalError::unavailable())
}

fn require_next_claim_capacity(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    current: &RootComponentProvisioningView,
) -> Result<(), InternalError> {
    if current.phase != RootComponentProvisioningPhase::Accepted
        || current.reservation_cursor.reserved_component_count != current.component_count
        || current.claim_cursor.claimed_component_count >= current.component_count
    {
        return Ok(());
    }
    let member = RootComponentProvisioningOps::next_member_claim(current)?;
    let config = ConfigOps::get()?;
    let required_cycles = config
        .component_specs
        .get(&member.component_spec)
        .map(|component| &component.initial_cycles)
        .ok_or_else(InternalError::conflict)?;
    if CanisterPoolOps::has_ready_asset_for(required_cycles) {
        return Ok(());
    }
    if CanisterPoolOps::standby_capacity_is_exhausted(&authority.binding.limits.canister_pool) {
        return Err(InternalError::public(
            canic_core::diagnostics::codes::CAPACITY_INSUFFICIENT,
        ));
    }
    Ok(())
}

/// Project an active full-pool capacity defect only after the observer is authorized.
pub(super) fn require_current_claim_capacity(
    current: &RootComponentProvisioningView,
) -> Result<(), InternalError> {
    let (authority, _root) = validated_root_authority()?;
    require_next_claim_capacity(&authority, current)
}

fn validate_store_artifacts(
    store: &canic_core::dto::root_store::RootStoreBootstrapResponse,
    roles: &std::collections::BTreeSet<canic_core::ids::CanisterRole>,
) -> Result<(), InternalError> {
    for role in roles {
        let count = store
            .catalog
            .iter()
            .filter(|artifact| &artifact.role == role)
            .count();
        if count != 1 {
            return Err(InternalError::conflict());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_exact_protected_coordinator_is_authorized() {
        let coordinator = Principal::from_slice(&[7; 29]);
        assert!(require_coordinator(coordinator, coordinator).is_ok());
        assert!(require_coordinator(Principal::from_slice(&[8; 29]), coordinator).is_err());
    }

    #[test]
    fn capacity_helpers_reject_first_excess_without_mutation() {
        assert!(validate_group_placement_capacity(3, 2, 5).is_ok());
        assert!(validate_group_placement_capacity(3, 3, 5).is_err());
    }

    #[test]
    fn registry_progress_allows_only_exact_or_response_lost_reservation() {
        assert!(validate_reservation_registry_progress(3, 3, false).is_ok());
        assert!(validate_reservation_registry_progress(4, 3, true).is_ok());
        assert!(validate_reservation_registry_progress(4, 3, false).is_err());
        assert!(validate_reservation_registry_progress(3, 3, true).is_err());
    }

    #[test]
    fn registry_commit_progress_allows_only_exact_or_response_lost_commitment() {
        assert!(validate_registry_commit_progress(3, 3, 0, false).is_ok());
        assert!(validate_registry_commit_progress(2, 3, 0, true).is_ok());
        assert!(validate_registry_commit_progress(2, 3, 0, false).is_err());
        assert!(validate_registry_commit_progress(3, 3, 0, true).is_err());
        assert!(validate_registry_commit_progress(1, 3, 1, true).is_ok());
    }

    #[test]
    fn active_registry_inventory_binds_the_exact_runtime_activation() {
        let inventory = RootComponentInitialInventoryView {
            fleet_activation_operation_id: [7; 32],
            component_count: 1,
            inventory_hash: [8; 32],
            sealed_at_ns: 9,
            directories_converged: true,
            root_runtime_activated: true,
        };
        assert_eq!(
            component_provisioning_runtime_mode(None, FleetActivationPhase::Prepared, [7; 32],),
            Some(RootComponentProvisioningRuntimeMode::FreshRoot)
        );
        assert_eq!(
            component_provisioning_runtime_mode(
                Some(inventory),
                FleetActivationPhase::Prepared,
                [7; 32],
            ),
            None
        );
        assert_eq!(
            component_provisioning_runtime_mode(
                Some(inventory),
                FleetActivationPhase::Active,
                [7; 32],
            ),
            Some(RootComponentProvisioningRuntimeMode::ActiveRoot)
        );
        assert_eq!(
            component_provisioning_runtime_mode(
                Some(inventory),
                FleetActivationPhase::Active,
                [6; 32],
            ),
            None
        );
        assert_eq!(
            component_provisioning_runtime_mode(None, FleetActivationPhase::Active, [7; 32],),
            None
        );

        let unconverged = RootComponentInitialInventoryView {
            directories_converged: false,
            ..inventory
        };
        assert_eq!(
            component_provisioning_runtime_mode(
                Some(unconverged),
                FleetActivationPhase::Active,
                [7; 32],
            ),
            None
        );
    }
}
