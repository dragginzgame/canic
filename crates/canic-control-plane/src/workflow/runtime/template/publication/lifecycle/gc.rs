//! Module: workflow::runtime::template::publication::lifecycle::gc
//!
//! Responsibility: orchestrate root-owned Store write fencing, retirement and deletion.
//! Does not own: store-local GC execution, endpoint authorization, or persisted schemas.
//! Boundary: binds remote GC effects to one generation-checked publication state.

use super::super::super::store_pid_for_binding;
use super::super::{
    WASM_STORE_ROLE, WasmStorePublicationWorkflow,
    error::PublicationWorkflowError,
    store::{
        store_begin_gc, store_catalog, store_complete_gc, store_prepare_gc,
        store_reclaim_deletion_cycles, store_status,
    },
};
use crate::{
    dto::template::{
        WASM_STORE_DELETION_EXECUTION_RESERVE_CYCLES, WASM_STORE_DELETION_MAXIMUM_RETAINED_CYCLES,
        WasmStoreDeletionCycleReclamationResponse, WasmStoreGcStatusResponse,
        WasmStoreStatusResponse,
    },
    ids::{WasmStoreBinding, WasmStoreGcMode},
    ops::storage::state::subnet::SubnetStateOps,
    view::{
        component_registry::{
            RootFleetSubnetFinalInventoryView, RootFleetSubnetStoreBindingAuthority,
            RootFleetSubnetStoreBindingFinalizationEvidence,
            RootFleetSubnetStoreBindingFinalizationIntentView,
            RootFleetSubnetStoreBindingFinalizationView,
            RootFleetSubnetStoreCycleReclamationEvidence, RootFleetSubnetStoreDeletionAuthority,
            RootFleetSubnetStoreDeletionEvidence, RootFleetSubnetStoreDeletionIntentView,
            RootFleetSubnetStoreReclamationEvidence,
        },
        state::{PublicationStoreStateView, WasmStoreView},
    },
};
use canic_core::cdk::candid::Nat;
use canic_core::cdk::types::Principal;
use canic_core::control_plane_support::{
    error::{InternalError, InternalErrorOrigin},
    ops::{
        ic::{
            IcOps,
            mgmt::{CanisterStatus, CanisterStatusObservation, CanisterStatusType, MgmtOps},
        },
        storage::registry::subnet::SubnetRegistryOps,
    },
    workflow::ic::provision::ProvisionWorkflow,
};
use canic_core::{log, log::Topic};
use std::cell::Cell;

const SECONDS_PER_DAY: u128 = 86_400;

thread_local! {
    static LIFECYCLE_OPERATION_IN_FLIGHT: Cell<bool> = const { Cell::new(false) };
}

#[derive(Debug)]
struct LifecycleOperationGuard;

#[derive(Debug, Eq, PartialEq)]
struct StoreGcAuthority {
    pid: Principal,
    mode: WasmStoreGcMode,
    changed_at: u64,
    prepared_at: Option<u64>,
    started_at: Option<u64>,
    completed_at: Option<u64>,
    runs_completed: u32,
}

#[derive(Debug, Eq, PartialEq)]
struct StoreManagementAuthority {
    module_hash: [u8; 32],
    controllers: Vec<Principal>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReclaimedStoreBindingPhase {
    Active,
    Detached,
    Retired,
    Finalized,
}

impl StoreGcAuthority {
    const fn from_runtime(store: &WasmStoreView) -> Self {
        Self {
            pid: store.pid,
            mode: store.gc.mode,
            changed_at: store.gc.changed_at,
            prepared_at: store.gc.prepared_at,
            started_at: store.gc.started_at,
            completed_at: store.gc.completed_at,
            runs_completed: store.gc.runs_completed,
        }
    }

    const fn from_live(pid: Principal, status: &WasmStoreGcStatusResponse) -> Self {
        Self {
            pid,
            mode: status.mode,
            changed_at: status.changed_at,
            prepared_at: status.prepared_at,
            started_at: status.started_at,
            completed_at: status.completed_at,
            runs_completed: status.runs_completed,
        }
    }
}

impl LifecycleOperationGuard {
    fn try_enter() -> Result<Self, InternalError> {
        let entered = LIFECYCLE_OPERATION_IN_FLIGHT.with(|in_flight| {
            if in_flight.get() {
                false
            } else {
                in_flight.set(true);
                true
            }
        });

        if entered {
            Ok(Self)
        } else {
            Err(PublicationWorkflowError::LifecycleBusy.into())
        }
    }
}

impl Drop for LifecycleOperationGuard {
    fn drop(&mut self) {
        LIFECYCLE_OPERATION_IN_FLIGHT.with(|in_flight| {
            debug_assert!(in_flight.get());
            in_flight.set(false);
        });
    }
}

impl WasmStorePublicationWorkflow {
    /// One-way write-fence the sole root-local Store while retaining its exact inventory.
    pub async fn quiesce_single_root_store_for_final_inventory()
    -> Result<(Principal, WasmStoreStatusResponse), InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        Self::sync_registered_wasm_store_inventory()?;
        let stores = SubnetStateOps::wasm_stores();
        if stores.len() != 1 {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "root final inventory requires exactly one local wasm store, found {}",
                stores.len()
            ))
            .into());
        }
        let runtime = stores.into_iter().next().expect("validated one Store");
        let mut live = store_status(runtime.pid).await?;
        match (runtime.gc.mode, live.gc.mode) {
            (WasmStoreGcMode::Normal, WasmStoreGcMode::Normal) => {
                store_prepare_gc(runtime.pid).await?;
                live = store_status(runtime.pid).await?;
            }
            (WasmStoreGcMode::Normal | WasmStoreGcMode::Prepared, WasmStoreGcMode::Prepared) => {}
            (runtime_mode, live_mode) => {
                return Err(PublicationWorkflowError::InvalidState(format!(
                    "root final inventory requires normal/prepared GC authority, found runtime={runtime_mode:?} live={live_mode:?}"
                ))
                .into());
            }
        }
        validate_live_prepared_store(&live)?;

        if runtime.gc.mode == WasmStoreGcMode::Normal {
            let persisted = SubnetStateOps::transition_wasm_store_gc(
                &runtime.binding,
                WasmStoreGcMode::Prepared,
                live.gc.changed_at,
            );
            if !persisted {
                return Err(PublicationWorkflowError::InvalidState(format!(
                    "failed to persist prepared GC authority for '{}'",
                    runtime.binding
                ))
                .into());
            }
        }
        let persisted = Self::runtime_store(&runtime.binding)?;
        let runtime_is_exact = StoreGcAuthority::from_runtime(&persisted)
            == StoreGcAuthority::from_live(runtime.pid, &live.gc);
        if !runtime_is_exact {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "persisted GC authority for '{}' differs from its live Store",
                runtime.binding
            ))
            .into());
        }
        Ok((runtime.pid, live))
    }

    /// Re-read the sole root Store and require the retained final-inventory write fence.
    pub async fn verify_single_root_store_for_removal()
    -> Result<(Principal, WasmStoreStatusResponse), InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        Self::sync_registered_wasm_store_inventory()?;
        let stores = SubnetStateOps::wasm_stores();
        if stores.len() != 1 {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "root removal requires exactly one local wasm store, found {}",
                stores.len()
            ))
            .into());
        }
        let runtime = stores.into_iter().next().expect("validated one Store");
        let live = store_status(runtime.pid).await?;
        validate_live_prepared_store(&live)?;
        let runtime_is_exact = StoreGcAuthority::from_runtime(&runtime)
            == StoreGcAuthority::from_live(runtime.pid, &live.gc);
        if !runtime_is_exact {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "persisted GC authority for '{}' differs from its live Store",
                runtime.binding
            ))
            .into());
        }
        Ok((runtime.pid, live))
    }

    /// Reclaim the sole retained Store after its exact final inventory was logically removed.
    pub async fn reclaim_single_root_store(
        inventory: &RootFleetSubnetFinalInventoryView,
    ) -> Result<RootFleetSubnetStoreReclamationEvidence, InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        Self::reclaim_single_root_store_inner(inventory).await
    }

    async fn reclaim_single_root_store_inner(
        inventory: &RootFleetSubnetFinalInventoryView,
    ) -> Result<RootFleetSubnetStoreReclamationEvidence, InternalError> {
        Self::sync_registered_wasm_store_inventory()?;
        let stores = SubnetStateOps::wasm_stores();
        if stores.len() != 1 {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "root Store reclamation requires exactly one local wasm store, found {}",
                stores.len()
            ))
            .into());
        }
        let runtime = stores.into_iter().next().expect("validated one Store");
        if runtime.pid != inventory.wasm_store {
            return Err(PublicationWorkflowError::InvalidState(
                "root Store reclamation target differs from final inventory".to_string(),
            )
            .into());
        }

        let mut live = store_status(runtime.pid).await?;
        validate_live_store_gc_lineage(inventory, &live)?;
        if live.gc.mode == WasmStoreGcMode::Clearing {
            store_begin_gc(runtime.pid).await?;
            live = store_status(runtime.pid).await?;
            validate_live_store_gc_lineage(inventory, &live)?;
        }
        if live.gc.mode == WasmStoreGcMode::Prepared {
            store_begin_gc(runtime.pid).await?;
            live = store_status(runtime.pid).await?;
            validate_live_store_gc_lineage(inventory, &live)?;
        }
        if live.gc.mode == WasmStoreGcMode::InProgress {
            Self::reconcile_single_root_store_gc(&runtime, &live.gc)?;
            store_complete_gc(runtime.pid).await?;
            live = store_status(runtime.pid).await?;
            validate_live_store_gc_lineage(inventory, &live)?;
        }
        if live.gc.mode != WasmStoreGcMode::Complete {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "root Store reclamation did not reach GC Complete; live={:?}",
                live.gc.mode
            ))
            .into());
        }

        Self::reconcile_single_root_store_gc(&runtime, &live.gc)?;
        let catalog = store_catalog(runtime.pid).await?;
        let catalog_entries = u32::try_from(catalog.len()).map_err(|_| {
            PublicationWorkflowError::InvalidState(
                "reclaimed root Store catalog exceeds u32".to_string(),
            )
        })?;
        validate_live_reclaimed_store(inventory, &live, catalog_entries)?;
        let gc_started_at_secs = live.gc.started_at.expect("validated GC start time");
        let gc_completed_at_secs = live.gc.completed_at.expect("validated GC completion time");
        Ok(RootFleetSubnetStoreReclamationEvidence {
            wasm_store: runtime.pid,
            occupied_store_bytes: live.occupied_store_bytes,
            catalog_entries,
            template_count: live.template_count,
            release_count: live.release_count,
            gc_prepared_at_secs: inventory.wasm_store_gc_prepared_at_secs,
            gc_started_at_secs,
            gc_completed_at_secs,
            gc_runs_completed: live.gc.runs_completed,
        })
    }

    /// Verify the exact active binding for one reclaimed root Store before local finalization.
    pub async fn verify_single_reclaimed_root_store_binding(
        inventory: &RootFleetSubnetFinalInventoryView,
    ) -> Result<RootFleetSubnetStoreBindingAuthority, InternalError> {
        Self::sync_registered_wasm_store_inventory()?;
        let stores = SubnetStateOps::wasm_stores();
        if stores.len() != 1 {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "root Store binding finalization requires exactly one local Store, found {}",
                stores.len()
            ))
            .into());
        }
        let runtime = stores.into_iter().next().expect("validated one Store");
        let publication = SubnetStateOps::publication_store_state();
        let binding_is_exact = [
            runtime.pid == inventory.wasm_store,
            publication.active_binding.as_ref() == Some(&runtime.binding),
            publication.detached_binding.is_none(),
            publication.retired_binding.is_none(),
            publication.retired_at == 0,
            publication.generation > 0,
            runtime.gc.mode == WasmStoreGcMode::Complete,
        ]
        .into_iter()
        .all(|valid| valid);
        if !binding_is_exact {
            return Err(PublicationWorkflowError::InvalidState(
                "reclaimed root Store is not the sole exact active publication binding".to_string(),
            )
            .into());
        }

        let live = store_status(runtime.pid).await?;
        let catalog = store_catalog(runtime.pid).await?;
        let catalog_entries = u32::try_from(catalog.len()).map_err(|_| {
            PublicationWorkflowError::InvalidState(
                "reclaimed root Store catalog exceeds u32".to_string(),
            )
        })?;
        validate_live_reclaimed_store(inventory, &live, catalog_entries)?;
        if StoreGcAuthority::from_runtime(&runtime)
            != StoreGcAuthority::from_live(runtime.pid, &live.gc)
        {
            return Err(PublicationWorkflowError::InvalidState(
                "reclaimed root Store runtime GC authority differs from live status".to_string(),
            )
            .into());
        }
        Ok(RootFleetSubnetStoreBindingAuthority {
            wasm_store: runtime.pid,
            binding: runtime.binding,
            source_generation: publication.generation,
        })
    }

    /// Converge one reclaimed Store through active, detached and retired binding slots.
    pub fn finalize_single_reclaimed_root_store_binding(
        intent: &RootFleetSubnetStoreBindingFinalizationIntentView,
    ) -> Result<RootFleetSubnetStoreBindingFinalizationEvidence, InternalError> {
        let runtime = Self::runtime_store(&intent.binding)?;
        let runtime_is_exact = [
            runtime.pid == intent.wasm_store,
            runtime.gc.mode == WasmStoreGcMode::Complete,
            runtime.gc.runs_completed == 1,
        ]
        .into_iter()
        .all(|valid| valid);
        if !runtime_is_exact {
            return Err(PublicationWorkflowError::InvalidState(
                "reclaimed root Store runtime differs from binding finalization intent".to_string(),
            )
            .into());
        }

        for _ in 0..4 {
            let previous = SubnetStateOps::publication_store_state();
            match reclaimed_store_binding_phase(&previous, intent)? {
                ReclaimedStoreBindingPhase::Active => {
                    let changed_at = IcOps::now_secs();
                    if !SubnetStateOps::clear_publication_store_binding(changed_at) {
                        return Err(binding_finalization_transition_error("clear active"));
                    }
                    Self::log_publication_state_transition(
                        "finalize_reclaimed_active_binding",
                        &previous,
                        &SubnetStateOps::publication_store_state(),
                        changed_at,
                    );
                }
                ReclaimedStoreBindingPhase::Detached => {
                    let changed_at = IcOps::now_secs();
                    let retired =
                        SubnetStateOps::retire_detached_publication_store_binding(changed_at);
                    if retired.as_ref() != Some(&intent.binding) {
                        return Err(binding_finalization_transition_error("retire detached"));
                    }
                    Self::log_publication_state_transition(
                        "finalize_reclaimed_detached_binding",
                        &previous,
                        &SubnetStateOps::publication_store_state(),
                        changed_at,
                    );
                }
                ReclaimedStoreBindingPhase::Retired => {
                    let changed_at = IcOps::now_secs();
                    let finalized =
                        SubnetStateOps::finalize_retired_publication_store_binding(changed_at);
                    if finalized.as_ref() != Some(&intent.binding) {
                        return Err(binding_finalization_transition_error("finalize retired"));
                    }
                    Self::log_publication_state_transition(
                        "finalize_reclaimed_retired_binding",
                        &previous,
                        &SubnetStateOps::publication_store_state(),
                        changed_at,
                    );
                }
                ReclaimedStoreBindingPhase::Finalized => {
                    return Ok(RootFleetSubnetStoreBindingFinalizationEvidence {
                        wasm_store: intent.wasm_store,
                        binding: intent.binding.clone(),
                        source_generation: intent.source_generation,
                        finalized_generation: previous.generation,
                        finalized_at_secs: previous.changed_at,
                    });
                }
            }
        }
        Err(binding_finalization_transition_error(
            "reach terminal state",
        ))
    }

    /// Reverify one reclaimed, unbound Store before physical deletion intent is committed.
    pub async fn verify_single_finalized_root_store_for_deletion(
        inventory: &RootFleetSubnetFinalInventoryView,
        finalization: &RootFleetSubnetStoreBindingFinalizationView,
    ) -> Result<RootFleetSubnetStoreDeletionAuthority, InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        validate_store_deletion_lineage(inventory, finalization)?;
        Self::sync_registered_wasm_store_inventory()?;
        let runtime = single_finalized_runtime_store(finalization)?;

        let live = store_status(runtime.pid).await?;
        let catalog = store_catalog(runtime.pid).await?;
        let catalog_entries = u32::try_from(catalog.len()).map_err(|_| {
            PublicationWorkflowError::InvalidState(
                "reclaimed root Store catalog exceeds u32".to_string(),
            )
        })?;
        validate_live_reclaimed_store(inventory, &live, catalog_entries)?;
        if StoreGcAuthority::from_runtime(&runtime)
            != StoreGcAuthority::from_live(runtime.pid, &live.gc)
        {
            return Err(PublicationWorkflowError::InvalidState(
                "finalized root Store runtime GC authority differs from live status".to_string(),
            )
            .into());
        }

        let root = IcOps::canister_self();
        require_exact_store_registration(runtime.pid, root)?;
        let observation = MgmtOps::observe_canister_status(runtime.pid).await?;
        let CanisterStatusObservation::Present(status) = observation else {
            return Err(PublicationWorkflowError::InvalidState(
                "root Store disappeared before deletion intent was durable".to_string(),
            )
            .into());
        };
        let management = store_management_authority(&status, root)?;
        let (observed_cycles_before_reclamation, maximum_cycles_to_retain) =
            store_deletion_cycle_authority(&status)?;
        if status.status != CanisterStatusType::Running {
            return Err(PublicationWorkflowError::InvalidState(
                "root Store is not running before deletion intent".to_string(),
            )
            .into());
        }
        Ok(RootFleetSubnetStoreDeletionAuthority {
            wasm_store: runtime.pid,
            binding: runtime.binding,
            observed_module_hash: management.module_hash,
            observed_controllers: management.controllers,
            observed_cycles_before_reclamation,
            maximum_cycles_to_retain,
        })
    }

    /// Return excess Store cycles to the root and independently freeze the remaining balance.
    pub async fn reclaim_single_finalized_root_store_cycles(
        intent: &RootFleetSubnetStoreDeletionIntentView,
        finalization: &RootFleetSubnetStoreBindingFinalizationView,
    ) -> Result<RootFleetSubnetStoreCycleReclamationEvidence, InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        validate_store_deletion_intent_lineage(intent, finalization)?;
        if intent.observed_cycles_after_reclamation.is_some()
            || intent.cycles_reclaimed_at_ns.is_some()
        {
            return Err(PublicationWorkflowError::InvalidState(
                "root Store cycle reclamation is already durable".to_string(),
            )
            .into());
        }
        let root = IcOps::canister_self();
        if !validate_deletion_runtime_inventory(intent, finalization)?
            || !optional_exact_store_registration(intent.wasm_store, root)?
        {
            return Err(PublicationWorkflowError::InvalidState(
                "root Store cycle reclamation requires exact local deletion inventory".to_string(),
            )
            .into());
        }

        let status = present_running_store_for_cycle_reclamation(intent, root).await?;
        let cycles_before_call = status_cycles(&status.cycles, "Store balance")?;
        if cycles_before_call > intent.observed_cycles_before_reclamation {
            return Err(PublicationWorkflowError::InvalidState(
                "root Store balance increased after deletion intent".to_string(),
            )
            .into());
        }
        if cycles_before_call > intent.maximum_cycles_to_retain {
            let response =
                store_reclaim_deletion_cycles(intent.wasm_store, intent.maximum_cycles_to_retain)
                    .await?;
            validate_store_cycle_reclamation_response(intent, root, &response)?;
        }

        let observed = present_running_store_for_cycle_reclamation(intent, root).await?;
        let observed_cycles_after_reclamation =
            status_cycles(&observed.cycles, "post-reclamation Store balance")?;
        let balance_is_reclaimed = [
            observed_cycles_after_reclamation <= intent.observed_cycles_before_reclamation,
            observed_cycles_after_reclamation <= intent.maximum_cycles_to_retain,
        ]
        .into_iter()
        .all(|valid| valid);
        if !balance_is_reclaimed {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "root Store still exceeds its durable deletion cycle reserve: observed={observed_cycles_after_reclamation} maximum={}",
                intent.maximum_cycles_to_retain
            ))
            .into());
        }
        Ok(RootFleetSubnetStoreCycleReclamationEvidence {
            observed_cycles_after_reclamation,
            cycles_reclaimed_at_ns: IcOps::now_nanos(),
        })
    }

    /// Stop and delete one Store, accepting completion only from typed live absence.
    pub async fn delete_single_finalized_root_store(
        intent: &RootFleetSubnetStoreDeletionIntentView,
        finalization: &RootFleetSubnetStoreBindingFinalizationView,
    ) -> Result<RootFleetSubnetStoreDeletionEvidence, InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        validate_store_deletion_intent_lineage(intent, finalization)?;
        require_durable_store_cycle_reclamation(intent)?;
        let root = IcOps::canister_self();
        let runtime_present = validate_deletion_runtime_inventory(intent, finalization)?;
        let registration_present = optional_exact_store_registration(intent.wasm_store, root)?;
        let observation = MgmtOps::observe_canister_status(intent.wasm_store).await?;

        let observed_absent_at_ns = match observation {
            CanisterStatusObservation::Absent => IcOps::now_nanos(),
            CanisterStatusObservation::Present(status) => {
                if !runtime_present || !registration_present {
                    return Err(PublicationWorkflowError::InvalidState(
                        "present root Store is missing exact local deletion inventory".to_string(),
                    )
                    .into());
                }
                stop_store_for_deletion(intent, root, *status).await?;
                delete_store_and_observe_absence(intent, root).await?
            }
        };
        reconcile_deleted_store_inventory(intent, finalization, root)?;
        log!(
            Topic::Wasm,
            Ok,
            "ws physically deleted {} ({})",
            intent.binding,
            intent.wasm_store
        );
        Ok(RootFleetSubnetStoreDeletionEvidence {
            wasm_store: intent.wasm_store,
            binding: intent.binding.clone(),
            observed_module_hash: intent.observed_module_hash,
            observed_controllers: intent.observed_controllers.clone(),
            observed_cycles_before_reclamation: intent.observed_cycles_before_reclamation,
            maximum_cycles_to_retain: intent.maximum_cycles_to_retain,
            observed_cycles_after_reclamation: intent
                .observed_cycles_after_reclamation
                .expect("validated Store cycle reclamation"),
            cycles_reclaimed_at_ns: intent
                .cycles_reclaimed_at_ns
                .expect("validated Store cycle-reclamation time"),
            observed_absent_at_ns,
        })
    }

    fn reconcile_single_root_store_gc(
        runtime: &WasmStoreView,
        live: &WasmStoreGcStatusResponse,
    ) -> Result<(), InternalError> {
        let runtime_is_exact = StoreGcAuthority::from_runtime(runtime)
            == StoreGcAuthority::from_live(runtime.pid, live);
        if runtime_is_exact {
            return Ok(());
        }
        if !SubnetStateOps::reconcile_wasm_store_gc(&runtime.binding, runtime.pid, live) {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "failed to reconcile root Store GC authority for '{}'",
                runtime.binding
            ))
            .into());
        }
        let persisted = Self::runtime_store(&runtime.binding)?;
        if StoreGcAuthority::from_runtime(&persisted)
            != StoreGcAuthority::from_live(runtime.pid, live)
        {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "reconciled GC authority for '{}' differs from its live Store",
                runtime.binding
            ))
            .into());
        }
        Ok(())
    }

    fn ensure_finalized_store_is_deletable(
        binding: &WasmStoreBinding,
        store_pid: Principal,
    ) -> Result<(), InternalError> {
        let state = SubnetStateOps::publication_store_state();
        let is_unbound = [
            state.active_binding.as_ref() != Some(binding),
            state.detached_binding.as_ref() != Some(binding),
            state.retired_binding.as_ref() != Some(binding),
        ]
        .into_iter()
        .all(|valid| valid);
        let runtime = Self::runtime_store(binding)?;
        let runtime_is_exact = [
            runtime.pid == store_pid,
            runtime.gc.mode == WasmStoreGcMode::Complete,
        ]
        .into_iter()
        .all(|valid| valid);
        if !is_unbound || !runtime_is_exact {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "ws '{binding}' is not exact finalized deletion authority"
            ))
            .into());
        }
        Ok(())
    }

    // Delete one ordinary finalized publication Store through the existing admin surface.
    pub async fn delete_finalized_publication_store(
        binding: WasmStoreBinding,
        store_pid: Principal,
    ) -> Result<(), InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        Self::ensure_finalized_store_is_deletable(&binding, store_pid)?;

        let store = store_status(store_pid).await?;
        let store_is_empty = [
            store.gc.mode == WasmStoreGcMode::Complete,
            store.occupied_store_bytes == 0,
            store.template_count == 0,
            store.release_count == 0,
        ]
        .into_iter()
        .all(|valid| valid);
        if !store_is_empty {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "finalized ws '{binding}' is not empty and GC-complete"
            ))
            .into());
        }

        ProvisionWorkflow::uninstall_and_delete_canister(store_pid).await?;
        if !SubnetStateOps::remove_wasm_store(&binding) {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "deleted ws '{binding}' was missing from runtime inventory"
            ))
            .into());
        }
        log!(Topic::Wasm, Ok, "ws deleted {} ({})", binding, store_pid);
        Ok(())
    }

    // Resolve one binding from authoritative runtime inventory.
    fn runtime_store(binding: &WasmStoreBinding) -> Result<WasmStoreView, InternalError> {
        SubnetStateOps::wasm_stores()
            .into_iter()
            .find(|store| &store.binding == binding)
            .ok_or_else(|| {
                PublicationWorkflowError::InvalidState(format!(
                    "ws binding '{binding}' is missing from runtime inventory"
                ))
                .into()
            })
    }

    // Reject a post-await commit when publication ownership changed while the call was in flight.
    fn ensure_lifecycle_state_is_current(
        expected: &PublicationStoreStateView,
        binding: &WasmStoreBinding,
    ) -> Result<(), InternalError> {
        let current = SubnetStateOps::publication_store_state();
        if current.generation != expected.generation
            || current.retired_binding.as_ref() != Some(binding)
        {
            return Err(PublicationWorkflowError::LifecycleStateChanged {
                binding: binding.clone(),
                expected_generation: expected.generation,
                actual_generation: current.generation,
            }
            .into());
        }

        Ok(())
    }

    // Commit a remote GC transition only when the same retired binding still owns the lifecycle.
    fn persist_retired_gc_transition(
        expected: &PublicationStoreStateView,
        binding: &WasmStoreBinding,
        next: WasmStoreGcMode,
        changed_at: u64,
    ) -> Result<(), InternalError> {
        Self::ensure_lifecycle_state_is_current(expected, binding)?;
        let store = Self::runtime_store(binding)?;
        if store.gc.mode == next {
            return Ok(());
        }

        let required = match next {
            WasmStoreGcMode::Prepared => WasmStoreGcMode::Normal,
            WasmStoreGcMode::InProgress => WasmStoreGcMode::Prepared,
            WasmStoreGcMode::Complete => WasmStoreGcMode::InProgress,
            WasmStoreGcMode::Normal | WasmStoreGcMode::Clearing => {
                return Err(PublicationWorkflowError::InvalidState(format!(
                    "root lifecycle cannot persist gc mode {next:?} for '{binding}'"
                ))
                .into());
            }
        };

        if store.gc.mode != required {
            return Err(PublicationWorkflowError::StoreGcStateChanged {
                binding: binding.clone(),
                expected: required,
                actual: store.gc.mode,
            }
            .into());
        }

        if !SubnetStateOps::transition_wasm_store_gc(binding, next, changed_at) {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "failed to persist gc mode {next:?} for '{binding}'"
            ))
            .into());
        }

        Ok(())
    }

    // Mark the current retired publication store as prepared for store-local GC execution.
    pub async fn prepare_retired_publication_store_for_gc()
    -> Result<Option<WasmStoreBinding>, InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        store_prepare_gc(store_pid).await?;
        Self::persist_retired_gc_transition(
            &state,
            &retired_binding,
            WasmStoreGcMode::Prepared,
            IcOps::now_secs(),
        )?;

        log!(
            Topic::Wasm,
            Ok,
            "ws gc prepared {} gen={} retired_at={}",
            retired_binding,
            state.generation,
            state.retired_at
        );

        Ok(Some(retired_binding))
    }

    // Mark the current retired publication store as actively executing store-local GC.
    pub async fn begin_retired_publication_store_gc()
    -> Result<Option<WasmStoreBinding>, InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        store_begin_gc(store_pid).await?;
        Self::persist_retired_gc_transition(
            &state,
            &retired_binding,
            WasmStoreGcMode::InProgress,
            IcOps::now_secs(),
        )?;

        log!(
            Topic::Wasm,
            Ok,
            "ws gc begin {} gen={} retired_at={}",
            retired_binding,
            state.generation,
            state.retired_at
        );

        Ok(Some(retired_binding))
    }

    // Mark the current retired publication store as having completed its local GC pass.
    pub async fn complete_retired_publication_store_gc()
    -> Result<Option<WasmStoreBinding>, InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        store_complete_gc(store_pid).await?;
        Self::persist_retired_gc_transition(
            &state,
            &retired_binding,
            WasmStoreGcMode::Complete,
            IcOps::now_secs(),
        )?;

        log!(
            Topic::Wasm,
            Ok,
            "ws gc complete {} gen={} retired_at={}",
            retired_binding,
            state.generation,
            state.retired_at
        );

        Ok(Some(retired_binding))
    }

    // Finalize the current retired publication store after its local GC run has completed.
    pub async fn finalize_retired_publication_store_binding()
    -> Result<Option<(WasmStoreBinding, Principal)>, InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        let store = store_status(store_pid).await?;

        if store.gc.mode != WasmStoreGcMode::Complete {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "retired ws '{}' not ready for finalize; gc={:?}",
                    retired_binding, store.gc.mode
                ),
            ));
        }

        Self::ensure_lifecycle_state_is_current(&state, &retired_binding)?;
        let runtime_store = Self::runtime_store(&retired_binding)?;
        if runtime_store.gc.mode != WasmStoreGcMode::Complete {
            return Err(PublicationWorkflowError::StoreGcStateChanged {
                binding: retired_binding.clone(),
                expected: WasmStoreGcMode::Complete,
                actual: runtime_store.gc.mode,
            }
            .into());
        }

        let changed_at = IcOps::now_secs();
        let previous = SubnetStateOps::publication_store_state();
        let finalized_binding = SubnetStateOps::finalize_retired_publication_store_binding(
            changed_at,
        )
        .ok_or_else(|| {
            PublicationWorkflowError::InvalidState(format!(
                "retired ws '{retired_binding}' disappeared before finalize commit"
            ))
        })?;
        if finalized_binding != retired_binding {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "finalized ws '{finalized_binding}' did not match expected '{retired_binding}'"
            ))
            .into());
        }
        let current = SubnetStateOps::publication_store_state();
        Self::log_publication_state_transition(
            "finalize_retired_binding",
            &previous,
            &current,
            changed_at,
        );
        log!(
            Topic::Wasm,
            Ok,
            "ws finalized {} ({})",
            finalized_binding,
            store_pid
        );

        Ok(Some((finalized_binding, store_pid)))
    }
}

fn validate_store_deletion_lineage(
    inventory: &RootFleetSubnetFinalInventoryView,
    finalization: &RootFleetSubnetStoreBindingFinalizationView,
) -> Result<(), InternalError> {
    let expected_finalized_generation = finalization.source_generation.checked_add(3);
    let lineage_is_exact = [
        finalization.operation_id == inventory.operation_id,
        finalization.fleet_subnet_root == inventory.fleet_subnet_root,
        finalization.wasm_store == inventory.wasm_store,
        finalization.final_inventory_hash == inventory.inventory_hash,
        finalization.binding.as_str() == finalization.wasm_store.to_text(),
        Some(finalization.finalized_generation) == expected_finalized_generation,
        finalization.finalized_at_secs > 0,
        finalization.finalization_hash != [0; 32],
    ]
    .into_iter()
    .all(|valid| valid);
    if !lineage_is_exact {
        return Err(PublicationWorkflowError::InvalidState(
            "Store deletion lineage differs from exact binding finalization".to_string(),
        )
        .into());
    }
    validate_finalized_publication_state(finalization)
}

fn validate_store_deletion_intent_lineage(
    intent: &RootFleetSubnetStoreDeletionIntentView,
    finalization: &RootFleetSubnetStoreBindingFinalizationView,
) -> Result<(), InternalError> {
    let cycle_reclamation_is_valid = match (
        intent.observed_cycles_after_reclamation,
        intent.cycles_reclaimed_at_ns,
    ) {
        (None, None) => true,
        (Some(observed_after), Some(reclaimed_at_ns)) => [
            observed_after <= intent.observed_cycles_before_reclamation,
            observed_after <= intent.maximum_cycles_to_retain,
            reclaimed_at_ns >= intent.prepared_at_ns,
        ]
        .into_iter()
        .all(|valid| valid),
        _ => false,
    };
    let intent_is_exact = [
        intent.operation_id == finalization.operation_id,
        intent.binding_finalization_hash == finalization.finalization_hash,
        intent.wasm_store == finalization.wasm_store,
        intent.binding == finalization.binding,
        intent.observed_module_hash != [0; 32],
        canonical_controller_set(&intent.observed_controllers),
        intent
            .observed_controllers
            .contains(&finalization.fleet_subnet_root),
        intent.observed_cycles_before_reclamation > 0,
        intent.maximum_cycles_to_retain > 0,
        intent.maximum_cycles_to_retain <= WASM_STORE_DELETION_MAXIMUM_RETAINED_CYCLES,
        cycle_reclamation_is_valid,
        intent.prepared_at_ns >= finalization.completed_at_ns,
    ]
    .into_iter()
    .all(|valid| valid);
    if !intent_is_exact {
        return Err(PublicationWorkflowError::InvalidState(
            "Store deletion intent differs from exact binding finalization".to_string(),
        )
        .into());
    }
    validate_finalized_publication_state(finalization)
}

fn validate_finalized_publication_state(
    finalization: &RootFleetSubnetStoreBindingFinalizationView,
) -> Result<(), InternalError> {
    let state = SubnetStateOps::publication_store_state();
    let state_is_exact = [
        state.active_binding.is_none(),
        state.detached_binding.is_none(),
        state.retired_binding.is_none(),
        state.retired_at == 0,
        state.generation == finalization.finalized_generation,
        state.changed_at == finalization.finalized_at_secs,
    ]
    .into_iter()
    .all(|valid| valid);
    if !state_is_exact {
        return Err(PublicationWorkflowError::InvalidState(
            "publication state differs from terminal Store binding authority".to_string(),
        )
        .into());
    }
    Ok(())
}

fn single_finalized_runtime_store(
    finalization: &RootFleetSubnetStoreBindingFinalizationView,
) -> Result<WasmStoreView, InternalError> {
    validate_finalized_publication_state(finalization)?;
    let stores = SubnetStateOps::wasm_stores();
    if stores.len() != 1 {
        return Err(PublicationWorkflowError::InvalidState(format!(
            "Store deletion preparation requires exactly one runtime Store, found {}",
            stores.len()
        ))
        .into());
    }
    let runtime = stores.into_iter().next().expect("validated one Store");
    let runtime_is_exact = [
        runtime.pid == finalization.wasm_store,
        runtime.binding == finalization.binding,
        runtime.gc.mode == WasmStoreGcMode::Complete,
        runtime.gc.runs_completed == 1,
    ]
    .into_iter()
    .all(|valid| valid);
    if !runtime_is_exact {
        return Err(PublicationWorkflowError::InvalidState(
            "runtime Store differs from terminal binding authority".to_string(),
        )
        .into());
    }
    Ok(runtime)
}

fn validate_deletion_runtime_inventory(
    intent: &RootFleetSubnetStoreDeletionIntentView,
    finalization: &RootFleetSubnetStoreBindingFinalizationView,
) -> Result<bool, InternalError> {
    validate_finalized_publication_state(finalization)?;
    let stores = SubnetStateOps::wasm_stores();
    match stores.as_slice() {
        [] => Ok(false),
        [runtime]
            if [
                runtime.pid == intent.wasm_store,
                runtime.binding == intent.binding,
                runtime.gc.mode == WasmStoreGcMode::Complete,
                runtime.gc.runs_completed == 1,
            ]
            .into_iter()
            .all(|valid| valid) =>
        {
            Ok(true)
        }
        _ => Err(PublicationWorkflowError::InvalidState(
            "runtime Store inventory differs from physical deletion intent".to_string(),
        )
        .into()),
    }
}

fn require_exact_store_registration(
    wasm_store: Principal,
    root: Principal,
) -> Result<(), InternalError> {
    if optional_exact_store_registration(wasm_store, root)? {
        return Ok(());
    }
    Err(PublicationWorkflowError::InvalidState(
        "root Store is missing from exact Subnet Registry authority".to_string(),
    )
    .into())
}

fn optional_exact_store_registration(
    wasm_store: Principal,
    root: Principal,
) -> Result<bool, InternalError> {
    match SubnetRegistryOps::role_parent(wasm_store) {
        None => Ok(false),
        Some((role, parent)) if role == WASM_STORE_ROLE && parent == Some(root) => Ok(true),
        Some(_) => Err(PublicationWorkflowError::InvalidState(
            "root Store Subnet Registry authority differs from deletion intent".to_string(),
        )
        .into()),
    }
}

fn store_management_authority(
    status: &CanisterStatus,
    root: Principal,
) -> Result<StoreManagementAuthority, InternalError> {
    let mut controllers = status.settings.controllers.clone();
    controllers.sort();
    controllers.dedup();
    if !controllers.contains(&root) {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store controllers omit protected root deletion authority".to_string(),
        )
        .into());
    }
    let module_hash = status
        .module_hash
        .as_deref()
        .and_then(|hash| <[u8; 32]>::try_from(hash).ok())
        .filter(|hash| hash != &[0; 32])
        .ok_or_else(|| {
            PublicationWorkflowError::InvalidState(
                "root Store deletion requires one installed module hash".to_string(),
            )
        })?;
    Ok(StoreManagementAuthority {
        module_hash,
        controllers,
    })
}

fn require_store_management_authority(
    status: &CanisterStatus,
    root: Principal,
    intent: &RootFleetSubnetStoreDeletionIntentView,
) -> Result<(), InternalError> {
    let observed = store_management_authority(status, root)?;
    let expected = StoreManagementAuthority {
        module_hash: intent.observed_module_hash,
        controllers: intent.observed_controllers.clone(),
    };
    if observed != expected {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store module or controllers differ from durable deletion authority".to_string(),
        )
        .into());
    }
    Ok(())
}

fn store_deletion_cycle_authority(status: &CanisterStatus) -> Result<(u128, u128), InternalError> {
    require_no_reserved_store_cycles(status)?;
    let observed_cycles = status_cycles(&status.cycles, "Store balance")?;
    if observed_cycles == 0 {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store has no cycle balance to retain for deletion".to_string(),
        )
        .into());
    }
    let idle_cycles_burned_per_day = status_cycles(
        &status.idle_cycles_burned_per_day,
        "Store idle cycles burned per day",
    )?;
    let freezing_threshold_seconds = status_cycles(
        &status.settings.freezing_threshold,
        "Store freezing threshold",
    )?;
    let freezing_reserve = idle_cycles_burned_per_day
        .checked_mul(freezing_threshold_seconds)
        .ok_or_else(|| {
            PublicationWorkflowError::InvalidState(
                "root Store freezing reserve overflows u128".to_string(),
            )
        })?
        .div_ceil(SECONDS_PER_DAY);
    let maximum_cycles_to_retain = freezing_reserve
        .checked_add(WASM_STORE_DELETION_EXECUTION_RESERVE_CYCLES)
        .ok_or_else(|| {
            PublicationWorkflowError::InvalidState(
                "root Store deletion cycle reserve overflows u128".to_string(),
            )
        })?;
    if maximum_cycles_to_retain > WASM_STORE_DELETION_MAXIMUM_RETAINED_CYCLES {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store deletion cycle reserve exceeds the supported ceiling".to_string(),
        )
        .into());
    }
    Ok((observed_cycles, maximum_cycles_to_retain))
}

fn status_cycles(value: &Nat, label: &str) -> Result<u128, InternalError> {
    u128::try_from(value.0.clone())
        .map_err(|_| PublicationWorkflowError::InvalidState(format!("{label} exceeds u128")).into())
}

fn require_no_reserved_store_cycles(status: &CanisterStatus) -> Result<(), InternalError> {
    if status_cycles(&status.reserved_cycles, "Store reserved cycles")? != 0 {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store has reserved cycles that cannot be reclaimed before deletion".to_string(),
        )
        .into());
    }
    Ok(())
}

async fn present_running_store_for_cycle_reclamation(
    intent: &RootFleetSubnetStoreDeletionIntentView,
    root: Principal,
) -> Result<CanisterStatus, InternalError> {
    let CanisterStatusObservation::Present(status) =
        MgmtOps::observe_canister_status(intent.wasm_store).await?
    else {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store disappeared before cycle reclamation was durable".to_string(),
        )
        .into());
    };
    require_store_management_authority(&status, root, intent)?;
    require_no_reserved_store_cycles(&status)?;
    if status.status != CanisterStatusType::Running {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store must remain running through cycle reclamation".to_string(),
        )
        .into());
    }
    Ok(*status)
}

fn validate_store_cycle_reclamation_response(
    intent: &RootFleetSubnetStoreDeletionIntentView,
    root: Principal,
    response: &WasmStoreDeletionCycleReclamationResponse,
) -> Result<(), InternalError> {
    let response_is_exact = [
        response.destination == root,
        response.maximum_cycles_to_retain == intent.maximum_cycles_to_retain,
        response.cycles_before <= intent.observed_cycles_before_reclamation,
        response.cycles_transferred <= response.cycles_before,
        response.cycles_after <= intent.maximum_cycles_to_retain,
    ]
    .into_iter()
    .all(|valid| valid);
    if !response_is_exact {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store cycle-reclamation response differs from durable authority".to_string(),
        )
        .into());
    }
    Ok(())
}

fn require_durable_store_cycle_reclamation(
    intent: &RootFleetSubnetStoreDeletionIntentView,
) -> Result<(), InternalError> {
    if !matches!(
        (
            intent.observed_cycles_after_reclamation,
            intent.cycles_reclaimed_at_ns,
        ),
        (Some(_), Some(_))
    ) {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store cannot stop before cycle reclamation is durable".to_string(),
        )
        .into());
    }
    Ok(())
}

fn require_store_deletion_authority(
    status: &CanisterStatus,
    root: Principal,
    intent: &RootFleetSubnetStoreDeletionIntentView,
) -> Result<(), InternalError> {
    require_store_management_authority(status, root, intent)?;
    require_no_reserved_store_cycles(status)?;
    let observed_after = intent
        .observed_cycles_after_reclamation
        .expect("validated Store cycle reclamation");
    if status_cycles(&status.cycles, "Store deletion balance")? > observed_after {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store balance increased after cycle reclamation".to_string(),
        )
        .into());
    }
    Ok(())
}

fn canonical_controller_set(controllers: &[Principal]) -> bool {
    !controllers.is_empty() && controllers.windows(2).all(|pair| pair[0] < pair[1])
}

async fn stop_store_for_deletion(
    intent: &RootFleetSubnetStoreDeletionIntentView,
    root: Principal,
    status: CanisterStatus,
) -> Result<(), InternalError> {
    require_store_deletion_authority(&status, root, intent)?;
    match status.status {
        CanisterStatusType::Stopped => return Ok(()),
        CanisterStatusType::Stopping => {
            return Err(InternalError::unavailable(
                "root Store deletion stop is still in progress",
            ));
        }
        CanisterStatusType::Running => {}
    }

    let stop_error = MgmtOps::stop_canister(intent.wasm_store).await.err();
    match MgmtOps::observe_canister_status(intent.wasm_store).await? {
        CanisterStatusObservation::Absent => Ok(()),
        CanisterStatusObservation::Present(status) => {
            require_store_deletion_authority(&status, root, intent)?;
            match status.status {
                CanisterStatusType::Stopped => Ok(()),
                CanisterStatusType::Stopping => Err(InternalError::unavailable(
                    "root Store deletion stop is still in progress",
                )),
                CanisterStatusType::Running => match stop_error {
                    Some(error) => Err(error),
                    None => Err(InternalError::unavailable(
                        "root Store remains running after its stop call completed",
                    )),
                },
            }
        }
    }
}

async fn delete_store_and_observe_absence(
    intent: &RootFleetSubnetStoreDeletionIntentView,
    root: Principal,
) -> Result<u64, InternalError> {
    match MgmtOps::observe_canister_status(intent.wasm_store).await? {
        CanisterStatusObservation::Absent => return Ok(IcOps::now_nanos()),
        CanisterStatusObservation::Present(status) => {
            require_store_deletion_authority(&status, root, intent)?;
            if status.status != CanisterStatusType::Stopped {
                return Err(PublicationWorkflowError::InvalidState(
                    "root Store is not stopped under deletion authority".to_string(),
                )
                .into());
            }
        }
    }

    let delete_error = MgmtOps::delete_canister(intent.wasm_store).await.err();
    match MgmtOps::observe_canister_status(intent.wasm_store).await? {
        CanisterStatusObservation::Absent => Ok(IcOps::now_nanos()),
        CanisterStatusObservation::Present(status) => {
            require_store_deletion_authority(&status, root, intent)?;
            match delete_error {
                Some(error) => Err(error),
                None => Err(InternalError::unavailable(
                    "root Store remains present after its deletion call completed",
                )),
            }
        }
    }
}

fn reconcile_deleted_store_inventory(
    intent: &RootFleetSubnetStoreDeletionIntentView,
    finalization: &RootFleetSubnetStoreBindingFinalizationView,
    root: Principal,
) -> Result<(), InternalError> {
    let runtime_present = validate_deletion_runtime_inventory(intent, finalization)?;
    let registration_present = optional_exact_store_registration(intent.wasm_store, root)?;
    if registration_present
        && !SubnetRegistryOps::unregister_exact(intent.wasm_store, &WASM_STORE_ROLE, root)?
    {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store registration disappeared before deletion reconciliation".to_string(),
        )
        .into());
    }
    if runtime_present && !SubnetStateOps::remove_wasm_store(&intent.binding) {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store runtime inventory disappeared before deletion reconciliation".to_string(),
        )
        .into());
    }
    let inventory_is_empty = [
        SubnetStateOps::wasm_stores().is_empty(),
        SubnetStateOps::wasm_store_pid(&intent.binding).is_none(),
        SubnetStateOps::wasm_store_binding_for_pid(intent.wasm_store).is_none(),
        SubnetRegistryOps::role_parent(intent.wasm_store).is_none(),
    ]
    .into_iter()
    .all(|valid| valid);
    if !inventory_is_empty {
        return Err(PublicationWorkflowError::InvalidState(
            "deleted root Store remains in local runtime inventory".to_string(),
        )
        .into());
    }
    Ok(())
}

fn reclaimed_store_binding_phase(
    state: &PublicationStoreStateView,
    intent: &RootFleetSubnetStoreBindingFinalizationIntentView,
) -> Result<ReclaimedStoreBindingPhase, InternalError> {
    let detached_generation = intent.source_generation.checked_add(1);
    let retired_generation = intent.source_generation.checked_add(2);
    let finalized_generation = intent.source_generation.checked_add(3);
    let is_active = [
        state.generation == intent.source_generation,
        state.active_binding.as_ref() == Some(&intent.binding),
        state.detached_binding.is_none(),
        state.retired_binding.is_none(),
        state.retired_at == 0,
    ]
    .into_iter()
    .all(|valid| valid);
    let is_detached = [
        Some(state.generation) == detached_generation,
        state.active_binding.is_none(),
        state.detached_binding.as_ref() == Some(&intent.binding),
        state.retired_binding.is_none(),
        state.retired_at == 0,
    ]
    .into_iter()
    .all(|valid| valid);
    let is_retired = [
        Some(state.generation) == retired_generation,
        state.active_binding.is_none(),
        state.detached_binding.is_none(),
        state.retired_binding.as_ref() == Some(&intent.binding),
        state.retired_at > 0,
    ]
    .into_iter()
    .all(|valid| valid);
    let is_finalized = [
        Some(state.generation) == finalized_generation,
        state.active_binding.is_none(),
        state.detached_binding.is_none(),
        state.retired_binding.is_none(),
        state.retired_at == 0,
        state.changed_at > 0,
    ]
    .into_iter()
    .all(|valid| valid);
    match (is_active, is_detached, is_retired, is_finalized) {
        (true, false, false, false) => Ok(ReclaimedStoreBindingPhase::Active),
        (false, true, false, false) => Ok(ReclaimedStoreBindingPhase::Detached),
        (false, false, true, false) => Ok(ReclaimedStoreBindingPhase::Retired),
        (false, false, false, true) => Ok(ReclaimedStoreBindingPhase::Finalized),
        _ => Err(PublicationWorkflowError::InvalidState(
            "publication state differs from durable Store binding finalization progress"
                .to_string(),
        )
        .into()),
    }
}

fn binding_finalization_transition_error(transition: &str) -> InternalError {
    PublicationWorkflowError::InvalidState(format!(
        "failed to {transition} reclaimed root Store binding"
    ))
    .into()
}

fn validate_live_prepared_store(status: &WasmStoreStatusResponse) -> Result<(), InternalError> {
    let prepared_at = status.gc.prepared_at.unwrap_or_default();
    let evidence_is_exact = [
        status.gc.mode == WasmStoreGcMode::Prepared,
        prepared_at > 0,
        status.gc.changed_at == prepared_at,
        status.gc.started_at.is_none(),
        status.gc.completed_at.is_none(),
        status.gc.runs_completed == 0,
    ]
    .into_iter()
    .all(|valid| valid);
    if !evidence_is_exact {
        return Err(PublicationWorkflowError::InvalidState(
            "root final inventory requires one retained Store at exact GC Prepared authority"
                .to_string(),
        )
        .into());
    }
    Ok(())
}

fn validate_live_store_gc_lineage(
    inventory: &RootFleetSubnetFinalInventoryView,
    status: &WasmStoreStatusResponse,
) -> Result<(), InternalError> {
    let prepared_at = status.gc.prepared_at.unwrap_or_default();
    let lineage_is_exact = match status.gc.mode {
        WasmStoreGcMode::Normal => false,
        WasmStoreGcMode::Prepared => {
            validate_live_prepared_store(status)?;
            true
        }
        WasmStoreGcMode::InProgress | WasmStoreGcMode::Clearing => {
            let started_at = status.gc.started_at.unwrap_or_default();
            [
                started_at >= prepared_at,
                status.gc.changed_at >= started_at,
                status.gc.completed_at.is_none(),
                status.gc.runs_completed == 0,
            ]
            .into_iter()
            .all(|valid| valid)
        }
        WasmStoreGcMode::Complete => {
            let started_at = status.gc.started_at.unwrap_or_default();
            let completed_at = status.gc.completed_at.unwrap_or_default();
            [
                started_at >= prepared_at,
                completed_at >= started_at,
                status.gc.changed_at == completed_at,
                status.gc.runs_completed == 1,
            ]
            .into_iter()
            .all(|valid| valid)
        }
    };
    let inventory_is_exact = [
        prepared_at > 0,
        prepared_at == inventory.wasm_store_gc_prepared_at_secs,
    ]
    .into_iter()
    .all(|valid| valid);
    if ![lineage_is_exact, inventory_is_exact]
        .into_iter()
        .all(|valid| valid)
    {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store GC lineage differs from retained final inventory".to_string(),
        )
        .into());
    }
    Ok(())
}

fn validate_live_reclaimed_store(
    inventory: &RootFleetSubnetFinalInventoryView,
    status: &WasmStoreStatusResponse,
    catalog_entries: u32,
) -> Result<(), InternalError> {
    validate_live_store_gc_lineage(inventory, status)?;
    let store_is_empty = [
        status.gc.mode == WasmStoreGcMode::Complete,
        status.occupied_store_bytes == 0,
        catalog_entries == 0,
        status.template_count == 0,
        status.release_count == 0,
        status.templates.is_empty(),
    ]
    .into_iter()
    .all(|valid| valid);
    if !store_is_empty {
        return Err(PublicationWorkflowError::InvalidState(
            "root Store GC completed without an exact empty inventory".to_string(),
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::storage::state::subnet::{
        PublicationStoreStateTestInput, WasmStoreStateTestInput,
    };
    use canic_core::dto::error::ErrorCode;

    fn import_retired_store(mode: WasmStoreGcMode) -> (WasmStoreBinding, Principal) {
        let binding = WasmStoreBinding::new("retired");
        let pid = Principal::from_slice(&[7; 29]);
        SubnetStateOps::import_test_state(
            PublicationStoreStateTestInput {
                active_binding: Some(WasmStoreBinding::new("active")),
                detached_binding: None,
                retired_binding: Some(binding.clone()),
                generation: 3,
                changed_at: 30,
                retired_at: 20,
            },
            vec![WasmStoreStateTestInput {
                binding: binding.clone(),
                pid,
                created_at: 10,
                gc_mode: mode,
                gc_changed_at: 20,
                prepared_at: (mode != WasmStoreGcMode::Normal).then_some(11),
                started_at: matches!(
                    mode,
                    WasmStoreGcMode::InProgress
                        | WasmStoreGcMode::Clearing
                        | WasmStoreGcMode::Complete
                )
                .then_some(12),
                completed_at: (mode == WasmStoreGcMode::Complete).then_some(20),
                runs_completed: u32::from(mode == WasmStoreGcMode::Complete),
            }],
        );
        (binding, pid)
    }

    #[test]
    fn lifecycle_guard_rejects_concurrent_entry_and_releases_on_drop() {
        let guard = LifecycleOperationGuard::try_enter().expect("first operation enters");
        let err = LifecycleOperationGuard::try_enter().expect_err("second operation must reject");
        assert_eq!(
            err.public_error().map(|public| public.code),
            Some(ErrorCode::Conflict)
        );

        drop(guard);
        LifecycleOperationGuard::try_enter().expect("guard should release on drop");
    }

    #[test]
    fn retired_gc_commit_is_generation_bound_and_idempotent() {
        let (binding, _) = import_retired_store(WasmStoreGcMode::Normal);
        let expected = SubnetStateOps::publication_store_state();

        WasmStorePublicationWorkflow::persist_retired_gc_transition(
            &expected,
            &binding,
            WasmStoreGcMode::Prepared,
            40,
        )
        .expect("matching retired generation should commit");
        WasmStorePublicationWorkflow::persist_retired_gc_transition(
            &expected,
            &binding,
            WasmStoreGcMode::Prepared,
            41,
        )
        .expect("same transition should be idempotent");

        let store = WasmStorePublicationWorkflow::runtime_store(&binding).expect("runtime store");
        assert_eq!(store.gc.mode, WasmStoreGcMode::Prepared);
        assert_eq!(store.gc.changed_at, 40);

        assert!(SubnetStateOps::clear_publication_store_binding(42));
        let err = WasmStorePublicationWorkflow::persist_retired_gc_transition(
            &expected,
            &binding,
            WasmStoreGcMode::InProgress,
            43,
        )
        .expect_err("generation drift must reject post-await commit");
        assert_eq!(
            err.public_error().map(|public| public.code),
            Some(ErrorCode::Conflict)
        );
        let store = WasmStorePublicationWorkflow::runtime_store(&binding).expect("runtime store");
        assert_eq!(store.gc.mode, WasmStoreGcMode::Prepared);
    }

    #[test]
    fn root_store_gc_reconciliation_preserves_live_retry_lineage() {
        let binding = WasmStoreBinding::new("root");
        let pid = Principal::from_slice(&[9; 29]);
        SubnetStateOps::import_test_state(
            PublicationStoreStateTestInput {
                active_binding: Some(binding.clone()),
                detached_binding: None,
                retired_binding: None,
                generation: 1,
                changed_at: 10,
                retired_at: 0,
            },
            vec![WasmStoreStateTestInput {
                binding: binding.clone(),
                pid,
                created_at: 9,
                gc_mode: WasmStoreGcMode::Prepared,
                gc_changed_at: 11,
                prepared_at: Some(11),
                started_at: None,
                completed_at: None,
                runs_completed: 0,
            }],
        );
        let runtime = WasmStorePublicationWorkflow::runtime_store(&binding).expect("runtime Store");
        let recovered = WasmStoreGcStatusResponse {
            mode: WasmStoreGcMode::InProgress,
            changed_at: 14,
            prepared_at: Some(11),
            started_at: Some(12),
            completed_at: None,
            runs_completed: 0,
        };
        WasmStorePublicationWorkflow::reconcile_single_root_store_gc(&runtime, &recovered)
            .expect("reconcile recovered in-progress GC");
        let runtime = WasmStorePublicationWorkflow::runtime_store(&binding).expect("runtime Store");
        assert_eq!(
            StoreGcAuthority::from_runtime(&runtime),
            StoreGcAuthority::from_live(pid, &recovered)
        );

        let completed = WasmStoreGcStatusResponse {
            mode: WasmStoreGcMode::Complete,
            changed_at: 15,
            prepared_at: Some(11),
            started_at: Some(12),
            completed_at: Some(15),
            runs_completed: 1,
        };
        WasmStorePublicationWorkflow::reconcile_single_root_store_gc(&runtime, &completed)
            .expect("reconcile completed GC");
        let runtime = WasmStorePublicationWorkflow::runtime_store(&binding).expect("runtime Store");
        assert_eq!(
            StoreGcAuthority::from_runtime(&runtime),
            StoreGcAuthority::from_live(pid, &completed)
        );
    }

    #[test]
    fn reclaimed_store_binding_finalization_resumes_from_every_durable_slot() {
        let pid = Principal::from_slice(&[10; 29]);
        let binding = WasmStoreBinding::owned(pid.to_text());
        let source_generation = 3;
        let phases = [
            (source_generation, Some(binding.clone()), None, None, 0),
            (source_generation + 1, None, Some(binding.clone()), None, 0),
            (source_generation + 2, None, None, Some(binding.clone()), 12),
            (source_generation + 3, None, None, None, 0),
        ];
        let intent = RootFleetSubnetStoreBindingFinalizationIntentView {
            operation_id: [11; 32],
            final_inventory_hash: [12; 32],
            reclamation_hash: [13; 32],
            wasm_store: pid,
            binding: binding.clone(),
            source_generation,
            prepared_at_ns: 14,
        };

        for (generation, active, detached, retired, retired_at) in phases {
            SubnetStateOps::import_test_state(
                PublicationStoreStateTestInput {
                    active_binding: active,
                    detached_binding: detached,
                    retired_binding: retired,
                    generation,
                    changed_at: 11,
                    retired_at,
                },
                vec![WasmStoreStateTestInput {
                    binding: binding.clone(),
                    pid,
                    created_at: 9,
                    gc_mode: WasmStoreGcMode::Complete,
                    gc_changed_at: 10,
                    prepared_at: Some(7),
                    started_at: Some(8),
                    completed_at: Some(10),
                    runs_completed: 1,
                }],
            );
            let evidence =
                WasmStorePublicationWorkflow::finalize_single_reclaimed_root_store_binding(&intent)
                    .expect("resume exact Store binding finalization");
            assert_eq!(evidence.wasm_store, pid);
            assert_eq!(evidence.binding, binding);
            assert_eq!(evidence.source_generation, source_generation);
            assert_eq!(evidence.finalized_generation, source_generation + 3);
            assert!(evidence.finalized_at_secs > 0);
            let state = SubnetStateOps::publication_store_state();
            assert_eq!(state.active_binding, None);
            assert_eq!(state.detached_binding, None);
            assert_eq!(state.retired_binding, None);
            assert_eq!(SubnetStateOps::wasm_stores().len(), 1);
        }
    }

    #[test]
    fn finalized_delete_preflight_binds_inventory_identity_and_gc_state() {
        let (binding, pid) = import_retired_store(WasmStoreGcMode::Complete);
        SubnetStateOps::finalize_retired_publication_store_binding(40)
            .expect("retired binding finalizes");

        WasmStorePublicationWorkflow::ensure_finalized_store_is_deletable(&binding, pid)
            .expect("exact finalized store should be deletable");

        let err = WasmStorePublicationWorkflow::ensure_finalized_store_is_deletable(
            &binding,
            Principal::from_slice(&[8; 29]),
        )
        .expect_err("pid mismatch must reject deletion");
        assert_eq!(
            err.public_error().map(|public| public.code),
            Some(ErrorCode::InvariantViolation)
        );
    }
}
