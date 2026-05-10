use super::{
    WASM_STORE_ROLE, WasmStorePublicationWorkflow,
    fleet::{PublicationPlacement, PublicationPlacementAction, PublicationStoreFleet},
    store::{store_begin_gc, store_catalog, store_complete_gc, store_prepare_gc, store_status},
};
use crate::{
    config,
    dto::template::WasmStoreRetiredStoreStatusResponse,
    ids::{WasmStoreBinding, WasmStoreGcMode},
    ops::storage::state::subnet::SubnetStateOps,
    storage::stable::state::subnet::PublicationStoreStateRecord,
};
use canic_core::{__control_plane_core as cp_core, log, log::Topic};
use cp_core::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    ops::{ic::IcOps, storage::registry::subnet::SubnetRegistryOps},
    workflow::{
        canister_lifecycle::{CanisterLifecycleEvent, CanisterLifecycleWorkflow},
        ic::provision::ProvisionWorkflow,
    },
};

use super::super::store_pid_for_binding;

impl WasmStorePublicationWorkflow {
    // Build the canonical runtime-managed binding for one wasm store canister id.
    fn binding_for_store_pid(store_pid: Principal) -> WasmStoreBinding {
        WasmStoreBinding::owned(store_pid.to_text())
    }

    // Import any already-registered wasm stores into runtime subnet state.
    pub fn sync_registered_wasm_store_inventory() -> Vec<WasmStoreBinding> {
        let mut bindings = Vec::new();

        for pid in SubnetRegistryOps::pids_for_role(&WASM_STORE_ROLE).unwrap_or_default() {
            let binding = Self::binding_for_store_pid(pid);
            let created_at = SubnetRegistryOps::get(pid).map_or(0, |record| record.created_at);
            let _ = SubnetStateOps::upsert_wasm_store(binding.clone(), pid, created_at);
            bindings.push(binding);
        }

        bindings
    }

    // Return the current retired runtime-managed publication store status, if one exists.
    pub async fn retired_publication_store_status()
    -> Result<Option<WasmStoreRetiredStoreStatusResponse>, InternalError> {
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        let store = store_status(store_pid).await?;

        Ok(Some(WasmStoreRetiredStoreStatusResponse {
            retired_binding,
            generation: state.generation,
            retired_at: state.retired_at,
            gc_ready: store.gc.mode == WasmStoreGcMode::Complete,
            reclaimable_store_bytes: store.occupied_store_bytes,
            store,
        }))
    }

    // Create one new wasm store canister and register its runtime-managed binding.
    async fn create_publication_store() -> Result<WasmStoreBinding, InternalError> {
        let result = CanisterLifecycleWorkflow::apply(CanisterLifecycleEvent::Create {
            role: WASM_STORE_ROLE,
            parent: IcOps::canister_self(),
            extra_arg: None,
        })
        .await?;
        let pid = result.new_canister_pid.ok_or_else(|| {
            InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "wasm store creation did not return a pid",
            )
        })?;
        let binding = Self::binding_for_store_pid(pid);
        let created_at =
            SubnetRegistryOps::get(pid).map_or_else(IcOps::now_secs, |record| record.created_at);
        let _ = SubnetStateOps::upsert_wasm_store(binding.clone(), pid, created_at);

        log!(Topic::Wasm, Ok, "ws created {} ({})", binding, pid);

        Ok(binding)
    }

    // Snapshot the current writable store fleet and the current preferred write hint.
    pub(super) async fn snapshot_publication_store_fleet()
    -> Result<PublicationStoreFleet, InternalError> {
        Self::sync_registered_wasm_store_inventory();

        let preferred_binding = match SubnetStateOps::publication_store_binding() {
            Some(binding) if store_pid_for_binding(&binding).is_ok() => Some(binding),
            Some(binding) => Some(Self::clear_stale_publication_binding(binding)?),
            None => Self::oldest_registered_store_binding(),
        };
        let reserved_state = SubnetStateOps::publication_store_state();
        let mut stores = Vec::new();

        for record in SubnetStateOps::wasm_stores() {
            let status = store_status(record.pid).await?;
            let releases = store_catalog(record.pid).await?;
            stores.push(super::fleet::PublicationStoreSnapshot {
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

    // Allocate one additional empty store and add it to the managed publication fleet.
    pub(super) async fn create_store_for_fleet(
        fleet: &mut PublicationStoreFleet,
    ) -> Result<PublicationPlacement, InternalError> {
        let binding = match fleet.preferred_binding.clone() {
            Some(_) => Self::create_publication_store().await?,
            None => Self::create_and_activate_first_publication_store().await?,
        };
        let store_pid = store_pid_for_binding(&binding)?;
        let record = SubnetStateOps::wasm_stores()
            .into_iter()
            .find(|record| record.binding == binding)
            .ok_or_else(|| {
                InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!("new ws '{binding}' missing from subnet state"),
                )
            })?;

        fleet.push_store(record, config::current_subnet_default_wasm_store());
        if fleet.preferred_binding.is_none() {
            fleet.preferred_binding = Some(binding.clone());
        }
        fleet.reserved_state = SubnetStateOps::publication_store_state();

        Ok(PublicationPlacement {
            binding,
            pid: store_pid,
            action: PublicationPlacementAction::Create,
        })
    }

    // Format one publication-state binding slot for structured transition logs.
    fn binding_slot(slot: Option<&WasmStoreBinding>) -> String {
        slot.map_or_else(|| "-".to_string(), std::string::ToString::to_string)
    }

    // Return true when a binding is already reserved for detached or retired lifecycle state.
    pub(super) fn binding_is_reserved_for_publication(
        state: &PublicationStoreStateRecord,
        binding: &WasmStoreBinding,
    ) -> bool {
        state.detached_binding.as_ref() == Some(binding)
            || state.retired_binding.as_ref() == Some(binding)
    }

    // Reject explicit publication selection when the binding is already detached or retired.
    fn ensure_binding_is_selectable_for_publication(
        state: &PublicationStoreStateRecord,
        binding: &WasmStoreBinding,
    ) -> Result<(), InternalError> {
        if Self::binding_is_reserved_for_publication(state, binding) {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("ws binding '{binding}' is detached/retired"),
            ));
        }

        Ok(())
    }

    // Emit one structured publication-binding transition record after root-owned state changes.
    fn log_publication_state_transition(
        transition_kind: &str,
        previous: &PublicationStoreStateRecord,
        current: &PublicationStoreStateRecord,
        changed_at: u64,
    ) {
        if previous == current {
            return;
        }

        log!(
            Topic::Wasm,
            Info,
            "ws.transition kind={} gen={} at={} old_a={} old_d={} old_r={} new_a={} new_d={} new_r={}",
            transition_kind,
            current.generation,
            changed_at,
            Self::binding_slot(previous.active_binding.as_ref()),
            Self::binding_slot(previous.detached_binding.as_ref()),
            Self::binding_slot(previous.retired_binding.as_ref()),
            Self::binding_slot(current.active_binding.as_ref()),
            Self::binding_slot(current.detached_binding.as_ref()),
            Self::binding_slot(current.retired_binding.as_ref()),
        );
    }

    // Reject rollover when it would overwrite an older retired store.
    pub(super) fn ensure_retired_binding_slot_available_for_promotion() -> Result<(), InternalError>
    {
        let state = SubnetStateOps::publication_store_state();

        if state.detached_binding.is_some() && state.retired_binding.is_some() {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "ws rollover blocked: retired slot occupied".to_string(),
            ));
        }

        Ok(())
    }

    // Reject explicit retirement when one retired store is already pending cleanup.
    pub(super) fn ensure_retired_binding_slot_available_for_retirement() -> Result<(), InternalError>
    {
        let state = SubnetStateOps::publication_store_state();

        if state.retired_binding.is_some() {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "ws retirement blocked: retired slot occupied".to_string(),
            ));
        }

        Ok(())
    }

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

    // Move the current detached publication binding into retired state.
    pub fn retire_detached_publication_store_binding() -> Option<WasmStoreBinding> {
        if let Err(err) = Self::ensure_retired_binding_slot_available_for_retirement() {
            log!(Topic::Wasm, Warn, "{err}");
            return None;
        }

        let changed_at = IcOps::now_secs();
        let previous = SubnetStateOps::publication_store_state();
        let retired = SubnetStateOps::retire_detached_publication_store_binding(changed_at);

        if let Some(binding) = retired.as_ref() {
            let current = SubnetStateOps::publication_store_state();
            Self::log_publication_state_transition(
                "retire_detached_binding",
                &previous,
                &current,
                changed_at,
            );
            log!(Topic::Wasm, Ok, "ws retired {}", binding);
        }

        retired
    }

    // Persist one explicit publication binding after validating that it exists in subnet config.
    pub fn set_current_publication_store_binding(
        binding: WasmStoreBinding,
    ) -> Result<(), InternalError> {
        let _ = store_pid_for_binding(&binding)?;
        Self::ensure_retired_binding_slot_available_for_promotion()?;
        let previous = SubnetStateOps::publication_store_state();
        Self::ensure_binding_is_selectable_for_publication(&previous, &binding)?;
        let changed_at = IcOps::now_secs();

        if SubnetStateOps::activate_publication_store_binding(binding, changed_at) {
            let current = SubnetStateOps::publication_store_state();
            Self::log_publication_state_transition(
                "pin_publication_binding",
                &previous,
                &current,
                changed_at,
            );
        }

        Ok(())
    }

    // Clear the explicit publication binding and fall back to configured store selection.
    pub fn clear_current_publication_store_binding() {
        if let Err(err) = Self::ensure_retired_binding_slot_available_for_promotion() {
            log!(Topic::Wasm, Warn, "{err}");
            return;
        }

        let changed_at = IcOps::now_secs();
        let previous = SubnetStateOps::publication_store_state();

        if SubnetStateOps::clear_publication_store_binding(changed_at) {
            let current = SubnetStateOps::publication_store_state();
            Self::log_publication_state_transition(
                "clear_publication_binding",
                &previous,
                &current,
                changed_at,
            );
        }
    }

    // Return the oldest known runtime-managed wasm-store binding for this subnet.
    fn oldest_registered_store_binding() -> Option<WasmStoreBinding> {
        SubnetStateOps::wasm_stores()
            .into_iter()
            .min_by(|left, right| left.created_at.cmp(&right.created_at))
            .map(|record| record.binding)
    }

    // Clear one stale publication binding and fall back to the oldest known runtime store.
    fn clear_stale_publication_binding(
        binding: WasmStoreBinding,
    ) -> Result<WasmStoreBinding, InternalError> {
        log!(Topic::Wasm, Warn, "ws clear stale binding {}", binding);
        let changed_at = IcOps::now_secs();
        Self::ensure_retired_binding_slot_available_for_promotion()?;
        let previous = SubnetStateOps::publication_store_state();
        let _ = SubnetStateOps::clear_publication_store_binding(changed_at);
        let current = SubnetStateOps::publication_store_state();
        Self::log_publication_state_transition(
            "clear_stale_publication_binding",
            &previous,
            &current,
            changed_at,
        );

        Self::oldest_registered_store_binding().ok_or_else(|| {
            InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "no registered wasm stores after clearing stale publication binding",
            )
        })
    }

    // Create the first runtime-managed store and promote it into the active publication slot.
    async fn create_and_activate_first_publication_store() -> Result<WasmStoreBinding, InternalError>
    {
        let binding = Self::create_publication_store().await?;
        Self::ensure_retired_binding_slot_available_for_promotion()?;
        let changed_at = IcOps::now_secs();
        let previous = SubnetStateOps::publication_store_state();
        let _ = SubnetStateOps::activate_publication_store_binding(binding.clone(), changed_at);
        let current = SubnetStateOps::publication_store_state();
        Self::log_publication_state_transition(
            "activate_first_publication_binding",
            &previous,
            &current,
            changed_at,
        );

        Ok(binding)
    }
}
