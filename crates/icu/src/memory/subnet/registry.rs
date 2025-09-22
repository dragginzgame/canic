//! Canister Registry (root-authoritative)
//!
//! Purpose
//! - Authoritative ledger of canisters managed by root: type, parent, lifecycle status,
//!   and optional module hash.
//! - Drives operational flows (create/install) and serves as the source for generating
//!   the directory read-model.
//!
//! Lifecycle
//! - `init_root` inserts root as Installed at startup.
//! - `create(pid, ty, parent)` records a new canister as Created immediately after allocation.
//! - `install(pid, module_hash)` flips to Installed once code is installed and records the hash.
//! - `export()` is used by root to derive the directory view.
//!
//! Invariants
//! - An Installed canister remains Installed (idempotent guard on `install`).
//! - Every PID in the registry has an associated `CanisterType`.
//!

use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    config::Config,
    icu_register_memory,
    memory::{
        CanisterEntry, CanisterStatus, MemoryError, SUBNET_REGISTRY_MEMORY_ID,
        subnet::{
            SubnetChildren, SubnetChildrenView, SubnetDirectory, SubnetDirectoryView, SubnetError,
            SubnetParents,
        },
    },
    types::CanisterType,
    utils::time::now_secs,
};
use candid::Principal;
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// SUBNET_REGISTRY
// (root-only)
//

thread_local! {
    static SUBNET_REGISTRY: RefCell<SubnetRegistryCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(SubnetRegistryCore::new(BTreeMap::init(
            icu_register_memory!(SUBNET_REGISTRY_MEMORY_ID),
        )));
}

///
/// SubnetRegistryError
///

#[derive(Debug, ThisError)]
pub enum SubnetRegistryError {
    #[error("canister already installed: {0}")]
    AlreadyInstalled(Principal),
}

///
/// SubnetRegistry
///

pub type SubnetRegistryView = Vec<(Principal, CanisterEntry)>;

pub struct SubnetRegistry;

impl SubnetRegistry {
    /// Initialize the registry with the root canister marked as Installed.
    pub fn init_root(pid: Principal) {
        let entry = CanisterEntry {
            pid,
            ty: CanisterType::ROOT,
            parent_pid: None,
            status: CanisterStatus::Installed,
            module_hash: None,
            created_at: now_secs(),
        };

        SUBNET_REGISTRY.with_borrow_mut(|core| core.insert(pid, entry));
    }

    #[must_use]
    pub fn get(pid: Principal) -> Option<CanisterEntry> {
        SUBNET_REGISTRY.with_borrow(|core| core.get(pid))
    }

    pub fn try_get(pid: Principal) -> Result<CanisterEntry, Error> {
        SUBNET_REGISTRY.with_borrow(|core| core.try_get(pid))
    }

    pub fn create(pid: Principal, ty: &CanisterType, parent_pid: Principal) {
        let entry = CanisterEntry {
            pid,
            ty: ty.clone(),
            parent_pid: Some(parent_pid),
            status: CanisterStatus::Created,
            module_hash: None,
            created_at: now_secs(),
        };

        SUBNET_REGISTRY.with_borrow_mut(|core| core.insert(pid, entry));
    }

    pub fn install(pid: Principal, module_hash: Vec<u8>) -> Result<(), Error> {
        SUBNET_REGISTRY.with_borrow_mut(|core| {
            let entry = core.try_get(pid)?; // clone for guard check
            if entry.status == CanisterStatus::Installed {
                return Err(MemoryError::from(SubnetError::from(
                    SubnetRegistryError::AlreadyInstalled(pid),
                ))
                .into());
            }

            core.update(pid, |e| {
                e.status = CanisterStatus::Installed;
                e.module_hash = Some(module_hash.clone());
            })
        })
    }

    #[must_use]
    pub fn remove(pid: &Principal) -> Option<CanisterEntry> {
        SUBNET_REGISTRY.with_borrow_mut(|core| core.remove(pid))
    }

    #[must_use]
    pub fn export() -> SubnetRegistryView {
        SUBNET_REGISTRY.with_borrow(SubnetRegistryCore::export)
    }

    ///
    /// Views
    ///

    /// Build a directory view from the registry, refresh the global cache,
    /// and return a handle for querying it.
    ///
    /// On root, this derives the view from the registry; on children, it
    /// just refreshes the cached projection.
    ///
    /// Returns a zero-sized handle; the data lives in thread-local storage.
    #[must_use]
    pub fn subnet_directory() -> SubnetDirectory {
        use std::collections::BTreeMap as StdBTreeMap;

        let mut map: StdBTreeMap<CanisterType, CanisterEntry> = StdBTreeMap::new();

        for (_, entry) in Self::export() {
            if entry.status != CanisterStatus::Installed {
                continue;
            }

            if entry.ty == CanisterType::ROOT {
                map.insert(CanisterType::ROOT, entry);
                continue;
            }

            if Config::try_get_canister(&entry.ty)
                .map(|cfg| cfg.uses_directory)
                .unwrap_or(false)
            {
                map.insert(entry.ty.clone(), entry);
            }
        }

        let view: SubnetDirectoryView = map.into_iter().collect();

        let dir = SubnetDirectory; // zero-sized handle
        dir.import(view); // refresh global cache

        dir
    }

    /// Children of a canister.
    ///
    /// Builds a view of all children for the given `pid`, refreshes the
    /// global cache, and returns a handle for querying them.
    #[must_use]
    pub fn subnet_children(pid: Principal) -> SubnetChildren {
        let view: SubnetChildrenView = Self::export()
            .into_iter()
            .filter_map(|(_, e)| (e.parent_pid == Some(pid)).then_some(e))
            .collect();

        let children = SubnetChildren;
        children.import(view);

        children
    }

    /// Parents of a canister.
    ///
    /// Walks the parent chain for the given `pid`, refreshes the
    /// global cache, and returns a handle for querying it.
    #[must_use]
    pub fn subnet_parents(pid: Principal) -> SubnetParents {
        let mut result = Vec::new();
        let mut current = Some(pid);

        while let Some(child_pid) = current {
            if let Some(entry) = Self::get(child_pid)
                && let Some(parent_pid) = entry.parent_pid
                && let Some(parent_entry) = Self::get(parent_pid)
            {
                result.push(parent_entry.clone());
                current = Some(parent_pid);

                continue;
            }
            current = None;
        }

        result.reverse();

        let parents = SubnetParents; // zero-sized handle
        parents.import(result); // refresh global cache

        parents
    }
}

///
/// SubnetRegistryCore
///

pub struct SubnetRegistryCore<M: Memory> {
    map: BTreeMap<Principal, CanisterEntry, M>,
}

impl<M: Memory> SubnetRegistryCore<M> {
    pub const fn new(map: BTreeMap<Principal, CanisterEntry, M>) -> Self {
        Self { map }
    }

    pub fn get(&self, pid: Principal) -> Option<CanisterEntry> {
        self.map.get(&pid)
    }

    pub fn try_get(&self, pid: Principal) -> Result<CanisterEntry, Error> {
        self.get(pid)
            .ok_or_else(|| MemoryError::from(SubnetError::PrincipalNotFound(pid)).into())
    }

    pub fn insert(&mut self, pid: Principal, entry: CanisterEntry) {
        self.map.insert(pid, entry);
    }

    pub fn remove(&mut self, pid: &Principal) -> Option<CanisterEntry> {
        self.map.remove(pid)
    }

    /// Generic update helper: mutate entry in place if it exists
    pub fn update<F>(&mut self, pid: Principal, f: F) -> Result<(), Error>
    where
        F: FnOnce(&mut CanisterEntry),
    {
        match self.map.get(&pid) {
            Some(mut entry) => {
                f(&mut entry);
                self.map.insert(pid, entry);
                Ok(())
            }
            None => Err(MemoryError::from(SubnetError::PrincipalNotFound(pid)).into()),
        }
    }

    pub fn export(&self) -> SubnetRegistryView {
        self.map.to_vec()
    }
}
