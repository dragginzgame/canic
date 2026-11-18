//! Standalone application's backing policy. Composed actors supply their own runtime.
use crate::state::{STORAGE_AUTHORITY, STORAGE_PREFIX, State, memory_requests};
use ic_storage::{
    AllocationGrants, GenericAllocationPolicy, MemoryAuthority, MemoryBacking, MemoryRuntime,
    SealedDeclarationSnapshot, StorageConfig,
};

/// Standalone physical page ceiling, including Storage controls.
pub const MAX_PAGES: u64 = 32;

/// Open the standalone profile application's original allocation and capacity contract.
/// Composed actors contribute `state::memory_requests` to their own bootstrap and
/// call `State::create` or `State::open` with that runtime instead.
/// # Panics
/// Traps on missing/corrupt existing allocations, capacity or profile-store failure.
/// A failed reopen must never be retried as a fresh installation.
pub fn open<M: MemoryBacking>(backing: M, fresh: bool) -> State<M> {
    let mut runtime = MemoryRuntime::new_with_config(
        backing,
        StorageConfig {
            allocations: 2,
            extents: 3,
            pages: MAX_PAGES,
            ..Default::default()
        },
    )
    .expect("admit profile backing");
    if !fresh {
        assert_eq!(
            runtime
                .memory_allocations()
                .expect("retained allocation integrity")
                .len(),
            2,
            "missing retained profile allocations"
        );
    }
    runtime
        .bootstrap(
            &SealedDeclarationSnapshot::new(memory_requests().to_vec())
                .expect("profile declarations"),
            &AllocationGrants::new(vec![
                MemoryAuthority::new(STORAGE_AUTHORITY, STORAGE_PREFIX).expect("profile authority"),
            ])
            .expect("profile grants"),
            &GenericAllocationPolicy,
        )
        .expect("commit profile admission");
    if fresh {
        State::create(&runtime)
    } else {
        State::open(&runtime)
    }
}
