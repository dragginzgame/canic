//! Explicit host fixtures using Storage's production namespace/bootstrap boundary.

use ic_storage::{MemoryRuntime, StorageConfig, VectorMemory};

pub fn runtime(keys: &[&str]) -> MemoryRuntime<VectorMemory> {
    ic_storage::testing::fixture_runtime(
        VectorMemory::default(),
        "test",
        "test.",
        keys,
        StorageConfig::default(),
    )
    .unwrap()
}
