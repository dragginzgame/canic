//! Module: ops::component_registry::root_retirement
//!
//! Responsibility: validate and hash Root terminal inventory and Wasm Store retirement evidence.
//! Does not own: durable storage, Store effects, Root orchestration, or Fleet publication.
//! Boundary: compiles exact retained records from already-authenticated, single-step evidence.

use super::{
    ROOT_FINAL_INVENTORY_HASH_DOMAIN, ROOT_STORE_BINDING_FINALIZATION_HASH_DOMAIN,
    ROOT_STORE_DELETION_HASH_DOMAIN, ROOT_STORE_FINAL_CATALOG_HASH_DOMAIN,
    ROOT_STORE_RECLAMATION_HASH_DOMAIN, RootFleetSubnetFinalInventoryPlan, domain_hash,
    terminal_root_inventory_plan,
};
use crate::{
    dto::template::WasmStoreStatusResponse,
    ids::WasmStoreGcMode,
    storage::stable::component_registry::{
        RootComponentRegistryMetaRecord, RootFleetSubnetDrainingRecord,
        RootFleetSubnetFinalInventoryIntentRecord, RootFleetSubnetFinalInventoryRecord,
        RootFleetSubnetStoreBindingFinalizationRecord, RootFleetSubnetStoreDeletionRecord,
        RootFleetSubnetStoreReclamationRecord,
    },
    view::component_registry::{
        RootFleetSubnetStoreBindingFinalizationEvidence, RootFleetSubnetStoreDeletionAuthority,
        RootFleetSubnetStoreDeletionEvidence, RootFleetSubnetStoreReclamationEvidence,
    },
};
use candid::CandidType;
use canic_core::{
    cdk::types::Principal,
    control_plane_support::error::InternalError,
    dto::{
        fleet_registry::FleetRegistryVersion,
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
