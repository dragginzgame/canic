use crate::ids::WasmStoreBinding;
use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    storage::{prelude::*, stable::memory::state::SUBNET_STATE_ID},
};
use std::cell::RefCell;

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
/// SubnetStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetStateRecord {
    pub publication_store: PublicationStoreStateRecord,
}

impl_storable_bounded!(SubnetStateRecord, 512, true);

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
    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn publication_store_binding() -> Option<WasmStoreBinding> {
        Self::export().publication_store.active_binding
    }

    #[must_use]
    pub(crate) fn publication_store_state() -> PublicationStoreStateRecord {
        Self::export().publication_store
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

    #[allow(clippy::missing_const_for_fn)]
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

    pub(crate) fn import(data: SubnetStateRecord) {
        Self::validate_publication_store_state(&data.publication_store);
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
        });
    }
}
