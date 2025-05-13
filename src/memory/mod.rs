pub mod counter;

pub use counter::MemoryCounter;

use crate::{
    Log,
    ic::structures::{DefaultMemory, memory::MemoryId},
    icu_memory_manager, log,
};

//
// MEMORY_MANAGER
//

icu_memory_manager!();

// allocate_state
pub fn allocate_state<T>(init_fn: impl FnOnce(DefaultMemory) -> T) -> T {
    let memory_id = MEMORY_COUNTER
        .with_borrow_mut(|this| this.next_memory_id())
        .unwrap();

    let memory = MEMORY_MANAGER.with_borrow(|mgr| mgr.get(MemoryId::new(memory_id)));

    log!(Log::Info, "allocating memory {memory_id}");

    init_fn(memory)
}
