//! Module: ops::component_registry::root_retirement
//!
//! Responsibility: commit and validate Root terminal inventory and Wasm Store retirement progress.
//! Does not own: Store effects, Root orchestration, or Fleet publication.
//! Boundary: advances exact retained records from already-authenticated, single-step evidence.

use super::{
    ComponentRegistryOps, ROOT_FINAL_INVENTORY_HASH_DOMAIN,
    ROOT_STORE_BINDING_FINALIZATION_HASH_DOMAIN, ROOT_STORE_DELETION_HASH_DOMAIN,
    ROOT_STORE_FINAL_CATALOG_HASH_DOMAIN, ROOT_STORE_RECLAMATION_HASH_DOMAIN,
    RootFleetSubnetFinalInventoryPlan, deletion_retained_cycles_target, domain_hash,
    root_deletion_preparation_intent_record_to_view, root_deletion_preparation_record_to_view,
    root_draining_record_to_view, root_final_inventory_record_matches_response,
    root_final_inventory_record_to_view, root_removal_publication_record_to_view,
    root_store_binding_finalization_intent_record_to_view,
    root_store_binding_finalization_record_to_view, root_store_deletion_intent_record_to_view,
    root_store_deletion_record_to_view, root_store_reclamation_intent_record_to_view,
    root_store_reclamation_record_to_view, terminal_root_inventory_plan,
    validate_root_draining_record,
};
use crate::{
    dto::template::WasmStoreStatusResponse,
    ids::{WasmStoreBinding, WasmStoreGcMode},
    storage::stable::component_registry::{
        RootComponentRegistryCommitError, RootComponentRegistryMetaRecord,
        RootComponentRegistryStore, RootFleetSubnetDeletionPreparationIntentRecord,
        RootFleetSubnetDeletionPreparationRecord, RootFleetSubnetDrainingRecord,
        RootFleetSubnetFinalInventoryIntentRecord, RootFleetSubnetFinalInventoryRecord,
        RootFleetSubnetRemovalPublicationRecord,
        RootFleetSubnetStoreBindingFinalizationIntentRecord,
        RootFleetSubnetStoreBindingFinalizationRecord, RootFleetSubnetStoreDeletionIntentRecord,
        RootFleetSubnetStoreDeletionRecord, RootFleetSubnetStoreReclamationIntentRecord,
        RootFleetSubnetStoreReclamationRecord,
    },
    view::component_registry::{
        RootFleetSubnetDeletionPreparationAuthority, RootFleetSubnetDeletionPreparationIntentView,
        RootFleetSubnetDeletionPreparationView, RootFleetSubnetDrainingView,
        RootFleetSubnetFinalInventoryView, RootFleetSubnetRemovalPublicationView,
        RootFleetSubnetStoreBindingFinalizationEvidence,
        RootFleetSubnetStoreBindingFinalizationIntentView,
        RootFleetSubnetStoreBindingFinalizationView, RootFleetSubnetStoreCycleReclamationEvidence,
        RootFleetSubnetStoreDeletionAuthority, RootFleetSubnetStoreDeletionEvidence,
        RootFleetSubnetStoreDeletionIntentView, RootFleetSubnetStoreDeletionView,
        RootFleetSubnetStoreReclamationEvidence, RootFleetSubnetStoreReclamationIntentView,
        RootFleetSubnetStoreReclamationView,
    },
};
use candid::CandidType;
use canic_core::{
    cdk::types::Principal,
    control_plane_support::error::InternalError,
    dto::{
        fleet_registry::{
            FleetRegistryVersion, FleetSubnetRootDrainingReservationResponse,
            FleetSubnetRootRemovalPublicationResponse, FleetSubnetRootStatus,
        },
        root_store::{RootStoreBootstrapResponse, RootStoreCatalogEntry},
    },
    ids::{ComponentTopologyDigest, FleetSubnetRootReleaseSet, SubnetId},
};

#[derive(CandidType)]
struct RootStoreFinalCatalogHashAuthority<'a> {
    fleet_subnet_root: Principal,
    wasm_store: Principal,
    release_set: FleetSubnetRootReleaseSet,
    catalog: &'a [RootStoreCatalogEntry],
    occupied_store_bytes: u64,
    template_count: u32,
    release_count: u32,
    gc_prepared_at_secs: u64,
}

#[derive(CandidType)]
struct RootFleetSubnetFinalInventoryHashAuthority<'a> {
    operation_id: [u8; 32],
    fleet_subnet_root: Principal,
    placement_subnet: SubnetId,
    registry: &'a FleetRegistryVersion,
    component_topology_digest: ComponentTopologyDigest,
    active_release_set: FleetSubnetRootReleaseSet,
    next_allocation_sequence: u64,
    removed_component_instances: u32,
    terminal_component_history_hash: [u8; 32],
    root_registry_encoded_bytes: u64,
    wasm_store: Principal,
    wasm_store_catalog_hash: [u8; 32],
    wasm_store_catalog_entries: u32,
    wasm_store_occupied_bytes: u64,
    wasm_store_template_count: u32,
    wasm_store_release_count: u32,
    wasm_store_gc_prepared_at_secs: u64,
    finalized_at_ns: u64,
}

#[derive(CandidType)]
struct RootFleetSubnetStoreReclamationHashAuthority {
    operation_id: [u8; 32],
    fleet_subnet_root: Principal,
    wasm_store: Principal,
    final_inventory_hash: [u8; 32],
    reclaimed_store_bytes: u64,
    reclaimed_catalog_entries: u32,
    reclaimed_template_count: u32,
    reclaimed_release_count: u32,
    gc_prepared_at_secs: u64,
    gc_started_at_secs: u64,
    gc_completed_at_secs: u64,
    gc_runs_completed: u32,
    completed_at_ns: u64,
}

#[derive(CandidType)]
struct RootFleetSubnetStoreBindingFinalizationHashAuthority<'a> {
    operation_id: [u8; 32],
    fleet_subnet_root: Principal,
    wasm_store: Principal,
    binding: &'a str,
    final_inventory_hash: [u8; 32],
    reclamation_hash: [u8; 32],
    source_generation: u64,
    finalized_generation: u64,
    finalized_at_secs: u64,
    completed_at_ns: u64,
}

#[derive(CandidType)]
struct RootFleetSubnetStoreDeletionHashAuthority<'a> {
    operation_id: [u8; 32],
    fleet_subnet_root: Principal,
    wasm_store: Principal,
    binding: &'a str,
    binding_finalization_hash: [u8; 32],
    observed_module_hash: [u8; 32],
    observed_controllers: &'a [Principal],
    observed_cycles_before_reclamation: u128,
    retained_cycles_target: u128,
    observed_cycles_after_reclamation: u128,
    cycles_reclaimed_at_ns: u64,
    prepared_at_ns: u64,
    observed_absent_at_ns: u64,
    completed_at_ns: u64,
}

pub(super) struct RootStoreFinalInventoryEvidence {
    pub(super) catalog_hash: [u8; 32],
    pub(super) catalog_entries: u32,
    pub(super) gc_prepared_at_secs: u64,
}

pub(super) fn root_store_final_inventory_evidence(
    current: &RootComponentRegistryMetaRecord,
    store: &RootStoreBootstrapResponse,
    status: &WasmStoreStatusResponse,
) -> Result<RootStoreFinalInventoryEvidence, InternalError> {
    let catalog_entries =
        u32::try_from(store.catalog.len()).map_err(|_| InternalError::invariant())?;
    let expected_store_entries = catalog_entries
        .checked_add(1)
        .ok_or_else(InternalError::invariant)?;
    let template_entries =
        u32::try_from(status.templates.len()).map_err(|_| InternalError::invariant())?;
    let gc_prepared_at_secs = status.gc.prepared_at.ok_or_else(InternalError::conflict)?;
    let source_is_exact = [
        store.fleet_subnet_root == current.root.fleet_subnet_root,
        store.release_set == current.release_set,
        store.wasm_store != Principal::anonymous(),
    ]
    .into_iter()
    .all(|valid| valid);
    let catalog_is_exact = [
        catalog_entries > 0,
        status.release_count == expected_store_entries,
        status.template_count == expected_store_entries,
        template_entries == expected_store_entries,
        status.occupied_store_bytes <= status.max_store_bytes,
    ]
    .into_iter()
    .all(|valid| valid);
    let gc_is_exact = [
        status.gc.mode == WasmStoreGcMode::Prepared,
        status.gc.changed_at == gc_prepared_at_secs,
        gc_prepared_at_secs > 0,
        status.gc.started_at.is_none(),
        status.gc.completed_at.is_none(),
        status.gc.runs_completed == 0,
    ]
    .into_iter()
    .all(|valid| valid);
    let evidence_is_exact = [source_is_exact, catalog_is_exact, gc_is_exact]
        .into_iter()
        .all(|valid| valid);
    if !evidence_is_exact {
        return Err(InternalError::conflict());
    }

    let mut catalog = store.catalog.clone();
    catalog.sort();
    let payload = candid::encode_one(RootStoreFinalCatalogHashAuthority {
        fleet_subnet_root: store.fleet_subnet_root,
        wasm_store: store.wasm_store,
        release_set: store.release_set,
        catalog: &catalog,
        occupied_store_bytes: status.occupied_store_bytes,
        template_count: status.template_count,
        release_count: status.release_count,
        gc_prepared_at_secs,
    })
    .map_err(|_error| InternalError::invariant())?;
    Ok(RootStoreFinalInventoryEvidence {
        catalog_hash: domain_hash(ROOT_STORE_FINAL_CATALOG_HASH_DOMAIN, &payload),
        catalog_entries,
        gc_prepared_at_secs,
    })
}

pub(super) fn validate_root_final_inventory_record(
    current: &RootComponentRegistryMetaRecord,
    draining: &RootFleetSubnetDrainingRecord,
    inventory: &RootFleetSubnetFinalInventoryRecord,
) -> Result<(), InternalError> {
    let plan = terminal_root_inventory_plan(
        current,
        draining,
        inventory.operation_id,
        &inventory.registry,
    )?;
    let component_authority_is_exact = [
        inventory.removed_component_instances == plan.removed_component_instances,
        inventory.terminal_component_history_hash == plan.terminal_component_history_hash,
        inventory.root_registry_encoded_bytes == plan.root_registry_encoded_bytes,
    ]
    .into_iter()
    .all(|valid| valid);
    let inventory_hash_is_exact = inventory.inventory_hash == root_final_inventory_hash(inventory)?;
    if ![component_authority_is_exact, inventory_hash_is_exact]
        .into_iter()
        .all(|valid| valid)
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

pub(super) fn validate_root_final_inventory_intent_record(
    current: &RootComponentRegistryMetaRecord,
    draining: &RootFleetSubnetDrainingRecord,
    intent: &RootFleetSubnetFinalInventoryIntentRecord,
    plan: &RootFleetSubnetFinalInventoryPlan,
) -> Result<(), InternalError> {
    let intent_is_exact = [
        intent.operation_id == plan.operation_id,
        intent.registry == plan.registry,
        intent.removed_component_instances == plan.removed_component_instances,
        intent.terminal_component_history_hash == plan.terminal_component_history_hash,
        intent.root_registry_encoded_bytes == plan.root_registry_encoded_bytes,
        intent.root_registry_encoded_bytes == current.encoded_bytes,
        intent.prepared_at_ns >= draining.started_at_ns,
    ]
    .into_iter()
    .all(|valid| valid);
    if !intent_is_exact {
        return Err(InternalError::invariant());
    }
    Ok(())
}

pub(super) fn root_final_inventory_hash(
    inventory: &RootFleetSubnetFinalInventoryRecord,
) -> Result<[u8; 32], InternalError> {
    let payload = candid::encode_one(RootFleetSubnetFinalInventoryHashAuthority {
        operation_id: inventory.operation_id,
        fleet_subnet_root: inventory.fleet_subnet_root,
        placement_subnet: inventory.placement_subnet,
        registry: &inventory.registry,
        component_topology_digest: inventory.component_topology_digest,
        active_release_set: inventory.active_release_set,
        next_allocation_sequence: inventory.next_allocation_sequence,
        removed_component_instances: inventory.removed_component_instances,
        terminal_component_history_hash: inventory.terminal_component_history_hash,
        root_registry_encoded_bytes: inventory.root_registry_encoded_bytes,
        wasm_store: inventory.wasm_store,
        wasm_store_catalog_hash: inventory.wasm_store_catalog_hash,
        wasm_store_catalog_entries: inventory.wasm_store_catalog_entries,
        wasm_store_occupied_bytes: inventory.wasm_store_occupied_bytes,
        wasm_store_template_count: inventory.wasm_store_template_count,
        wasm_store_release_count: inventory.wasm_store_release_count,
        wasm_store_gc_prepared_at_secs: inventory.wasm_store_gc_prepared_at_secs,
        finalized_at_ns: inventory.finalized_at_ns,
    })
    .map_err(|_error| InternalError::invariant())?;
    Ok(domain_hash(ROOT_FINAL_INVENTORY_HASH_DOMAIN, &payload))
}

pub(super) fn root_store_reclamation_record(
    draining: &RootFleetSubnetDrainingRecord,
    evidence: RootFleetSubnetStoreReclamationEvidence,
    completed_at_ns: u64,
) -> Result<RootFleetSubnetStoreReclamationRecord, InternalError> {
    let inventory = draining
        .final_inventory
        .as_ref()
        .expect("validated final root inventory");
    let intent = draining
        .store_reclamation_intent
        .as_ref()
        .ok_or_else(InternalError::unavailable)?;
    let terminal_store_is_exact = [
        evidence.wasm_store == intent.wasm_store,
        evidence.occupied_store_bytes == 0,
        evidence.catalog_entries == 0,
        evidence.template_count == 0,
        evidence.release_count == 0,
        evidence.gc_prepared_at_secs == inventory.wasm_store_gc_prepared_at_secs,
        evidence.gc_started_at_secs >= evidence.gc_prepared_at_secs,
        evidence.gc_completed_at_secs >= evidence.gc_started_at_secs,
        evidence.gc_runs_completed == 1,
        completed_at_ns >= intent.prepared_at_ns,
    ]
    .into_iter()
    .all(|valid| valid);
    if !terminal_store_is_exact {
        return Err(InternalError::conflict());
    }
    let mut record = RootFleetSubnetStoreReclamationRecord {
        operation_id: draining.operation_id,
        fleet_subnet_root: draining.fleet_subnet_root,
        wasm_store: intent.wasm_store,
        final_inventory_hash: intent.final_inventory_hash,
        reclaimed_store_bytes: inventory.wasm_store_occupied_bytes,
        reclaimed_catalog_entries: inventory.wasm_store_catalog_entries,
        reclaimed_template_count: inventory.wasm_store_template_count,
        reclaimed_release_count: inventory.wasm_store_release_count,
        gc_prepared_at_secs: evidence.gc_prepared_at_secs,
        gc_started_at_secs: evidence.gc_started_at_secs,
        gc_completed_at_secs: evidence.gc_completed_at_secs,
        gc_runs_completed: evidence.gc_runs_completed,
        completed_at_ns,
        reclamation_hash: [0; 32],
    };
    record.reclamation_hash = root_store_reclamation_hash(&record)?;
    Ok(record)
}

pub(super) fn root_store_reclamation_hash(
    reclamation: &RootFleetSubnetStoreReclamationRecord,
) -> Result<[u8; 32], InternalError> {
    let payload = candid::encode_one(RootFleetSubnetStoreReclamationHashAuthority {
        operation_id: reclamation.operation_id,
        fleet_subnet_root: reclamation.fleet_subnet_root,
        wasm_store: reclamation.wasm_store,
        final_inventory_hash: reclamation.final_inventory_hash,
        reclaimed_store_bytes: reclamation.reclaimed_store_bytes,
        reclaimed_catalog_entries: reclamation.reclaimed_catalog_entries,
        reclaimed_template_count: reclamation.reclaimed_template_count,
        reclaimed_release_count: reclamation.reclaimed_release_count,
        gc_prepared_at_secs: reclamation.gc_prepared_at_secs,
        gc_started_at_secs: reclamation.gc_started_at_secs,
        gc_completed_at_secs: reclamation.gc_completed_at_secs,
        gc_runs_completed: reclamation.gc_runs_completed,
        completed_at_ns: reclamation.completed_at_ns,
    })
    .map_err(|_error| InternalError::invariant())?;
    Ok(domain_hash(ROOT_STORE_RECLAMATION_HASH_DOMAIN, &payload))
}

pub(super) fn root_store_binding_finalization_record(
    draining: &RootFleetSubnetDrainingRecord,
    evidence: RootFleetSubnetStoreBindingFinalizationEvidence,
    completed_at_ns: u64,
) -> Result<RootFleetSubnetStoreBindingFinalizationRecord, InternalError> {
    let intent = draining
        .store_binding_finalization_intent
        .as_ref()
        .ok_or_else(InternalError::unavailable)?;
    let expected_finalized_generation = intent
        .source_generation
        .checked_add(3)
        .ok_or_else(InternalError::invariant)?;
    let terminal_binding_is_exact = [
        evidence.wasm_store == intent.wasm_store,
        evidence.binding.as_str() == intent.binding,
        evidence.source_generation == intent.source_generation,
        evidence.finalized_generation == expected_finalized_generation,
        evidence.finalized_at_secs > 0,
        completed_at_ns >= intent.prepared_at_ns,
    ]
    .into_iter()
    .all(|valid| valid);
    if !terminal_binding_is_exact {
        return Err(InternalError::conflict());
    }
    let mut record = RootFleetSubnetStoreBindingFinalizationRecord {
        operation_id: draining.operation_id,
        fleet_subnet_root: draining.fleet_subnet_root,
        wasm_store: intent.wasm_store,
        binding: intent.binding.clone(),
        final_inventory_hash: intent.final_inventory_hash,
        reclamation_hash: intent.reclamation_hash,
        source_generation: intent.source_generation,
        finalized_generation: evidence.finalized_generation,
        finalized_at_secs: evidence.finalized_at_secs,
        completed_at_ns,
        finalization_hash: [0; 32],
    };
    record.finalization_hash = root_store_binding_finalization_hash(&record)?;
    Ok(record)
}

pub(super) fn root_store_binding_finalization_hash(
    finalization: &RootFleetSubnetStoreBindingFinalizationRecord,
) -> Result<[u8; 32], InternalError> {
    let payload = candid::encode_one(RootFleetSubnetStoreBindingFinalizationHashAuthority {
        operation_id: finalization.operation_id,
        fleet_subnet_root: finalization.fleet_subnet_root,
        wasm_store: finalization.wasm_store,
        binding: &finalization.binding,
        final_inventory_hash: finalization.final_inventory_hash,
        reclamation_hash: finalization.reclamation_hash,
        source_generation: finalization.source_generation,
        finalized_generation: finalization.finalized_generation,
        finalized_at_secs: finalization.finalized_at_secs,
        completed_at_ns: finalization.completed_at_ns,
    })
    .map_err(|_error| InternalError::invariant())?;
    Ok(domain_hash(
        ROOT_STORE_BINDING_FINALIZATION_HASH_DOMAIN,
        &payload,
    ))
}

pub(super) fn root_store_deletion_record(
    draining: &RootFleetSubnetDrainingRecord,
    evidence: RootFleetSubnetStoreDeletionEvidence,
    completed_at_ns: u64,
) -> Result<RootFleetSubnetStoreDeletionRecord, InternalError> {
    let intent = draining
        .store_deletion_intent
        .as_ref()
        .ok_or_else(InternalError::unavailable)?;
    let observed_cycles_after_reclamation = intent
        .observed_cycles_after_reclamation
        .ok_or_else(InternalError::unavailable)?;
    let cycles_reclaimed_at_ns = intent
        .cycles_reclaimed_at_ns
        .ok_or_else(InternalError::unavailable)?;
    let terminal_absence_is_exact = [
        evidence.wasm_store == intent.wasm_store,
        evidence.binding.as_str() == intent.binding,
        evidence.observed_module_hash == intent.observed_module_hash,
        evidence.observed_controllers == intent.observed_controllers,
        evidence.observed_cycles_before_reclamation == intent.observed_cycles_before_reclamation,
        evidence.retained_cycles_target == intent.retained_cycles_target,
        evidence.observed_cycles_after_reclamation == observed_cycles_after_reclamation,
        evidence.cycles_reclaimed_at_ns == cycles_reclaimed_at_ns,
        evidence.observed_absent_at_ns >= cycles_reclaimed_at_ns,
        completed_at_ns >= evidence.observed_absent_at_ns,
    ]
    .into_iter()
    .all(|valid| valid);
    if !terminal_absence_is_exact {
        return Err(InternalError::conflict());
    }
    let mut record = RootFleetSubnetStoreDeletionRecord {
        operation_id: draining.operation_id,
        fleet_subnet_root: draining.fleet_subnet_root,
        wasm_store: intent.wasm_store,
        binding: intent.binding.clone(),
        binding_finalization_hash: intent.binding_finalization_hash,
        observed_module_hash: intent.observed_module_hash,
        observed_controllers: intent.observed_controllers.clone(),
        observed_cycles_before_reclamation: intent.observed_cycles_before_reclamation,
        retained_cycles_target: intent.retained_cycles_target,
        observed_cycles_after_reclamation,
        cycles_reclaimed_at_ns,
        prepared_at_ns: intent.prepared_at_ns,
        observed_absent_at_ns: evidence.observed_absent_at_ns,
        completed_at_ns,
        deletion_hash: [0; 32],
    };
    record.deletion_hash = root_store_deletion_hash(&record)?;
    Ok(record)
}

pub(super) fn root_store_deletion_hash(
    deletion: &RootFleetSubnetStoreDeletionRecord,
) -> Result<[u8; 32], InternalError> {
    let payload = candid::encode_one(RootFleetSubnetStoreDeletionHashAuthority {
        operation_id: deletion.operation_id,
        fleet_subnet_root: deletion.fleet_subnet_root,
        wasm_store: deletion.wasm_store,
        binding: &deletion.binding,
        binding_finalization_hash: deletion.binding_finalization_hash,
        observed_module_hash: deletion.observed_module_hash,
        observed_controllers: &deletion.observed_controllers,
        observed_cycles_before_reclamation: deletion.observed_cycles_before_reclamation,
        retained_cycles_target: deletion.retained_cycles_target,
        observed_cycles_after_reclamation: deletion.observed_cycles_after_reclamation,
        cycles_reclaimed_at_ns: deletion.cycles_reclaimed_at_ns,
        prepared_at_ns: deletion.prepared_at_ns,
        observed_absent_at_ns: deletion.observed_absent_at_ns,
        completed_at_ns: deletion.completed_at_ns,
    })
    .map_err(|_error| InternalError::invariant())?;
    Ok(domain_hash(ROOT_STORE_DELETION_HASH_DOMAIN, &payload))
}

fn canonical_controller_set(controllers: &[Principal]) -> bool {
    !controllers.is_empty() && controllers.windows(2).all(|pair| pair[0] < pair[1])
}

pub(super) fn validate_root_store_deletion_authority(
    expected_binding_finalization_hash: [u8; 32],
    authority: &RootFleetSubnetStoreDeletionAuthority,
    prepared_at_ns: u64,
) -> Result<(), InternalError> {
    let authority_is_complete = [
        expected_binding_finalization_hash != [0; 32],
        !authority.binding.as_str().is_empty(),
        authority.observed_module_hash != [0; 32],
        canonical_controller_set(&authority.observed_controllers),
        authority.observed_cycles_before_reclamation > 0,
        authority.retained_cycles_target > 0,
        prepared_at_ns > 0,
    ]
    .into_iter()
    .all(|valid| valid);
    if !authority_is_complete {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

impl ComponentRegistryOps {
    pub(crate) fn begin_root_draining(
        operation_id: [u8; 32],
        expected_registry: &FleetRegistryVersion,
        reservation: &FleetSubnetRootDrainingReservationResponse,
        started_at_ns: u64,
    ) -> Result<RootFleetSubnetDrainingView, InternalError> {
        if operation_id == [0; 32] {
            return Err(InternalError::invalid_input());
        }
        if started_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        if reservation.reservation_hash == [0; 32] {
            return Err(InternalError::invalid_input());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        if let Some(existing) = current.root_draining.as_ref() {
            validate_root_draining_record(&current, existing)?;
            return if existing.operation_id == operation_id
                && &existing.active_registry == expected_registry
                && &existing.reservation == reservation
            {
                Ok(root_draining_record_to_view(existing.clone()))
            } else {
                Err(InternalError::conflict())
            };
        }
        if !Self::registry_covers_preparation(&current.prepared_against_registry, expected_registry)
        {
            return Err(InternalError::conflict());
        }
        let record = RootFleetSubnetDrainingRecord {
            operation_id,
            fleet_subnet_root: current.root.fleet_subnet_root,
            placement_subnet: current.root.placement_subnet,
            active_registry: expected_registry.clone(),
            reservation: reservation.clone(),
            component_topology_digest: current.root.component_topology_digest,
            active_release_set: current.release_set,
            next_allocation_sequence: current.next_allocation_sequence,
            reserved_component_instances: current.reserved_component_instances,
            committed_component_instances: current.committed_component_instances,
            managed_descendants: current.managed_descendants,
            known_created_component_canisters: current.known_created_component_canisters,
            root_registry_encoded_bytes: current.encoded_bytes,
            started_at_ns,
            funding_fenced_at_ns: None,
            final_inventory_intent: None,
            final_inventory: None,
            removal_publication: None,
            store_reclamation_intent: None,
            store_reclamation: None,
            store_binding_finalization_intent: None,
            store_binding_finalization: None,
            store_deletion_intent: None,
            store_deletion: None,
            root_deletion_preparation_intent: None,
            root_deletion_preparation: None,
        };
        RootComponentRegistryStore::begin_root_draining(&current, record.clone()).map_err(
            |error| match error {
                RootComponentRegistryCommitError::ConflictingState => InternalError::conflict(),
            },
        )?;
        Ok(root_draining_record_to_view(record))
    }

    pub(crate) fn root_draining(
        operation_id: [u8; 32],
    ) -> Result<RootFleetSubnetDrainingView, InternalError> {
        Self::root_draining_if_present(operation_id)?.ok_or_else(InternalError::unavailable)
    }

    pub(crate) fn root_draining_if_present(
        operation_id: [u8; 32],
    ) -> Result<Option<RootFleetSubnetDrainingView>, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let Some(record) = current.root_draining.as_ref() else {
            return Ok(None);
        };
        validate_root_draining_record(&current, record)?;
        if record.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        Ok(Some(root_draining_record_to_view(record.clone())))
    }

    /// Resolve funding eligibility from the exact local lifecycle fence and Registry state.
    pub(crate) fn root_funding_eligible(
        status: FleetSubnetRootStatus,
    ) -> Result<bool, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current.root_draining.as_ref();
        if let Some(draining) = draining {
            validate_root_draining_record(&current, draining)?;
        }
        match status {
            FleetSubnetRootStatus::Active => {
                Ok(draining.is_none_or(|draining| draining.funding_fenced_at_ns.is_none()))
            }
            FleetSubnetRootStatus::Draining => draining
                .map(|draining| draining.funding_fenced_at_ns.is_none())
                .ok_or_else(InternalError::invariant),
            FleetSubnetRootStatus::Joining | FleetSubnetRootStatus::Removed => Ok(false),
        }
    }

    pub(crate) fn validate_published_root_draining(
        current_registry: &FleetRegistryVersion,
    ) -> Result<RootFleetSubnetDrainingView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::invariant)?;
        validate_root_draining_record(&current, record)?;
        let publication_is_later = record.active_registry.authority == current_registry.authority
            && record.active_registry.revision < current_registry.revision;
        if !publication_is_later {
            return Err(InternalError::invariant());
        }
        Ok(root_draining_record_to_view(record.clone()))
    }

    pub(crate) fn require_root_store_admin_open() -> Result<(), InternalError> {
        let Some(current) = RootComponentRegistryStore::current() else {
            return Ok(());
        };
        if current.root_draining.is_some() {
            return Err(InternalError::conflict());
        }
        Ok(())
    }

    pub(crate) fn prepare_root_final_inventory(
        operation_id: [u8; 32],
        expected_registry: &FleetRegistryVersion,
    ) -> Result<RootFleetSubnetFinalInventoryPlan, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        let plan =
            terminal_root_inventory_plan(&current, draining, operation_id, expected_registry)?;
        if let Some(intent) = draining.final_inventory_intent.as_ref() {
            validate_root_final_inventory_intent_record(&current, draining, intent, &plan)?;
        }
        Ok(plan)
    }

    pub(crate) fn root_final_inventory_intent_registry(
        operation_id: [u8; 32],
    ) -> Result<Option<FleetRegistryVersion>, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        let Some(intent) = draining.final_inventory_intent.as_ref() else {
            return Ok(None);
        };
        let plan =
            terminal_root_inventory_plan(&current, draining, operation_id, &intent.registry)?;
        validate_root_final_inventory_intent_record(&current, draining, intent, &plan)?;
        Ok(Some(intent.registry.clone()))
    }

    pub(crate) fn record_root_funding_fence(
        operation_id: [u8; 32],
        fenced_at_ns: u64,
    ) -> Result<RootFleetSubnetDrainingView, InternalError> {
        if fenced_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        if draining.funding_fenced_at_ns.is_some() {
            return Ok(root_draining_record_to_view(draining.clone()));
        }
        RootComponentRegistryStore::record_root_funding_fence(&current, fenced_at_ns).map_err(
            |RootComponentRegistryCommitError::ConflictingState| InternalError::conflict(),
        )?;
        Self::root_draining(operation_id)
    }

    pub(crate) fn begin_root_final_inventory(
        operation_id: [u8; 32],
        expected_registry: &FleetRegistryVersion,
        prepared_at_ns: u64,
    ) -> Result<RootFleetSubnetFinalInventoryPlan, InternalError> {
        if prepared_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        let plan = Self::prepare_root_final_inventory(operation_id, expected_registry)?;
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        if let Some(intent) = draining.final_inventory_intent.as_ref() {
            validate_root_final_inventory_intent_record(&current, draining, intent, &plan)?;
            return Ok(plan);
        }
        if prepared_at_ns < draining.started_at_ns {
            return Err(InternalError::invalid_input());
        }
        let record = RootFleetSubnetFinalInventoryIntentRecord {
            operation_id,
            registry: expected_registry.clone(),
            removed_component_instances: plan.removed_component_instances,
            terminal_component_history_hash: plan.terminal_component_history_hash,
            root_registry_encoded_bytes: plan.root_registry_encoded_bytes,
            prepared_at_ns,
        };
        RootComponentRegistryStore::prepare_root_final_inventory(&current, record).map_err(
            |RootComponentRegistryCommitError::ConflictingState| InternalError::conflict(),
        )?;
        let committed = Self::prepare_root_final_inventory(operation_id, expected_registry)?;
        if committed != plan {
            return Err(InternalError::invariant());
        }
        Ok(committed)
    }

    pub(crate) fn root_final_inventory(
        operation_id: [u8; 32],
    ) -> Result<RootFleetSubnetFinalInventoryView, InternalError> {
        Self::root_final_inventory_if_present(operation_id)?.ok_or_else(InternalError::unavailable)
    }

    pub(crate) fn root_final_inventory_if_present(
        operation_id: [u8; 32],
    ) -> Result<Option<RootFleetSubnetFinalInventoryView>, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        let Some(inventory) = draining.final_inventory.as_ref() else {
            return Ok(None);
        };
        validate_root_final_inventory_record(&current, draining, inventory)?;
        Ok(Some(root_final_inventory_record_to_view(inventory.clone())))
    }

    pub(crate) fn verify_root_final_inventory_store(
        operation_id: [u8; 32],
        store: &RootStoreBootstrapResponse,
        store_status: &WasmStoreStatusResponse,
    ) -> Result<RootFleetSubnetFinalInventoryView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        let inventory = draining
            .final_inventory
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_final_inventory_record(&current, draining, inventory)?;
        let evidence = root_store_final_inventory_evidence(&current, store, store_status)?;
        let store_is_exact = [
            store.wasm_store == inventory.wasm_store,
            evidence.catalog_hash == inventory.wasm_store_catalog_hash,
            evidence.catalog_entries == inventory.wasm_store_catalog_entries,
            store_status.occupied_store_bytes == inventory.wasm_store_occupied_bytes,
            store_status.template_count == inventory.wasm_store_template_count,
            store_status.release_count == inventory.wasm_store_release_count,
            evidence.gc_prepared_at_secs == inventory.wasm_store_gc_prepared_at_secs,
        ]
        .into_iter()
        .all(|valid| valid);
        if !store_is_exact {
            return Err(InternalError::conflict());
        }
        Ok(root_final_inventory_record_to_view(inventory.clone()))
    }

    pub(crate) fn root_removal_publication_if_present(
        operation_id: [u8; 32],
    ) -> Result<Option<RootFleetSubnetRemovalPublicationView>, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        Ok(draining
            .removal_publication
            .clone()
            .map(root_removal_publication_record_to_view))
    }

    pub(crate) fn record_root_removal_publication(
        operation_id: [u8; 32],
        response: &FleetSubnetRootRemovalPublicationResponse,
        recorded_at_ns: u64,
    ) -> Result<RootFleetSubnetRemovalPublicationView, InternalError> {
        if let Some(existing) = Self::root_removal_publication_if_present(operation_id)? {
            let response_is_exact = [
                existing.operation_id == operation_id,
                existing.final_inventory_hash == response.final_inventory.inventory_hash,
                existing.previous_registry == response.previous_version,
                existing.registry == response.version,
            ]
            .into_iter()
            .all(|valid| valid);
            if response_is_exact {
                return Ok(existing);
            }
            return Err(InternalError::conflict());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        let inventory = draining
            .final_inventory
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        if !root_final_inventory_record_matches_response(inventory, &response.final_inventory) {
            return Err(InternalError::invalid_input());
        }
        let record = RootFleetSubnetRemovalPublicationRecord {
            operation_id,
            final_inventory_hash: inventory.inventory_hash,
            previous_registry: response.previous_version.clone(),
            registry: response.version.clone(),
            recorded_at_ns,
        };
        RootComponentRegistryStore::record_root_removal_publication(&current, record).map_err(
            |RootComponentRegistryCommitError::ConflictingState| InternalError::conflict(),
        )?;
        Self::root_removal_publication_if_present(operation_id)?
            .ok_or_else(InternalError::invariant)
    }

    pub(crate) fn root_store_reclamation_intent_if_present(
        operation_id: [u8; 32],
    ) -> Result<Option<RootFleetSubnetStoreReclamationIntentView>, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        Ok(draining
            .store_reclamation_intent
            .map(root_store_reclamation_intent_record_to_view))
    }

    pub(crate) fn begin_root_store_reclamation(
        operation_id: [u8; 32],
        expected_final_inventory_hash: [u8; 32],
        prepared_at_ns: u64,
    ) -> Result<RootFleetSubnetStoreReclamationIntentView, InternalError> {
        if expected_final_inventory_hash == [0; 32] {
            return Err(InternalError::invalid_input());
        }
        if prepared_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        if let Some(existing) = Self::root_store_reclamation_intent_if_present(operation_id)? {
            if existing.final_inventory_hash == expected_final_inventory_hash {
                return Ok(existing);
            }
            return Err(InternalError::conflict());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        let inventory = draining
            .final_inventory
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        if draining.removal_publication.is_none() {
            return Err(InternalError::unavailable());
        }
        if inventory.inventory_hash != expected_final_inventory_hash {
            return Err(InternalError::conflict());
        }
        let record = RootFleetSubnetStoreReclamationIntentRecord {
            operation_id,
            final_inventory_hash: inventory.inventory_hash,
            wasm_store: inventory.wasm_store,
            prepared_at_ns,
        };
        RootComponentRegistryStore::prepare_root_store_reclamation(&current, record).map_err(
            |RootComponentRegistryCommitError::ConflictingState| InternalError::conflict(),
        )?;
        Self::root_store_reclamation_intent_if_present(operation_id)?
            .ok_or_else(InternalError::invariant)
    }

    pub(crate) fn root_store_reclamation_if_present(
        operation_id: [u8; 32],
    ) -> Result<Option<RootFleetSubnetStoreReclamationView>, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        Ok(draining
            .store_reclamation
            .map(root_store_reclamation_record_to_view))
    }

    pub(crate) fn record_root_store_reclamation(
        operation_id: [u8; 32],
        evidence: RootFleetSubnetStoreReclamationEvidence,
        completed_at_ns: u64,
    ) -> Result<RootFleetSubnetStoreReclamationView, InternalError> {
        if let Some(existing) = Self::root_store_reclamation_if_present(operation_id)? {
            return Ok(existing);
        }
        if completed_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        let record = root_store_reclamation_record(draining, evidence, completed_at_ns)?;
        RootComponentRegistryStore::record_root_store_reclamation(&current, record).map_err(
            |RootComponentRegistryCommitError::ConflictingState| InternalError::conflict(),
        )?;
        let committed = Self::root_store_reclamation_if_present(operation_id)?
            .ok_or_else(InternalError::invariant)?;
        if committed != root_store_reclamation_record_to_view(record) {
            return Err(InternalError::invariant());
        }
        Ok(committed)
    }

    pub(crate) fn root_store_binding_finalization_intent_if_present(
        operation_id: [u8; 32],
    ) -> Result<Option<RootFleetSubnetStoreBindingFinalizationIntentView>, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        Ok(draining
            .store_binding_finalization_intent
            .clone()
            .map(root_store_binding_finalization_intent_record_to_view))
    }

    pub(crate) fn begin_root_store_binding_finalization(
        operation_id: [u8; 32],
        expected_reclamation_hash: [u8; 32],
        binding: WasmStoreBinding,
        source_generation: u64,
        prepared_at_ns: u64,
    ) -> Result<RootFleetSubnetStoreBindingFinalizationIntentView, InternalError> {
        let request_is_valid = [
            expected_reclamation_hash != [0; 32],
            !binding.as_str().is_empty(),
            source_generation > 0,
            prepared_at_ns > 0,
        ]
        .into_iter()
        .all(|valid| valid);
        if !request_is_valid {
            return Err(InternalError::invalid_input());
        }
        if let Some(existing) =
            Self::root_store_binding_finalization_intent_if_present(operation_id)?
        {
            let retry_is_exact = [
                existing.reclamation_hash == expected_reclamation_hash,
                existing.binding == binding,
                existing.source_generation == source_generation,
            ]
            .into_iter()
            .all(|valid| valid);
            if retry_is_exact {
                return Ok(existing);
            }
            return Err(InternalError::conflict());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        let reclamation = draining
            .store_reclamation
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        if reclamation.reclamation_hash != expected_reclamation_hash {
            return Err(InternalError::conflict());
        }
        let record = RootFleetSubnetStoreBindingFinalizationIntentRecord {
            operation_id,
            final_inventory_hash: reclamation.final_inventory_hash,
            reclamation_hash: reclamation.reclamation_hash,
            wasm_store: reclamation.wasm_store,
            binding: binding.as_str().to_string(),
            source_generation,
            prepared_at_ns,
        };
        RootComponentRegistryStore::prepare_root_store_binding_finalization(&current, record)
            .map_err(|RootComponentRegistryCommitError::ConflictingState| {
                InternalError::conflict()
            })?;
        Self::root_store_binding_finalization_intent_if_present(operation_id)?
            .ok_or_else(InternalError::invariant)
    }

    pub(crate) fn root_store_binding_finalization_if_present(
        operation_id: [u8; 32],
    ) -> Result<Option<RootFleetSubnetStoreBindingFinalizationView>, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        Ok(draining
            .store_binding_finalization
            .clone()
            .map(root_store_binding_finalization_record_to_view))
    }

    pub(crate) fn record_root_store_binding_finalization(
        operation_id: [u8; 32],
        evidence: RootFleetSubnetStoreBindingFinalizationEvidence,
        completed_at_ns: u64,
    ) -> Result<RootFleetSubnetStoreBindingFinalizationView, InternalError> {
        if let Some(existing) = Self::root_store_binding_finalization_if_present(operation_id)? {
            return Ok(existing);
        }
        if completed_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        let record = root_store_binding_finalization_record(draining, evidence, completed_at_ns)?;
        RootComponentRegistryStore::record_root_store_binding_finalization(
            &current,
            record.clone(),
        )
        .map_err(|RootComponentRegistryCommitError::ConflictingState| InternalError::conflict())?;
        let committed = Self::root_store_binding_finalization_if_present(operation_id)?
            .ok_or_else(InternalError::invariant)?;
        if committed != root_store_binding_finalization_record_to_view(record) {
            return Err(InternalError::invariant());
        }
        Ok(committed)
    }

    pub(crate) fn root_store_deletion_intent_if_present(
        operation_id: [u8; 32],
    ) -> Result<Option<RootFleetSubnetStoreDeletionIntentView>, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        Ok(draining
            .store_deletion_intent
            .clone()
            .map(root_store_deletion_intent_record_to_view))
    }

    pub(crate) fn begin_root_store_deletion(
        operation_id: [u8; 32],
        expected_binding_finalization_hash: [u8; 32],
        authority: RootFleetSubnetStoreDeletionAuthority,
        prepared_at_ns: u64,
    ) -> Result<RootFleetSubnetStoreDeletionIntentView, InternalError> {
        validate_root_store_deletion_authority(
            expected_binding_finalization_hash,
            &authority,
            prepared_at_ns,
        )?;
        let RootFleetSubnetStoreDeletionAuthority {
            wasm_store,
            binding,
            observed_module_hash,
            observed_controllers,
            observed_cycles_before_reclamation,
            retained_cycles_target,
        } = authority;
        if let Some(existing) = Self::root_store_deletion_intent_if_present(operation_id)? {
            let retry_is_exact = [
                existing.binding_finalization_hash == expected_binding_finalization_hash,
                existing.wasm_store == wasm_store,
                existing.binding == binding,
                existing.observed_module_hash == observed_module_hash,
                existing.observed_controllers == observed_controllers,
                existing.observed_cycles_before_reclamation == observed_cycles_before_reclamation,
                existing.retained_cycles_target == retained_cycles_target,
            ]
            .into_iter()
            .all(|valid| valid);
            if retry_is_exact {
                return Ok(existing);
            }
            return Err(InternalError::conflict());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        let finalization = draining
            .store_binding_finalization
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        if finalization.finalization_hash != expected_binding_finalization_hash {
            return Err(InternalError::conflict());
        }
        if finalization.binding != binding.as_str() {
            return Err(InternalError::conflict());
        }
        if finalization.wasm_store != wasm_store {
            return Err(InternalError::conflict());
        }
        if !observed_controllers.contains(&draining.fleet_subnet_root) {
            return Err(InternalError::conflict());
        }
        let record = RootFleetSubnetStoreDeletionIntentRecord {
            operation_id,
            binding_finalization_hash: finalization.finalization_hash,
            wasm_store: finalization.wasm_store,
            binding: finalization.binding.clone(),
            observed_module_hash,
            observed_controllers,
            observed_cycles_before_reclamation,
            retained_cycles_target,
            observed_cycles_after_reclamation: None,
            cycles_reclaimed_at_ns: None,
            prepared_at_ns,
        };
        RootComponentRegistryStore::prepare_root_store_deletion(&current, record).map_err(
            |RootComponentRegistryCommitError::ConflictingState| InternalError::conflict(),
        )?;
        Self::root_store_deletion_intent_if_present(operation_id)?
            .ok_or_else(InternalError::invariant)
    }

    pub(crate) fn record_root_store_cycle_reclamation(
        operation_id: [u8; 32],
        evidence: RootFleetSubnetStoreCycleReclamationEvidence,
    ) -> Result<RootFleetSubnetStoreDeletionIntentView, InternalError> {
        let existing = Self::root_store_deletion_intent_if_present(operation_id)?
            .ok_or_else(InternalError::unavailable)?;
        if existing.observed_cycles_after_reclamation.is_some() {
            let retry_is_exact = [
                existing.observed_cycles_after_reclamation
                    == Some(evidence.observed_cycles_after_reclamation),
                existing.cycles_reclaimed_at_ns == Some(evidence.cycles_reclaimed_at_ns),
            ]
            .into_iter()
            .all(|valid| valid);
            if retry_is_exact {
                return Ok(existing);
            }
            return Err(InternalError::conflict());
        }
        let evidence_is_valid = [
            evidence.observed_cycles_after_reclamation
                <= existing.observed_cycles_before_reclamation,
            evidence.observed_cycles_after_reclamation <= existing.retained_cycles_target,
            evidence.cycles_reclaimed_at_ns >= existing.prepared_at_ns,
        ]
        .into_iter()
        .all(|valid| valid);
        if !evidence_is_valid {
            return Err(InternalError::conflict());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        let mut record = draining
            .store_deletion_intent
            .clone()
            .expect("validated Store deletion intent");
        record.observed_cycles_after_reclamation = Some(evidence.observed_cycles_after_reclamation);
        record.cycles_reclaimed_at_ns = Some(evidence.cycles_reclaimed_at_ns);
        RootComponentRegistryStore::record_root_store_cycle_reclamation(&current, record).map_err(
            |RootComponentRegistryCommitError::ConflictingState| InternalError::conflict(),
        )?;
        Self::root_store_deletion_intent_if_present(operation_id)?
            .ok_or_else(InternalError::invariant)
    }

    pub(crate) fn root_store_deletion_if_present(
        operation_id: [u8; 32],
    ) -> Result<Option<RootFleetSubnetStoreDeletionView>, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        Ok(draining
            .store_deletion
            .clone()
            .map(root_store_deletion_record_to_view))
    }

    pub(crate) fn record_root_store_deletion(
        operation_id: [u8; 32],
        evidence: RootFleetSubnetStoreDeletionEvidence,
        completed_at_ns: u64,
    ) -> Result<RootFleetSubnetStoreDeletionView, InternalError> {
        if let Some(existing) = Self::root_store_deletion_if_present(operation_id)? {
            return Ok(existing);
        }
        if completed_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        let record = root_store_deletion_record(draining, evidence, completed_at_ns)?;
        RootComponentRegistryStore::record_root_store_deletion(&current, record.clone()).map_err(
            |RootComponentRegistryCommitError::ConflictingState| InternalError::conflict(),
        )?;
        let committed = Self::root_store_deletion_if_present(operation_id)?
            .ok_or_else(InternalError::invariant)?;
        if committed != root_store_deletion_record_to_view(record) {
            return Err(InternalError::invariant());
        }
        Ok(committed)
    }

    pub(crate) fn root_deletion_preparation_intent_if_present(
        operation_id: [u8; 32],
    ) -> Result<Option<RootFleetSubnetDeletionPreparationIntentView>, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        Ok(draining
            .root_deletion_preparation_intent
            .clone()
            .map(root_deletion_preparation_intent_record_to_view))
    }

    pub(crate) fn begin_root_deletion_preparation(
        operation_id: [u8; 32],
        authority: RootFleetSubnetDeletionPreparationAuthority,
        prepared_at_ns: u64,
    ) -> Result<RootFleetSubnetDeletionPreparationIntentView, InternalError> {
        let RootFleetSubnetDeletionPreparationAuthority {
            store_deletion_hash: expected_store_deletion_hash,
            coordinator,
            observed_cycles_before_reclamation,
            retained_cycles_target,
            observed_reserved_cycles,
            observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds,
        } = authority;
        let expected_target = deletion_retained_cycles_target(
            observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds,
        );
        let input_is_valid = [
            expected_store_deletion_hash != [0; 32],
            coordinator != Principal::anonymous(),
            observed_cycles_before_reclamation > 0,
            retained_cycles_target > 0,
            expected_target == Some(retained_cycles_target),
            observed_reserved_cycles == 0,
            prepared_at_ns > 0,
        ]
        .into_iter()
        .all(|valid| valid);
        if !input_is_valid {
            return Err(InternalError::invalid_input());
        }
        if let Some(existing) = Self::root_deletion_preparation_intent_if_present(operation_id)? {
            let retry_is_exact = [
                existing.store_deletion_hash == expected_store_deletion_hash,
                existing.coordinator == coordinator,
                existing.observed_cycles_before_reclamation == observed_cycles_before_reclamation,
                existing.retained_cycles_target == retained_cycles_target,
                existing.observed_reserved_cycles == observed_reserved_cycles,
                existing.observed_idle_cycles_burned_per_day == observed_idle_cycles_burned_per_day,
                existing.observed_freezing_threshold_seconds == observed_freezing_threshold_seconds,
            ]
            .into_iter()
            .all(|valid| valid);
            if retry_is_exact {
                return Ok(existing);
            }
            return Err(InternalError::conflict());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        let inventory = draining
            .final_inventory
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        let deletion = draining
            .store_deletion
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        if deletion.deletion_hash != expected_store_deletion_hash {
            return Err(InternalError::conflict());
        }
        if draining.active_registry.authority.binding.coordinator != coordinator {
            return Err(InternalError::conflict());
        }
        let record = RootFleetSubnetDeletionPreparationIntentRecord {
            operation_id,
            coordinator,
            final_inventory_hash: inventory.inventory_hash,
            store_deletion_hash: deletion.deletion_hash,
            observed_cycles_before_reclamation,
            retained_cycles_target,
            observed_reserved_cycles,
            observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds,
            coordinator_intent_hash: None,
            observed_cycles_after_reclamation: None,
            cycles_reclaimed_at_ns: None,
            prepared_at_ns,
        };
        RootComponentRegistryStore::prepare_root_deletion(&current, record).map_err(
            |RootComponentRegistryCommitError::ConflictingState| InternalError::conflict(),
        )?;
        Self::root_deletion_preparation_intent_if_present(operation_id)?
            .ok_or_else(InternalError::invariant)
    }

    pub(crate) fn record_root_deletion_cycle_reclamation(
        operation_id: [u8; 32],
        coordinator_intent_hash: [u8; 32],
        observed_cycles_after_reclamation: u128,
        cycles_reclaimed_at_ns: u64,
    ) -> Result<RootFleetSubnetDeletionPreparationIntentView, InternalError> {
        let existing = Self::root_deletion_preparation_intent_if_present(operation_id)?
            .ok_or_else(InternalError::unavailable)?;
        if existing.coordinator_intent_hash.is_some() {
            let retry_is_exact = [
                existing.coordinator_intent_hash == Some(coordinator_intent_hash),
                existing.observed_cycles_after_reclamation
                    == Some(observed_cycles_after_reclamation),
                existing.cycles_reclaimed_at_ns == Some(cycles_reclaimed_at_ns),
            ]
            .into_iter()
            .all(|valid| valid);
            if retry_is_exact {
                return Ok(existing);
            }
            return Err(InternalError::conflict());
        }
        let evidence_is_valid = [
            coordinator_intent_hash != [0; 32],
            observed_cycles_after_reclamation <= existing.observed_cycles_before_reclamation,
            observed_cycles_after_reclamation <= existing.retained_cycles_target,
            cycles_reclaimed_at_ns >= existing.prepared_at_ns,
        ]
        .into_iter()
        .all(|valid| valid);
        if !evidence_is_valid {
            return Err(InternalError::conflict());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        let mut record = draining
            .root_deletion_preparation_intent
            .clone()
            .expect("validated root deletion preparation intent");
        record.coordinator_intent_hash = Some(coordinator_intent_hash);
        record.observed_cycles_after_reclamation = Some(observed_cycles_after_reclamation);
        record.cycles_reclaimed_at_ns = Some(cycles_reclaimed_at_ns);
        RootComponentRegistryStore::record_root_deletion_cycle_reclamation(&current, record)
            .map_err(|RootComponentRegistryCommitError::ConflictingState| {
                InternalError::conflict()
            })?;
        Self::root_deletion_preparation_intent_if_present(operation_id)?
            .ok_or_else(InternalError::invariant)
    }

    pub(crate) fn root_deletion_preparation_if_present(
        operation_id: [u8; 32],
    ) -> Result<Option<RootFleetSubnetDeletionPreparationView>, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        validate_root_draining_record(&current, draining)?;
        if draining.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        Ok(draining
            .root_deletion_preparation
            .clone()
            .map(root_deletion_preparation_record_to_view))
    }

    pub(crate) fn record_root_deletion_preparation(
        operation_id: [u8; 32],
        coordinator_readiness_hash: [u8; 32],
        completed_at_ns: u64,
    ) -> Result<RootFleetSubnetDeletionPreparationView, InternalError> {
        if let Some(existing) = Self::root_deletion_preparation_if_present(operation_id)? {
            return Ok(existing);
        }
        let fields_are_valid = [coordinator_readiness_hash != [0; 32], completed_at_ns > 0]
            .into_iter()
            .all(|valid| valid);
        if !fields_are_valid {
            return Err(InternalError::invalid_input());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        let intent = draining
            .root_deletion_preparation_intent
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        let record = RootFleetSubnetDeletionPreparationRecord {
            operation_id,
            fleet_subnet_root: draining.fleet_subnet_root,
            coordinator: intent.coordinator,
            final_inventory_hash: intent.final_inventory_hash,
            store_deletion_hash: intent.store_deletion_hash,
            observed_cycles_before_reclamation: intent.observed_cycles_before_reclamation,
            retained_cycles_target: intent.retained_cycles_target,
            observed_reserved_cycles: intent.observed_reserved_cycles,
            observed_idle_cycles_burned_per_day: intent.observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: intent.observed_freezing_threshold_seconds,
            observed_cycles_after_reclamation: intent
                .observed_cycles_after_reclamation
                .ok_or_else(InternalError::unavailable)?,
            cycles_reclaimed_at_ns: intent
                .cycles_reclaimed_at_ns
                .ok_or_else(InternalError::unavailable)?,
            coordinator_intent_hash: intent
                .coordinator_intent_hash
                .ok_or_else(InternalError::unavailable)?,
            coordinator_readiness_hash,
            prepared_at_ns: intent.prepared_at_ns,
            completed_at_ns,
        };
        RootComponentRegistryStore::record_root_deletion_preparation(&current, record).map_err(
            |RootComponentRegistryCommitError::ConflictingState| InternalError::conflict(),
        )?;
        Self::root_deletion_preparation_if_present(operation_id)?
            .ok_or_else(InternalError::invariant)
    }

    pub(crate) fn finalize_root_inventory(
        operation_id: [u8; 32],
        expected_registry: &FleetRegistryVersion,
        store: &RootStoreBootstrapResponse,
        store_status: &WasmStoreStatusResponse,
        finalized_at_ns: u64,
    ) -> Result<RootFleetSubnetFinalInventoryView, InternalError> {
        if let Some(existing) = Self::root_final_inventory_if_present(operation_id)? {
            if &existing.registry != expected_registry {
                return Err(InternalError::conflict());
            }
            return Ok(existing);
        }
        if finalized_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        let intent_registry = Self::root_final_inventory_intent_registry(operation_id)?
            .ok_or_else(InternalError::unavailable)?;
        if &intent_registry != expected_registry {
            return Err(InternalError::conflict());
        }
        let plan = Self::prepare_root_final_inventory(operation_id, expected_registry)?;
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = current
            .root_draining
            .as_ref()
            .expect("validated root draining authority");
        if finalized_at_ns < draining.started_at_ns {
            return Err(InternalError::invalid_input());
        }
        let store_evidence = root_store_final_inventory_evidence(&current, store, store_status)?;
        let mut record = RootFleetSubnetFinalInventoryRecord {
            operation_id,
            fleet_subnet_root: current.root.fleet_subnet_root,
            placement_subnet: current.root.placement_subnet,
            registry: plan.registry,
            component_topology_digest: current.root.component_topology_digest,
            active_release_set: current.release_set,
            next_allocation_sequence: current.next_allocation_sequence,
            removed_component_instances: plan.removed_component_instances,
            terminal_component_history_hash: plan.terminal_component_history_hash,
            root_registry_encoded_bytes: plan.root_registry_encoded_bytes,
            wasm_store: store.wasm_store,
            wasm_store_catalog_hash: store_evidence.catalog_hash,
            wasm_store_catalog_entries: store_evidence.catalog_entries,
            wasm_store_occupied_bytes: store_status.occupied_store_bytes,
            wasm_store_template_count: store_status.template_count,
            wasm_store_release_count: store_status.release_count,
            wasm_store_gc_prepared_at_secs: store_evidence.gc_prepared_at_secs,
            finalized_at_ns,
            inventory_hash: [0; 32],
        };
        record.inventory_hash = root_final_inventory_hash(&record)?;
        RootComponentRegistryStore::finalize_root_inventory(&current, record.clone()).map_err(
            |RootComponentRegistryCommitError::ConflictingState| InternalError::conflict(),
        )?;
        let committed = Self::root_final_inventory(operation_id)?;
        if committed != root_final_inventory_record_to_view(record) {
            return Err(InternalError::invariant());
        }
        Ok(committed)
    }
}
