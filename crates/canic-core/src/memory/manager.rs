use canic_cdk::structures::{DefaultMemoryImpl, Memory, memory::MemoryManager};
use std::cell::RefCell;

const MEMORY_MANAGER_MAGIC: &[u8; 3] = b"MGR";

///
/// RawStableMemoryState
///
/// Classification of the underlying raw stable memory before `MemoryManager`
/// is allowed to initialize or repair its own metadata.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RawStableMemoryState {
    Empty,
    MemoryManager,
    ForeignOrCorrupt,
}

// -----------------------------------------------------------------------------
// MEMORY_MANAGER
// -----------------------------------------------------------------------------
// Shared stable-memory manager used by all Canic consumers. Stored as a
// thread-local so stable structures can grab virtual memory slots without
// global mutable state.
// -----------------------------------------------------------------------------

thread_local! {
    /// Shared stable-memory manager used by the exported memory macros.
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
}

pub fn classify_raw_stable_memory() -> RawStableMemoryState {
    classify_stable_memory(&DefaultMemoryImpl::default())
}

fn classify_stable_memory<M: Memory>(memory: &M) -> RawStableMemoryState {
    if memory.size() == 0 {
        return RawStableMemoryState::Empty;
    }

    let mut magic = [0; 3];
    memory.read(0, &mut magic);
    if &magic == MEMORY_MANAGER_MAGIC {
        RawStableMemoryState::MemoryManager
    } else {
        RawStableMemoryState::ForeignOrCorrupt
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_stable_memory_distinguishes_empty_manager_and_foreign_state() {
        let memory = DefaultMemoryImpl::default();
        assert_eq!(classify_stable_memory(&memory), RawStableMemoryState::Empty);

        memory.grow(1);
        memory.write(0, MEMORY_MANAGER_MAGIC);
        assert_eq!(
            classify_stable_memory(&memory),
            RawStableMemoryState::MemoryManager
        );

        memory.write(0, b"BAD");
        assert_eq!(
            classify_stable_memory(&memory),
            RawStableMemoryState::ForeignOrCorrupt
        );
    }
}
