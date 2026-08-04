//! Module: workflow::fleet_subnet_root
//!
//! Responsibility: validate root authority, orchestrate draining/final inventory and project summaries.
//! Does not own: durable records, Coordinator Registry mutation, Component effects, or CLI output.
//! Boundary: root actions require consistent protected, mirror, runtime and Component authority.

use crate::{
    ops::{
        canister_pool::CanisterPoolOps, component_registry::ComponentRegistryOps,
        fleet_registry_mirror::FleetRegistryMirrorOps,
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
        bootstrap::root_store, runtime::template::publication::WasmStorePublicationWorkflow,
    },
};
use candid::Principal;
use canic_core::{
    api::fleet_activation::FleetActivationApi,
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        model::replay::CommandKind,
        ops::{
            cost_guard::{CostGuardPermit, CostGuardRequest},
            ic::{IcOps, call::CallOps, mgmt::MgmtOps},
        },
        workflow::cost_guard::{CostGuardWorkflow, map_cost_guard_reserve_error},
        workflow::runtime::fleet_activation::FleetActivationWorkflow,
    },
    dto::{
        error::Error,
        fleet_registry::{
            FleetRegistryVersion, FleetSubnetRootDeletionReadinessIntentRequest,
            FleetSubnetRootDeletionReadinessIntentResponse,
            FleetSubnetRootDeletionReadinessRequest, FleetSubnetRootDeletionReadinessResponse,
            FleetSubnetRootEntry, FleetSubnetRootRemovalPublicationRequest,
            FleetSubnetRootRemovalPublicationResponse, FleetSubnetRootStatus,
        },
        fleet_subnet_root::{
            FLEET_SUBNET_ROOT_DELETION_CALL_REFUND_HEADROOM_CYCLES,
            FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES,
            FLEET_SUBNET_ROOT_DELETION_MAXIMUM_RETAINED_CYCLES, FleetSubnetRootAuthority,
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
    },
    ids::{FleetSubnetRootBinding, FleetSubnetRootReleaseSet, FleetSubnetWasmStoreAuthority},
    protocol,
    replay_policy::CostClass,
};

const ROOT_DELETION_CYCLE_RECLAMATION_COMMAND_KIND: &str =
    "fleet_subnet_root.reclaim_deletion_cycles.v1";
const VALUE_TRANSFER_QUOTA_WINDOW_SECONDS: u64 = 60;
const MAX_VALUE_TRANSFERS_PER_WINDOW: u64 = 60;

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
    running: bool,
    module_hash: Option<Vec<u8>>,
    controllers: Vec<Principal>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SiblingWasmStoreControllerPhase {
    Temporary,
    Final,
}

/// Adopt the independently installed sibling Store under sole root control.
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

    let temporary_controllers = temporary_sibling_wasm_store_controllers(&authority);
    let final_controllers = vec![authority.fleet_subnet_root];
    RootWasmStoreStateOps::begin_sibling_wasm_store_adoption(
        &crate::ops::storage::state::root_wasm_store::SiblingWasmStoreAdoptionPlan {
            operation_id: request.operation_id,
            authority: authority.clone(),
            temporary_controllers: temporary_controllers.clone(),
            final_controllers: final_controllers.clone(),
        },
    )?;

    let observed = observe_sibling_wasm_store(&authority).await?;
    match require_sibling_wasm_store_controller_phase(
        &observed,
        &temporary_controllers,
        &final_controllers,
    )? {
        SiblingWasmStoreControllerPhase::Temporary => {
            MgmtOps::update_settings(
                &canic_core::control_plane_support::ops::ic::mgmt::UpdateSettingsArgs {
                    canister_id: authority.wasm_store,
                    settings: canic_core::control_plane_support::ops::ic::mgmt::CanisterSettings {
                        controllers: Some(final_controllers.clone()),
                        ..Default::default()
                    },
                    sender_canister_version: None,
                },
            )
            .await?;
        }
        SiblingWasmStoreControllerPhase::Final => {}
    }

    let final_observation = observe_sibling_wasm_store(&authority).await?;
    require_final_sibling_wasm_store_controllers(&final_observation, &final_controllers)?;
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
        .ok_or_else(|| InternalError::unavailable("sibling Wasm Store adoption is not complete"))
}

fn protected_sibling_wasm_store_authority(
    request: &FleetSubnetWasmStoreAdoptionRequest,
) -> Result<FleetSubnetWasmStoreAuthority, InternalError> {
    if request.operation_id == [0; 32] {
        return Err(InternalError::invalid_input(
            "sibling Wasm Store adoption operation ID must be nonzero",
        ));
    }
    let (root_authority, _) = crate::workflow::root_authority::validated_root_authority()?;
    let activation = FleetActivationWorkflow::status()?;
    if request.operation_id != activation.identity.operation_id {
        return Err(InternalError::conflict(
            "sibling Wasm Store adoption operation differs from protected install identity",
        ));
    }
    if request.authority != root_authority.wasm_store_authority {
        return Err(InternalError::conflict(
            "sibling Wasm Store adoption request differs from protected root authority",
        ));
    }
    Ok(root_authority.wasm_store_authority)
}

fn temporary_sibling_wasm_store_controllers(
    authority: &FleetSubnetWasmStoreAuthority,
) -> Vec<Principal> {
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
    use canic_core::control_plane_support::ops::ic::mgmt::CanisterStatusType;

    let status = MgmtOps::canister_status(authority.wasm_store).await?;
    let mut controllers = status.settings.controllers;
    controllers.sort();
    let evidence = SiblingWasmStoreLiveEvidence {
        running: status.status == CanisterStatusType::Running,
        module_hash: status.module_hash,
        controllers,
    };
    let module_is_exact = evidence.module_hash.as_deref() == Some(&authority.wasm_module_hash);
    if !evidence.running || !module_is_exact {
        return Err(InternalError::conflict(
            "sibling Wasm Store live status differs from protected module authority",
        ));
    }
    Ok(evidence)
}

fn require_sibling_wasm_store_controller_phase(
    observed: &SiblingWasmStoreLiveEvidence,
    temporary_controllers: &[Principal],
    final_controllers: &[Principal],
) -> Result<SiblingWasmStoreControllerPhase, InternalError> {
    if observed.controllers == temporary_controllers {
        return Ok(SiblingWasmStoreControllerPhase::Temporary);
    }
    if observed.controllers == final_controllers {
        return Ok(SiblingWasmStoreControllerPhase::Final);
    }
    Err(InternalError::conflict(
        "sibling Wasm Store controller set is neither planned temporary nor final authority",
    ))
}

fn require_final_sibling_wasm_store_controllers(
    observed: &SiblingWasmStoreLiveEvidence,
    final_controllers: &[Principal],
) -> Result<(), InternalError> {
    if observed.controllers != final_controllers {
        return Err(InternalError::conflict(
            "sibling Wasm Store did not converge to sole root control",
        ));
    }
    Ok(())
}

/// Durably fence new top-level Component allocation under exact active authority.
pub fn begin_draining(
    request: FleetSubnetRootDrainingRequest,
) -> Result<FleetSubnetRootDrainingResponse, InternalError> {
    let state = validated_root_state()?;
    if state.root_entry.status != FleetSubnetRootStatus::Active {
        return Err(InternalError::conflict(
            "only an Active Fleet Subnet Root can begin draining",
        ));
    }
    if request.expected_registry != state.fleet_registry {
        return Err(InternalError::conflict(
            "Fleet Subnet Root draining request differs from the active Registry mirror",
        ));
    }
    let draining = ComponentRegistryOps::begin_root_draining(
        request.operation_id,
        &request.expected_registry,
        IcOps::now_nanos(),
    )?;
    crate::workflow::canister_pool::stop();
    Ok(draining_response(draining))
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
            return Err(InternalError::conflict(
                "Fleet Subnet Root final inventory retry names a different Registry",
            ));
        }
        return Ok(final_inventory_response(existing));
    }
    let pooled_canisters = CanisterPoolOps::asset_count();
    if pooled_canisters != 0 {
        return Err(InternalError::unavailable(format!(
            "Fleet Subnet Root final inventory cannot orphan {pooled_canisters} prepaid pool Canisters; handoff must complete first",
        )));
    }
    if CanisterPoolOps::pending_creation().is_some() || CanisterPoolOps::pending_handoff().is_some()
    {
        return Err(InternalError::unavailable(
            "Fleet Subnet Root final inventory requires all Canister pool work to reconcile",
        ));
    }
    let intent_registry =
        ComponentRegistryOps::root_final_inventory_intent_registry(request.operation_id)?;
    if let Some(intent_registry) = intent_registry {
        if request.expected_registry != intent_registry {
            return Err(InternalError::conflict(
                "Fleet Subnet Root final inventory differs from its durable intent",
            ));
        }
    } else if request.expected_registry != state.fleet_registry {
        return Err(InternalError::conflict(
            "Fleet Subnet Root final inventory request differs from the active Registry mirror",
        ));
    }
    ComponentRegistryOps::begin_root_final_inventory(
        request.operation_id,
        &request.expected_registry,
        IcOps::now_nanos(),
    )?;
    let (wasm_store, store_status) =
        WasmStorePublicationWorkflow::quiesce_single_root_store_for_final_inventory().await?;
    let store = root_store::status(state.component_registry.store_bootstrap).await?;
    if store.wasm_store != wasm_store {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "write-fenced Store differs from the exact root release-set catalog",
        ));
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
            return Err(InternalError::conflict(
                "Fleet Subnet Root removal retry names a different Registry",
            ));
        }
        let inventory = ComponentRegistryOps::root_final_inventory(request.operation_id)?;
        return removal_publication_response(existing, inventory);
    }
    if request.expected_registry != state.fleet_registry {
        return Err(InternalError::conflict(
            "Fleet Subnet Root removal request differs from the active Registry mirror",
        ));
    }
    let coordinator = state.component_registry.root.authority.binding.coordinator;
    let (wasm_store, store_status) =
        WasmStorePublicationWorkflow::verify_single_root_store_for_removal().await?;
    let store = root_store::status(state.component_registry.store_bootstrap).await?;
    if store.wasm_store != wasm_store {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "verified removal Store differs from the exact root release-set catalog",
        ));
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
    let response = publish_removed_to_coordinator(coordinator, publication_request.clone()).await?;
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
            .ok_or_else(|| {
                InternalError::unavailable("Fleet Subnet Root removal has not been published")
            })?;
    let inventory = ComponentRegistryOps::root_final_inventory(request.operation_id)?;
    removal_publication_response(publication, inventory)
}

/// Reclaim the retained Store only after exact logical root removal is durable.
pub async fn reclaim_store(
    request: FleetSubnetRootStoreReclamationRequest,
) -> Result<FleetSubnetRootStoreReclamationResponse, InternalError> {
    let state = validated_root_state()?;
    let inventory = removed_root_inventory(request.operation_id)?;
    if request.expected_final_inventory_hash != inventory.inventory_hash {
        return Err(InternalError::conflict(
            "Fleet Subnet Root Store reclamation names a different final inventory",
        ));
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
            return Err(InternalError::conflict(
                "Fleet Subnet Root Store reclamation differs from its durable intent",
            ));
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
        .ok_or_else(|| {
            InternalError::unavailable("Fleet Subnet Root Store reclamation is not complete")
        })
        .map(store_reclamation_response)
}

/// Finalize the reclaimed Store's publication binding before physical deletion is prepared.
pub async fn finalize_store_binding(
    request: FleetSubnetRootStoreBindingFinalizationRequest,
) -> Result<FleetSubnetRootStoreBindingFinalizationResponse, InternalError> {
    let _state = validated_root_state()?;
    let inventory = removed_root_inventory(request.operation_id)?;
    let reclamation =
        ComponentRegistryOps::root_store_reclamation_if_present(request.operation_id)?.ok_or_else(
            || InternalError::unavailable("Fleet Subnet Root Store reclamation is not complete"),
        )?;
    if request.expected_reclamation_hash != reclamation.reclamation_hash {
        return Err(InternalError::conflict(
            "Fleet Subnet Root Store binding finalization names a different reclamation receipt",
        ));
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
            return Err(InternalError::conflict(
                "Fleet Subnet Root Store binding finalization differs from its durable intent",
            ));
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
        .ok_or_else(|| {
            InternalError::unavailable(
                "Fleet Subnet Root Store binding finalization is not complete",
            )
        })
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
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Fleet Subnet Root Store binding finalization is not complete",
                )
            })?;
    if request.expected_binding_finalization_hash != finalization.finalization_hash {
        return Err(InternalError::conflict(
            "Fleet Subnet Root Store deletion names a different binding finalization receipt",
        ));
    }
    if let Some(existing) =
        ComponentRegistryOps::root_store_deletion_if_present(request.operation_id)?
    {
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
            return Err(InternalError::conflict(
                "Fleet Subnet Root Store deletion differs from its durable intent",
            ));
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
    ComponentRegistryOps::record_root_store_deletion(
        request.operation_id,
        evidence,
        IcOps::now_nanos(),
    )
    .map(store_deletion_response)
}

/// Read one durable Store-deletion receipt without a management or Store call.
pub fn store_deletion_status(
    request: FleetSubnetRootStoreDeletionStatusRequest,
) -> Result<FleetSubnetRootStoreDeletionResponse, InternalError> {
    let _state = validated_root_state()?;
    ComponentRegistryOps::root_store_deletion_if_present(request.operation_id)?
        .ok_or_else(|| {
            InternalError::unavailable("Fleet Subnet Root Store deletion is not complete")
        })
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
        ComponentRegistryOps::root_store_deletion_if_present(request.operation_id)?.ok_or_else(
            || InternalError::unavailable("Fleet Subnet Root Store deletion is not complete"),
        )?;
    if request.expected_store_deletion_hash != store_deletion.deletion_hash {
        return Err(InternalError::conflict(
            "Fleet Subnet Root deletion preparation names a different Store deletion receipt",
        ));
    }
    validate_root_deletion_cycle_reserve(&request)?;
    let coordinator = state.fleet_registry.authority.binding.coordinator;
    let observed_cycles_before_reclamation = IcOps::canister_cycle_balance().to_u128();
    let intent = ComponentRegistryOps::begin_root_deletion_preparation(
        request.operation_id,
        RootFleetSubnetDeletionPreparationAuthority {
            store_deletion_hash: request.expected_store_deletion_hash,
            coordinator,
            observed_cycles_before_reclamation,
            maximum_cycles_to_retain: request.maximum_cycles_to_retain,
            observed_reserved_cycles: request.observed_reserved_cycles,
            observed_idle_cycles_burned_per_day: request.observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: request.observed_freezing_threshold_seconds,
        },
        IcOps::now_nanos(),
    )?;

    let intent = if intent.coordinator_intent_hash.is_some() {
        intent
    } else {
        let coordinator_intent_request = root_deletion_readiness_intent_request(&intent);
        let coordinator_intent = prepare_root_deletion_readiness_with_coordinator(
            coordinator,
            coordinator_intent_request.clone(),
        )
        .await?;
        validate_root_deletion_readiness_intent_response(
            coordinator,
            &coordinator_intent_request,
            &coordinator_intent,
        )?;
        let observed_cycles_after_reclamation = reclaim_root_deletion_cycles(
            coordinator,
            intent.maximum_cycles_to_retain,
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
        record_root_deletion_readiness_with_coordinator(coordinator, readiness_request.clone())
            .await?;
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
        .ok_or_else(|| {
            InternalError::unavailable("Fleet Subnet Root deletion preparation is not complete")
        })
        .map(deletion_preparation_response)
}

/// Return one compact, fail-closed inventory for this active Fleet Subnet Root.
pub fn canister_summary() -> Result<FleetSubnetRootCanisterSummary, InternalError> {
    let state = validated_root_state()?;
    let store_canisters =
        u32::try_from(RootWasmStoreStateOps::wasm_stores().len()).map_err(|_| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "root-local Wasm Store count exceeds u32",
            )
        })?;
    if store_canisters != 1 {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            format!(
                "active Fleet Subnet Root requires exactly one known local Wasm Store, found {store_canisters}"
            ),
        ));
    }

    summary(
        state.fleet_registry,
        state.root_entry,
        &state.component_registry,
        store_canisters,
        CanisterPoolOps::asset_count(),
    )
}

fn validated_root_state() -> Result<ValidatedFleetSubnetRootState, InternalError> {
    let authority = FleetActivationApi::root_authority().map_err(InternalError::public)?;
    FleetActivationApi::require_active().map_err(InternalError::public)?;
    let root = IcOps::canister_self();
    validate_protected_root(&authority, root)?;

    let mirror = FleetRegistryMirrorOps::validated_current(&authority, root)?;
    let fleet_registry = mirror.active.snapshot.version;
    let root_entry = mirror.root_entry;
    let component_registry = ComponentRegistryOps::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
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
        return Err(InternalError::invalid_input(
            "protected Fleet Subnet Root authority does not name this Canister",
        ));
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
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component Registry preparation authority is not covered by the current root mirror",
        ));
    }

    let allocated_canisters = registry
        .reserved_component_instances
        .checked_add(registry.committed_component_instances)
        .and_then(|count| count.checked_add(registry.managed_descendants))
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Component Registry allocation counters overflow",
            )
        })?;
    if registry.known_created_component_canisters > allocated_canisters {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "known-created Component Canisters exceed allocated Component-tree capacity",
        ));
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
        return Err(InternalError::conflict(
            "Fleet Subnet Root final inventory requires a published Draining Registry entry",
        ));
    }
    Ok(())
}

fn removed_root_inventory(
    operation_id: [u8; 32],
) -> Result<RootFleetSubnetFinalInventoryView, InternalError> {
    let publication = ComponentRegistryOps::root_removal_publication_if_present(operation_id)?
        .ok_or_else(|| {
            InternalError::unavailable("Fleet Subnet Root logical removal has not been published")
        })?;
    let inventory = ComponentRegistryOps::root_final_inventory(operation_id)?;
    if publication.final_inventory_hash != inventory.inventory_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root removal publication differs from retained final inventory",
        ));
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
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "verified reclamation Store differs from the exact root release-set catalog",
        ));
    }
    let verified = ComponentRegistryOps::verify_root_final_inventory_store(
        inventory.operation_id,
        &store,
        &store_status,
    )?;
    if &verified != inventory {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "verified reclamation Store differs from retained final inventory",
        ));
    }
    Ok(())
}

fn summary(
    fleet_registry: FleetRegistryVersion,
    root_entry: FleetSubnetRootEntry,
    registry: &RootComponentRegistryView,
    store_canisters: u32,
    pooled_canisters: u32,
) -> Result<FleetSubnetRootCanisterSummary, InternalError> {
    let infrastructure_canisters = 1_u32.checked_add(store_canisters).ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root-local infrastructure Canister count overflow",
        )
    })?;
    let managed_canisters = store_canisters
        .checked_add(registry.known_created_component_canisters)
        .and_then(|count| count.checked_add(pooled_canisters))
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "root-managed Canister count overflow",
            )
        })?;
    if managed_canisters > root_entry.limits.maximum_managed_canisters {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "known-created Canisters exceed the protected root limit",
        ));
    }
    let total_canisters = infrastructure_canisters
        .checked_add(registry.known_created_component_canisters)
        .and_then(|count| count.checked_add(pooled_canisters))
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Fleet Subnet Root Canister summary total overflow",
            )
        })?;

    Ok(FleetSubnetRootCanisterSummary {
        fleet_registry,
        placement_subnet: root_entry.placement_subnet,
        fleet_subnet_root: root_entry.fleet_subnet_root,
        status: root_entry.status,
        infrastructure_canisters,
        component_canisters: registry.known_created_component_canisters,
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

async fn publish_removed_to_coordinator(
    coordinator: candid::Principal,
    request: FleetSubnetRootRemovalPublicationRequest,
) -> Result<FleetSubnetRootRemovalPublicationResponse, InternalError> {
    let call = CallOps::unbounded_wait(
        coordinator,
        protocol::CANIC_FLEET_REGISTRY_PUBLISH_ROOT_REMOVED,
    )
    .with_arg(request)?
    .execute()
    .await?;
    let result: Result<FleetSubnetRootRemovalPublicationResponse, Error> = call.candid()?;
    result.map_err(InternalError::public)
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
        return Err(InternalError::invalid_input(
            "Coordinator root removal response differs from requested authority",
        ));
    }
    Ok(())
}

fn removal_publication_response(
    publication: RootFleetSubnetRemovalPublicationView,
    inventory: RootFleetSubnetFinalInventoryView,
) -> Result<FleetSubnetRootRemovalPublicationResponse, InternalError> {
    if publication.final_inventory_hash != inventory.inventory_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root removal publication differs from retained final inventory",
        ));
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
        maximum_cycles_to_retain: view.maximum_cycles_to_retain,
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
        request.maximum_cycles_to_retain == existing.maximum_cycles_to_retain,
        request.observed_reserved_cycles == existing.observed_reserved_cycles,
        request.observed_idle_cycles_burned_per_day == existing.observed_idle_cycles_burned_per_day,
        request.observed_freezing_threshold_seconds == existing.observed_freezing_threshold_seconds,
    ]
    .into_iter()
    .all(|valid| valid);
    if !retry_is_exact {
        return Err(InternalError::conflict(
            "Fleet Subnet Root deletion preparation differs from its durable receipt",
        ));
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
        maximum_cycles_to_retain: view.maximum_cycles_to_retain,
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
    let maximum_cycles_to_retain = request
        .observed_idle_cycles_burned_per_day
        .checked_mul(request.observed_freezing_threshold_seconds)
        .map(|reserve| reserve.div_ceil(86_400))
        .and_then(|reserve| {
            reserve.checked_add(FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES)
        });
    if maximum_cycles_to_retain != Some(request.maximum_cycles_to_retain)
        || request.observed_reserved_cycles != 0
        || request.maximum_cycles_to_retain
            <= FLEET_SUBNET_ROOT_DELETION_CALL_REFUND_HEADROOM_CYCLES
        || request.maximum_cycles_to_retain > FLEET_SUBNET_ROOT_DELETION_MAXIMUM_RETAINED_CYCLES
    {
        return Err(InternalError::invalid_input(
            "Fleet Subnet Root deletion cycle reserve is outside the supported range",
        ));
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
        maximum_cycles_to_retain: intent.maximum_cycles_to_retain,
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
        expected_intent_hash: intent.coordinator_intent_hash.ok_or_else(|| {
            InternalError::unavailable(
                "Coordinator root-deletion readiness intent has not been recorded",
            )
        })?,
        observed_cycles_after_reclamation: intent.observed_cycles_after_reclamation.ok_or_else(
            || {
                InternalError::unavailable(
                    "Fleet Subnet Root cycle reclamation has not been recorded",
                )
            },
        )?,
        cycles_reclaimed_at_ns: intent.cycles_reclaimed_at_ns.ok_or_else(|| {
            InternalError::unavailable(
                "Fleet Subnet Root cycle-reclamation time has not been recorded",
            )
        })?,
    })
}

async fn prepare_root_deletion_readiness_with_coordinator(
    coordinator: candid::Principal,
    request: FleetSubnetRootDeletionReadinessIntentRequest,
) -> Result<FleetSubnetRootDeletionReadinessIntentResponse, InternalError> {
    let call = CallOps::unbounded_wait(
        coordinator,
        protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_READINESS_PREPARE,
    )
    .with_arg(request)?
    .execute()
    .await?;
    let result: Result<FleetSubnetRootDeletionReadinessIntentResponse, Error> = call.candid()?;
    result.map_err(InternalError::public)
}

async fn record_root_deletion_readiness_with_coordinator(
    coordinator: candid::Principal,
    request: FleetSubnetRootDeletionReadinessRequest,
) -> Result<FleetSubnetRootDeletionReadinessResponse, InternalError> {
    let call = CallOps::unbounded_wait(
        coordinator,
        protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_READY,
    )
    .with_arg(request)?
    .execute()
    .await?;
    let result: Result<FleetSubnetRootDeletionReadinessResponse, Error> = call.candid()?;
    result.map_err(InternalError::public)
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
        return Err(InternalError::invalid_input(
            "Coordinator root-deletion readiness intent response differs from request",
        ));
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
        response.maximum_cycles_to_retain == intent.maximum_cycles_to_retain,
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
        return Err(InternalError::invalid_input(
            "Coordinator root-deletion readiness response differs from local authority",
        ));
    }
    Ok(())
}

async fn reclaim_root_deletion_cycles(
    coordinator: candid::Principal,
    maximum_cycles_to_retain: u128,
    observed_cycles_before_reclamation: u128,
) -> Result<u128, InternalError> {
    let current_cycles = IcOps::canister_cycle_balance().to_u128();
    if current_cycles > observed_cycles_before_reclamation {
        return Err(InternalError::conflict(
            "Fleet Subnet Root cycle balance increased after deletion intent",
        ));
    }
    let deposit_call_cost = MgmtOps::deposit_cycles_call_cost(coordinator)?;
    let target_cycles_to_retain = maximum_cycles_to_retain
        .checked_sub(FLEET_SUBNET_ROOT_DELETION_CALL_REFUND_HEADROOM_CYCLES)
        .and_then(|remaining| remaining.checked_sub(deposit_call_cost))
        .ok_or_else(|| {
            InternalError::conflict(
                "Fleet Subnet Root deletion reserve does not cover call-refund headroom and the exact deposit call cost",
            )
        })?;
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
        observed_after <= maximum_cycles_to_retain,
    ]
    .into_iter()
    .all(|valid| valid);
    if !balance_is_reclaimed {
        return Err(InternalError::conflict(
            "Fleet Subnet Root still exceeds its durable deletion cycle reserve",
        ));
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
        let maximum_cycles_to_retain = 400_u128;
        let delayed_refund_headroom = 150;
        let call_cost = 60;
        let target_before_call = maximum_cycles_to_retain
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
            4,
        )
        .expect("build summary");

        assert_eq!(summary.infrastructure_canisters, 2);
        assert_eq!(summary.component_canisters, 3);
        assert_eq!(summary.pooled_canisters, 4);
        assert_eq!(summary.total_canisters, 9);
    }

    #[test]
    fn summary_rejects_counter_and_protected_limit_drift() {
        let authority = authority();
        let version = version(&authority);
        let invalid_registry = component_registry(&authority, version.clone(), 4, 1, 2, 0);
        assert!(
            validate_component_registry(&authority, &version, &invalid_registry).is_err(),
            "known-created count must not exceed allocated Component-tree capacity"
        );

        let registry = component_registry(&authority, version.clone(), 3, 1, 2, 0);
        let mut entry = root_entry(&authority, FleetSubnetRootStatus::Active);
        entry.limits.maximum_managed_canisters = 3;
        assert!(
            summary(version, entry, &registry, 1, 1).is_err(),
            "Store, pool, and Component Canisters must not exceed the protected managed limit"
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
            maximum_cycles_to_retain: FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES + 1,
            observed_reserved_cycles: 0,
            observed_idle_cycles_burned_per_day: 86_400,
            observed_freezing_threshold_seconds: 1,
        };
        validate_root_deletion_cycle_reserve(&valid)
            .expect("exact live freezing reserve plus execution reserve is valid");

        let mut wrong_ceiling = valid;
        wrong_ceiling.maximum_cycles_to_retain += 1;
        assert!(
            validate_root_deletion_cycle_reserve(&wrong_ceiling).is_err(),
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
    fn sibling_store_controller_phase_accepts_only_planned_temporary_or_final_authority() {
        let authority = authority().wasm_store_authority;
        let temporary = temporary_sibling_wasm_store_controllers(&authority);
        let final_controllers = vec![authority.fleet_subnet_root];
        let evidence = |controllers| SiblingWasmStoreLiveEvidence {
            running: true,
            module_hash: Some(authority.wasm_module_hash.to_vec()),
            controllers,
        };

        assert_eq!(
            require_sibling_wasm_store_controller_phase(
                &evidence(temporary.clone()),
                &temporary,
                &final_controllers,
            )
            .expect("planned temporary controllers"),
            SiblingWasmStoreControllerPhase::Temporary,
        );
        assert_eq!(
            require_sibling_wasm_store_controller_phase(
                &evidence(final_controllers.clone()),
                &temporary,
                &final_controllers,
            )
            .expect("planned final controllers"),
            SiblingWasmStoreControllerPhase::Final,
        );
        assert!(
            require_sibling_wasm_store_controller_phase(
                &evidence(vec![candid::Principal::anonymous()]),
                &temporary,
                &final_controllers,
            )
            .is_err(),
            "foreign controllers must fail closed",
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
                limits: FleetSubnetRootLimits {
                    maximum_component_instances: 10,
                    maximum_managed_canisters: 10,
                    maximum_registry_bytes: 1_024,
                    maximum_wasm_store_bytes: 2_048,
                    canister_pool: canic_core::ids::FleetSubnetCanisterPoolConfig {
                        minimum_size: 1,
                        maximum_size: 10,
                        canister_cycles: Cycles::new(500_000),
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
            limits: authority.binding.limits.clone(),
            status,
        }
    }
}
