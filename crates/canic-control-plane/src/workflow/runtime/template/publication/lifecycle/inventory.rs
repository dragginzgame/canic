use super::super::super::store_pid_for_binding;
use super::super::{
    WasmStorePublicationWorkflow,
    fleet::{PublicationStoreFleet, PublicationStoreSnapshot},
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

    // Snapshot the current writable store fleet and the current preferred write hint.
    pub(in crate::workflow::runtime::template::publication) async fn snapshot_publication_store_fleet(
        _publication_permit: &CostGuardPermit,
    ) -> Result<PublicationStoreFleet, InternalError> {
        let _ = Self::resume_pending_wasm_store_creation().await?;

        let preferred_binding = match RootWasmStoreStateOps::publication_store_binding() {
            Some(binding) if store_pid_for_binding(&binding).is_ok() => Some(binding),
            Some(binding) => Some(Self::clear_stale_publication_binding(binding)?),
            None => Self::oldest_runtime_store_binding(),
        };
        let reserved_state = RootWasmStoreStateOps::publication_store_state();
        let mut stores = Vec::new();

        for record in RootWasmStoreStateOps::wasm_stores() {
            let status = store_status(record.pid).await?;
            let releases = store_catalog(record.pid).await?;
            stores.push(PublicationStoreSnapshot {
                binding: record.binding,
                pid: record.pid,
                created_at: record.created_at,
                status,
                releases,
                stored_chunk_hashes: None,
            });
        }

        Ok(PublicationStoreFleet {
            preferred_binding,
            reserved_state,
            stores,
        })
    }
}
