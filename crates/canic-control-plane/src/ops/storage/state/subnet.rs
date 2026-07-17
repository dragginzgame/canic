#[cfg(test)]
use crate::storage::stable::state::subnet::{
    ControlPlaneSubnetStateData, PublicationStoreStateRecord, SubnetStateRecord, WasmStoreGcRecord,
    WasmStoreRecord,
};
use crate::{
    dto::{state::SubnetStateResponse, template::WasmStorePublicationStateResponse},
    ids::{WasmStoreBinding, WasmStoreGcMode},
    ops::storage::state::mapper::SubnetStateMapper,
    storage::stable::state::subnet::{
        SubnetState, WasmStoreInventoryConflict, WasmStoreUpsertOutcome,
    },
    view::state::{PublicationStoreStateView, WasmStoreView},
};
use canic_core::{
    cdk::types::Principal,
    control_plane_support::error::{InternalError, InternalErrorOrigin},
};

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
/// SubnetStateOps
///

pub struct SubnetStateOps;

impl SubnetStateOps {
    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    /// Export the current subnet state as a response snapshot.
    #[must_use]
    pub fn snapshot_response() -> SubnetStateResponse {
        SubnetStateMapper::data_to_response(SubnetState::export())
    }

    /// Return the current root-owned publication binding, if one is pinned.
    #[must_use]
    pub fn publication_store_binding() -> Option<WasmStoreBinding> {
        SubnetState::publication_store_binding()
    }

    /// Return the current root-owned publication binding lifecycle state.
    #[must_use]
    pub fn publication_store_state() -> PublicationStoreStateView {
        SubnetStateMapper::publication_store_record_to_view(SubnetState::publication_store_state())
    }

    /// Return all known runtime-managed wasm stores for the current subnet.
    #[must_use]
    pub fn wasm_stores() -> Vec<WasmStoreView> {
        SubnetState::wasm_stores()
            .into_iter()
            .map(SubnetStateMapper::wasm_store_record_to_view)
            .collect()
    }

    /// Resolve one runtime-managed wasm store principal by logical binding.
    #[must_use]
    pub fn wasm_store_pid(binding: &WasmStoreBinding) -> Option<Principal> {
        SubnetState::wasm_store_pid(binding)
    }

    /// Resolve one runtime-managed wasm store binding by canister principal.
    #[must_use]
    pub fn wasm_store_binding_for_pid(pid: Principal) -> Option<WasmStoreBinding> {
        SubnetState::wasm_store_binding_for_pid(pid)
    }

    /// Persist one runtime-managed wasm store record.
    pub fn upsert_wasm_store(
        binding: WasmStoreBinding,
        pid: Principal,
        created_at: u64,
    ) -> Result<(), InternalError> {
        match SubnetState::upsert_wasm_store(binding, pid, created_at) {
            WasmStoreUpsertOutcome::Inserted | WasmStoreUpsertOutcome::Existing => Ok(()),
            WasmStoreUpsertOutcome::Conflict(conflict) => {
                Err(Self::wasm_store_inventory_conflict_error(conflict))
            }
        }
    }

    fn wasm_store_inventory_conflict_error(conflict: WasmStoreInventoryConflict) -> InternalError {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!(
                "wasm store inventory conflict: existing binding '{}' / pid {}; requested binding '{}' / pid {}",
                conflict.existing_binding,
                conflict.existing_pid,
                conflict.requested_binding,
                conflict.requested_pid,
            ),
        )
    }

    /// Remove one runtime-managed wasm store record by binding.
    #[must_use]
    pub fn remove_wasm_store(binding: &WasmStoreBinding) -> bool {
        SubnetState::remove_wasm_store(binding).is_some()
    }

    /// Persist one GC lifecycle transition for a runtime-managed wasm store.
    #[must_use]
    pub fn transition_wasm_store_gc(
        binding: &WasmStoreBinding,
        next: WasmStoreGcMode,
        changed_at: u64,
    ) -> bool {
        SubnetState::transition_wasm_store_gc(binding, next, changed_at)
    }

    /// Return the current root-owned publication binding lifecycle state as a DTO response.
    #[must_use]
    pub fn publication_store_state_response() -> WasmStorePublicationStateResponse {
        SubnetStateMapper::publication_store_record_to_response(
            SubnetState::publication_store_state(),
        )
    }

    /// Persist the current root-owned publication binding.
    #[must_use]
    pub fn activate_publication_store_binding(binding: WasmStoreBinding, changed_at: u64) -> bool {
        SubnetState::activate_publication_store_binding(binding, changed_at)
    }

    /// Clear the current root-owned publication binding.
    #[must_use]
    pub fn clear_publication_store_binding(changed_at: u64) -> bool {
        SubnetState::clear_publication_store_binding(changed_at)
    }

    /// Move the current detached binding into retired state.
    #[must_use]
    pub fn retire_detached_publication_store_binding(changed_at: u64) -> Option<WasmStoreBinding> {
        SubnetState::retire_detached_publication_store_binding(changed_at)
    }

    /// Clear the current retired binding after root verifies retirement is complete.
    #[must_use]
    pub fn finalize_retired_publication_store_binding(changed_at: u64) -> Option<WasmStoreBinding> {
        SubnetState::finalize_retired_publication_store_binding(changed_at)
    }

    #[cfg(test)]
    pub fn import_test_state(
        publication_store: PublicationStoreStateTestInput,
        wasm_stores: Vec<WasmStoreStateTestInput>,
    ) {
        SubnetState::import(ControlPlaneSubnetStateData {
            record: SubnetStateRecord {
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
            },
        });
    }
}
