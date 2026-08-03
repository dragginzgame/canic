//! Module: workflow::runtime::template::publication::lifecycle::binding
//!
//! Responsibility: pin and verify the one adopted Store publication binding.
//! Does not own: Store selection, replacement, rotation, GC, or physical deletion.
//! Boundary: bootstrap may pin an empty publication state exactly once; later publication is read-only.

use super::super::WasmStorePublicationWorkflow;
use crate::{
    ids::{WasmStoreBinding, WasmStoreGcMode},
    ops::storage::state::root_wasm_store::RootWasmStoreStateOps,
    view::state::PublicationStoreStateView,
    workflow::runtime::template::publication::error::PublicationWorkflowError,
};
use canic_core::control_plane_support::{error::InternalError, ops::ic::IcOps};
use canic_core::{log, log::Topic};

impl WasmStorePublicationWorkflow {
    // Format one publication-state binding slot for structured transition logs.
    fn binding_slot(slot: Option<&WasmStoreBinding>) -> String {
        slot.map_or_else(|| "-".to_string(), std::string::ToString::to_string)
    }

    // Emit one structured publication-binding transition record after root-owned state changes.
    pub(in crate::workflow::runtime::template::publication::lifecycle) fn log_publication_state_transition(
        transition_kind: &str,
        previous: &PublicationStoreStateView,
        current: &PublicationStoreStateView,
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

    // Reject publication through any state other than the sole adopted Store as the active binding.
    pub(in crate::workflow::runtime::template::publication) fn require_active_publication_store(
        binding: &WasmStoreBinding,
    ) -> Result<(), InternalError> {
        let state = RootWasmStoreStateOps::publication_store_state();
        let is_exact = [
            state.active_binding.as_ref() == Some(binding),
            state.detached_binding.is_none(),
            state.retired_binding.is_none(),
        ]
        .into_iter()
        .all(|valid| valid);
        if !is_exact {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "publication state does not name adopted sibling Store '{binding}' as its sole active binding"
            ))
            .into());
        }
        Ok(())
    }

    // Pin the one adopted sibling Store into an otherwise empty bootstrap publication state.
    pub(in crate::workflow::runtime::template::publication) fn pin_initial_publication_store(
        binding: WasmStoreBinding,
    ) -> Result<(), InternalError> {
        let stores = RootWasmStoreStateOps::wasm_stores();
        let [store] = stores.as_slice() else {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "initial publication binding requires exactly one adopted sibling Store, found {}",
                stores.len()
            ))
            .into());
        };
        if store.binding != binding {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "initial publication binding '{binding}' does not match adopted sibling Store '{}'",
                store.binding
            ))
            .into());
        }
        if store.gc.mode != WasmStoreGcMode::Normal {
            return Err(PublicationWorkflowError::StoreNotWritable {
                binding,
                mode: store.gc.mode,
            }
            .into());
        }

        let previous = RootWasmStoreStateOps::publication_store_state();
        if previous.active_binding.as_ref() == Some(&store.binding) {
            return Self::require_active_publication_store(&store.binding);
        }
        let state_is_empty = [
            previous.active_binding.is_none(),
            previous.detached_binding.is_none(),
            previous.retired_binding.is_none(),
        ]
        .into_iter()
        .all(|empty| empty);
        if !state_is_empty {
            return Err(PublicationWorkflowError::InvalidState(
                "initial publication binding cannot replace or rotate existing Store authority"
                    .to_string(),
            )
            .into());
        }

        let changed_at = IcOps::now_secs();
        if !RootWasmStoreStateOps::activate_publication_store_binding(
            store.binding.clone(),
            changed_at,
        ) {
            return Err(PublicationWorkflowError::InvalidState(
                "initial publication binding did not commit".to_string(),
            )
            .into());
        }
        let current = RootWasmStoreStateOps::publication_store_state();
        Self::log_publication_state_transition(
            "pin_publication_binding",
            &previous,
            &current,
            changed_at,
        );
        Self::require_active_publication_store(&store.binding)
    }
}
