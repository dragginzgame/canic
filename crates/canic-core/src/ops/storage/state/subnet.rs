#[cfg(test)]
use crate::storage::stable::state::subnet::SubnetStateRecord;
use crate::{
    dto::state::{SubnetStateInput, SubnetStateResponse},
    dto::template::WasmStorePublicationStateResponse,
    ids::WasmStoreBinding,
    ops::storage::state::mapper::SubnetStateMapper,
    storage::stable::state::subnet::{PublicationStoreStateRecord, SubnetState},
};

///
/// SubnetStateOps
///

pub struct SubnetStateOps;

impl SubnetStateOps {
    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    /// Export the current subnet state as a DTO snapshot.
    #[must_use]
    pub fn snapshot_input() -> SubnetStateInput {
        SubnetStateMapper::record_to_input(SubnetState::export())
    }

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

    /// Return the current root-owned publication binding lifecycle state as a DTO response.
    #[must_use]
    pub fn publication_store_state_response() -> WasmStorePublicationStateResponse {
        SubnetStateMapper::publication_store_record_to_response(
            SubnetState::publication_store_state(),
        )
    }

    /// Persist the current root-owned publication binding.
    pub fn activate_publication_store_binding(binding: WasmStoreBinding, changed_at: u64) -> bool {
        SubnetState::activate_publication_store_binding(binding, changed_at)
    }

    /// Clear the current root-owned publication binding.
    pub fn clear_publication_store_binding(changed_at: u64) -> bool {
        SubnetState::clear_publication_store_binding(changed_at)
    }

    /// Move the current detached binding into retired state.
    pub fn retire_detached_publication_store_binding(changed_at: u64) -> Option<WasmStoreBinding> {
        SubnetState::retire_detached_publication_store_binding(changed_at)
    }

    /// Clear the current retired binding after root verifies retirement is complete.
    pub fn finalize_retired_publication_store_binding(changed_at: u64) -> Option<WasmStoreBinding> {
        SubnetState::finalize_retired_publication_store_binding(changed_at)
    }

    #[cfg(test)]
    pub fn import(data: SubnetStateRecord) {
        SubnetState::import(data);
    }

    pub fn import_input(view: SubnetStateInput) {
        let record = SubnetStateMapper::input_to_record(view);
        SubnetState::import(record);
    }
}
