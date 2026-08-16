//! Module: workflow::runtime::template::publication::lifecycle::creation
//!
//! Responsibility: require the one host-installed Store adopted by this root.
//! Does not own: Store creation, installation, replacement, rotation, or controller handoff.
//! Boundary: 0.100 roots fail closed unless adoption committed exactly one Store inventory row.

use super::super::WasmStorePublicationWorkflow;
use crate::{ids::WasmStoreBinding, ops::storage::state::root_wasm_store::RootWasmStoreStateOps};
use canic_core::control_plane_support::error::InternalError;

impl WasmStorePublicationWorkflow {
    /// Require the one sibling Store imported by the prepared-root adoption boundary.
    pub(crate) fn ensure_bootstrap_wasm_store() -> Result<WasmStoreBinding, InternalError> {
        let stores = RootWasmStoreStateOps::wasm_stores();
        let [store] = stores.as_slice() else {
            return Err(InternalError::invariant());
        };
        Ok(store.binding.clone())
    }
}
