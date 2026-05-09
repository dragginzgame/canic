use crate::ids::{WasmStoreBinding, WasmStoreGcMode};
use canic_cdk::{
    structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    types::Principal,
};
use canic_memory::{eager_static, ic_memory, impl_storable_bounded};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const SUBNET_STATE_ID: u8 = 60;

eager_static! {
    //
    // SUBNET_STATE
    // EMPTY FOR NOW - if we ever want to store subnet-specific state it's here
    //
    static SUBNET_STATE: RefCell<Cell<SubnetStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(SubnetState, SUBNET_STATE_ID),
            SubnetStateRecord::default(),
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

///
/// SubnetStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetStateRecord {
    pub publication_store: PublicationStoreStateRecord,
    pub wasm_stores: Vec<WasmStoreRecord>,
}

impl_storable_bounded!(SubnetStateRecord, 16_384, true);

enum PublicationStoreTransition {
    Activate(WasmStoreBinding),
    ClearActive,
    RetireDetached,
    FinalizeRetired,
}

struct PublicationStoreTransitionOutcome {
    changed: bool,
    binding: Option<WasmStoreBinding>,
}

///
/// SubnetState
///

pub struct SubnetState;

impl SubnetState {
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
        SUBNET_STATE.with_borrow_mut(|cell| {
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
    pub(crate) fn publication_store_binding() -> Option<WasmStoreBinding> {
        Self::export().publication_store.active_binding
    }

    #[must_use]
    pub(crate) fn publication_store_state() -> PublicationStoreStateRecord {
        Self::export().publication_store
    }

    #[must_use]
    pub(crate) fn wasm_stores() -> Vec<WasmStoreRecord> {
        Self::export().wasm_stores
    }

    #[must_use]
    pub(crate) fn wasm_store_pid(binding: &WasmStoreBinding) -> Option<Principal> {
        Self::export()
            .wasm_stores
            .into_iter()
            .find(|record| &record.binding == binding)
            .map(|record| record.pid)
    }

    #[must_use]
    pub(crate) fn wasm_store_binding_for_pid(pid: Principal) -> Option<WasmStoreBinding> {
        Self::export()
            .wasm_stores
            .into_iter()
            .find(|record| record.pid == pid)
            .map(|record| record.binding)
    }

    pub(crate) fn upsert_wasm_store(
        binding: WasmStoreBinding,
        pid: Principal,
        created_at: u64,
    ) -> bool {
        SUBNET_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();

            if let Some(existing) = data
                .wasm_stores
                .iter()
                .find(|record| record.binding == binding || record.pid == pid)
            {
                if existing.binding == binding && existing.pid == pid {
                    return false;
                }

                panic!("wasm store inventory conflict for binding '{binding}' / pid {pid}");
            }

            data.wasm_stores.push(WasmStoreRecord {
                binding,
                pid,
                created_at,
                gc: WasmStoreGcRecord::default(),
            });
            data.wasm_stores
                .sort_by(|left, right| left.binding.cmp(&right.binding));
            cell.set(data);
            true
        })
    }

    pub(crate) fn transition_wasm_store_gc(
        binding: &WasmStoreBinding,
        next: WasmStoreGcMode,
        changed_at: u64,
    ) -> bool {
        SUBNET_STATE.with_borrow_mut(|cell| {
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
                WasmStoreGcMode::Complete => {
                    record.gc.completed_at = Some(changed_at);
                    record.gc.runs_completed = record.gc.runs_completed.saturating_add(1);
                }
            }

            cell.set(data);
            true
        })
    }

    pub(crate) fn remove_wasm_store(binding: &WasmStoreBinding) -> Option<WasmStoreRecord> {
        SUBNET_STATE.with_borrow_mut(|cell| {
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
    pub(crate) fn import(data: SubnetStateRecord) {
        Self::validate_publication_store_state(&data.publication_store);
        let mut seen_bindings = std::collections::BTreeSet::new();
        let mut seen_pids = std::collections::BTreeSet::new();
        for record in &data.wasm_stores {
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
        SUBNET_STATE.with_borrow_mut(|cell| cell.set(data));
    }

    #[must_use]
    pub(crate) fn export() -> SubnetStateRecord {
        SUBNET_STATE.with_borrow(|cell| cell.get().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_store_binding_round_trips() {
        SubnetState::import(SubnetStateRecord::default());
        assert_eq!(SubnetState::publication_store_binding(), None);
        assert_eq!(SubnetState::publication_store_state().generation, 0);

        let binding = WasmStoreBinding::new("primary");
        assert!(SubnetState::activate_publication_store_binding(
            binding.clone(),
            11
        ));
        assert_eq!(SubnetState::publication_store_binding(), Some(binding));
        assert_eq!(SubnetState::publication_store_state().generation, 1);
        assert_eq!(SubnetState::publication_store_state().changed_at, 11);

        assert!(SubnetState::clear_publication_store_binding(12));
        assert_eq!(SubnetState::publication_store_binding(), None);
        assert_eq!(
            SubnetState::publication_store_state().detached_binding,
            Some(WasmStoreBinding::new("primary"))
        );
        assert_eq!(SubnetState::publication_store_state().generation, 2);
        assert_eq!(SubnetState::publication_store_state().changed_at, 12);
        assert_eq!(SubnetState::publication_store_state().retired_binding, None);
    }

    #[test]
    fn activate_same_binding_is_idempotent() {
        SubnetState::import(SubnetStateRecord::default());

        let binding = WasmStoreBinding::new("primary");
        assert!(SubnetState::activate_publication_store_binding(
            binding.clone(),
            20
        ));
        assert!(!SubnetState::activate_publication_store_binding(
            binding, 21
        ));
        assert_eq!(SubnetState::publication_store_state().generation, 1);
        assert_eq!(SubnetState::publication_store_state().changed_at, 20);
    }

    #[test]
    fn retiring_detached_binding_moves_it_to_retired() {
        SubnetState::import(SubnetStateRecord::default());

        assert!(SubnetState::activate_publication_store_binding(
            WasmStoreBinding::new("primary"),
            30,
        ));
        assert!(SubnetState::activate_publication_store_binding(
            WasmStoreBinding::new("secondary"),
            31,
        ));

        let retired = SubnetState::retire_detached_publication_store_binding(32);
        assert_eq!(retired, Some(WasmStoreBinding::new("primary")));
        assert_eq!(
            SubnetState::publication_store_state().detached_binding,
            None
        );
        assert_eq!(
            SubnetState::publication_store_state().retired_binding,
            Some(WasmStoreBinding::new("primary"))
        );
        assert_eq!(SubnetState::publication_store_state().retired_at, 32);
        assert_eq!(SubnetState::publication_store_state().generation, 3);
    }

    #[test]
    fn finalizing_retired_binding_clears_it() {
        SubnetState::import(SubnetStateRecord::default());

        assert!(SubnetState::activate_publication_store_binding(
            WasmStoreBinding::new("primary"),
            40,
        ));
        assert!(SubnetState::activate_publication_store_binding(
            WasmStoreBinding::new("secondary"),
            41,
        ));
        let retired = SubnetState::retire_detached_publication_store_binding(42);
        assert_eq!(retired, Some(WasmStoreBinding::new("primary")));

        let finalized = SubnetState::finalize_retired_publication_store_binding(43);
        assert_eq!(finalized, Some(WasmStoreBinding::new("primary")));
        assert_eq!(SubnetState::publication_store_state().retired_binding, None);
        assert_eq!(SubnetState::publication_store_state().retired_at, 0);
        assert_eq!(SubnetState::publication_store_state().generation, 4);
        assert_eq!(SubnetState::publication_store_state().changed_at, 43);
    }

    #[test]
    #[should_panic(expected = "publication store active/detached bindings must differ")]
    fn import_rejects_duplicate_publication_slots() {
        let binding = WasmStoreBinding::new("duplicate");

        SubnetState::import(SubnetStateRecord {
            publication_store: PublicationStoreStateRecord {
                active_binding: Some(binding.clone()),
                detached_binding: Some(binding),
                retired_binding: None,
                generation: 1,
                changed_at: 10,
                retired_at: 0,
            },
            wasm_stores: Vec::new(),
        });
    }

    #[test]
    #[should_panic(
        expected = "publication store retired_at must be set iff retired_binding is present"
    )]
    fn import_rejects_incoherent_retired_timestamp() {
        SubnetState::import(SubnetStateRecord {
            publication_store: PublicationStoreStateRecord {
                active_binding: None,
                detached_binding: None,
                retired_binding: Some(WasmStoreBinding::new("retired")),
                generation: 1,
                changed_at: 10,
                retired_at: 0,
            },
            wasm_stores: Vec::new(),
        });
    }
}
