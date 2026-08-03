//! Module: ops::storage::state::root_wasm_store
//!
//! Responsibility: provide deterministic access and conversion for root-owned Wasm Store state.
//! Does not own: lifecycle orchestration, endpoint authorization, or stable record schemas.
//! Boundary: workflows use this ops facade instead of opening stable storage directly.

#[cfg(test)]
use crate::storage::stable::state::root_wasm_store::{
    PublicationStoreStateRecord, RootWasmStoreStateData, RootWasmStoreStateRecord, WasmStoreRecord,
};
use crate::{
    dto::template::{WasmStoreGcStatusResponse, WasmStorePublicationStateResponse},
    ids::{WasmStoreBinding, WasmStoreCreationPurpose, WasmStoreGcMode},
    ops::storage::state::mapper::RootWasmStoreStateMapper,
    storage::stable::state::root_wasm_store::{
        RootWasmStoreState, WasmStoreCreationProgressRecord, WasmStoreCreationRecord,
        WasmStoreGcRecord,
    },
    view::state::{PublicationStoreStateView, WasmStoreCreationView, WasmStoreView},
};
use canic_core::{
    cdk::types::Principal,
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        model::replay::ReplayCostGuardSettlement,
    },
};

///
/// WasmStoreCreationPlan
///
/// Ops-owned immutable authority frozen before one root-owned Store creation effect.
/// Consumed by the Store creation workflow and persisted as a stable record.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmStoreCreationPlan {
    pub purpose: WasmStoreCreationPurpose,
    pub expected_module_hash: [u8; 32],
    pub payload_size_bytes: u64,
    pub controllers: Vec<Principal>,
    pub initial_cycles: u128,
}

///
/// PublicationStoreStateTestInput
///
/// Ops-owned test input for publication-store lifecycle state.
///

#[cfg(test)]
pub struct PublicationStoreStateTestInput {
    pub active_binding: Option<WasmStoreBinding>,
    pub detached_binding: Option<WasmStoreBinding>,
    pub retired_binding: Option<WasmStoreBinding>,
    pub generation: u64,
    pub changed_at: u64,
    pub retired_at: u64,
}

///
/// WasmStoreStateTestInput
///
/// Ops-owned test input for one runtime-managed Wasm store.
///

#[cfg(test)]
pub struct WasmStoreStateTestInput {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub created_at: u64,
    pub gc_mode: WasmStoreGcMode,
    pub gc_changed_at: u64,
    pub prepared_at: Option<u64>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub runs_completed: u32,
}

///
/// RootWasmStoreStateOps
///

pub struct RootWasmStoreStateOps;

impl RootWasmStoreStateOps {
    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    /// Return the current root-owned publication binding, if one is pinned.
    #[must_use]
    pub fn publication_store_binding() -> Option<WasmStoreBinding> {
        RootWasmStoreState::publication_store_binding()
    }

    /// Return the current root-owned publication binding lifecycle state.
    #[must_use]
    pub fn publication_store_state() -> PublicationStoreStateView {
        RootWasmStoreStateMapper::publication_store_record_to_view(
            RootWasmStoreState::publication_store_state(),
        )
    }

    /// Return all known runtime-managed wasm stores for the current subnet.
    #[must_use]
    pub fn wasm_stores() -> Vec<WasmStoreView> {
        RootWasmStoreState::wasm_stores()
            .into_iter()
            .map(RootWasmStoreStateMapper::wasm_store_record_to_view)
            .collect()
    }

    #[must_use]
    pub fn wasm_store_creation() -> Option<WasmStoreCreationView> {
        RootWasmStoreState::wasm_store_creation()
            .map(RootWasmStoreStateMapper::wasm_store_creation_record_to_view)
    }

    pub fn begin_wasm_store_creation(
        plan: &WasmStoreCreationPlan,
        creation_cost_guard_settlement: ReplayCostGuardSettlement,
        prepared_at: u64,
    ) -> Result<WasmStoreCreationView, InternalError> {
        RootWasmStoreState::begin_wasm_store_creation(WasmStoreCreationRecord {
            sequence: 0,
            purpose: plan.purpose,
            expected_module_hash: plan.expected_module_hash,
            payload_size_bytes: plan.payload_size_bytes,
            controllers: plan.controllers.clone(),
            initial_cycles: plan.initial_cycles,
            creation_cost_guard_settlement,
            prepared_at,
            progress: WasmStoreCreationProgressRecord::CreationIntent,
        })
        .map(RootWasmStoreStateMapper::wasm_store_creation_record_to_view)
        .map_err(|reason| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                format!("failed to begin root-owned Wasm Store creation: {reason:?}"),
            )
        })
    }

    pub fn mark_wasm_store_created(
        sequence: u64,
        pid: Principal,
        created_at: u64,
    ) -> Result<WasmStoreCreationView, InternalError> {
        RootWasmStoreState::mark_wasm_store_created(sequence, pid, created_at)
            .map(RootWasmStoreStateMapper::wasm_store_creation_record_to_view)
            .ok_or_else(|| Self::store_creation_transition_error("record created Canister"))
    }

    pub fn begin_wasm_store_install(
        sequence: u64,
        settlement: ReplayCostGuardSettlement,
    ) -> Result<WasmStoreCreationView, InternalError> {
        RootWasmStoreState::begin_wasm_store_install(sequence, settlement)
            .map(RootWasmStoreStateMapper::wasm_store_creation_record_to_view)
            .ok_or_else(|| Self::store_creation_transition_error("begin install"))
    }

    pub fn renew_wasm_store_install(
        sequence: u64,
        settlement: ReplayCostGuardSettlement,
    ) -> Result<WasmStoreCreationView, InternalError> {
        RootWasmStoreState::renew_wasm_store_install(sequence, settlement)
            .map(RootWasmStoreStateMapper::wasm_store_creation_record_to_view)
            .ok_or_else(|| Self::store_creation_transition_error("renew install"))
    }

    pub fn mark_wasm_store_installed(
        sequence: u64,
    ) -> Result<WasmStoreCreationView, InternalError> {
        RootWasmStoreState::mark_wasm_store_installed(sequence)
            .map(RootWasmStoreStateMapper::wasm_store_creation_record_to_view)
            .ok_or_else(|| Self::store_creation_transition_error("record installed Canister"))
    }

    pub fn commit_wasm_store_creation(
        sequence: u64,
        binding: WasmStoreBinding,
    ) -> Result<WasmStoreView, InternalError> {
        RootWasmStoreState::commit_wasm_store_creation(sequence, binding)
            .map(RootWasmStoreStateMapper::wasm_store_record_to_view)
            .ok_or_else(|| Self::store_creation_transition_error("commit Store inventory"))
    }

    fn store_creation_transition_error(transition: &str) -> InternalError {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            format!("failed to {transition} for root-owned Wasm Store creation"),
        )
    }

    /// Resolve one runtime-managed wasm store principal by logical binding.
    #[must_use]
    pub fn wasm_store_pid(binding: &WasmStoreBinding) -> Option<Principal> {
        RootWasmStoreState::wasm_store_pid(binding)
    }

    /// Resolve one runtime-managed wasm store binding by canister principal.
    #[must_use]
    pub fn wasm_store_binding_for_pid(pid: Principal) -> Option<WasmStoreBinding> {
        RootWasmStoreState::wasm_store_binding_for_pid(pid)
    }

    /// Remove one runtime-managed wasm store record by binding.
    #[must_use]
    pub fn remove_wasm_store(binding: &WasmStoreBinding) -> bool {
        RootWasmStoreState::remove_wasm_store(binding).is_some()
    }

    /// Persist one GC lifecycle transition for a runtime-managed wasm store.
    #[must_use]
    pub fn transition_wasm_store_gc(
        binding: &WasmStoreBinding,
        next: WasmStoreGcMode,
        changed_at: u64,
    ) -> bool {
        RootWasmStoreState::transition_wasm_store_gc(binding, next, changed_at)
    }

    /// Reconcile runtime GC authority from one independently observed exact live Store.
    #[must_use]
    pub fn reconcile_wasm_store_gc(
        binding: &WasmStoreBinding,
        pid: Principal,
        live: &WasmStoreGcStatusResponse,
    ) -> bool {
        RootWasmStoreState::reconcile_wasm_store_gc(
            binding,
            pid,
            WasmStoreGcRecord {
                mode: live.mode,
                changed_at: live.changed_at,
                prepared_at: live.prepared_at,
                started_at: live.started_at,
                completed_at: live.completed_at,
                runs_completed: live.runs_completed,
            },
        )
    }

    /// Return the current root-owned publication binding lifecycle state as a DTO response.
    #[must_use]
    pub fn publication_store_state_response() -> WasmStorePublicationStateResponse {
        RootWasmStoreStateMapper::publication_store_record_to_response(
            RootWasmStoreState::publication_store_state(),
        )
    }

    /// Persist the current root-owned publication binding.
    #[must_use]
    pub fn activate_publication_store_binding(binding: WasmStoreBinding, changed_at: u64) -> bool {
        RootWasmStoreState::activate_publication_store_binding(binding, changed_at)
    }

    /// Clear the current root-owned publication binding.
    #[must_use]
    pub fn clear_publication_store_binding(changed_at: u64) -> bool {
        RootWasmStoreState::clear_publication_store_binding(changed_at)
    }

    /// Move the current detached binding into retired state.
    #[must_use]
    pub fn retire_detached_publication_store_binding(changed_at: u64) -> Option<WasmStoreBinding> {
        RootWasmStoreState::retire_detached_publication_store_binding(changed_at)
    }

    /// Clear the current retired binding after root verifies retirement is complete.
    #[must_use]
    pub fn finalize_retired_publication_store_binding(changed_at: u64) -> Option<WasmStoreBinding> {
        RootWasmStoreState::finalize_retired_publication_store_binding(changed_at)
    }

    #[cfg(test)]
    pub fn import_test_state(
        publication_store: PublicationStoreStateTestInput,
        wasm_stores: Vec<WasmStoreStateTestInput>,
    ) {
        RootWasmStoreState::import(RootWasmStoreStateData {
            record: RootWasmStoreStateRecord {
                publication_store: PublicationStoreStateRecord {
                    active_binding: publication_store.active_binding,
                    detached_binding: publication_store.detached_binding,
                    retired_binding: publication_store.retired_binding,
                    generation: publication_store.generation,
                    changed_at: publication_store.changed_at,
                    retired_at: publication_store.retired_at,
                },
                wasm_stores: wasm_stores
                    .into_iter()
                    .map(|store| WasmStoreRecord {
                        binding: store.binding,
                        pid: store.pid,
                        created_at: store.created_at,
                        gc: WasmStoreGcRecord {
                            mode: store.gc_mode,
                            changed_at: store.gc_changed_at,
                            prepared_at: store.prepared_at,
                            started_at: store.started_at,
                            completed_at: store.completed_at,
                            runs_completed: store.runs_completed,
                        },
                    })
                    .collect(),
                next_wasm_store_creation_sequence: 0,
                wasm_store_creation: None,
            },
        });
    }
}
