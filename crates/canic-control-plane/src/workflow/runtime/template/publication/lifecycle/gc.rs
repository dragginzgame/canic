use super::super::super::store_pid_for_binding;
use super::super::{
    WasmStorePublicationWorkflow,
    store::{store_begin_gc, store_complete_gc, store_prepare_gc, store_status},
};
use crate::{
    ids::{WasmStoreBinding, WasmStoreGcMode},
    ops::storage::state::subnet::SubnetStateOps,
};
use canic_core::{__control_plane_core as cp_core, log, log::Topic};
use cp_core::{
    InternalError, InternalErrorOrigin, cdk::types::Principal, ops::ic::IcOps,
    workflow::ic::provision::ProvisionWorkflow,
};

impl WasmStorePublicationWorkflow {
    // Mark the current retired publication store as prepared for store-local GC execution.
    pub async fn prepare_retired_publication_store_for_gc()
    -> Result<Option<WasmStoreBinding>, InternalError> {
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        store_prepare_gc(store_pid).await?;
        let _ = SubnetStateOps::transition_wasm_store_gc(
            &retired_binding,
            WasmStoreGcMode::Prepared,
            IcOps::now_secs(),
        );

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
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        store_begin_gc(store_pid).await?;
        let _ = SubnetStateOps::transition_wasm_store_gc(
            &retired_binding,
            WasmStoreGcMode::InProgress,
            IcOps::now_secs(),
        );

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
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        store_complete_gc(store_pid).await?;
        let _ = SubnetStateOps::transition_wasm_store_gc(
            &retired_binding,
            WasmStoreGcMode::Complete,
            IcOps::now_secs(),
        );

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

        let changed_at = IcOps::now_secs();
        let previous = SubnetStateOps::publication_store_state();
        let finalized = SubnetStateOps::finalize_retired_publication_store_binding(changed_at)
            .map(|binding| (binding, store_pid));

        if let Some((binding, finalized_store_pid)) = finalized.as_ref() {
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
                binding,
                finalized_store_pid
            );
        }

        Ok(finalized)
    }

    // Delete one previously finalized retired publication store after local GC and root finalization complete.
    pub async fn delete_finalized_publication_store(
        binding: WasmStoreBinding,
        store_pid: Principal,
    ) -> Result<(), InternalError> {
        let state = SubnetStateOps::publication_store_state();

        if state.active_binding.as_ref() == Some(&binding)
            || state.detached_binding.as_ref() == Some(&binding)
            || state.retired_binding.as_ref() == Some(&binding)
        {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("ws '{binding}' is still referenced"),
            ));
        }

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

        ProvisionWorkflow::uninstall_and_delete_canister(store_pid).await?;
        let _ = SubnetStateOps::remove_wasm_store(&binding);

        log!(Topic::Wasm, Ok, "ws deleted {} ({})", binding, store_pid);

        Ok(())
    }
}
