use super::super::super::store_pid_for_binding;
use super::super::{
    WASM_STORE_ROLE, WasmStorePublicationWorkflow,
    fleet::{PublicationStoreFleet, PublicationStoreSnapshot},
    store::{store_catalog, store_status},
};
use crate::{ids::WasmStoreBinding, ops::storage::state::subnet::SubnetStateOps};
use canic_core::control_plane_support::{
    error::InternalError,
    ops::{cost_guard::CostGuardPermit, storage::registry::subnet::SubnetRegistryOps},
};

impl WasmStorePublicationWorkflow {
    // Import any already-registered wasm stores into runtime subnet state.
    pub fn sync_registered_wasm_store_inventory() -> Result<Vec<WasmStoreBinding>, InternalError> {
        let mut bindings = Vec::new();

        for registration in SubnetRegistryOps::registrations_for_role(&WASM_STORE_ROLE) {
            let binding = Self::binding_for_store_pid(registration.pid);
            SubnetStateOps::upsert_wasm_store(
                binding.clone(),
                registration.pid,
                registration.created_at,
            )?;
            bindings.push(binding);
        }

        Ok(bindings)
    }

    // Snapshot the current writable store fleet and the current preferred write hint.
    pub(in crate::workflow::runtime::template::publication) async fn snapshot_publication_store_fleet(
        publication_permit: &CostGuardPermit,
    ) -> Result<PublicationStoreFleet, InternalError> {
        Self::sync_registered_wasm_store_inventory()?;

        let preferred_binding = match SubnetStateOps::publication_store_binding() {
            Some(binding) if store_pid_for_binding(&binding).is_ok() => Some(binding),
            Some(binding) => Some(Self::clear_stale_publication_binding(binding)?),
            None => Self::oldest_registered_store_binding(),
        };
        let reserved_state = SubnetStateOps::publication_store_state();
        let mut stores = Vec::new();

        for record in SubnetStateOps::wasm_stores() {
            let status = store_status(record.pid).await?;
            let releases = store_catalog(publication_permit, record.pid).await?;
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
