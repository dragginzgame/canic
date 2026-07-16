//! Module: workflow::runtime::template::publication::lifecycle::gc
//!
//! Responsibility: orchestrate root-owned Wasm-store retirement and deletion.
//! Does not own: store-local GC execution, endpoint authorization, or persisted schemas.
//! Boundary: binds remote GC effects to one generation-checked publication state.

use super::super::super::store_pid_for_binding;
use super::super::{
    WasmStorePublicationWorkflow,
    error::PublicationWorkflowError,
    store::{store_begin_gc, store_complete_gc, store_prepare_gc, store_status},
};
use crate::{
    ids::{WasmStoreBinding, WasmStoreGcMode},
    ops::storage::state::subnet::SubnetStateOps,
    view::state::{PublicationStoreStateView, WasmStoreView},
};
use canic_core::cdk::types::Principal;
use canic_core::control_plane_support::{
    error::{InternalError, InternalErrorOrigin},
    ops::ic::IcOps,
    workflow::ic::provision::ProvisionWorkflow,
};
use canic_core::{log, log::Topic};
use std::cell::Cell;

thread_local! {
    static LIFECYCLE_OPERATION_IN_FLIGHT: Cell<bool> = const { Cell::new(false) };
}

#[derive(Debug)]
struct LifecycleOperationGuard;

impl LifecycleOperationGuard {
    fn try_enter() -> Result<Self, InternalError> {
        let entered = LIFECYCLE_OPERATION_IN_FLIGHT.with(|in_flight| {
            if in_flight.get() {
                false
            } else {
                in_flight.set(true);
                true
            }
        });

        if entered {
            Ok(Self)
        } else {
            Err(PublicationWorkflowError::LifecycleBusy.into())
        }
    }
}

impl Drop for LifecycleOperationGuard {
    fn drop(&mut self) {
        LIFECYCLE_OPERATION_IN_FLIGHT.with(|in_flight| {
            debug_assert!(in_flight.get());
            in_flight.set(false);
        });
    }
}

impl WasmStorePublicationWorkflow {
    // Resolve one binding from authoritative runtime inventory.
    fn runtime_store(binding: &WasmStoreBinding) -> Result<WasmStoreView, InternalError> {
        SubnetStateOps::wasm_stores()
            .into_iter()
            .find(|store| &store.binding == binding)
            .ok_or_else(|| {
                PublicationWorkflowError::InvalidState(format!(
                    "ws binding '{binding}' is missing from runtime inventory"
                ))
                .into()
            })
    }

    // Reject a post-await commit when publication ownership changed while the call was in flight.
    fn ensure_lifecycle_state_is_current(
        expected: &PublicationStoreStateView,
        binding: &WasmStoreBinding,
    ) -> Result<(), InternalError> {
        let current = SubnetStateOps::publication_store_state();
        if current.generation != expected.generation
            || current.retired_binding.as_ref() != Some(binding)
        {
            return Err(PublicationWorkflowError::LifecycleStateChanged {
                binding: binding.clone(),
                expected_generation: expected.generation,
                actual_generation: current.generation,
            }
            .into());
        }

        Ok(())
    }

    // Commit a remote GC transition only when the same retired binding still owns the lifecycle.
    fn persist_retired_gc_transition(
        expected: &PublicationStoreStateView,
        binding: &WasmStoreBinding,
        next: WasmStoreGcMode,
        changed_at: u64,
    ) -> Result<(), InternalError> {
        Self::ensure_lifecycle_state_is_current(expected, binding)?;
        let store = Self::runtime_store(binding)?;
        if store.gc.mode == next {
            return Ok(());
        }

        let required = match next {
            WasmStoreGcMode::Prepared => WasmStoreGcMode::Normal,
            WasmStoreGcMode::InProgress => WasmStoreGcMode::Prepared,
            WasmStoreGcMode::Complete => WasmStoreGcMode::InProgress,
            WasmStoreGcMode::Normal | WasmStoreGcMode::Clearing => {
                return Err(PublicationWorkflowError::InvalidState(format!(
                    "root lifecycle cannot persist gc mode {next:?} for '{binding}'"
                ))
                .into());
            }
        };

        if store.gc.mode != required {
            return Err(PublicationWorkflowError::StoreGcStateChanged {
                binding: binding.clone(),
                expected: required,
                actual: store.gc.mode,
            }
            .into());
        }

        if !SubnetStateOps::transition_wasm_store_gc(binding, next, changed_at) {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "failed to persist gc mode {next:?} for '{binding}'"
            ))
            .into());
        }

        Ok(())
    }

    // Require an exact finalized inventory entry before destructive canister deletion.
    fn ensure_finalized_store_is_deletable(
        binding: &WasmStoreBinding,
        store_pid: Principal,
    ) -> Result<(), InternalError> {
        let state = SubnetStateOps::publication_store_state();
        if state.active_binding.as_ref() == Some(binding)
            || state.detached_binding.as_ref() == Some(binding)
            || state.retired_binding.as_ref() == Some(binding)
        {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "ws '{binding}' is still referenced"
            ))
            .into());
        }

        let store = Self::runtime_store(binding)?;
        if store.pid != store_pid {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "ws binding '{binding}' resolves to {}, not deletion target {store_pid}",
                store.pid
            ))
            .into());
        }
        if store.gc.mode != WasmStoreGcMode::Complete {
            return Err(PublicationWorkflowError::StoreGcStateChanged {
                binding: binding.clone(),
                expected: WasmStoreGcMode::Complete,
                actual: store.gc.mode,
            }
            .into());
        }

        Ok(())
    }

    // Mark the current retired publication store as prepared for store-local GC execution.
    pub async fn prepare_retired_publication_store_for_gc()
    -> Result<Option<WasmStoreBinding>, InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        store_prepare_gc(store_pid).await?;
        Self::persist_retired_gc_transition(
            &state,
            &retired_binding,
            WasmStoreGcMode::Prepared,
            IcOps::now_secs(),
        )?;

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
        let _guard = LifecycleOperationGuard::try_enter()?;
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        store_begin_gc(store_pid).await?;
        Self::persist_retired_gc_transition(
            &state,
            &retired_binding,
            WasmStoreGcMode::InProgress,
            IcOps::now_secs(),
        )?;

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
        let _guard = LifecycleOperationGuard::try_enter()?;
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        store_complete_gc(store_pid).await?;
        Self::persist_retired_gc_transition(
            &state,
            &retired_binding,
            WasmStoreGcMode::Complete,
            IcOps::now_secs(),
        )?;

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
        let _guard = LifecycleOperationGuard::try_enter()?;
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

        Self::ensure_lifecycle_state_is_current(&state, &retired_binding)?;
        let runtime_store = Self::runtime_store(&retired_binding)?;
        if runtime_store.gc.mode != WasmStoreGcMode::Complete {
            return Err(PublicationWorkflowError::StoreGcStateChanged {
                binding: retired_binding.clone(),
                expected: WasmStoreGcMode::Complete,
                actual: runtime_store.gc.mode,
            }
            .into());
        }

        let changed_at = IcOps::now_secs();
        let previous = SubnetStateOps::publication_store_state();
        let finalized_binding = SubnetStateOps::finalize_retired_publication_store_binding(
            changed_at,
        )
        .ok_or_else(|| {
            PublicationWorkflowError::InvalidState(format!(
                "retired ws '{retired_binding}' disappeared before finalize commit"
            ))
        })?;
        if finalized_binding != retired_binding {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "finalized ws '{finalized_binding}' did not match expected '{retired_binding}'"
            ))
            .into());
        }
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
            finalized_binding,
            store_pid
        );

        Ok(Some((finalized_binding, store_pid)))
    }

    // Delete one previously finalized retired publication store after local GC and root finalization complete.
    pub async fn delete_finalized_publication_store(
        binding: WasmStoreBinding,
        store_pid: Principal,
    ) -> Result<(), InternalError> {
        let _guard = LifecycleOperationGuard::try_enter()?;
        Self::ensure_finalized_store_is_deletable(&binding, store_pid)?;

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

        Self::ensure_finalized_store_is_deletable(&binding, store_pid)?;
        ProvisionWorkflow::uninstall_and_delete_canister(store_pid).await?;
        if !SubnetStateOps::remove_wasm_store(&binding) {
            return Err(PublicationWorkflowError::InvalidState(format!(
                "deleted ws '{binding}' was missing from runtime inventory"
            ))
            .into());
        }

        log!(Topic::Wasm, Ok, "ws deleted {} ({})", binding, store_pid);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::stable::state::subnet::{
        ControlPlaneSubnetStateData, PublicationStoreStateRecord, SubnetStateRecord,
        WasmStoreGcRecord, WasmStoreRecord,
    };
    use canic_core::dto::error::ErrorCode;

    fn import_retired_store(mode: WasmStoreGcMode) -> (WasmStoreBinding, Principal) {
        let binding = WasmStoreBinding::new("retired");
        let pid = Principal::from_slice(&[7; 29]);
        SubnetStateOps::import(ControlPlaneSubnetStateData {
            record: SubnetStateRecord {
                publication_store: PublicationStoreStateRecord {
                    active_binding: Some(WasmStoreBinding::new("active")),
                    detached_binding: None,
                    retired_binding: Some(binding.clone()),
                    generation: 3,
                    changed_at: 30,
                    retired_at: 20,
                },
                wasm_stores: vec![WasmStoreRecord {
                    binding: binding.clone(),
                    pid,
                    created_at: 10,
                    gc: WasmStoreGcRecord {
                        mode,
                        changed_at: 20,
                        prepared_at: (mode != WasmStoreGcMode::Normal).then_some(11),
                        started_at: matches!(
                            mode,
                            WasmStoreGcMode::InProgress
                                | WasmStoreGcMode::Clearing
                                | WasmStoreGcMode::Complete
                        )
                        .then_some(12),
                        completed_at: (mode == WasmStoreGcMode::Complete).then_some(20),
                        runs_completed: u32::from(mode == WasmStoreGcMode::Complete),
                    },
                }],
            },
        });
        (binding, pid)
    }

    #[test]
    fn lifecycle_guard_rejects_concurrent_entry_and_releases_on_drop() {
        let guard = LifecycleOperationGuard::try_enter().expect("first operation enters");
        let err = LifecycleOperationGuard::try_enter().expect_err("second operation must reject");
        assert_eq!(
            err.public_error().map(|public| public.code),
            Some(ErrorCode::Conflict)
        );

        drop(guard);
        LifecycleOperationGuard::try_enter().expect("guard should release on drop");
    }

    #[test]
    fn retired_gc_commit_is_generation_bound_and_idempotent() {
        let (binding, _) = import_retired_store(WasmStoreGcMode::Normal);
        let expected = SubnetStateOps::publication_store_state();

        WasmStorePublicationWorkflow::persist_retired_gc_transition(
            &expected,
            &binding,
            WasmStoreGcMode::Prepared,
            40,
        )
        .expect("matching retired generation should commit");
        WasmStorePublicationWorkflow::persist_retired_gc_transition(
            &expected,
            &binding,
            WasmStoreGcMode::Prepared,
            41,
        )
        .expect("same transition should be idempotent");

        let store = WasmStorePublicationWorkflow::runtime_store(&binding).expect("runtime store");
        assert_eq!(store.gc.mode, WasmStoreGcMode::Prepared);
        assert_eq!(store.gc.changed_at, 40);

        assert!(SubnetStateOps::clear_publication_store_binding(42));
        let err = WasmStorePublicationWorkflow::persist_retired_gc_transition(
            &expected,
            &binding,
            WasmStoreGcMode::InProgress,
            43,
        )
        .expect_err("generation drift must reject post-await commit");
        assert_eq!(
            err.public_error().map(|public| public.code),
            Some(ErrorCode::Conflict)
        );
        let store = WasmStorePublicationWorkflow::runtime_store(&binding).expect("runtime store");
        assert_eq!(store.gc.mode, WasmStoreGcMode::Prepared);
    }

    #[test]
    fn finalized_delete_preflight_binds_inventory_identity_and_gc_state() {
        let (binding, pid) = import_retired_store(WasmStoreGcMode::Complete);
        SubnetStateOps::finalize_retired_publication_store_binding(40)
            .expect("retired binding finalizes");

        WasmStorePublicationWorkflow::ensure_finalized_store_is_deletable(&binding, pid)
            .expect("exact finalized store should be deletable");

        let err = WasmStorePublicationWorkflow::ensure_finalized_store_is_deletable(
            &binding,
            Principal::from_slice(&[8; 29]),
        )
        .expect_err("pid mismatch must reject deletion");
        assert_eq!(
            err.public_error().map(|public| public.code),
            Some(ErrorCode::InvariantViolation)
        );
    }
}
