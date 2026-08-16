//! Module: ops::storage::state::root_wasm_store
//!
//! Responsibility: provide deterministic access and conversion for root-owned Wasm Store state.
//! Does not own: lifecycle orchestration, endpoint authorization, or stable record schemas.
//! Boundary: workflows use this ops facade instead of opening stable storage directly.

#[cfg(test)]
use crate::storage::stable::state::root_wasm_store::{
    PublicationStoreStateRecord, RootWasmStoreStateData, RootWasmStoreStateRecord, WasmStoreRecord,
};
use crate::{
    dto::template::{WasmStoreGcStatusResponse, WasmStorePublicationStateResponse},
    ids::{WasmStoreBinding, WasmStoreGcMode},
    ops::storage::state::mapper::RootWasmStoreStateMapper,
    storage::stable::state::root_wasm_store::{
        RootWasmStoreState, SiblingWasmStoreAdoptionPhaseRecord, SiblingWasmStoreAdoptionRecord,
        WasmStoreGcRecord,
    },
    view::state::{PublicationStoreStateView, WasmStoreView},
};
use canic_core::{
    cdk::types::Principal, control_plane_support::error::InternalError,
    dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionResponse,
    ids::FleetSubnetWasmStoreAuthority,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SiblingWasmStoreAdoptionPlan {
    pub operation_id: [u8; 32],
    pub authority: FleetSubnetWasmStoreAuthority,
    pub temporary_controllers: Vec<Principal>,
    pub final_controllers: Vec<Principal>,
}

///
/// PublicationStoreStateTestInput
///
/// Ops-owned test input for publication-store lifecycle state.
///

#[cfg(test)]
pub struct PublicationStoreStateTestInput {
    pub active_binding: Option<WasmStoreBinding>,
    pub detached_binding: Option<WasmStoreBinding>,
    pub retired_binding: Option<WasmStoreBinding>,
    pub generation: u64,
    pub changed_at: u64,
    pub retired_at: u64,
}

///
/// WasmStoreStateTestInput
///
/// Ops-owned test input for one runtime-managed Wasm store.
///

#[cfg(test)]
pub struct WasmStoreStateTestInput {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub created_at: u64,
    pub gc_mode: WasmStoreGcMode,
    pub gc_changed_at: u64,
    pub prepared_at: Option<u64>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub runs_completed: u32,
}

///
/// RootWasmStoreStateOps
///

pub struct RootWasmStoreStateOps;

impl RootWasmStoreStateOps {
    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    /// Return the current root-owned publication binding lifecycle state.
    #[must_use]
    pub fn publication_store_state() -> PublicationStoreStateView {
        RootWasmStoreStateMapper::publication_store_record_to_view(
            RootWasmStoreState::publication_store_state(),
        )
    }

    /// Return all known runtime-managed wasm stores for the current subnet.
    #[must_use]
    pub fn wasm_stores() -> Vec<WasmStoreView> {
        RootWasmStoreState::wasm_stores()
            .into_iter()
            .map(RootWasmStoreStateMapper::wasm_store_record_to_view)
            .collect()
    }

    pub fn begin_sibling_wasm_store_adoption(
        plan: &SiblingWasmStoreAdoptionPlan,
    ) -> Result<(), InternalError> {
        RootWasmStoreState::begin_sibling_wasm_store_adoption(SiblingWasmStoreAdoptionRecord {
            operation_id: plan.operation_id,
            wasm_store: plan.authority.wasm_store,
            expected_module_hash: plan.authority.wasm_module_hash,
            temporary_controllers: plan.temporary_controllers.clone(),
            final_controllers: plan.final_controllers.clone(),
            phase: SiblingWasmStoreAdoptionPhaseRecord::MutationInFlight,
            adopted_at_ns: None,
        })
        .map(|_| ())
        .map_err(|_reason| InternalError::invariant())
    }

    pub fn commit_sibling_wasm_store_adoption(
        operation_id: [u8; 32],
        authority: FleetSubnetWasmStoreAuthority,
        adopted_at_ns: u64,
    ) -> Result<FleetSubnetWasmStoreAdoptionResponse, InternalError> {
        let record =
            RootWasmStoreState::commit_sibling_wasm_store_adoption(operation_id, adopted_at_ns)
                .map_err(|_reason| InternalError::invariant())?;
        adoption_response(record, authority)
    }

    pub fn sibling_wasm_store_adoption_receipt(
        operation_id: [u8; 32],
        authority: FleetSubnetWasmStoreAuthority,
    ) -> Result<Option<FleetSubnetWasmStoreAdoptionResponse>, InternalError> {
        let Some(record) = RootWasmStoreState::sibling_wasm_store_adoption() else {
            return Ok(None);
        };
        validate_adoption_authority(&record, operation_id, &authority)?;
        if record.phase == SiblingWasmStoreAdoptionPhaseRecord::MutationInFlight {
            return Ok(None);
        }
        adoption_response(record, authority).map(Some)
    }

    /// Resolve one runtime-managed wasm store principal by logical binding.
    #[must_use]
    pub fn wasm_store_pid(binding: &WasmStoreBinding) -> Option<Principal> {
        RootWasmStoreState::wasm_store_pid(binding)
    }

    /// Resolve one runtime-managed wasm store binding by canister principal.
    #[must_use]
    pub fn wasm_store_binding_for_pid(pid: Principal) -> Option<WasmStoreBinding> {
        RootWasmStoreState::wasm_store_binding_for_pid(pid)
    }

    /// Remove one runtime-managed wasm store record by binding.
    #[must_use]
    pub fn remove_wasm_store(binding: &WasmStoreBinding) -> bool {
        RootWasmStoreState::remove_wasm_store(binding).is_some()
    }

    /// Persist one GC lifecycle transition for a runtime-managed wasm store.
    #[must_use]
    pub fn transition_wasm_store_gc(
        binding: &WasmStoreBinding,
        next: WasmStoreGcMode,
        changed_at: u64,
    ) -> bool {
        RootWasmStoreState::transition_wasm_store_gc(binding, next, changed_at)
    }

    /// Reconcile runtime GC authority from one independently observed exact live Store.
    #[must_use]
    pub fn reconcile_wasm_store_gc(
        binding: &WasmStoreBinding,
        pid: Principal,
        live: &WasmStoreGcStatusResponse,
    ) -> bool {
        RootWasmStoreState::reconcile_wasm_store_gc(
            binding,
            pid,
            WasmStoreGcRecord {
                mode: live.mode,
                changed_at: live.changed_at,
                prepared_at: live.prepared_at,
                started_at: live.started_at,
                completed_at: live.completed_at,
                runs_completed: live.runs_completed,
            },
        )
    }

    /// Return the current root-owned publication binding lifecycle state as a DTO response.
    #[must_use]
    pub fn publication_store_state_response() -> WasmStorePublicationStateResponse {
        RootWasmStoreStateMapper::publication_store_record_to_response(
            RootWasmStoreState::publication_store_state(),
        )
    }

    /// Persist the current root-owned publication binding.
    #[must_use]
    pub fn activate_publication_store_binding(binding: WasmStoreBinding, changed_at: u64) -> bool {
        RootWasmStoreState::activate_publication_store_binding(binding, changed_at)
    }

    /// Clear the current root-owned publication binding.
    #[must_use]
    pub fn clear_publication_store_binding(changed_at: u64) -> bool {
        RootWasmStoreState::clear_publication_store_binding(changed_at)
    }

    /// Move the current detached binding into retired state.
    #[must_use]
    pub fn retire_detached_publication_store_binding(changed_at: u64) -> Option<WasmStoreBinding> {
        RootWasmStoreState::retire_detached_publication_store_binding(changed_at)
    }

    /// Clear the current retired binding after root verifies retirement is complete.
    #[must_use]
    pub fn finalize_retired_publication_store_binding(changed_at: u64) -> Option<WasmStoreBinding> {
        RootWasmStoreState::finalize_retired_publication_store_binding(changed_at)
    }

    #[cfg(test)]
    pub fn import_test_state(
        publication_store: PublicationStoreStateTestInput,
        wasm_stores: Vec<WasmStoreStateTestInput>,
    ) {
        RootWasmStoreState::import(RootWasmStoreStateData {
            record: RootWasmStoreStateRecord {
                publication_store: PublicationStoreStateRecord {
                    active_binding: publication_store.active_binding,
                    detached_binding: publication_store.detached_binding,
                    retired_binding: publication_store.retired_binding,
                    generation: publication_store.generation,
                    changed_at: publication_store.changed_at,
                    retired_at: publication_store.retired_at,
                },
                wasm_stores: wasm_stores
                    .into_iter()
                    .map(|store| WasmStoreRecord {
                        binding: store.binding,
                        pid: store.pid,
                        created_at: store.created_at,
                        gc: WasmStoreGcRecord {
                            mode: store.gc_mode,
                            changed_at: store.gc_changed_at,
                            prepared_at: store.prepared_at,
                            started_at: store.started_at,
                            completed_at: store.completed_at,
                            runs_completed: store.runs_completed,
                        },
                    })
                    .collect(),
                sibling_wasm_store_adoption: None,
            },
        });
    }
}

#[derive(Eq, PartialEq)]
struct SiblingWasmStoreAdoptionAuthority {
    operation_id: [u8; 32],
    wasm_store: Principal,
    expected_module_hash: [u8; 32],
    temporary_controllers: Vec<Principal>,
    final_controllers: Vec<Principal>,
}

fn adoption_response(
    record: SiblingWasmStoreAdoptionRecord,
    authority: FleetSubnetWasmStoreAuthority,
) -> Result<FleetSubnetWasmStoreAdoptionResponse, InternalError> {
    validate_adoption_authority(&record, record.operation_id, &authority)?;
    if record.phase != SiblingWasmStoreAdoptionPhaseRecord::Verified {
        return Err(InternalError::unavailable());
    }
    let adopted_at_ns = record
        .adopted_at_ns
        .ok_or_else(|| InternalError::invariant())?;
    Ok(FleetSubnetWasmStoreAdoptionResponse {
        operation_id: record.operation_id,
        authority,
        temporary_controllers: record.temporary_controllers,
        final_controllers: record.final_controllers,
        adopted_at_ns,
    })
}

fn validate_adoption_authority(
    record: &SiblingWasmStoreAdoptionRecord,
    operation_id: [u8; 32],
    authority: &FleetSubnetWasmStoreAuthority,
) -> Result<(), InternalError> {
    let mut temporary_controllers = vec![
        authority.installation_controller,
        authority.fleet_subnet_root,
    ];
    temporary_controllers.sort();
    let expected = SiblingWasmStoreAdoptionAuthority {
        operation_id,
        wasm_store: authority.wasm_store,
        expected_module_hash: authority.wasm_module_hash,
        temporary_controllers,
        final_controllers: vec![authority.fleet_subnet_root],
    };
    let observed = SiblingWasmStoreAdoptionAuthority {
        operation_id: record.operation_id,
        wasm_store: record.wasm_store,
        expected_module_hash: record.expected_module_hash,
        temporary_controllers: record.temporary_controllers.clone(),
        final_controllers: record.final_controllers.clone(),
    };
    if observed != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
}
