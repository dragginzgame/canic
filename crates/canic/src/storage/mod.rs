//! Named allocations for Canic actors with a fixed storage composition.

#[cfg(test)]
mod tests;

use ic_storage::{
    AllocationGrants, GenericAllocationPolicy, MemoryAuthority, MemoryBacking, MemoryRequest,
    MemoryRuntime, RuntimeMemory, SealedDeclarationSnapshot, StableKey, StorageConfig,
    adapter::StableCell,
};

/// Admit the exact set of stable names selected by the actor lifecycle.
/// # Panics
/// Traps on a fresh/open mismatch, invalid declarations, or incompatible backing.
pub fn bootstrap<M: MemoryBacking>(
    memory: M,
    fresh: bool,
    authority: &str,
    keys: &[&str],
    pages: u64,
) -> MemoryRuntime<M> {
    bootstrap_with_prefix(
        memory,
        fresh,
        authority,
        &format!("{authority}."),
        keys,
        StorageConfig {
            allocations: keys.len(),
            pages,
            ..Default::default()
        },
    )
}

/// Admit a single owner's complete key set under an independently named prefix.
/// Supply the original Storage configuration on reopen, including page, extent,
/// allocation and directory budgets. The directory must contain exactly these
/// keys. Layouts shared with other owners need their original composed bootstrap.
///
/// # Panics
/// Traps on lifecycle mismatch, missing or extra allocations, invalid grants or
/// incompatible backing. A failed reopen must never be retried as fresh.
pub fn bootstrap_with_prefix<M: MemoryBacking>(
    memory: M,
    fresh: bool,
    authority: &str,
    prefix: &str,
    keys: &[&str],
    config: StorageConfig,
) -> MemoryRuntime<M> {
    assert_eq!(memory.size() == 0, fresh, "storage lifecycle mismatch");
    let mut runtime =
        MemoryRuntime::new_with_config(memory, config).expect("open storage directory");
    if !fresh {
        assert_eq!(
            runtime.memory_allocations().expect("directory").len(),
            keys.len(),
            "storage membership"
        );
        for key in keys {
            assert!(
                runtime
                    .allocation_id(&StableKey::parse(key).expect("stable key"))
                    .is_some(),
                "missing retained allocation"
            );
        }
    }
    let requests = keys
        .iter()
        .map(|key| MemoryRequest::new(authority, key).expect("request"))
        .collect();
    runtime
        .bootstrap(
            &SealedDeclarationSnapshot::new(requests).expect("declarations"),
            &AllocationGrants::new(vec![
                MemoryAuthority::new(authority, prefix).expect("authority"),
            ])
            .expect("grants"),
            &GenericAllocationPolicy,
        )
        .expect("storage admission");
    runtime
}

/// Create or reopen a bounded application payload in an admitted cell.
/// # Panics
/// Traps on invalid existing bytes, absent authority, or oversized payloads.
#[must_use]
pub fn cell<M: MemoryBacking>(
    runtime: &MemoryRuntime<M>,
    key: &str,
    maximum: usize,
    initial: Option<Vec<u8>>,
) -> StableCell<Vec<u8>, M> {
    let lease: RuntimeMemory<M> = runtime
        .open_memory(&StableKey::parse(key).expect("cell key"))
        .expect("cell lease");
    if let Some(bytes) = initial {
        assert!(bytes.len() <= maximum);
        StableCell::create(lease, bytes)
    } else {
        ic_storage::stable_cell_payload_length(&lease, maximum).expect("bounded cell envelope");
        StableCell::load(lease).expect("load retained cell")
    }
}
