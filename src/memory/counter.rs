use crate::{
    ic::structures::{Cell, DefaultMemory, cell::CellError},
    impl_storable_unbounded,
};
use candid::CandidType;
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// MemoryCounterError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum MemoryCounterError {
    #[error("no more free memories")]
    NoMoreMemories,

    #[error(transparent)]
    CellError(#[from] CellError),
}

///
/// MemoryCounter
///

#[derive(Deref, DerefMut)]
pub struct MemoryCounter(Cell<MemoryCounterData>);

impl MemoryCounter {
    // init
    #[must_use]
    pub fn init(memory: DefaultMemory) -> Self {
        let cell = Cell::init(memory, MemoryCounterData::default()).unwrap();

        Self(cell)
    }

    // next_memory_id
    pub fn next_memory_id(&mut self) -> Result<u8, MemoryCounterError> {
        let mut cur_state = self.get();

        if cur_state.last_memory_id == 255 {
            return Err(MemoryCounterError::NoMoreMemories);
        }

        cur_state.last_memory_id += 1;
        self.set(cur_state)?;

        Ok(cur_state.last_memory_id)
    }
}

#[cfg(test)]
impl MemoryCounter {
    pub fn reset(&mut self) {
        let mut state = self.get();
        state.last_memory_id = 0;
        self.set(state).expect("Failed to reset memory counter");
    }
}

///
/// MemoryCounterData
///

#[derive(CandidType, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct MemoryCounterData {
    last_memory_id: u8,
}

impl_storable_unbounded!(MemoryCounterData);

///
/// TESTS
///

#[cfg(test)]
mod test {
    use crate::memory::{MEMORY_COUNTER, counter::MemoryCounterError};
    use std::collections::HashSet;

    /// Resets the memory counter before a test.
    fn reset_counter() {
        MEMORY_COUNTER.with_borrow_mut(|counter| counter.reset());
    }

    #[test]
    fn test_first_memory_id_is_1() {
        reset_counter();

        let first_id = MEMORY_COUNTER.with_borrow_mut(|counter| counter.next_memory_id().unwrap());

        assert_eq!(first_id, 1, "Expected first memory ID to be 1");
    }

    #[test]
    fn test_memory_ids_are_unique() {
        reset_counter();

        let mut seen = HashSet::new();

        MEMORY_COUNTER.with_borrow_mut(|counter| {
            for _ in 0..20 {
                let id = counter.next_memory_id().unwrap();
                assert!(seen.insert(id), "Duplicate memory ID allocated: {id}");
            }
        });
    }

    #[test]
    fn test_last_valid_memory_id_is_255() {
        reset_counter();

        let last_id = MEMORY_COUNTER.with_borrow_mut(|counter| {
            // Drain up to 254
            while counter.get().last_memory_id < 254 {
                counter.next_memory_id().unwrap();
            }

            counter.next_memory_id().unwrap()
        });

        assert_eq!(last_id, 255, "Expected last memory ID to be 255");
    }

    #[test]
    fn test_exhausting_memory_counter_returns_error() {
        reset_counter();

        let result = MEMORY_COUNTER.with_borrow_mut(|counter| {
            // Drain up to 255
            while counter.get().last_memory_id < 255 {
                counter.next_memory_id().unwrap();
            }

            // This should now fail
            counter.next_memory_id()
        });

        assert!(
            matches!(result, Err(MemoryCounterError::NoMoreMemories)),
            "Expected NoMoreMemories error after exhausting all memory IDs"
        );
    }

    #[test]
    fn test_reset_allows_reuse_from_id_1() {
        reset_counter();

        // Allocate some IDs
        let id1 = MEMORY_COUNTER.with_borrow_mut(|counter| counter.next_memory_id().unwrap());
        let id2 = MEMORY_COUNTER.with_borrow_mut(|counter| counter.next_memory_id().unwrap());

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);

        // Reset and check if it starts over
        reset_counter();

        let reset_id = MEMORY_COUNTER.with_borrow_mut(|counter| counter.next_memory_id().unwrap());
        assert_eq!(reset_id, 1, "Expected reset to restart at memory ID 1");
    }
}
