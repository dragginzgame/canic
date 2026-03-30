#[cfg(test)]
use crate::storage::stable::state::subnet::SubnetStateRecord;
use crate::{
    dto::{state::SubnetStateResponse, template::WasmStorePublicationStateResponse},
    ids::{WasmStoreBinding, WasmStoreGcMode},
    ops::storage::state::mapper::SubnetStateMapper,
    storage::stable::state::subnet::{PublicationStoreStateRecord, SubnetState, WasmStoreRecord},
};
use canic_cdk::types::Principal;

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
        SubnetStateMapper::record_to_response(SubnetState::export())
    }

    /// Return the current root-owned publication binding, if one is pinned.
    #[must_use]
    pub fn publication_store_binding() -> Option<WasmStoreBinding> {
        SubnetState::publication_store_binding()
    }

    /// Return the current root-owned publication binding lifecycle state.
    #[must_use]
    pub fn publication_store_state() -> PublicationStoreStateRecord {
        SubnetState::publication_store_state()
    }

    /// Return all known runtime-managed wasm stores for the current subnet.
    #[must_use]
    pub fn wasm_stores() -> Vec<WasmStoreRecord> {
        SubnetState::wasm_stores()
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
    #[must_use]
    pub fn upsert_wasm_store(binding: WasmStoreBinding, pid: Principal, created_at: u64) -> bool {
        SubnetState::upsert_wasm_store(binding, pid, created_at)
    }

    /// Remove one runtime-managed wasm store record by binding.
    #[must_use]
    pub fn remove_wasm_store(binding: &WasmStoreBinding) -> Option<WasmStoreRecord> {
        SubnetState::remove_wasm_store(binding)
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
    pub fn import(data: SubnetStateRecord) {
        SubnetState::import(data);
    }
}
