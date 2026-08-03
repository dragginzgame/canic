use super::super::{
    WasmStorePublicationWorkflow,
    snapshot::PublicationStoreSnapshot,
    store::{store_catalog, store_status},
};
use crate::ops::storage::state::root_wasm_store::RootWasmStoreStateOps;
use canic_core::control_plane_support::{
    error::{InternalError, InternalErrorOrigin},
    ops::cost_guard::CostGuardPermit,
    view::fleet_activation::FleetActivationWasmStoreView,
};

impl WasmStorePublicationWorkflow {
    /// Project the authoritative root-owned Store inventory for fresh Fleet activation.
    pub fn root_activation_wasm_store() -> Result<FleetActivationWasmStoreView, InternalError> {
        let stores = RootWasmStoreStateOps::wasm_stores();
        let [store] = stores.as_slice() else {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                format!(
                    "fresh Fleet activation requires exactly one root-owned Wasm Store, found {}",
                    stores.len()
                ),
            ));
        };
        Ok(FleetActivationWasmStoreView { pid: store.pid })
    }

    // Snapshot the one sibling Store imported by the prepared-root adoption boundary.
    pub(in crate::workflow::runtime::template::publication) async fn snapshot_adopted_wasm_store(
        _publication_permit: &CostGuardPermit,
    ) -> Result<PublicationStoreSnapshot, InternalError> {
        let stores = RootWasmStoreStateOps::wasm_stores();
        let [record] = stores.as_slice() else {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                format!(
                    "root publication requires exactly one adopted sibling Wasm Store, found {}",
                    stores.len()
                ),
            ));
        };
        Ok(PublicationStoreSnapshot {
            binding: record.binding.clone(),
            pid: record.pid,
            status: store_status(record.pid).await?,
            releases: store_catalog(record.pid).await?,
            stored_chunk_hashes: None,
        })
    }
}
