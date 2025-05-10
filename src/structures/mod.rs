pub mod cell;

// helper
pub type DefaultMemory =
    ic_stable_structures::memory_manager::VirtualMemory<ic_stable_structures::DefaultMemoryImpl>;
