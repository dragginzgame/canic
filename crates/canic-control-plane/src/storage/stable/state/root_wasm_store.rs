//! Module: storage::stable::state::root_wasm_store
//!
//! Responsibility: persist root-owned Wasm Store publication, inventory, GC, and creation state.
//! Does not own: Store lifecycle orchestration, DTO projection, or endpoint authorization.
//! Boundary: storage ops wrap this complete root authority before workflow access.

use crate::ids::{WasmStoreBinding, WasmStoreGcMode};
#[cfg(feature = "root-control-plane")]
use canic_core::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static,
    role_contract::allocation::memory::control_plane::ROOT_WASM_STORE_STATE_ID,
};
use canic_core::{
    cdk::types::Principal, dto::root_store::RootStoreBootstrapResponse, impl_storable_bounded,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "root-control-plane")]
use std::cell::RefCell;

#[cfg(feature = "root-control-plane")]
eager_static! {
    static ROOT_WASM_STORE_STATE: RefCell<Cell<RootWasmStoreStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            canic_core::ic_memory_key!(authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY, key = "canic.control_plane.root.wasm_store.state.v1", ty = RootWasmStoreState, id = ROOT_WASM_STORE_STATE_ID),
            RootWasmStoreStateRecord::default(),
        ));
}

///
/// PublicationStoreStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PublicationStoreStateRecord {
    pub active_binding: Option<WasmStoreBinding>,
    pub detached_binding: Option<WasmStoreBinding>,
    pub retired_binding: Option<WasmStoreBinding>,
    pub generation: u64,
    pub changed_at: u64,
    pub retired_at: u64,
}

///
/// WasmStoreRecord
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct WasmStoreGcRecord {
    pub mode: WasmStoreGcMode,
    pub changed_at: u64,
    pub prepared_at: Option<u64>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub runs_completed: u32,
}

///
/// WasmStoreRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WasmStoreRecord {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub created_at: u64,
    pub gc: WasmStoreGcRecord,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SiblingWasmStoreAdoptionPhaseRecord {
    MutationInFlight,
    Verified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SiblingWasmStoreAdoptionRecord {
    pub operation_id: [u8; 32],
    pub wasm_store: Principal,
    pub expected_module_hash: [u8; 32],
    pub temporary_controllers: Vec<Principal>,
    pub final_controllers: Vec<Principal>,
    pub phase: SiblingWasmStoreAdoptionPhaseRecord,
    pub adopted_at_ns: Option<u64>,
}

/// Endpoint-specific terminal receipt for one completed Root Store bootstrap.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootStoreBootstrapRecord {
    pub operation_id: [u8; 32],
    pub manifest_payload_size_bytes: u64,
    pub response: RootStoreBootstrapResponse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "root-control-plane")]
pub enum SiblingWasmStoreAdoptionError {
    ConflictingAuthority,
    InventoryAlreadyPopulated,
    InvalidAuthority,
    MissingIntent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "root-control-plane")]
pub enum RootStoreBootstrapCommitError {
    ConflictingAuthority,
    InvalidReceipt,
}

#[cfg(feature = "root-control-plane")]
fn sibling_wasm_store_adoption_record_is_valid(record: &SiblingWasmStoreAdoptionRecord) -> bool {
    let canonical_temporary = record.temporary_controllers.len() == 2
        && record
            .temporary_controllers
            .windows(2)
            .all(|pair| pair[0] < pair[1]);
    let [final_controller] = record.final_controllers.as_slice() else {
        return false;
    };
    [
        record.operation_id != [0; 32],
        record.wasm_store != Principal::anonymous(),
        record.expected_module_hash != [0; 32],
        canonical_temporary,
        record.temporary_controllers.contains(final_controller),
        record.phase == SiblingWasmStoreAdoptionPhaseRecord::MutationInFlight,
        record.adopted_at_ns.is_none(),
    ]
    .into_iter()
    .all(|valid| valid)
}

///
/// RootWasmStoreStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootWasmStoreStateRecord {
    pub publication_store: PublicationStoreStateRecord,
    pub wasm_stores: Vec<WasmStoreRecord>,
    pub sibling_wasm_store_adoption: Option<SiblingWasmStoreAdoptionRecord>,
    pub root_store_bootstrap: Option<RootStoreBootstrapRecord>,
}

impl RootWasmStoreStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootWasmStoreStateRecord";
}

impl_storable_bounded!(RootWasmStoreStateRecord, 16_384, true);

///
/// RootWasmStoreStateData
///
/// Canonical root-owned Wasm Store state snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RootWasmStoreStateData {
    pub record: RootWasmStoreStateRecord,
}

impl RootWasmStoreStateData {
    pub const STATE_CONTRACT_NAME: &'static str = "RootWasmStoreStateData";
}

#[cfg(feature = "root-control-plane")]
enum PublicationStoreTransition {
    Activate(WasmStoreBinding),
    ClearActive,
    RetireDetached,
    FinalizeRetired,
}

#[cfg(feature = "root-control-plane")]
struct PublicationStoreTransitionOutcome {
    changed: bool,
    binding: Option<WasmStoreBinding>,
}

///
/// RootWasmStoreState
///

#[cfg(feature = "root-control-plane")]
pub struct RootWasmStoreState;

#[cfg(feature = "root-control-plane")]
impl RootWasmStoreState {
    fn validate_publication_store_state(state: &PublicationStoreStateRecord) {
        let active = state.active_binding.as_ref();
        let detached = state.detached_binding.as_ref();
        let retired = state.retired_binding.as_ref();

        assert!(
            active.is_none() || detached.is_none() || active != detached,
            "publication store active/detached bindings must differ"
        );
        assert!(
            active.is_none() || retired.is_none() || active != retired,
            "publication store active/retired bindings must differ"
        );
        assert!(
            detached.is_none() || retired.is_none() || detached != retired,
            "publication store detached/retired bindings must differ"
        );
        assert_eq!(
            state.retired_binding.is_some(),
            state.retired_at != 0,
            "publication store retired_at must be set iff retired_binding is present"
        );
    }

    fn validate_publication_store_transition(
        previous: &PublicationStoreStateRecord,
        current: &PublicationStoreStateRecord,
        changed: bool,
    ) {
        Self::validate_publication_store_state(current);

        if changed {
            let expected_generation = previous
                .generation
                .checked_add(1)
                .expect("publication store generation overflow");

            assert_eq!(
                current.generation, expected_generation,
                "publication store generation must increment exactly once per state change"
            );
        } else {
            assert_eq!(
                current, previous,
                "publication store state must remain unchanged when no transition is applied"
            );
        }
    }

    fn apply_publication_store_transition(
        transition: PublicationStoreTransition,
        changed_at: u64,
    ) -> PublicationStoreTransitionOutcome {
        ROOT_WASM_STORE_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let previous = data.publication_store.clone();
            let mut changed = false;
            let mut binding = None;

            match transition {
                PublicationStoreTransition::Activate(next_binding) => {
                    if data.publication_store.active_binding.as_ref() != Some(&next_binding) {
                        if let Some(detached_binding) =
                            data.publication_store.detached_binding.take()
                        {
                            data.publication_store.retired_binding = Some(detached_binding);
                            data.publication_store.retired_at = changed_at;
                        }

                        data.publication_store.detached_binding =
                            data.publication_store.active_binding.take();
                        data.publication_store.active_binding = Some(next_binding);
                        changed = true;
                    }
                }
                PublicationStoreTransition::ClearActive => {
                    if let Some(active_binding) = data.publication_store.active_binding.take() {
                        if let Some(detached_binding) =
                            data.publication_store.detached_binding.take()
                        {
                            data.publication_store.retired_binding = Some(detached_binding);
                            data.publication_store.retired_at = changed_at;
                        }

                        data.publication_store.detached_binding = Some(active_binding);
                        changed = true;
                    }
                }
                PublicationStoreTransition::RetireDetached => {
                    if let Some(detached_binding) = data.publication_store.detached_binding.take() {
                        data.publication_store.retired_binding = Some(detached_binding.clone());
                        data.publication_store.retired_at = changed_at;
                        binding = Some(detached_binding);
                        changed = true;
                    }
                }
                PublicationStoreTransition::FinalizeRetired => {
                    if let Some(retired_binding) = data.publication_store.retired_binding.take() {
                        data.publication_store.retired_at = 0;
                        binding = Some(retired_binding);
                        changed = true;
                    }
                }
            }

            if changed {
                data.publication_store.generation = previous
                    .generation
                    .checked_add(1)
                    .expect("publication store generation overflow");
                data.publication_store.changed_at = changed_at;
                Self::validate_publication_store_transition(
                    &previous,
                    &data.publication_store,
                    true,
                );
                cell.set(data);
            } else {
                Self::validate_publication_store_transition(&previous, &previous, false);
            }

            PublicationStoreTransitionOutcome { changed, binding }
        })
    }

    #[must_use]
    pub(crate) fn publication_store_state() -> PublicationStoreStateRecord {
        Self::export().record.publication_store
    }

    #[must_use]
    pub(crate) fn wasm_stores() -> Vec<WasmStoreRecord> {
        Self::export().record.wasm_stores
    }

    pub(crate) fn sibling_wasm_store_adoption() -> Option<SiblingWasmStoreAdoptionRecord> {
        Self::export().record.sibling_wasm_store_adoption
    }

    pub(crate) fn root_store_bootstrap() -> Option<RootStoreBootstrapRecord> {
        Self::export().record.root_store_bootstrap
    }

    pub(crate) fn commit_root_store_bootstrap(
        record: RootStoreBootstrapRecord,
    ) -> Result<RootStoreBootstrapRecord, RootStoreBootstrapCommitError> {
        let receipt_is_valid = [
            record.operation_id != [0; 32],
            record.manifest_payload_size_bytes != 0,
            record.response.fleet_subnet_root != Principal::anonymous(),
            record.response.wasm_store != Principal::anonymous(),
        ]
        .into_iter()
        .all(|valid| valid);
        if !receipt_is_valid {
            return Err(RootStoreBootstrapCommitError::InvalidReceipt);
        }
        ROOT_WASM_STORE_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            if let Some(existing) = &data.root_store_bootstrap {
                return if existing == &record {
                    Ok(existing.clone())
                } else {
                    Err(RootStoreBootstrapCommitError::ConflictingAuthority)
                };
            }
            data.root_store_bootstrap = Some(record.clone());
            cell.set(data);
            Ok(record)
        })
    }

    pub(crate) fn begin_sibling_wasm_store_adoption(
        record: SiblingWasmStoreAdoptionRecord,
    ) -> Result<SiblingWasmStoreAdoptionRecord, SiblingWasmStoreAdoptionError> {
        if !sibling_wasm_store_adoption_record_is_valid(&record) {
            return Err(SiblingWasmStoreAdoptionError::InvalidAuthority);
        }
        ROOT_WASM_STORE_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            if let Some(existing) = &data.sibling_wasm_store_adoption {
                return if existing == &record {
                    Ok(existing.clone())
                } else {
                    Err(SiblingWasmStoreAdoptionError::ConflictingAuthority)
                };
            }
            if !data.wasm_stores.is_empty() {
                return Err(SiblingWasmStoreAdoptionError::InventoryAlreadyPopulated);
            }
            data.sibling_wasm_store_adoption = Some(record.clone());
            cell.set(data);
            Ok(record)
        })
    }

    pub(crate) fn commit_sibling_wasm_store_adoption(
        operation_id: [u8; 32],
        adopted_at_ns: u64,
    ) -> Result<SiblingWasmStoreAdoptionRecord, SiblingWasmStoreAdoptionError> {
        if adopted_at_ns == 0 {
            return Err(SiblingWasmStoreAdoptionError::InvalidAuthority);
        }
        ROOT_WASM_STORE_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let adoption = data
                .sibling_wasm_store_adoption
                .as_mut()
                .ok_or(SiblingWasmStoreAdoptionError::MissingIntent)?;
            if adoption.operation_id != operation_id {
                return Err(SiblingWasmStoreAdoptionError::ConflictingAuthority);
            }
            if adoption.phase == SiblingWasmStoreAdoptionPhaseRecord::Verified {
                return Ok(adoption.clone());
            }
            if !data.wasm_stores.is_empty() {
                return Err(SiblingWasmStoreAdoptionError::InventoryAlreadyPopulated);
            }
            adoption.phase = SiblingWasmStoreAdoptionPhaseRecord::Verified;
            adoption.adopted_at_ns = Some(adopted_at_ns);
            let binding = WasmStoreBinding::owned(adoption.wasm_store.to_text());
            data.wasm_stores.push(WasmStoreRecord {
                binding,
                pid: adoption.wasm_store,
                created_at: adopted_at_ns,
                gc: WasmStoreGcRecord::default(),
            });
            let committed = adoption.clone();
            cell.set(data);
            Ok(committed)
        })
    }

    #[must_use]
    pub(crate) fn wasm_store_pid(binding: &WasmStoreBinding) -> Option<Principal> {
        Self::export()
            .record
            .wasm_stores
            .into_iter()
            .find(|record| &record.binding == binding)
            .map(|record| record.pid)
    }

    #[must_use]
    pub(crate) fn wasm_store_binding_for_pid(pid: Principal) -> Option<WasmStoreBinding> {
        Self::export()
            .record
            .wasm_stores
            .into_iter()
            .find(|record| record.pid == pid)
            .map(|record| record.binding)
    }

    pub(crate) fn transition_wasm_store_gc(
        binding: &WasmStoreBinding,
        next: WasmStoreGcMode,
        changed_at: u64,
    ) -> bool {
        ROOT_WASM_STORE_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let Some(record) = data
                .wasm_stores
                .iter_mut()
                .find(|record| &record.binding == binding)
            else {
                return false;
            };

            if record.gc.mode == next {
                return false;
            }

            record.gc.mode = next;
            record.gc.changed_at = changed_at;

            match next {
                WasmStoreGcMode::Normal => {
                    record.gc.prepared_at = None;
                    record.gc.started_at = None;
                    record.gc.completed_at = None;
                }
                WasmStoreGcMode::Prepared => {
                    record.gc.prepared_at = Some(changed_at);
                    record.gc.started_at = None;
                    record.gc.completed_at = None;
                }
                WasmStoreGcMode::InProgress => {
                    record.gc.started_at = Some(changed_at);
                    record.gc.completed_at = None;
                }
                WasmStoreGcMode::Clearing => {}
                WasmStoreGcMode::Complete => {
                    record.gc.completed_at = Some(changed_at);
                    record.gc.runs_completed = record.gc.runs_completed.saturating_add(1);
                }
            }

            cell.set(data);
            true
        })
    }

    pub(crate) fn reconcile_wasm_store_gc(
        binding: &WasmStoreBinding,
        pid: Principal,
        next: WasmStoreGcRecord,
    ) -> bool {
        if !wasm_store_gc_record_is_valid(&next) {
            return false;
        }
        ROOT_WASM_STORE_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let Some(record) = data
                .wasm_stores
                .iter_mut()
                .find(|record| &record.binding == binding && record.pid == pid)
            else {
                return false;
            };
            if !wasm_store_gc_reconciliation_is_monotonic(&record.gc, &next) {
                return false;
            }
            if record.gc == next {
                return true;
            }
            record.gc = next;
            cell.set(data);
            true
        })
    }

    pub(crate) fn remove_wasm_store(binding: &WasmStoreBinding) -> Option<WasmStoreRecord> {
        ROOT_WASM_STORE_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            let index = data
                .wasm_stores
                .iter()
                .position(|record| &record.binding == binding)?;
            let removed = data.wasm_stores.remove(index);
            cell.set(data);
            Some(removed)
        })
    }

    pub(crate) fn activate_publication_store_binding(
        binding: WasmStoreBinding,
        changed_at: u64,
    ) -> bool {
        Self::apply_publication_store_transition(
            PublicationStoreTransition::Activate(binding),
            changed_at,
        )
        .changed
    }
    pub(crate) fn clear_publication_store_binding(changed_at: u64) -> bool {
        Self::apply_publication_store_transition(
            PublicationStoreTransition::ClearActive,
            changed_at,
        )
        .changed
    }

    pub(crate) fn retire_detached_publication_store_binding(
        changed_at: u64,
    ) -> Option<WasmStoreBinding> {
        Self::apply_publication_store_transition(
            PublicationStoreTransition::RetireDetached,
            changed_at,
        )
        .binding
    }

    pub(crate) fn finalize_retired_publication_store_binding(
        changed_at: u64,
    ) -> Option<WasmStoreBinding> {
        Self::apply_publication_store_transition(
            PublicationStoreTransition::FinalizeRetired,
            changed_at,
        )
        .binding
    }

    #[cfg(test)]
    pub(crate) fn import(data: RootWasmStoreStateData) {
        Self::validate_publication_store_state(&data.record.publication_store);
        let mut seen_bindings = std::collections::BTreeSet::new();
        let mut seen_pids = std::collections::BTreeSet::new();
        for record in &data.record.wasm_stores {
            assert!(
                seen_bindings.insert(record.binding.clone()),
                "duplicate wasm store binding '{}'",
                record.binding
            );
            assert!(
                seen_pids.insert(record.pid),
                "duplicate wasm store pid '{}'",
                record.pid
            );
        }
        ROOT_WASM_STORE_STATE.with_borrow_mut(|cell| cell.set(data.record));
    }

    #[must_use]
    pub(crate) fn export() -> RootWasmStoreStateData {
        RootWasmStoreStateData {
            record: ROOT_WASM_STORE_STATE.with_borrow(|cell| cell.get().clone()),
        }
    }
}

#[cfg(feature = "root-control-plane")]
fn wasm_store_gc_record_is_valid(record: &WasmStoreGcRecord) -> bool {
    match record.mode {
        WasmStoreGcMode::Normal => [
            record.prepared_at.is_none(),
            record.started_at.is_none(),
            record.completed_at.is_none(),
            record.runs_completed == 0,
        ]
        .into_iter()
        .all(|valid| valid),
        WasmStoreGcMode::Prepared => record.prepared_at.is_some_and(|prepared_at| {
            [
                prepared_at > 0,
                record.changed_at == prepared_at,
                record.started_at.is_none(),
                record.completed_at.is_none(),
                record.runs_completed == 0,
            ]
            .into_iter()
            .all(|valid| valid)
        }),
        WasmStoreGcMode::InProgress | WasmStoreGcMode::Clearing => {
            let Some(prepared_at) = record.prepared_at else {
                return false;
            };
            let Some(started_at) = record.started_at else {
                return false;
            };
            [
                prepared_at > 0,
                started_at >= prepared_at,
                record.changed_at >= started_at,
                record.completed_at.is_none(),
                record.runs_completed == 0,
            ]
            .into_iter()
            .all(|valid| valid)
        }
        WasmStoreGcMode::Complete => {
            let Some(prepared_at) = record.prepared_at else {
                return false;
            };
            let Some(started_at) = record.started_at else {
                return false;
            };
            let Some(completed_at) = record.completed_at else {
                return false;
            };
            [
                prepared_at > 0,
                started_at >= prepared_at,
                completed_at >= started_at,
                record.changed_at == completed_at,
                record.runs_completed == 1,
            ]
            .into_iter()
            .all(|valid| valid)
        }
    }
}

#[cfg(feature = "root-control-plane")]
fn wasm_store_gc_reconciliation_is_monotonic(
    current: &WasmStoreGcRecord,
    next: &WasmStoreGcRecord,
) -> bool {
    let mode_is_monotonic = match (current.mode, next.mode) {
        (left, right) if left == right => true,
        (WasmStoreGcMode::Normal, WasmStoreGcMode::Prepared)
        | (
            WasmStoreGcMode::Prepared | WasmStoreGcMode::Clearing,
            WasmStoreGcMode::InProgress | WasmStoreGcMode::Complete,
        )
        | (WasmStoreGcMode::InProgress, WasmStoreGcMode::Clearing | WasmStoreGcMode::Complete) => {
            true
        }
        _ => false,
    };
    let prepared_at_is_stable = current
        .prepared_at
        .is_none_or(|prepared_at| next.prepared_at == Some(prepared_at));
    let started_at_is_stable = current
        .started_at
        .is_none_or(|started_at| next.started_at == Some(started_at));
    [
        mode_is_monotonic,
        prepared_at_is_stable,
        started_at_is_stable,
        next.changed_at >= current.changed_at,
        next.runs_completed >= current.runs_completed,
    ]
    .into_iter()
    .all(|valid| valid)
}

#[cfg(all(test, feature = "root-control-plane"))]
mod tests {
    use super::*;

    #[test]
    fn root_wasm_store_state_round_trips_through_canonical_data_snapshot() {
        RootWasmStoreState::import(RootWasmStoreStateData {
            record: RootWasmStoreStateRecord {
                publication_store: PublicationStoreStateRecord::default(),
                wasm_stores: vec![WasmStoreRecord {
                    binding: WasmStoreBinding::new("primary"),
                    pid: Principal::from_slice(&[1; 29]),
                    created_at: 10,
                    gc: WasmStoreGcRecord::default(),
                }],
                sibling_wasm_store_adoption: None,
                root_store_bootstrap: None,
            },
        });

        let data = RootWasmStoreState::export();
        RootWasmStoreState::import(RootWasmStoreStateData::default());
        RootWasmStoreState::import(data.clone());

        assert_eq!(RootWasmStoreState::export(), data);
        RootWasmStoreState::import(RootWasmStoreStateData::default());
    }

    #[test]
    fn sibling_store_adoption_commits_receipt_and_inventory_atomically() {
        RootWasmStoreState::import(RootWasmStoreStateData::default());
        let pid = Principal::from_slice(&[8; 29]);
        let root = Principal::from_slice(&[9; 29]);
        let installer = Principal::from_slice(&[10; 29]);
        let mut temporary_controllers = vec![root, installer];
        temporary_controllers.sort();
        let intent = SiblingWasmStoreAdoptionRecord {
            operation_id: [1; 32],
            wasm_store: pid,
            expected_module_hash: [2; 32],
            temporary_controllers,
            final_controllers: vec![root],
            phase: SiblingWasmStoreAdoptionPhaseRecord::MutationInFlight,
            adopted_at_ns: None,
        };
        let prepared = RootWasmStoreState::begin_sibling_wasm_store_adoption(intent.clone())
            .expect("prepare sibling Store adoption");
        assert_eq!(prepared, intent);
        assert!(RootWasmStoreState::wasm_stores().is_empty());

        let committed = RootWasmStoreState::commit_sibling_wasm_store_adoption([1; 32], 11)
            .expect("commit sibling Store adoption");
        assert_eq!(
            committed.phase,
            SiblingWasmStoreAdoptionPhaseRecord::Verified
        );
        assert_eq!(committed.adopted_at_ns, Some(11));
        assert_eq!(RootWasmStoreState::wasm_stores()[0].pid, pid);
        RootWasmStoreState::import(RootWasmStoreStateData::default());
    }

    #[test]
    fn publication_store_binding_round_trips() {
        RootWasmStoreState::import(RootWasmStoreStateData::default());
        assert_eq!(
            RootWasmStoreState::publication_store_state().active_binding,
            None
        );
        assert_eq!(RootWasmStoreState::publication_store_state().generation, 0);

        let binding = WasmStoreBinding::new("primary");
        assert!(RootWasmStoreState::activate_publication_store_binding(
            binding.clone(),
            11
        ));
        assert_eq!(
            RootWasmStoreState::publication_store_state().active_binding,
            Some(binding)
        );
        assert_eq!(RootWasmStoreState::publication_store_state().generation, 1);
        assert_eq!(RootWasmStoreState::publication_store_state().changed_at, 11);

        assert!(RootWasmStoreState::clear_publication_store_binding(12));
        assert_eq!(
            RootWasmStoreState::publication_store_state().active_binding,
            None
        );
        assert_eq!(
            RootWasmStoreState::publication_store_state().detached_binding,
            Some(WasmStoreBinding::new("primary"))
        );
        assert_eq!(RootWasmStoreState::publication_store_state().generation, 2);
        assert_eq!(RootWasmStoreState::publication_store_state().changed_at, 12);
        assert_eq!(
            RootWasmStoreState::publication_store_state().retired_binding,
            None
        );
    }

    #[test]
    fn activate_same_binding_is_idempotent() {
        RootWasmStoreState::import(RootWasmStoreStateData::default());

        let binding = WasmStoreBinding::new("primary");
        assert!(RootWasmStoreState::activate_publication_store_binding(
            binding.clone(),
            20
        ));
        assert!(!RootWasmStoreState::activate_publication_store_binding(
            binding, 21
        ));
        assert_eq!(RootWasmStoreState::publication_store_state().generation, 1);
        assert_eq!(RootWasmStoreState::publication_store_state().changed_at, 20);
    }

    #[test]
    fn retiring_detached_binding_moves_it_to_retired() {
        RootWasmStoreState::import(RootWasmStoreStateData::default());

        assert!(RootWasmStoreState::activate_publication_store_binding(
            WasmStoreBinding::new("primary"),
            30,
        ));
        assert!(RootWasmStoreState::activate_publication_store_binding(
            WasmStoreBinding::new("secondary"),
            31,
        ));

        let retired = RootWasmStoreState::retire_detached_publication_store_binding(32);
        assert_eq!(retired, Some(WasmStoreBinding::new("primary")));
        assert_eq!(
            RootWasmStoreState::publication_store_state().detached_binding,
            None
        );
        assert_eq!(
            RootWasmStoreState::publication_store_state().retired_binding,
            Some(WasmStoreBinding::new("primary"))
        );
        assert_eq!(RootWasmStoreState::publication_store_state().retired_at, 32);
        assert_eq!(RootWasmStoreState::publication_store_state().generation, 3);
    }

    #[test]
    fn finalizing_retired_binding_clears_it() {
        RootWasmStoreState::import(RootWasmStoreStateData::default());

        assert!(RootWasmStoreState::activate_publication_store_binding(
            WasmStoreBinding::new("primary"),
            40,
        ));
        assert!(RootWasmStoreState::activate_publication_store_binding(
            WasmStoreBinding::new("secondary"),
            41,
        ));
        let retired = RootWasmStoreState::retire_detached_publication_store_binding(42);
        assert_eq!(retired, Some(WasmStoreBinding::new("primary")));

        let finalized = RootWasmStoreState::finalize_retired_publication_store_binding(43);
        assert_eq!(finalized, Some(WasmStoreBinding::new("primary")));
        assert_eq!(
            RootWasmStoreState::publication_store_state().retired_binding,
            None
        );
        assert_eq!(RootWasmStoreState::publication_store_state().retired_at, 0);
        assert_eq!(RootWasmStoreState::publication_store_state().generation, 4);
        assert_eq!(RootWasmStoreState::publication_store_state().changed_at, 43);
    }

    #[test]
    #[should_panic(expected = "publication store active/detached bindings must differ")]
    fn import_rejects_duplicate_publication_slots() {
        let binding = WasmStoreBinding::new("duplicate");

        RootWasmStoreState::import(RootWasmStoreStateData {
            record: RootWasmStoreStateRecord {
                publication_store: PublicationStoreStateRecord {
                    active_binding: Some(binding.clone()),
                    detached_binding: Some(binding),
                    retired_binding: None,
                    generation: 1,
                    changed_at: 10,
                    retired_at: 0,
                },
                wasm_stores: Vec::new(),
                sibling_wasm_store_adoption: None,
                root_store_bootstrap: None,
            },
        });
    }

    #[test]
    #[should_panic(
        expected = "publication store retired_at must be set iff retired_binding is present"
    )]
    fn import_rejects_incoherent_retired_timestamp() {
        RootWasmStoreState::import(RootWasmStoreStateData {
            record: RootWasmStoreStateRecord {
                publication_store: PublicationStoreStateRecord {
                    active_binding: None,
                    detached_binding: None,
                    retired_binding: Some(WasmStoreBinding::new("retired")),
                    generation: 1,
                    changed_at: 10,
                    retired_at: 0,
                },
                wasm_stores: Vec::new(),
                sibling_wasm_store_adoption: None,
                root_store_bootstrap: None,
            },
        });
    }
}
