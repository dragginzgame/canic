//! Explicit governed runtime setup and pinned unmanaged backing images for tests.

use ic_memory::{MemoryManagerConfig, MemoryRuntime, VectorMemory};

pub(crate) fn runtime(keys: &[&str], bucket_pages: u16) -> MemoryRuntime<VectorMemory> {
    ic_memory::testing::fixture_runtime(
        VectorMemory::default(),
        "test",
        "test.",
        keys,
        MemoryManagerConfig::new(bucket_pages).unwrap(),
    )
    .unwrap()
}

pub(crate) fn unmanaged(bucket_pages: u16) -> VectorMemory {
    let (pages, runs): (u64, &[(u64, String)]) = match bucket_pages {
        1 => include!("unmanaged-one.inc"),
        16 => include!("unmanaged-sixteen.inc"),
        _ => panic!("no unmanaged fixture for this geometry"),
    };
    ic_memory::testing::memory_from_sparse_hex(pages, runs, 17).unwrap()
}

pub(crate) fn all_slots() -> VectorMemory {
    let (pages, runs): (u64, &[(u64, String)]) = include!("all-slots.inc");
    ic_memory::testing::memory_from_sparse_hex(pages, runs, 255).unwrap()
}
