use canic_cdk::structures::{DefaultMemoryImpl, memory::MemoryManager};
use std::cell::RefCell;

// -----------------------------------------------------------------------------
// MEMORY_MANAGER
// -----------------------------------------------------------------------------
// Shared stable-memory manager used by all Canic consumers. Stored as a
// thread-local so stable structures can grab virtual memory slots without
// global mutable state.
// -----------------------------------------------------------------------------

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
}
