//! Module: workflow::fleet_subnet_root
//!
//! Responsibility: validate root authority, orchestrate draining/final inventory and project summaries.
//! Does not own: durable records, Coordinator Registry mutation, Component effects, or CLI output.
//! Boundary: root actions require consistent protected, mirror, runtime and Component authority.

use crate::{
    ops::{
        component_registry::ComponentRegistryOps, fleet_registry_mirror::FleetRegistryMirrorOps,
        storage::state::subnet::SubnetStateOps,
    },
    view::component_registry::{
        RootComponentRegistryView, RootFleetSubnetDrainingView, RootFleetSubnetFinalInventoryView,
        RootFleetSubnetRemovalPublicationView, RootFleetSubnetStoreBindingFinalizationView,
        RootFleetSubnetStoreReclamationView,
    },
    workflow::{
        bootstrap::root_store, runtime::template::publication::WasmStorePublicationWorkflow,
    },
};
use canic_core::{
    api::fleet_activation::FleetActivationApi,
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::ic::{IcOps, call::CallOps},
    },
    dto::{
        error::Error,
        fleet_registry::{
            FleetRegistryVersion, FleetSubnetRootEntry, FleetSubnetRootRemovalPublicationRequest,
            FleetSubnetRootRemovalPublicationResponse, FleetSubnetRootStatus,
        },
        fleet_subnet_root::{
            FleetSubnetRootAuthority, FleetSubnetRootCanisterSummary,
            FleetSubnetRootDrainingRequest, FleetSubnetRootDrainingResponse,
            FleetSubnetRootDrainingStatusRequest, FleetSubnetRootFinalInventoryRequest,
            FleetSubnetRootFinalInventoryResponse, FleetSubnetRootFinalInventoryStatusRequest,
            FleetSubnetRootRemovalRequest, FleetSubnetRootRemovalStatusRequest,
            FleetSubnetRootStoreBindingFinalizationRequest,
            FleetSubnetRootStoreBindingFinalizationResponse,
            FleetSubnetRootStoreBindingFinalizationStatusRequest,
            FleetSubnetRootStoreReclamationRequest, FleetSubnetRootStoreReclamationResponse,
            FleetSubnetRootStoreReclamationStatusRequest,
        },
    },
    ids::{FleetSubnetRootBinding, FleetSubnetRootReleaseSet},
    protocol,
};

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
    ComponentRegistryOps::begin_root_draining(
        request.operation_id,
        &request.expected_registry,
        IcOps::now_nanos(),
    )
    .map(draining_response)
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

/// Return one compact, fail-closed inventory for this active Fleet Subnet Root.
pub fn canister_summary() -> Result<FleetSubnetRootCanisterSummary, InternalError> {
    let state = validated_root_state()?;
    let store_canisters = u32::try_from(SubnetStateOps::wasm_stores().len()).map_err(|_| {
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
) -> Result<FleetSubnetRootCanisterSummary, InternalError> {
    let infrastructure_canisters = 1_u32.checked_add(store_canisters).ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root-local infrastructure Canister count overflow",
        )
    })?;
    let managed_canisters = store_canisters
        .checked_add(registry.known_created_component_canisters)
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
            ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
        },
    };

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
        )
        .expect("build summary");

        assert_eq!(summary.infrastructure_canisters, 2);
        assert_eq!(summary.component_canisters, 3);
        assert_eq!(summary.total_canisters, 5);
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
            summary(version, entry, &registry, 1).is_err(),
            "Store plus Component Canisters must not exceed the protected managed limit"
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

    fn authority() -> FleetSubnetRootAuthority {
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        FleetSubnetRootAuthority {
            binding: FleetSubnetRootBinding {
                authority: FleetRegistryAuthority {
                    binding: FleetCoordinatorBinding {
                        fleet: FleetBinding {
                            fleet: FleetKey {
                                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                                fleet_id: FleetId::from_generated_bytes([1; 32]),
                            },
                            app: AppId::from("toko"),
                        },
                        coordinator_subnet: SubnetId::from_principal(
                            candid::Principal::from_slice(&[2; 29]),
                        ),
                        coordinator: candid::Principal::from_slice(&[3; 29]),
                    },
                    epoch: 1,
                },
                placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[4; 29])),
                fleet_subnet_root: candid::Principal::from_slice(&[5; 29]),
                component_admissions: Vec::new(),
                component_topology_digest: ComponentTopologyDigest::from_bytes([6; 32]),
                limits: FleetSubnetRootLimits {
                    maximum_component_instances: 10,
                    maximum_managed_canisters: 10,
                    maximum_registry_bytes: 1_024,
                    maximum_wasm_store_bytes: 2_048,
                    cycles_funding: CyclesFundingBudget {
                        window_secs: 60,
                        maximum_cycles: Cycles::new(1_000_000),
                    },
                },
            },
            initial_release_set: release_set,
            expected_module_hash: [7; 32],
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
