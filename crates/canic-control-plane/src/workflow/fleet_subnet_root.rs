//! Module: workflow::fleet_subnet_root
//!
//! Responsibility: validate root authority, orchestrate draining/final inventory and project summaries.
//! Does not own: durable records, Coordinator Registry mutation, Component effects, or CLI output.
//! Boundary: root actions require consistent protected, mirror, runtime and Component authority.

use crate::{
    dto::root::RootRemovalOperationStatus,
    ops::{
        canister_pool::CanisterPoolOps, component_provisioning::RootComponentProvisioningOps,
        component_registry::ComponentRegistryOps, fleet_registry_mirror::FleetRegistryMirrorOps,
        storage::state::root_wasm_store::RootWasmStoreStateOps,
    },
    view::component_registry::{
        RootComponentRegistryView, RootFleetSubnetDeletionPreparationAuthority,
        RootFleetSubnetDeletionPreparationIntentView, RootFleetSubnetDeletionPreparationView,
        RootFleetSubnetDrainingView, RootFleetSubnetFinalInventoryView,
        RootFleetSubnetRemovalPublicationView, RootFleetSubnetStoreBindingFinalizationView,
        RootFleetSubnetStoreDeletionView, RootFleetSubnetStoreReclamationView,
    },
    workflow::{
        bootstrap::root_store, component_registry, fleet_coordinator_client, fleet_registry_mirror,
        runtime::template::publication::WasmStorePublicationWorkflow,
    },
};
use candid::{Nat, Principal};
use canic_core::{
    api::{
        fleet_activation::FleetActivationApi, runtime::root_funding::RootFundingTimerApi,
        timer::TimerApi,
    },
    control_plane_support::{
        error::InternalError,
        model::replay::CommandKind,
        ops::{
            cost_guard::{CostGuardPermit, CostGuardRequest},
            ic::{
                IcOps,
                mgmt::{CanisterSettings, MgmtOps, UpdateSettingsArgs},
            },
            icp_refill::IcpRefillStoreOps,
            root_draining_reservation::FleetSubnetRootDrainingReservationOps,
        },
        workflow::cost_guard::{CostGuardWorkflow, map_cost_guard_reserve_error},
    },
    dto::{
        component_registry::{ComponentRegistryHead, RootComponentDrainingRequest},
        fleet_registry::{
            FleetRegistryVersion, FleetSubnetRootDeletionReadinessIntentRequest,
            FleetSubnetRootDeletionReadinessIntentResponse,
            FleetSubnetRootDeletionReadinessRequest, FleetSubnetRootDeletionReadinessResponse,
            FleetSubnetRootDrainingReservationResponse, FleetSubnetRootEntry,
            FleetSubnetRootRemovalPublicationRequest, FleetSubnetRootRemovalPublicationResponse,
            FleetSubnetRootStatus,
        },
        fleet_subnet_root::{
            FLEET_SUBNET_ROOT_DELETION_CALL_REFUND_HEADROOM_CYCLES,
            FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES, FleetSubnetRootAuthority,
            FleetSubnetRootCanisterSummary, FleetSubnetRootDeletionPreparationRequest,
            FleetSubnetRootDeletionPreparationResponse,
            FleetSubnetRootDeletionPreparationStatusRequest, FleetSubnetRootDrainingRequest,
            FleetSubnetRootDrainingResponse, FleetSubnetRootDrainingStatusRequest,
            FleetSubnetRootFinalInventoryRequest, FleetSubnetRootFinalInventoryResponse,
            FleetSubnetRootFinalInventoryStatusRequest, FleetSubnetRootRemovalRequest,
            FleetSubnetRootRemovalStatusRequest, FleetSubnetRootStoreBindingFinalizationRequest,
            FleetSubnetRootStoreBindingFinalizationResponse,
            FleetSubnetRootStoreBindingFinalizationStatusRequest,
            FleetSubnetRootStoreDeletionRequest, FleetSubnetRootStoreDeletionResponse,
            FleetSubnetRootStoreDeletionStatusRequest, FleetSubnetRootStoreReclamationRequest,
            FleetSubnetRootStoreReclamationResponse, FleetSubnetRootStoreReclamationStatusRequest,
            FleetSubnetWasmStoreAdoptionRequest, FleetSubnetWasmStoreAdoptionResponse,
        },
        pool::{PoolAdminCommand, PoolAdminResponse},
        role::{OperationReceipt, RootRemovalRequest},
    },
    ids::{
        ComponentInstanceId, FleetSubnetRootBinding, FleetSubnetRootReleaseSet,
        FleetSubnetWasmStoreAuthority,
    },
    replay_policy::CostClass,
};
use sha2::{Digest, Sha256};
use std::time::Duration;

const ROOT_DELETION_CYCLE_RECLAMATION_COMMAND_KIND: &str =
    "fleet_subnet_root.reclaim_deletion_cycles.v1";
const VALUE_TRANSFER_QUOTA_WINDOW_SECONDS: u64 = 60;
const MAX_VALUE_TRANSFERS_PER_WINDOW: u64 = 60;
const ROOT_COMPONENT_REMOVAL_OPERATION_DOMAIN: &[u8] =
    b"canic.fleet-subnet-root.component-removal.v1";
const SECONDS_PER_DAY: u128 = 86_400;

struct ValidatedFleetSubnetRootState {
    fleet_registry: FleetRegistryVersion,
    root_entry: FleetSubnetRootEntry,
    component_registry: RootComponentRegistryView,
}

#[derive(Eq, PartialEq)]
struct ComponentRegistrySourceAuthority<'a> {
    root: &'a FleetSubnetRootBinding,
    release_set: FleetSubnetRootReleaseSet,
}

#[derive(Eq, PartialEq)]
struct SiblingWasmStoreLiveEvidence {
    controllers: Vec<Principal>,
}

/// Adopt the independently installed sibling Store under Root policy authority.
///
/// The immutable installation controller remains a direct controller so the
/// current Fleet reconciler can continue proving Store cycles and module state.
pub async fn adopt_wasm_store(
    request: FleetSubnetWasmStoreAdoptionRequest,
) -> Result<FleetSubnetWasmStoreAdoptionResponse, InternalError> {
    let authority = protected_sibling_wasm_store_authority(&request)?;
    if let Some(receipt) = RootWasmStoreStateOps::sibling_wasm_store_adoption_receipt(
        request.operation_id,
        authority.clone(),
    )? {
        return Ok(receipt);
    }

    let controllers = sibling_wasm_store_controllers(&authority);
    RootWasmStoreStateOps::begin_sibling_wasm_store_adoption(
        &crate::ops::storage::state::root_wasm_store::SiblingWasmStoreAdoptionPlan {
            operation_id: request.operation_id,
            authority: authority.clone(),
            controllers: controllers.clone(),
        },
    )?;

    let observed = observe_sibling_wasm_store(&authority).await?;
    prepare_sibling_wasm_store_controllers(&authority, &observed, &controllers).await?;
    let prepared = observe_sibling_wasm_store(&authority).await?;
    require_sibling_wasm_store_controllers(&prepared, &controllers)?;
    RootWasmStoreStateOps::commit_sibling_wasm_store_adoption(
        request.operation_id,
        authority,
        IcOps::now_nanos(),
    )
}

/// Read the terminal sibling Store adoption receipt without a management call.
pub fn wasm_store_adoption_status(
    request: FleetSubnetWasmStoreAdoptionRequest,
) -> Result<FleetSubnetWasmStoreAdoptionResponse, InternalError> {
    let authority = protected_sibling_wasm_store_authority(&request)?;
    RootWasmStoreStateOps::sibling_wasm_store_adoption_receipt(request.operation_id, authority)?
        .ok_or_else(InternalError::unavailable)
}

/// Resolve the terminal sibling Store adoption through its durable operation identity.
pub fn wasm_store_adoption_operation_status(
    operation_id: [u8; 32],
) -> Result<Option<FleetSubnetWasmStoreAdoptionResponse>, InternalError> {
    if operation_id == [0; 32] {
        return Err(InternalError::invalid_input());
    }
    let (authority, _) = crate::workflow::root_authority::validated_root_authority()?;
    RootWasmStoreStateOps::sibling_wasm_store_adoption_receipt_by_operation(
        operation_id,
        authority.wasm_store_authority,
    )
}

fn protected_sibling_wasm_store_authority(
    request: &FleetSubnetWasmStoreAdoptionRequest,
) -> Result<FleetSubnetWasmStoreAuthority, InternalError> {
    if request.operation_id == [0; 32] {
        return Err(InternalError::invalid_input());
    }
    let (root_authority, _) = crate::workflow::root_authority::validated_root_authority()?;
    if request.authority != root_authority.wasm_store_authority {
        return Err(InternalError::conflict());
    }
    Ok(root_authority.wasm_store_authority)
}

fn sibling_wasm_store_controllers(authority: &FleetSubnetWasmStoreAuthority) -> Vec<Principal> {
    let mut controllers = vec![
        authority.installation_controller,
        authority.fleet_subnet_root,
    ];
    controllers.sort();
    controllers
}

async fn observe_sibling_wasm_store(
    authority: &FleetSubnetWasmStoreAuthority,
) -> Result<SiblingWasmStoreLiveEvidence, InternalError> {
    let status = MgmtOps::canister_status(authority.wasm_store).await?;
    let mut controllers = status.settings.controllers;
    controllers.sort();
    Ok(SiblingWasmStoreLiveEvidence { controllers })
}

async fn prepare_sibling_wasm_store_controllers(
    authority: &FleetSubnetWasmStoreAuthority,
    observed: &SiblingWasmStoreLiveEvidence,
    expected: &[Principal],
) -> Result<(), InternalError> {
    if !sibling_wasm_store_requires_controller_update(authority, observed, expected)? {
        return Ok(());
    }
    MgmtOps::update_settings(&UpdateSettingsArgs {
        canister_id: authority.wasm_store,
        settings: CanisterSettings {
            controllers: Some(expected.to_vec()),
            ..CanisterSettings::default()
        },
        sender_canister_version: None,
    })
    .await
}

fn sibling_wasm_store_requires_controller_update(
    authority: &FleetSubnetWasmStoreAuthority,
    observed: &SiblingWasmStoreLiveEvidence,
    expected: &[Principal],
) -> Result<bool, InternalError> {
    if observed.controllers == expected {
        return Ok(false);
    }
    if observed.controllers == [authority.fleet_subnet_root] {
        return Ok(true);
    }
    Err(InternalError::conflict())
}

fn require_sibling_wasm_store_controllers(
    observed: &SiblingWasmStoreLiveEvidence,
    expected_controllers: &[Principal],
) -> Result<(), InternalError> {
    if observed.controllers != expected_controllers {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn begin_draining_with_reservation(
    request: FleetSubnetRootDrainingRequest,
    reservation: FleetSubnetRootDrainingReservationResponse,
) -> Result<FleetSubnetRootDrainingResponse, InternalError> {
    let state = validated_root_state()?;
    if let Some(existing) = ComponentRegistryOps::root_draining_if_present(request.operation_id)? {
        return exact_draining_retry(&request, existing);
    }
    if state.root_entry.status != FleetSubnetRootStatus::Active {
        return Err(InternalError::conflict());
    }
    if !ComponentRegistryOps::registry_covers_preparation(
        &request.expected_registry,
        &state.fleet_registry,
    ) {
        return Err(InternalError::conflict());
    }
    RootComponentProvisioningOps::require_root_draining_open()?;
    validate_root_draining_reservation(&state, &request, &reservation)?;

    let current = validated_root_state()?;
    validate_root_draining_reservation(&current, &request, &reservation)?;
    RootComponentProvisioningOps::require_root_draining_open()?;
    let draining = ComponentRegistryOps::begin_root_draining(
        request.operation_id,
        &request.expected_registry,
        &reservation,
        IcOps::now_nanos(),
    )?;
    crate::workflow::canister_pool::stop()?;
    Ok(draining_response(draining))
}

/// Accept one high-level root-removal intent and schedule its private reconciler once.
pub fn accept_root_removal(input: RootRemovalRequest) -> Result<OperationReceipt, InternalError> {
    let request = FleetSubnetRootDrainingRequest {
        operation_id: input.reservation.request.operation_id,
        expected_registry: input.reservation.request.expected_registry.clone(),
    };
    let operation_id = request.operation_id;
    if let Some(existing) = ComponentRegistryOps::root_draining_if_present(operation_id)? {
        exact_draining_retry(&request, existing)?;
        return Ok(OperationReceipt { operation_id });
    }
    begin_draining_with_reservation(request, input.reservation)?;
    schedule_root_removal(operation_id);
    Ok(OperationReceipt { operation_id })
}

/// Authorize the accepted removal command for the exact Coordinator.
pub fn authorize_root_removal_caller(
    caller: Principal,
    _caller_is_controller: bool,
) -> Result<(), InternalError> {
    let (authority, _) = crate::workflow::root_authority::validated_root_authority()?;
    if caller != authority.binding.authority.binding.coordinator {
        return Err(InternalError::forbidden());
    }
    Ok(())
}

fn exact_draining_retry(
    request: &FleetSubnetRootDrainingRequest,
    existing: RootFleetSubnetDrainingView,
) -> Result<FleetSubnetRootDrainingResponse, InternalError> {
    if request.expected_registry != existing.active_registry {
        return Err(InternalError::conflict());
    }
    Ok(draining_response(existing))
}

fn validate_root_draining_reservation(
    state: &ValidatedFleetSubnetRootState,
    request: &FleetSubnetRootDrainingRequest,
    reservation: &FleetSubnetRootDrainingReservationResponse,
) -> Result<(), InternalError> {
    let source_is_covered = ComponentRegistryOps::registry_covers_preparation(
        &reservation.request.expected_registry,
        &state.fleet_registry,
    );
    let reservation_is_exact = [
        reservation.request.operation_id == request.operation_id,
        reservation.request.expected_registry == request.expected_registry,
        reservation.request.expected_root == state.root_entry,
        reservation.request.expected_root.status == FleetSubnetRootStatus::Active,
        reservation.coordinator == state.fleet_registry.authority.binding.coordinator,
        reservation.prepared_at_ns > 0,
        reservation.reservation_hash != [0; 32],
        source_is_covered,
        FleetSubnetRootDrainingReservationOps::content_hash(reservation)?
            == reservation.reservation_hash,
    ]
    .into_iter()
    .all(|valid| valid);
    if !reservation_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}

/// Read one exact durable root-local draining fence without mutation.
pub fn draining_status(
    request: FleetSubnetRootDrainingStatusRequest,
) -> Result<FleetSubnetRootDrainingResponse, InternalError> {
    let _state = validated_root_state()?;
    ComponentRegistryOps::root_draining(request.operation_id).map(draining_response)
}

/// Freeze one exact terminal Component history and retained write-fenced Store inventory.
pub async fn finalize_inventory(
    request: FleetSubnetRootFinalInventoryRequest,
) -> Result<FleetSubnetRootFinalInventoryResponse, InternalError> {
    let state = validated_root_state()?;
    ensure_root_is_published_draining(&state)?;
    if let Some(existing) =
        ComponentRegistryOps::root_final_inventory_if_present(request.operation_id)?
    {
        if request.expected_registry != existing.registry {
            return Err(InternalError::conflict());
        }
        return Ok(final_inventory_response(existing));
    }
    let retained_workload_assets = CanisterPoolOps::non_store_asset_count();
    if retained_workload_assets != 0 {
        return Err(InternalError::unavailable());
    }
    if CanisterPoolOps::has_pending_lifecycle_work() {
        return Err(InternalError::unavailable());
    }
    let intent_registry =
        ComponentRegistryOps::root_final_inventory_intent_registry(request.operation_id)?;
    if let Some(intent_registry) = intent_registry {
        if request.expected_registry != intent_registry {
            return Err(InternalError::conflict());
        }
    } else if request.expected_registry != state.fleet_registry {
        return Err(InternalError::conflict());
    }
    ComponentRegistryOps::begin_root_final_inventory(
        request.operation_id,
        &request.expected_registry,
        IcOps::now_nanos(),
    )?;
    let (wasm_store, store_status) =
        WasmStorePublicationWorkflow::quiesce_single_root_store_for_final_inventory(
            request.operation_id,
        )
        .await?;
    let store = root_store::status(state.component_registry.store_bootstrap).await?;
    if store.wasm_store != wasm_store {
        return Err(InternalError::invariant());
    }
    ComponentRegistryOps::finalize_root_inventory(
        request.operation_id,
        &request.expected_registry,
        &store,
        &store_status,
        IcOps::now_nanos(),
    )
    .map(final_inventory_response)
}

/// Read one exact durable terminal root-local inventory without mutation.
pub fn final_inventory_status(
    request: FleetSubnetRootFinalInventoryStatusRequest,
) -> Result<FleetSubnetRootFinalInventoryResponse, InternalError> {
    let state = validated_root_state()?;
    ensure_root_is_published_draining(&state)?;
    ComponentRegistryOps::root_final_inventory(request.operation_id).map(final_inventory_response)
}

/// Revalidate the retained Store and publish this root as logically `Removed`.
pub async fn publish_removal(
    request: FleetSubnetRootRemovalRequest,
) -> Result<FleetSubnetRootRemovalPublicationResponse, InternalError> {
    let state = validated_root_state()?;
    ensure_root_is_published_draining(&state)?;
    if let Some(existing) =
        ComponentRegistryOps::root_removal_publication_if_present(request.operation_id)?
    {
        if existing.previous_registry != request.expected_registry {
            return Err(InternalError::conflict());
        }
        let inventory = ComponentRegistryOps::root_final_inventory(request.operation_id)?;
        return removal_publication_response(existing, inventory);
    }
    if request.expected_registry != state.fleet_registry {
        return Err(InternalError::conflict());
    }
    let coordinator = state.component_registry.root.authority.binding.coordinator;
    let (wasm_store, store_status) =
        WasmStorePublicationWorkflow::verify_single_root_store_for_removal().await?;
    let store = root_store::status(state.component_registry.store_bootstrap).await?;
    if store.wasm_store != wasm_store {
        return Err(InternalError::invariant());
    }
    let inventory = ComponentRegistryOps::verify_root_final_inventory_store(
        request.operation_id,
        &store,
        &store_status,
    )?;
    let publication_request = FleetSubnetRootRemovalPublicationRequest {
        expected_registry: request.expected_registry,
        final_inventory: final_inventory_response(inventory),
    };
    let response = fleet_coordinator_client::root_removal_status(coordinator, request.operation_id)
        .await?
        .removal
        .ok_or_else(InternalError::unavailable)?;
    validate_removal_publication_response(&publication_request, &response)?;
    let publication = ComponentRegistryOps::record_root_removal_publication(
        request.operation_id,
        &response,
        IcOps::now_nanos(),
    )?;
    removal_publication_response(
        publication,
        ComponentRegistryOps::root_final_inventory(request.operation_id)?,
    )
}

/// Read the locally retained exact Coordinator removal receipt without inter-Canister calls.
pub fn removal_status(
    request: FleetSubnetRootRemovalStatusRequest,
) -> Result<FleetSubnetRootRemovalPublicationResponse, InternalError> {
    let _state = validated_root_state()?;
    let publication =
        ComponentRegistryOps::root_removal_publication_if_present(request.operation_id)?
            .ok_or_else(InternalError::unavailable)?;
    let inventory = ComponentRegistryOps::root_final_inventory(request.operation_id)?;
    removal_publication_response(publication, inventory)
}

/// Resolve root-removal progress from the first durable draining fence onward.
pub fn removal_operation_status(
    operation_id: [u8; 32],
) -> Result<Option<RootRemovalOperationStatus>, InternalError> {
    if ComponentRegistryOps::current().is_none() {
        return Ok(None);
    }
    let Some(draining) = ComponentRegistryOps::root_draining_if_present(operation_id)? else {
        return Ok(None);
    };
    let final_inventory = ComponentRegistryOps::root_final_inventory_if_present(operation_id)?
        .map(final_inventory_response);
    let removal = match ComponentRegistryOps::root_removal_publication_if_present(operation_id)? {
        Some(publication) => {
            let inventory = ComponentRegistryOps::root_final_inventory(operation_id)?;
            Some(removal_publication_response(publication, inventory)?)
        }
        None => None,
    };
    let store_reclamation = ComponentRegistryOps::root_store_reclamation_if_present(operation_id)?
        .map(store_reclamation_response);
    let store_binding_finalization =
        ComponentRegistryOps::root_store_binding_finalization_if_present(operation_id)?
            .map(store_binding_finalization_response);
    let store_deletion = ComponentRegistryOps::root_store_deletion_if_present(operation_id)?
        .map(store_deletion_response);
    let deletion_preparation_intent =
        ComponentRegistryOps::root_deletion_preparation_intent_if_present(operation_id)?;
    let deletion_readiness_intent = deletion_preparation_intent
        .as_ref()
        .map(root_deletion_readiness_intent_request);
    let deletion_readiness = deletion_preparation_intent
        .as_ref()
        .filter(|intent| {
            intent.coordinator_intent_hash.is_some()
                && intent.observed_cycles_after_reclamation.is_some()
                && intent.cycles_reclaimed_at_ns.is_some()
        })
        .map(root_deletion_readiness_request)
        .transpose()?;
    let deletion_preparation =
        ComponentRegistryOps::root_deletion_preparation_if_present(operation_id)?
            .map(deletion_preparation_response);
    Ok(Some(RootRemovalOperationStatus {
        operation_id,
        draining: draining_response(draining),
        final_inventory,
        removal,
        store_reclamation,
        store_binding_finalization,
        store_deletion,
        deletion_readiness_intent,
        deletion_readiness,
        deletion_preparation,
    }))
}

/// Privately advance one accepted Root removal through its domain-owned journals.
pub fn schedule_root_removal(operation_id: [u8; 32]) {
    schedule_root_removal_after(operation_id, Duration::ZERO);
}

fn schedule_root_removal_after(operation_id: [u8; 32], delay: Duration) {
    TimerApi::defer_lifecycle_required(delay, "Fleet Subnet Root removal", async move {
        match Box::pin(advance_root_removal_once(operation_id)).await {
            Ok(true) => {}
            Ok(false) => schedule_root_removal_after(operation_id, Duration::ZERO),
            Err(_) => schedule_root_removal_after(operation_id, Duration::from_secs(1)),
        }
    });
}

async fn advance_root_removal_once(operation_id: [u8; 32]) -> Result<bool, InternalError> {
    if let Some(partition) = ComponentRegistryOps::root_component_partitions()?
        .into_iter()
        .next()
    {
        let component = partition.binding.component;
        let component_operation_id = root_component_removal_operation_id(operation_id, component);
        if let Some(existing) = ComponentRegistryOps::component_draining(component)? {
            if existing.operation_id != component_operation_id {
                return Err(InternalError::conflict());
            }
        } else {
            component_registry::begin_component_draining(RootComponentDrainingRequest {
                operation_id: component_operation_id,
                component,
                expected_registry: ComponentRegistryHead {
                    component,
                    revision: partition.revision,
                    content_hash: partition.content_hash,
                },
            })
            .await?;
            return Ok(false);
        }
        Box::pin(component_registry::advance_component_removal_once(
            component,
            component_operation_id,
        ))
        .await?;
        return Ok(false);
    }

    if let Some(canister_id) = CanisterPoolOps::handoff_candidate() {
        let (authority, _) = crate::workflow::root_authority::validated_root_authority()?;
        let recipient = authority.binding.authority.binding.coordinator;
        let response = crate::workflow::canister_pool::admin(PoolAdminCommand::Handoff {
            canister_id,
            recipient,
        })
        .await?;
        if response
            != (PoolAdminResponse::HandedOff {
                canister_id,
                recipient,
            })
        {
            return Err(InternalError::invariant());
        }
        return Ok(false);
    }

    if ComponentRegistryOps::root_final_inventory_if_present(operation_id)?.is_none() {
        fence_root_funding_for_terminal_removal(operation_id)?;
        let component_registry =
            ComponentRegistryOps::current().ok_or_else(InternalError::unavailable)?;
        fleet_registry_mirror::advance_to_draining_for_root_removal(
            component_registry.store_bootstrap,
        )
        .await?;
        let state = validated_root_state()?;
        finalize_inventory(FleetSubnetRootFinalInventoryRequest {
            operation_id,
            expected_registry: state.fleet_registry,
        })
        .await?;
        return Ok(false);
    }

    let inventory = ComponentRegistryOps::root_final_inventory(operation_id)?;
    Box::pin(advance_root_store_removal(operation_id, inventory)).await
}

fn fence_root_funding_for_terminal_removal(operation_id: [u8; 32]) -> Result<(), InternalError> {
    if crate::workflow::root_funding::current_request()?.is_some()
        || IcpRefillStoreOps::resumable_operation_count() != 0
    {
        return Err(InternalError::conflict());
    }
    ComponentRegistryOps::record_root_funding_fence(operation_id, IcOps::now_nanos())?;
    RootFundingTimerApi::fence_for_deletion()
}

async fn advance_root_store_removal(
    operation_id: [u8; 32],
    inventory: RootFleetSubnetFinalInventoryView,
) -> Result<bool, InternalError> {
    if ComponentRegistryOps::root_removal_publication_if_present(operation_id)?.is_none() {
        publish_removal(FleetSubnetRootRemovalRequest {
            operation_id,
            expected_registry: inventory.registry.clone(),
        })
        .await?;
        return Ok(false);
    }

    let Some(reclamation) = ComponentRegistryOps::root_store_reclamation_if_present(operation_id)?
    else {
        reclaim_store(FleetSubnetRootStoreReclamationRequest {
            operation_id,
            expected_final_inventory_hash: inventory.inventory_hash,
        })
        .await?;
        return Ok(false);
    };
    let Some(finalization) =
        ComponentRegistryOps::root_store_binding_finalization_if_present(operation_id)?
    else {
        finalize_store_binding(FleetSubnetRootStoreBindingFinalizationRequest {
            operation_id,
            expected_reclamation_hash: reclamation.reclamation_hash,
        })
        .await?;
        return Ok(false);
    };
    let Some(store_deletion) = ComponentRegistryOps::root_store_deletion_if_present(operation_id)?
    else {
        delete_store(FleetSubnetRootStoreDeletionRequest {
            operation_id,
            expected_binding_finalization_hash: finalization.finalization_hash,
        })
        .await?;
        return Ok(false);
    };
    if ComponentRegistryOps::root_deletion_preparation_if_present(operation_id)?.is_some() {
        return Ok(true);
    }
    let request =
        root_deletion_preparation_request(operation_id, store_deletion.deletion_hash).await?;
    prepare_deletion(request).await?;
    Ok(true)
}

fn root_component_removal_operation_id(
    root_operation_id: [u8; 32],
    component: ComponentInstanceId,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(ROOT_COMPONENT_REMOVAL_OPERATION_DOMAIN);
    hasher.update(root_operation_id);
    hasher.update(component.as_bytes());
    hasher.finalize().into()
}

async fn root_deletion_preparation_request(
    operation_id: [u8; 32],
    expected_store_deletion_hash: [u8; 32],
) -> Result<FleetSubnetRootDeletionPreparationRequest, InternalError> {
    let status = MgmtOps::canister_status(IcOps::canister_self()).await?;
    let observed_reserved_cycles = status_nat_as_u128(&status.reserved_cycles)?;
    let observed_idle_cycles_burned_per_day =
        status_nat_as_u128(&status.idle_cycles_burned_per_day)?;
    let observed_freezing_threshold_seconds =
        status_nat_as_u128(&status.settings.freezing_threshold)?;
    let retained_cycles_target = observed_idle_cycles_burned_per_day
        .checked_mul(observed_freezing_threshold_seconds)
        .map(|reserve| reserve.div_ceil(SECONDS_PER_DAY))
        .and_then(|reserve| {
            reserve.checked_add(FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES)
        })
        .ok_or_else(InternalError::invalid_input)?;
    Ok(FleetSubnetRootDeletionPreparationRequest {
        operation_id,
        expected_store_deletion_hash,
        retained_cycles_target,
        observed_reserved_cycles,
        observed_idle_cycles_burned_per_day,
        observed_freezing_threshold_seconds,
    })
}

fn status_nat_as_u128(value: &Nat) -> Result<u128, InternalError> {
    u128::try_from(value.0.clone()).map_err(|_| InternalError::invalid_input())
}

/// Reclaim the retained Store only after exact logical root removal is durable.
pub async fn reclaim_store(
    request: FleetSubnetRootStoreReclamationRequest,
) -> Result<FleetSubnetRootStoreReclamationResponse, InternalError> {
    let state = validated_root_state()?;
    let inventory = removed_root_inventory(request.operation_id)?;
    if request.expected_final_inventory_hash != inventory.inventory_hash {
        return Err(InternalError::conflict());
    }
    if let Some(existing) =
        ComponentRegistryOps::root_store_reclamation_if_present(request.operation_id)?
    {
        return Ok(store_reclamation_response(existing));
    }

    let intent =
        ComponentRegistryOps::root_store_reclamation_intent_if_present(request.operation_id)?;
    if let Some(intent) = intent {
        let intent_is_exact = [
            intent.final_inventory_hash == request.expected_final_inventory_hash,
            intent.wasm_store == inventory.wasm_store,
        ]
        .into_iter()
        .all(|valid| valid);
        if !intent_is_exact {
            return Err(InternalError::conflict());
        }
    } else {
        verify_store_before_reclamation(&state, &inventory).await?;
        ComponentRegistryOps::begin_root_store_reclamation(
            request.operation_id,
            request.expected_final_inventory_hash,
            IcOps::now_nanos(),
        )?;
    }

    let evidence = WasmStorePublicationWorkflow::reclaim_single_root_store(&inventory).await?;
    ComponentRegistryOps::record_root_store_reclamation(
        request.operation_id,
        evidence,
        IcOps::now_nanos(),
    )
    .map(store_reclamation_response)
}

/// Read one durable Store-reclamation receipt without inter-Canister calls.
pub fn store_reclamation_status(
    request: FleetSubnetRootStoreReclamationStatusRequest,
) -> Result<FleetSubnetRootStoreReclamationResponse, InternalError> {
    let _state = validated_root_state()?;
    ComponentRegistryOps::root_store_reclamation_if_present(request.operation_id)?
        .ok_or_else(InternalError::unavailable)
        .map(store_reclamation_response)
}

/// Finalize the reclaimed Store's publication binding before physical deletion is prepared.
pub async fn finalize_store_binding(
    request: FleetSubnetRootStoreBindingFinalizationRequest,
) -> Result<FleetSubnetRootStoreBindingFinalizationResponse, InternalError> {
    let _state = validated_root_state()?;
    let inventory = removed_root_inventory(request.operation_id)?;
    let reclamation =
        ComponentRegistryOps::root_store_reclamation_if_present(request.operation_id)?
            .ok_or_else(InternalError::unavailable)?;
    if request.expected_reclamation_hash != reclamation.reclamation_hash {
        return Err(InternalError::conflict());
    }
    if let Some(existing) =
        ComponentRegistryOps::root_store_binding_finalization_if_present(request.operation_id)?
    {
        return Ok(store_binding_finalization_response(existing));
    }

    let intent = ComponentRegistryOps::root_store_binding_finalization_intent_if_present(
        request.operation_id,
    )?;
    let intent = if let Some(intent) = intent {
        let intent_is_exact = [
            intent.final_inventory_hash == inventory.inventory_hash,
            intent.reclamation_hash == request.expected_reclamation_hash,
            intent.wasm_store == inventory.wasm_store,
        ]
        .into_iter()
        .all(|valid| valid);
        if !intent_is_exact {
            return Err(InternalError::conflict());
        }
        intent
    } else {
        let source =
            WasmStorePublicationWorkflow::verify_single_reclaimed_root_store_binding(&inventory)
                .await?;
        ComponentRegistryOps::begin_root_store_binding_finalization(
            request.operation_id,
            request.expected_reclamation_hash,
            source.binding,
            source.source_generation,
            IcOps::now_nanos(),
        )?
    };

    let evidence =
        WasmStorePublicationWorkflow::finalize_single_reclaimed_root_store_binding(&intent)?;
    ComponentRegistryOps::record_root_store_binding_finalization(
        request.operation_id,
        evidence,
        IcOps::now_nanos(),
    )
    .map(store_binding_finalization_response)
}

/// Read one durable Store-binding finalization receipt without inter-Canister calls.
pub fn store_binding_finalization_status(
    request: FleetSubnetRootStoreBindingFinalizationStatusRequest,
) -> Result<FleetSubnetRootStoreBindingFinalizationResponse, InternalError> {
    let _state = validated_root_state()?;
    ComponentRegistryOps::root_store_binding_finalization_if_present(request.operation_id)?
        .ok_or_else(InternalError::unavailable)
        .map(store_binding_finalization_response)
}

/// Physically delete the reclaimed Store after exact binding finalization is durable.
pub async fn delete_store(
    request: FleetSubnetRootStoreDeletionRequest,
) -> Result<FleetSubnetRootStoreDeletionResponse, InternalError> {
    let _state = validated_root_state()?;
    let inventory = removed_root_inventory(request.operation_id)?;
    let finalization =
        ComponentRegistryOps::root_store_binding_finalization_if_present(request.operation_id)?
            .ok_or_else(InternalError::unavailable)?;
    if request.expected_binding_finalization_hash != finalization.finalization_hash {
        return Err(InternalError::conflict());
    }
    if let Some(existing) =
        ComponentRegistryOps::root_store_deletion_if_present(request.operation_id)?
    {
        CanisterPoolOps::complete_store_deletion(existing.wasm_store, request.operation_id)?;
        return Ok(store_deletion_response(existing));
    }

    let intent = ComponentRegistryOps::root_store_deletion_intent_if_present(request.operation_id)?;
    let intent = if let Some(intent) = intent {
        let intent_is_exact = [
            intent.binding_finalization_hash == request.expected_binding_finalization_hash,
            intent.wasm_store == finalization.wasm_store,
            intent.binding == finalization.binding,
        ]
        .into_iter()
        .all(|valid| valid);
        if !intent_is_exact {
            return Err(InternalError::conflict());
        }
        intent
    } else {
        let authority =
            WasmStorePublicationWorkflow::verify_single_finalized_root_store_for_deletion(
                &inventory,
                &finalization,
            )
            .await?;
        ComponentRegistryOps::begin_root_store_deletion(
            request.operation_id,
            request.expected_binding_finalization_hash,
            authority,
            IcOps::now_nanos(),
        )?
    };
    CanisterPoolOps::begin_store_deletion(
        intent.wasm_store,
        request.operation_id,
        IcOps::now_nanos(),
    )?;

    let intent = if intent.observed_cycles_after_reclamation.is_some() {
        intent
    } else {
        let evidence = WasmStorePublicationWorkflow::reclaim_single_finalized_root_store_cycles(
            &intent,
            &finalization,
        )
        .await?;
        ComponentRegistryOps::record_root_store_cycle_reclamation(request.operation_id, evidence)?
    };

    let evidence =
        WasmStorePublicationWorkflow::delete_single_finalized_root_store(&intent, &finalization)
            .await?;
    let deletion = ComponentRegistryOps::record_root_store_deletion(
        request.operation_id,
        evidence,
        IcOps::now_nanos(),
    )?;
    CanisterPoolOps::complete_store_deletion(deletion.wasm_store, request.operation_id)?;
    Ok(store_deletion_response(deletion))
}

/// Read one durable Store-deletion receipt without a management or Store call.
pub fn store_deletion_status(
    request: FleetSubnetRootStoreDeletionStatusRequest,
) -> Result<FleetSubnetRootStoreDeletionResponse, InternalError> {
    let _state = validated_root_state()?;
    ComponentRegistryOps::root_store_deletion_if_present(request.operation_id)?
        .ok_or_else(InternalError::unavailable)
        .map(store_deletion_response)
}

/// Return excess root cycles to the Coordinator and publish external-deletion readiness.
pub async fn prepare_deletion(
    request: FleetSubnetRootDeletionPreparationRequest,
) -> Result<FleetSubnetRootDeletionPreparationResponse, InternalError> {
    let state = validated_root_state()?;
    if let Some(existing) =
        ComponentRegistryOps::root_deletion_preparation_if_present(request.operation_id)?
    {
        return validate_deletion_preparation_retry(&request, existing);
    }
    let store_deletion =
        ComponentRegistryOps::root_store_deletion_if_present(request.operation_id)?
            .ok_or_else(InternalError::unavailable)?;
    if request.expected_store_deletion_hash != store_deletion.deletion_hash {
        return Err(InternalError::conflict());
    }
    validate_root_deletion_cycle_reserve(&request)?;
    let coordinator = state.fleet_registry.authority.binding.coordinator;
    let intent = match ComponentRegistryOps::root_deletion_preparation_intent_if_present(
        request.operation_id,
    )? {
        Some(intent) => {
            let retry_is_exact = [
                intent.store_deletion_hash == request.expected_store_deletion_hash,
                intent.coordinator == coordinator,
                intent.retained_cycles_target == request.retained_cycles_target,
                intent.observed_reserved_cycles == request.observed_reserved_cycles,
                intent.observed_idle_cycles_burned_per_day
                    == request.observed_idle_cycles_burned_per_day,
                intent.observed_freezing_threshold_seconds
                    == request.observed_freezing_threshold_seconds,
            ]
            .into_iter()
            .all(|valid| valid);
            if !retry_is_exact {
                return Err(InternalError::conflict());
            }
            intent
        }
        None => ComponentRegistryOps::begin_root_deletion_preparation(
            request.operation_id,
            RootFleetSubnetDeletionPreparationAuthority {
                store_deletion_hash: request.expected_store_deletion_hash,
                coordinator,
                observed_cycles_before_reclamation: IcOps::canister_cycle_balance().to_u128(),
                retained_cycles_target: request.retained_cycles_target,
                observed_reserved_cycles: request.observed_reserved_cycles,
                observed_idle_cycles_burned_per_day: request.observed_idle_cycles_burned_per_day,
                observed_freezing_threshold_seconds: request.observed_freezing_threshold_seconds,
            },
            IcOps::now_nanos(),
        )?,
    };

    let intent = if intent.coordinator_intent_hash.is_some() {
        intent
    } else {
        let coordinator_intent_request = root_deletion_readiness_intent_request(&intent);
        let coordinator_intent =
            fleet_coordinator_client::root_removal_status(coordinator, request.operation_id)
                .await?
                .readiness_intent
                .ok_or_else(InternalError::unavailable)?;
        validate_root_deletion_readiness_intent_response(
            coordinator,
            &coordinator_intent_request,
            &coordinator_intent,
        )?;
        let observed_cycles_after_reclamation = reclaim_root_deletion_cycles(
            coordinator,
            intent.retained_cycles_target,
            intent.observed_cycles_before_reclamation,
        )
        .await?;
        ComponentRegistryOps::record_root_deletion_cycle_reclamation(
            request.operation_id,
            coordinator_intent.intent_hash,
            observed_cycles_after_reclamation,
            IcOps::now_nanos(),
        )?
    };

    let readiness_request = root_deletion_readiness_request(&intent)?;
    let readiness =
        fleet_coordinator_client::root_removal_status(coordinator, request.operation_id)
            .await?
            .readiness
            .ok_or_else(InternalError::unavailable)?;
    validate_root_deletion_readiness_response(
        coordinator,
        &intent,
        &readiness_request,
        &readiness,
    )?;
    ComponentRegistryOps::record_root_deletion_preparation(
        request.operation_id,
        readiness.readiness_hash,
        IcOps::now_nanos(),
    )
    .map(deletion_preparation_response)
}

/// Read the root-local readiness receipt without a Coordinator or management call.
pub fn deletion_preparation_status(
    request: FleetSubnetRootDeletionPreparationStatusRequest,
) -> Result<FleetSubnetRootDeletionPreparationResponse, InternalError> {
    let _state = validated_root_state()?;
    ComponentRegistryOps::root_deletion_preparation_if_present(request.operation_id)?
        .ok_or_else(InternalError::unavailable)
        .map(deletion_preparation_response)
}

/// Return one compact, fail-closed inventory for this active Fleet Subnet Root.
pub fn canister_summary() -> Result<FleetSubnetRootCanisterSummary, InternalError> {
    let state = validated_root_state()?;
    let stores = RootWasmStoreStateOps::wasm_stores();
    let store_canisters = u32::try_from(stores.len()).map_err(|_| InternalError::invariant())?;
    if store_canisters != 1 {
        return Err(InternalError::invariant());
    }
    CanisterPoolOps::require_store(stores[0].pid)?;
    if CanisterPoolOps::store_count() != store_canisters {
        return Err(InternalError::invariant());
    }

    summary(
        state.fleet_registry,
        state.root_entry,
        &state.component_registry,
        store_canisters,
        CanisterPoolOps::workload_count(),
        CanisterPoolOps::summary_pool_asset_count(),
    )
}

fn validated_root_state() -> Result<ValidatedFleetSubnetRootState, InternalError> {
    let authority = FleetActivationApi::root_authority().map_err(InternalError::observed_public)?;
    FleetActivationApi::require_active().map_err(InternalError::observed_public)?;
    let root = IcOps::canister_self();
    validate_protected_root(&authority, root)?;

    let mirror = FleetRegistryMirrorOps::validated_current(&authority, root)?;
    let fleet_registry = mirror.active.snapshot.version;
    let root_entry = mirror.root_entry;
    let component_registry =
        ComponentRegistryOps::current().ok_or_else(InternalError::unavailable)?;
    validate_component_registry(&authority, &fleet_registry, &component_registry)?;
    validate_draining_evidence(&root_entry, &fleet_registry)?;

    Ok(ValidatedFleetSubnetRootState {
        fleet_registry,
        root_entry,
        component_registry,
    })
}

fn validate_protected_root(
    authority: &FleetSubnetRootAuthority,
    root: candid::Principal,
) -> Result<(), InternalError> {
    if authority.binding.fleet_subnet_root != root {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

fn validate_component_registry(
    authority: &FleetSubnetRootAuthority,
    fleet_registry: &FleetRegistryVersion,
    registry: &RootComponentRegistryView,
) -> Result<(), InternalError> {
    let current = ComponentRegistrySourceAuthority {
        root: &registry.root,
        release_set: registry.release_set,
    };
    let expected = ComponentRegistrySourceAuthority {
        root: &authority.binding,
        release_set: authority.initial_release_set,
    };
    if current != expected
        || !ComponentRegistryOps::registry_covers_preparation(
            &registry.prepared_against_registry,
            fleet_registry,
        )
    {
        return Err(InternalError::invariant());
    }

    let allocated_canisters = registry
        .reserved_component_instances
        .checked_add(registry.committed_component_instances)
        .and_then(|count| count.checked_add(registry.managed_descendants))
        .ok_or_else(InternalError::invariant)?;
    if registry.known_created_component_canisters > allocated_canisters {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_draining_evidence(
    root_entry: &FleetSubnetRootEntry,
    fleet_registry: &FleetRegistryVersion,
) -> Result<(), InternalError> {
    if root_entry.status != FleetSubnetRootStatus::Draining {
        return Ok(());
    }
    ComponentRegistryOps::validate_published_root_draining(fleet_registry)?;
    Ok(())
}

fn ensure_root_is_published_draining(
    state: &ValidatedFleetSubnetRootState,
) -> Result<(), InternalError> {
    if state.root_entry.status != FleetSubnetRootStatus::Draining {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn removed_root_inventory(
    operation_id: [u8; 32],
) -> Result<RootFleetSubnetFinalInventoryView, InternalError> {
    let publication = ComponentRegistryOps::root_removal_publication_if_present(operation_id)?
        .ok_or_else(InternalError::unavailable)?;
    let inventory = ComponentRegistryOps::root_final_inventory(operation_id)?;
    if publication.final_inventory_hash != inventory.inventory_hash {
        return Err(InternalError::invariant());
    }
    Ok(inventory)
}

async fn verify_store_before_reclamation(
    state: &ValidatedFleetSubnetRootState,
    inventory: &RootFleetSubnetFinalInventoryView,
) -> Result<(), InternalError> {
    let (wasm_store, store_status) =
        WasmStorePublicationWorkflow::verify_single_root_store_for_removal().await?;
    let store = root_store::status(state.component_registry.store_bootstrap.clone()).await?;
    if store.wasm_store != wasm_store {
        return Err(InternalError::invariant());
    }
    let verified = ComponentRegistryOps::verify_root_final_inventory_store(
        inventory.operation_id,
        &store,
        &store_status,
    )?;
    if &verified != inventory {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn summary(
    fleet_registry: FleetRegistryVersion,
    root_entry: FleetSubnetRootEntry,
    registry: &RootComponentRegistryView,
    store_canisters: u32,
    workload_canisters: u32,
    pooled_canisters: u32,
) -> Result<FleetSubnetRootCanisterSummary, InternalError> {
    let infrastructure_canisters = 1_u32
        .checked_add(store_canisters)
        .ok_or_else(InternalError::invariant)?;
    if workload_canisters != registry.known_created_component_canisters {
        return Err(InternalError::invariant());
    }
    let total_canisters = infrastructure_canisters
        .checked_add(workload_canisters)
        .and_then(|count| count.checked_add(pooled_canisters))
        .ok_or_else(InternalError::invariant)?;

    Ok(FleetSubnetRootCanisterSummary {
        fleet_registry,
        placement_subnet: root_entry.placement_subnet,
        fleet_subnet_root: root_entry.fleet_subnet_root,
        status: root_entry.status,
        infrastructure_canisters,
        component_canisters: workload_canisters,
        pooled_canisters,
        total_canisters,
    })
}

fn draining_response(view: RootFleetSubnetDrainingView) -> FleetSubnetRootDrainingResponse {
    FleetSubnetRootDrainingResponse {
        operation_id: view.operation_id,
        fleet_subnet_root: view.fleet_subnet_root,
        placement_subnet: view.placement_subnet,
        active_registry: view.active_registry,
        reservation_hash: view.reservation_hash,
        component_topology_digest: view.component_topology_digest,
        active_release_set: view.active_release_set,
        next_allocation_sequence: view.next_allocation_sequence,
        reserved_component_instances: view.reserved_component_instances,
        committed_component_instances: view.committed_component_instances,
        managed_descendants: view.managed_descendants,
        known_created_component_canisters: view.known_created_component_canisters,
        root_registry_encoded_bytes: view.root_registry_encoded_bytes,
        started_at_ns: view.started_at_ns,
    }
}

fn final_inventory_response(
    view: RootFleetSubnetFinalInventoryView,
) -> FleetSubnetRootFinalInventoryResponse {
    FleetSubnetRootFinalInventoryResponse {
        operation_id: view.operation_id,
        fleet_subnet_root: view.fleet_subnet_root,
        placement_subnet: view.placement_subnet,
        registry: view.registry,
        component_topology_digest: view.component_topology_digest,
        active_release_set: view.active_release_set,
        next_allocation_sequence: view.next_allocation_sequence,
        removed_component_instances: view.removed_component_instances,
        terminal_component_history_hash: view.terminal_component_history_hash,
        root_registry_encoded_bytes: view.root_registry_encoded_bytes,
        wasm_store: view.wasm_store,
        wasm_store_catalog_hash: view.wasm_store_catalog_hash,
        wasm_store_catalog_entries: view.wasm_store_catalog_entries,
        wasm_store_occupied_bytes: view.wasm_store_occupied_bytes,
        wasm_store_template_count: view.wasm_store_template_count,
        wasm_store_release_count: view.wasm_store_release_count,
        wasm_store_gc_prepared_at_secs: view.wasm_store_gc_prepared_at_secs,
        finalized_at_ns: view.finalized_at_ns,
        inventory_hash: view.inventory_hash,
    }
}

fn validate_removal_publication_response(
    request: &FleetSubnetRootRemovalPublicationRequest,
    response: &FleetSubnetRootRemovalPublicationResponse,
) -> Result<(), InternalError> {
    let transition_revision = response.previous_version.revision.checked_add(1);
    let response_is_exact = [
        response.final_inventory == request.final_inventory,
        response.previous_version == request.expected_registry,
        response.version.authority == response.previous_version.authority,
        transition_revision.is_some_and(|revision| revision == response.version.revision),
        response.version.content_hash != [0; 32],
    ]
    .into_iter()
    .all(|valid| valid);
    if !response_is_exact {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

fn removal_publication_response(
    publication: RootFleetSubnetRemovalPublicationView,
    inventory: RootFleetSubnetFinalInventoryView,
) -> Result<FleetSubnetRootRemovalPublicationResponse, InternalError> {
    if publication.final_inventory_hash != inventory.inventory_hash {
        return Err(InternalError::invariant());
    }
    Ok(FleetSubnetRootRemovalPublicationResponse {
        final_inventory: final_inventory_response(inventory),
        previous_version: publication.previous_registry,
        version: publication.registry,
    })
}

const fn store_reclamation_response(
    view: RootFleetSubnetStoreReclamationView,
) -> FleetSubnetRootStoreReclamationResponse {
    FleetSubnetRootStoreReclamationResponse {
        operation_id: view.operation_id,
        fleet_subnet_root: view.fleet_subnet_root,
        wasm_store: view.wasm_store,
        final_inventory_hash: view.final_inventory_hash,
        reclaimed_store_bytes: view.reclaimed_store_bytes,
        reclaimed_catalog_entries: view.reclaimed_catalog_entries,
        reclaimed_template_count: view.reclaimed_template_count,
        reclaimed_release_count: view.reclaimed_release_count,
        gc_prepared_at_secs: view.gc_prepared_at_secs,
        gc_started_at_secs: view.gc_started_at_secs,
        gc_completed_at_secs: view.gc_completed_at_secs,
        gc_runs_completed: view.gc_runs_completed,
        completed_at_ns: view.completed_at_ns,
        reclamation_hash: view.reclamation_hash,
    }
}

fn store_binding_finalization_response(
    view: RootFleetSubnetStoreBindingFinalizationView,
) -> FleetSubnetRootStoreBindingFinalizationResponse {
    FleetSubnetRootStoreBindingFinalizationResponse {
        operation_id: view.operation_id,
        fleet_subnet_root: view.fleet_subnet_root,
        wasm_store: view.wasm_store,
        final_inventory_hash: view.final_inventory_hash,
        reclamation_hash: view.reclamation_hash,
        source_generation: view.source_generation,
        finalized_generation: view.finalized_generation,
        finalized_at_secs: view.finalized_at_secs,
        completed_at_ns: view.completed_at_ns,
        finalization_hash: view.finalization_hash,
    }
}

fn store_deletion_response(
    view: RootFleetSubnetStoreDeletionView,
) -> FleetSubnetRootStoreDeletionResponse {
    FleetSubnetRootStoreDeletionResponse {
        operation_id: view.operation_id,
        fleet_subnet_root: view.fleet_subnet_root,
        wasm_store: view.wasm_store,
        binding_finalization_hash: view.binding_finalization_hash,
        observed_module_hash: view.observed_module_hash,
        observed_controllers: view.observed_controllers,
        observed_cycles_before_reclamation: view.observed_cycles_before_reclamation,
        retained_cycles_target: view.retained_cycles_target,
        observed_cycles_after_reclamation: view.observed_cycles_after_reclamation,
        cycles_reclaimed_at_ns: view.cycles_reclaimed_at_ns,
        prepared_at_ns: view.prepared_at_ns,
        observed_absent_at_ns: view.observed_absent_at_ns,
        completed_at_ns: view.completed_at_ns,
        deletion_hash: view.deletion_hash,
    }
}

fn validate_deletion_preparation_retry(
    request: &FleetSubnetRootDeletionPreparationRequest,
    existing: RootFleetSubnetDeletionPreparationView,
) -> Result<FleetSubnetRootDeletionPreparationResponse, InternalError> {
    let retry_is_exact = [
        request.expected_store_deletion_hash == existing.store_deletion_hash,
        request.retained_cycles_target == existing.retained_cycles_target,
        request.observed_reserved_cycles == existing.observed_reserved_cycles,
        request.observed_idle_cycles_burned_per_day == existing.observed_idle_cycles_burned_per_day,
        request.observed_freezing_threshold_seconds == existing.observed_freezing_threshold_seconds,
    ]
    .into_iter()
    .all(|valid| valid);
    if !retry_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(deletion_preparation_response(existing))
}

const fn deletion_preparation_response(
    view: RootFleetSubnetDeletionPreparationView,
) -> FleetSubnetRootDeletionPreparationResponse {
    FleetSubnetRootDeletionPreparationResponse {
        operation_id: view.operation_id,
        fleet_subnet_root: view.fleet_subnet_root,
        coordinator: view.coordinator,
        final_inventory_hash: view.final_inventory_hash,
        store_deletion_hash: view.store_deletion_hash,
        observed_cycles_before_reclamation: view.observed_cycles_before_reclamation,
        retained_cycles_target: view.retained_cycles_target,
        observed_reserved_cycles: view.observed_reserved_cycles,
        observed_idle_cycles_burned_per_day: view.observed_idle_cycles_burned_per_day,
        observed_freezing_threshold_seconds: view.observed_freezing_threshold_seconds,
        observed_cycles_after_reclamation: view.observed_cycles_after_reclamation,
        cycles_reclaimed_at_ns: view.cycles_reclaimed_at_ns,
        coordinator_intent_hash: view.coordinator_intent_hash,
        coordinator_readiness_hash: view.coordinator_readiness_hash,
        prepared_at_ns: view.prepared_at_ns,
        completed_at_ns: view.completed_at_ns,
    }
}

fn validate_root_deletion_cycle_reserve(
    request: &FleetSubnetRootDeletionPreparationRequest,
) -> Result<(), InternalError> {
    let retained_cycles_target = request
        .observed_idle_cycles_burned_per_day
        .checked_mul(request.observed_freezing_threshold_seconds)
        .map(|reserve| reserve.div_ceil(86_400))
        .and_then(|reserve| {
            reserve.checked_add(FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES)
        });
    if retained_cycles_target != Some(request.retained_cycles_target)
        || request.observed_reserved_cycles != 0
        || request.retained_cycles_target <= FLEET_SUBNET_ROOT_DELETION_CALL_REFUND_HEADROOM_CYCLES
    {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

fn root_deletion_readiness_intent_request(
    intent: &RootFleetSubnetDeletionPreparationIntentView,
) -> FleetSubnetRootDeletionReadinessIntentRequest {
    FleetSubnetRootDeletionReadinessIntentRequest {
        operation_id: intent.operation_id,
        fleet_subnet_root: IcOps::canister_self(),
        final_inventory_hash: intent.final_inventory_hash,
        store_deletion_hash: intent.store_deletion_hash,
        observed_cycles_before_reclamation: intent.observed_cycles_before_reclamation,
        retained_cycles_target: intent.retained_cycles_target,
        observed_reserved_cycles: intent.observed_reserved_cycles,
        observed_idle_cycles_burned_per_day: intent.observed_idle_cycles_burned_per_day,
        observed_freezing_threshold_seconds: intent.observed_freezing_threshold_seconds,
        prepared_at_ns: intent.prepared_at_ns,
    }
}

fn root_deletion_readiness_request(
    intent: &RootFleetSubnetDeletionPreparationIntentView,
) -> Result<FleetSubnetRootDeletionReadinessRequest, InternalError> {
    Ok(FleetSubnetRootDeletionReadinessRequest {
        operation_id: intent.operation_id,
        fleet_subnet_root: IcOps::canister_self(),
        expected_intent_hash: intent
            .coordinator_intent_hash
            .ok_or_else(InternalError::unavailable)?,
        observed_cycles_after_reclamation: intent
            .observed_cycles_after_reclamation
            .ok_or_else(InternalError::unavailable)?,
        cycles_reclaimed_at_ns: intent
            .cycles_reclaimed_at_ns
            .ok_or_else(InternalError::unavailable)?,
    })
}

fn validate_root_deletion_readiness_intent_response(
    coordinator: candid::Principal,
    request: &FleetSubnetRootDeletionReadinessIntentRequest,
    response: &FleetSubnetRootDeletionReadinessIntentResponse,
) -> Result<(), InternalError> {
    let response_is_exact = [
        response.request == *request,
        response.coordinator == coordinator,
        response.recorded_at_ns >= request.prepared_at_ns,
        response.intent_hash != [0; 32],
    ]
    .into_iter()
    .all(|valid| valid);
    if !response_is_exact {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

fn validate_root_deletion_readiness_response(
    coordinator: candid::Principal,
    intent: &RootFleetSubnetDeletionPreparationIntentView,
    request: &FleetSubnetRootDeletionReadinessRequest,
    response: &FleetSubnetRootDeletionReadinessResponse,
) -> Result<(), InternalError> {
    let response_is_exact = [
        response.request == *request,
        response.coordinator == coordinator,
        response.final_inventory_hash == intent.final_inventory_hash,
        response.store_deletion_hash == intent.store_deletion_hash,
        response.observed_cycles_before_reclamation == intent.observed_cycles_before_reclamation,
        response.retained_cycles_target == intent.retained_cycles_target,
        response.observed_reserved_cycles == intent.observed_reserved_cycles,
        response.observed_idle_cycles_burned_per_day == intent.observed_idle_cycles_burned_per_day,
        response.observed_freezing_threshold_seconds == intent.observed_freezing_threshold_seconds,
        response.prepared_at_ns == intent.prepared_at_ns,
        response.recorded_at_ns >= request.cycles_reclaimed_at_ns,
        response.readiness_hash != [0; 32],
    ]
    .into_iter()
    .all(|valid| valid);
    if !response_is_exact {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

async fn reclaim_root_deletion_cycles(
    coordinator: candid::Principal,
    retained_cycles_target: u128,
    observed_cycles_before_reclamation: u128,
) -> Result<u128, InternalError> {
    let current_cycles = IcOps::canister_cycle_balance().to_u128();
    if current_cycles > observed_cycles_before_reclamation {
        return Err(InternalError::conflict());
    }
    let deposit_call_cost = MgmtOps::deposit_cycles_call_cost(coordinator)?;
    let target_cycles_to_retain = retained_cycles_target
        .checked_sub(FLEET_SUBNET_ROOT_DELETION_CALL_REFUND_HEADROOM_CYCLES)
        .and_then(|remaining| remaining.checked_sub(deposit_call_cost))
        .ok_or_else(InternalError::conflict)?;
    let maximum_transfer = transferable_root_deletion_cycles(
        current_cycles,
        target_cycles_to_retain,
        deposit_call_cost,
    );
    if maximum_transfer > 0 {
        let permit = reserve_root_deletion_cycle_reclamation(
            coordinator,
            maximum_transfer,
            target_cycles_to_retain,
            current_cycles,
        )?;
        let cycles_before_transfer = IcOps::canister_cycle_balance().to_u128();
        let cycles_to_transfer = transferable_root_deletion_cycles(
            cycles_before_transfer,
            target_cycles_to_retain,
            deposit_call_cost,
        );
        let result =
            MgmtOps::deposit_cycles_with_permit(&permit, coordinator, cycles_to_transfer).await;
        settle_root_deletion_cycle_reclamation(&permit, result)?;
    }
    let observed_after = IcOps::canister_cycle_balance().to_u128();
    let balance_is_reclaimed = [
        observed_after <= observed_cycles_before_reclamation,
        observed_after <= retained_cycles_target,
    ]
    .into_iter()
    .all(|valid| valid);
    if !balance_is_reclaimed {
        return Err(InternalError::conflict());
    }
    Ok(observed_after)
}

const fn transferable_root_deletion_cycles(
    current_cycles: u128,
    target_cycles_to_retain: u128,
    call_cost: u128,
) -> u128 {
    current_cycles
        .saturating_sub(target_cycles_to_retain)
        .saturating_sub(call_cost)
}

fn reserve_root_deletion_cycle_reclamation(
    coordinator: candid::Principal,
    maximum_transfer: u128,
    retained_cycles: u128,
    current_cycle_balance: u128,
) -> Result<CostGuardPermit, InternalError> {
    CostGuardWorkflow::reserve(CostGuardRequest {
        cost_class: CostClass::ValueTransfer,
        command_kind: CommandKind::new(ROOT_DELETION_CYCLE_RECLAMATION_COMMAND_KIND)
            .expect("root deletion cycle-reclamation command kind is valid"),
        quota_subject: coordinator,
        payer: IcOps::canister_self(),
        now_secs: IcOps::now_secs(),
        quota_window_secs: VALUE_TRANSFER_QUOTA_WINDOW_SECONDS,
        max_operations_per_window: MAX_VALUE_TRANSFERS_PER_WINDOW,
        current_cycle_balance,
        cycle_reservation_cycles: maximum_transfer,
        min_cycles_after_reservation: retained_cycles,
    })
    .map_err(map_cost_guard_reserve_error)
}

fn settle_root_deletion_cycle_reclamation(
    permit: &CostGuardPermit,
    result: Result<(), InternalError>,
) -> Result<(), InternalError> {
    match result {
        Ok(()) => CostGuardWorkflow::complete(permit, IcOps::now_secs()),
        Err(error) => Err(CostGuardWorkflow::recover_after_failure(
            permit,
            IcOps::now_secs(),
            error,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::{
        cdk::types::Cycles,
        dto::root_store::RootStoreBootstrapRequest,
        ids::{
            AppId, CanonicalNetworkId, ComponentTopologyDigest, CyclesFundingBudget, FleetBinding,
            FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
            FleetSubnetRootBinding, FleetSubnetRootLimits, FleetSubnetRootReleaseSet,
            FleetSubnetWasmStoreAuthority, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest,
            SubnetId,
        },
    };

    #[test]
    fn root_cycle_reclamation_retains_the_target_and_exact_call_cost() {
        let retained_cycles_target = 400_u128;
        let delayed_refund_headroom = 150;
        let call_cost = 60;
        let target_before_call = retained_cycles_target
            .saturating_sub(delayed_refund_headroom)
            .saturating_sub(call_cost);

        assert_eq!(
            transferable_root_deletion_cycles(1_500, target_before_call, call_cost),
            1_250
        );
        assert_eq!(
            transferable_root_deletion_cycles(190, target_before_call, call_cost),
            0
        );
    }

    #[test]
    fn summary_reports_exact_checked_counts_without_member_enumeration() {
        let authority = authority();
        let version = version(&authority);
        let registry = component_registry(&authority, version.clone(), 3, 1, 2, 0);
        validate_component_registry(&authority, &version, &registry)
            .expect("validate Component Registry counters");

        let summary = summary(
            version,
            root_entry(&authority, FleetSubnetRootStatus::Active),
            &registry,
            1,
            3,
            4,
        )
        .expect("build summary");

        assert_eq!(summary.infrastructure_canisters, 2);
        assert_eq!(summary.component_canisters, 3);
        assert_eq!(summary.pooled_canisters, 4);
        assert_eq!(summary.total_canisters, 9);
    }

    #[test]
    fn summary_rejects_counter_and_physical_inventory_drift() {
        let authority = authority();
        let version = version(&authority);
        let invalid_registry = component_registry(&authority, version.clone(), 4, 1, 2, 0);
        assert!(
            validate_component_registry(&authority, &version, &invalid_registry).is_err(),
            "known-created count must not exceed allocated Component-tree capacity"
        );

        let registry = component_registry(&authority, version.clone(), 3, 1, 2, 0);
        assert!(
            summary(
                version,
                root_entry(&authority, FleetSubnetRootStatus::Active),
                &registry,
                1,
                2,
                1,
            )
            .is_err(),
            "physical workload inventory must equal protected Registry principal accounting"
        );
    }

    #[test]
    fn component_registry_preparation_remains_valid_under_later_mirror_authority() {
        let authority = authority();
        let prepared = version(&authority);
        let registry = component_registry(&authority, prepared.clone(), 3, 1, 2, 0);
        let mut current = prepared.clone();
        current.revision += 3;
        current.content_hash = [11; 32];

        validate_component_registry(&authority, &current, &registry)
            .expect("later mirror covers immutable Component Registry preparation");

        let mut conflicting = prepared.clone();
        conflicting.content_hash = [12; 32];
        assert!(
            validate_component_registry(&authority, &conflicting, &registry).is_err(),
            "an equal Registry revision with a different hash must fail closed"
        );

        let mut stale = prepared;
        stale.revision -= 1;
        assert!(
            validate_component_registry(&authority, &stale, &registry).is_err(),
            "a mirror older than immutable preparation authority must fail closed"
        );
    }

    #[test]
    fn root_deletion_cycle_reserve_must_match_live_preflight_evidence() {
        let valid = FleetSubnetRootDeletionPreparationRequest {
            operation_id: [1; 32],
            expected_store_deletion_hash: [2; 32],
            retained_cycles_target: FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES + 1,
            observed_reserved_cycles: 0,
            observed_idle_cycles_burned_per_day: 86_400,
            observed_freezing_threshold_seconds: 1,
        };
        validate_root_deletion_cycle_reserve(&valid)
            .expect("exact live freezing reserve plus execution reserve is valid");

        let mut wrong_target = valid;
        wrong_target.retained_cycles_target += 1;
        assert!(
            validate_root_deletion_cycle_reserve(&wrong_target).is_err(),
            "caller cannot choose a deletion reserve different from live evidence"
        );

        let mut reserved_cycles = valid;
        reserved_cycles.observed_reserved_cycles = 1;
        assert!(
            validate_root_deletion_cycle_reserve(&reserved_cycles).is_err(),
            "reserved cycles must be cleared before the irreversible transfer"
        );
    }

    #[test]
    fn sibling_store_controllers_accept_only_exact_reconciler_authority() {
        let authority = authority().wasm_store_authority;
        let expected = sibling_wasm_store_controllers(&authority);
        let evidence = |controllers| SiblingWasmStoreLiveEvidence { controllers };

        require_sibling_wasm_store_controllers(&evidence(expected.clone()), &expected)
            .expect("exact current controllers");
        assert!(
            !sibling_wasm_store_requires_controller_update(
                &authority,
                &evidence(expected.clone()),
                &expected,
            )
            .expect("exact controllers are already prepared")
        );
        assert!(
            sibling_wasm_store_requires_controller_update(
                &authority,
                &evidence(vec![authority.fleet_subnet_root]),
                &expected,
            )
            .expect("Root-only ownership is the one accepted preparation source")
        );
        assert!(
            require_sibling_wasm_store_controllers(
                &evidence(vec![candid::Principal::anonymous()]),
                &expected,
            )
            .is_err(),
            "foreign controllers must fail closed",
        );
        assert!(
            sibling_wasm_store_requires_controller_update(
                &authority,
                &evidence(vec![candid::Principal::anonymous()]),
                &expected,
            )
            .is_err(),
            "foreign controllers cannot be replaced",
        );
    }

    fn authority() -> FleetSubnetRootAuthority {
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        let registry_authority = FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([1; 32]),
                    },
                    app: AppId::from("toko"),
                },
                coordinator_subnet: SubnetId::from_principal(candid::Principal::from_slice(
                    &[2; 29],
                )),
                coordinator: candid::Principal::from_slice(&[3; 29]),
            },
            epoch: 1,
        };
        let placement_subnet = SubnetId::from_principal(candid::Principal::from_slice(&[4; 29]));
        let fleet_subnet_root = candid::Principal::from_slice(&[5; 29]);
        FleetSubnetRootAuthority {
            binding: FleetSubnetRootBinding {
                authority: registry_authority.clone(),
                placement_subnet,
                fleet_subnet_root,
                component_admissions: Vec::new(),
                component_topology_digest: ComponentTopologyDigest::from_bytes([6; 32]),
                funding: crate::test_support::fleet_subnet_root_funding_authority(),
                limits: FleetSubnetRootLimits {
                    maximum_component_instances: 10,
                    maximum_registry_bytes: 1_024,
                    maximum_wasm_store_bytes: 2_048,
                    maximum_group_placements: 16,
                    canister_pool: canic_core::ids::FleetSubnetCanisterPoolConfig {
                        minimum_size: 1,
                        maximum_size: 10,
                        canister_cycles: Cycles::new(500_000),
                        creation_execution_margin: Cycles::new(100_000),
                    },
                    cycles_funding: CyclesFundingBudget {
                        window_secs: 60,
                        maximum_cycles: Cycles::new(1_000_000),
                    },
                },
            },
            initial_release_set: release_set,
            expected_module_hash: [7; 32],
            wasm_store_authority: FleetSubnetWasmStoreAuthority {
                authority: registry_authority,
                placement_subnet,
                fleet_subnet_root,
                wasm_store: candid::Principal::from_slice(&[10; 29]),
                installation_controller: candid::Principal::from_slice(&[11; 29]),
                release_build_id: release_set.release_build_id,
                wasm_module_hash: [12; 32],
            },
        }
    }

    fn version(authority: &FleetSubnetRootAuthority) -> FleetRegistryVersion {
        FleetRegistryVersion {
            authority: authority.binding.authority.clone(),
            revision: 4,
            content_hash: [10; 32],
        }
    }

    fn component_registry(
        authority: &FleetSubnetRootAuthority,
        prepared_against_registry: FleetRegistryVersion,
        known_created_component_canisters: u32,
        reserved_component_instances: u32,
        committed_component_instances: u32,
        managed_descendants: u32,
    ) -> RootComponentRegistryView {
        RootComponentRegistryView {
            root: authority.binding.clone(),
            prepared_against_registry,
            release_set: authority.initial_release_set,
            store_bootstrap: RootStoreBootstrapRequest {
                operation_id: [8; 32],
                manifest_payload_size_bytes: 128,
            },
            next_allocation_sequence: 4,
            reserved_component_instances,
            committed_component_instances,
            managed_descendants,
            known_created_component_canisters,
            encoded_bytes: 512,
            initial_inventory: None,
            root_draining: None,
        }
    }

    fn root_entry(
        authority: &FleetSubnetRootAuthority,
        status: FleetSubnetRootStatus,
    ) -> FleetSubnetRootEntry {
        FleetSubnetRootEntry {
            placement_subnet: authority.binding.placement_subnet,
            fleet_subnet_root: authority.binding.fleet_subnet_root,
            component_admissions: authority.binding.component_admissions.clone(),
            component_topology_digest: authority.binding.component_topology_digest,
            active_release_set: authority.initial_release_set,
            funding: authority.binding.funding.clone(),
            limits: authority.binding.limits.clone(),
            status,
        }
    }
}
