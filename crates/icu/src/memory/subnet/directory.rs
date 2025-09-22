//! Canister Directory
//!
//! Purpose
//! - Provides a read-only view of installed canisters, keyed by `CanisterType`.
//! - On the root canister, this view is generated from the authoritative `CanisterRegistry`.
//! - On child canisters, a full copy is imported to allow fast local reads without cross-canister calls.
//!
//! Lifecycle
//! - Root generates a fresh view from the registry and cascades it after installs/updates.
//! - Children accept a full re-import of the directory view via the cascade endpoint.
//! - There are no partial mutations: the only write API is `import(view)` which replaces everything.
//!
//! Invariants
//! - Root directory view must equal `generate_from_registry()`.
//! - Child directory view should match the root’s generated view after cascade.
//!
//! Implementation
//! - Internally stored as a `BTreeMap<CanisterType, CanisterEntry>` in stable memory.
//! - Wrapped in a `thread_local` for safe global access.
//! - `SubnetDirectory` exposes a small, invariant-preserving API: get, import, export.
use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    icu_register_memory,
    memory::{CanisterEntry, MemoryError, SUBNET_DIRECTORY_MEMORY_ID, subnet::SubnetError},
    types::CanisterType,
};
use std::cell::RefCell;

// thread_local
thread_local! {
    static SUBNET_DIRECTORY: RefCell<BTreeMap<CanisterType, CanisterEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(icu_register_memory!(SUBNET_DIRECTORY_MEMORY_ID)));
}

///
/// SubnetDirectory
///

pub struct SubnetDirectory;

impl SubnetDirectory {
    #[must_use]
    pub fn get(ty: &CanisterType) -> Option<CanisterEntry> {
        SUBNET_DIRECTORY.with_borrow(|map| map.get(ty))
    }

    pub fn try_get(ty: &CanisterType) -> Result<CanisterEntry, Error> {
        Self::get(ty).ok_or_else(|| MemoryError::from(SubnetError::TypeNotFound(ty.clone())).into())
    }

    pub fn import(view: Vec<CanisterEntry>) {
        SUBNET_DIRECTORY.with_borrow_mut(|map| {
            map.clear();
            for entry in view {
                map.insert(entry.ty.clone(), entry);
            }
        });
    }

    #[must_use]
    pub fn export() -> Vec<CanisterEntry> {
        SUBNET_DIRECTORY.with_borrow(|map| map.iter().map(|e| e.value()).collect())
    }
}
