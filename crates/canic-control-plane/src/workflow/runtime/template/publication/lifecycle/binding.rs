use super::super::super::store_pid_for_binding;
use super::super::WasmStorePublicationWorkflow;
use crate::{
    ids::WasmStoreBinding, ops::storage::state::subnet::SubnetStateOps,
    storage::stable::state::subnet::PublicationStoreStateRecord,
};
use canic_core::{__control_plane_core as cp_core, log, log::Topic};
use cp_core::{InternalError, InternalErrorOrigin, cdk::types::Principal, ops::ic::IcOps};

impl WasmStorePublicationWorkflow {
    // Build the canonical runtime-managed binding for one wasm store canister id.
    pub(in crate::workflow::runtime::template::publication::lifecycle) fn binding_for_store_pid(
        store_pid: Principal,
    ) -> WasmStoreBinding {
        WasmStoreBinding::owned(store_pid.to_text())
    }

    // Format one publication-state binding slot for structured transition logs.
    fn binding_slot(slot: Option<&WasmStoreBinding>) -> String {
        slot.map_or_else(|| "-".to_string(), std::string::ToString::to_string)
    }

    // Return true when a binding is already reserved for detached or retired lifecycle state.
    pub(in crate::workflow::runtime::template::publication) fn binding_is_reserved_for_publication(
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
    pub(in crate::workflow::runtime::template::publication::lifecycle) fn log_publication_state_transition(
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
    pub(in crate::workflow::runtime::template::publication) fn ensure_retired_binding_slot_available_for_promotion()
    -> Result<(), InternalError> {
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
    pub(in crate::workflow::runtime::template::publication) fn ensure_retired_binding_slot_available_for_retirement()
    -> Result<(), InternalError> {
        let state = SubnetStateOps::publication_store_state();

        if state.retired_binding.is_some() {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "ws retirement blocked: retired slot occupied".to_string(),
            ));
        }

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
    pub(in crate::workflow::runtime::template::publication::lifecycle) fn oldest_registered_store_binding()
    -> Option<WasmStoreBinding> {
        SubnetStateOps::wasm_stores()
            .into_iter()
            .min_by(|left, right| left.created_at.cmp(&right.created_at))
            .map(|record| record.binding)
    }

    // Clear one stale publication binding and fall back to the oldest known runtime store.
    pub(in crate::workflow::runtime::template::publication::lifecycle) fn clear_stale_publication_binding(
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
}
