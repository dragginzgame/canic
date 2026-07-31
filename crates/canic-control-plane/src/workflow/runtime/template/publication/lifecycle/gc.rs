//! Module: workflow::runtime::template::publication::lifecycle::gc
//!
//! Responsibility: orchestrate root-owned Store write fencing, retirement and deletion.
//! Does not own: store-local GC execution, endpoint authorization, or persisted schemas.
//! Boundary: binds remote GC effects to one generation-checked publication state.

use super::super::super::store_pid_for_binding;
use super::super::{
    WasmStorePublicationWorkflow,
    error::PublicationWorkflowError,
    store::{store_begin_gc, store_catalog, store_complete_gc, store_prepare_gc, store_status},
};
use crate::{
    dto::template::{WasmStoreGcStatusResponse, WasmStoreStatusResponse},
    ids::{WasmStoreBinding, WasmStoreGcMode},
    ops::storage::state::subnet::SubnetStateOps,
    view::{
        component_registry::{
            RootFleetSubnetFinalInventoryView, RootFleetSubnetStoreBindingAuthority,
            RootFleetSubnetStoreBindingFinalizationEvidence,
            RootFleetSubnetStoreBindingFinalizationIntentView,
            RootFleetSubnetStoreReclamationEvidence,
        },
        state::{PublicationStoreStateView, WasmStoreView},
    },
};
use canic_core::cdk::types::Principal;
use canic_core::control_plane_support::{
    error::{InternalError, InternalErrorOrigin},
    ops::ic::IcOps,
    workflow::ic::provision::ProvisionWorkflow,
};
use canic_core::{log, log::Topic};
use std::cell::Cell;

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

    // Require an exact finalized inventory entry before destructive canister deletion.
    fn ensure_finalized_store_is_deletable(
        binding: &WasmStoreBinding,
        store_pid: Principal,
    ) -> Result<(), InternalError> {
        let state = SubnetStateOps::publication_store_state();
        if state.active_binding.as_ref() == Some(binding)
            || state.detached_binding.as_ref() == Some(binding)
            || state.retired_binding.as_ref() == Some(binding)
        {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "ws '{binding}' is still referenced"
            ))
            .into());
        }

        let store = Self::runtime_store(binding)?;
        if store.pid != store_pid {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "ws binding '{binding}' resolves to {}, not deletion target {store_pid}",
                store.pid
            ))
            .into());
        }
        if store.gc.mode != WasmStoreGcMode::Complete {
            return Err(PublicationWorkflowError::StoreGcStateChanged {
                binding: binding.clone(),
                expected: WasmStoreGcMode::Complete,
                actual: store.gc.mode,
            }
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

    // Delete one previously finalized retired publication store after local GC and root finalization complete.
    pub async fn delete_finalized_publication_store(
        binding: WasmStoreBinding,
        store_pid: Principal,
    ) -> Result<(), InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        Self::ensure_finalized_store_is_deletable(&binding, store_pid)?;

        let store = store_status(store_pid).await?;

        if store.gc.mode != WasmStoreGcMode::Complete {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "finalized ws '{}' not ready for delete; gc={:?}",
                    binding, store.gc.mode
                ),
            ));
        }

        if store.occupied_store_bytes != 0 || store.template_count != 0 || store.release_count != 0
        {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "finalized ws '{}' not empty after gc; bytes={} templates={} releases={}",
                    binding, store.occupied_store_bytes, store.template_count, store.release_count
                ),
            ));
        }

        Self::ensure_finalized_store_is_deletable(&binding, store_pid)?;
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
