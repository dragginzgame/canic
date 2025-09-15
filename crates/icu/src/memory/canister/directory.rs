//! Canister Directory
//!
//! Purpose
//! - Directory is a read-model of installed canisters grouped by `CanisterType`.
//! - On root, the directory is not the source of truth and is generated from the
//!   `CanisterRegistry` on demand.
//! - On children, a local copy is stored to enable fast reads without cross-canister calls.
//!
//! Lifecycle
//! - Root generates a fresh view from the registry and cascades it after installs/updates.
//! - Children accept a full re-import of the directory view via the cascade endpoint.
//! - There are no partial mutations: the only write API is `import(view)`.
//!
//! Invariants
//! - Root directory view must equal `generate_from_registry()`.
//! - Child directory view should align with root’s generated view after cascade.
//!
use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    config::Config,
    icu_register_memory, impl_storable_unbounded,
    memory::{CANISTER_DIRECTORY_MEMORY_ID, CanisterRegistry, MemoryError},
    types::CanisterType,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// CANISTER_DIRECTORY
//

thread_local! {
    pub static CANISTER_DIRECTORY: RefCell<CanisterDirectoryCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(CanisterDirectoryCore::new(BTreeMap::init(
            icu_register_memory!(CANISTER_DIRECTORY_MEMORY_ID),
        )));
}

///
/// CanisterDirectoryError
///

#[derive(Debug, ThisError)]
pub enum CanisterDirectoryError {
    #[error("canister not found: {0}")]
    NotFound(CanisterType),

    #[error("canister type '{0}' is not a singleton")]
    NotSingleton(CanisterType),
}

///
/// CanisterDirectoryEntry
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct CanisterDirectoryEntry {
    pub canisters: Vec<Principal>,
}

impl_storable_unbounded!(CanisterDirectoryEntry);

///
/// CanisterDirectory
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct CanisterDirectoryView {
    pub entries: Vec<(CanisterType, CanisterDirectoryEntry)>,
}

pub struct CanisterDirectory;

impl CanisterDirectory {
    #[must_use]
    pub fn get(ty: &CanisterType) -> Option<CanisterDirectoryEntry> {
        CANISTER_DIRECTORY.with_borrow(|core| core.get(ty))
    }

    pub fn try_get(ty: &CanisterType) -> Result<CanisterDirectoryEntry, Error> {
        CANISTER_DIRECTORY.with_borrow(|core| core.try_get(ty))
    }

    pub fn try_get_singleton(ty: &CanisterType) -> Result<Principal, Error> {
        CANISTER_DIRECTORY.with_borrow(|core| core.try_get_singleton(ty))
    }

    #[must_use]
    pub fn export() -> CanisterDirectoryView {
        CanisterDirectoryView {
            entries: CANISTER_DIRECTORY.with_borrow(CanisterDirectoryCore::export),
        }
    }

    pub fn import(view: CanisterDirectoryView) {
        CANISTER_DIRECTORY.with_borrow_mut(|core| core.import(view.entries));
    }

    /// Generate the directory view from the CanisterRegistry (root authoritative)
    /// without relying on the stored directory state.
    #[must_use]
    pub fn generate_from_registry() -> CanisterDirectoryView {
        use std::collections::BTreeMap as StdBTreeMap;

        let mut map: StdBTreeMap<CanisterType, Vec<Principal>> = StdBTreeMap::new();
        for (pid, entry) in CanisterRegistry::export() {
            if entry.status != super::registry::CanisterStatus::Installed {
                continue;
            }

            if let Ok(canister_cfg) = Config::try_get_canister(&entry.canister_type)
                && canister_cfg.uses_directory
            {
                map.entry(entry.canister_type.clone())
                    .or_default()
                    .push(pid);
            }
        }

        CanisterDirectoryView {
            entries: map
                .into_iter()
                .map(|(k, v)| (k, CanisterDirectoryEntry { canisters: v }))
                .collect(),
        }
    }

    /// Current view: on root, generate from registry; on children, export local copy.
    #[must_use]
    pub fn current_view() -> CanisterDirectoryView {
        if crate::memory::CanisterState::is_root() {
            Self::generate_from_registry()
        } else {
            Self::export()
        }
    }
}

///
/// CanisterDirectoryCore
///

pub struct CanisterDirectoryCore<M: Memory> {
    map: BTreeMap<CanisterType, CanisterDirectoryEntry, M>,
}

impl<M: Memory> CanisterDirectoryCore<M> {
    pub const fn new(map: BTreeMap<CanisterType, CanisterDirectoryEntry, M>) -> Self {
        Self { map }
    }

    pub fn get(&self, ty: &CanisterType) -> Option<CanisterDirectoryEntry> {
        self.map.get(ty)
    }

    pub fn try_get(&self, ty: &CanisterType) -> Result<CanisterDirectoryEntry, Error> {
        if let Some(entry) = self.get(ty) {
            Ok(entry)
        } else {
            Err(MemoryError::from(CanisterDirectoryError::NotFound(
                ty.clone(),
            )))?
        }
    }

    pub fn try_get_singleton(&self, ty: &CanisterType) -> Result<Principal, Error> {
        let entry = self.try_get(ty)?;

        if entry.canisters.len() == 1 {
            Ok(entry.canisters[0])
        } else {
            Err(MemoryError::from(CanisterDirectoryError::NotSingleton(
                ty.clone(),
            )))?
        }
    }

    pub fn insert(&mut self, ty: CanisterType, id: Principal) -> Result<(), Error> {
        let mut entry = self.get(&ty).unwrap_or_default();

        if !entry.canisters.contains(&id) {
            entry.canisters.push(id);
            self.map.insert(ty, entry);
        }

        Ok(())
    }

    pub fn remove(&mut self, ty: &CanisterType, id: Principal) -> Result<(), Error> {
        if let Some(mut entry) = self.get(ty) {
            entry.canisters.retain(|p| p != &id);

            if entry.canisters.is_empty() {
                self.map.remove(ty);
            } else {
                self.map.insert(ty.clone(), entry);
            }
        }

        Ok(())
    }

    pub fn import(&mut self, entries: Vec<(CanisterType, CanisterDirectoryEntry)>) {
        self.map.clear();
        for (k, v) in entries {
            self.map.insert(k, v);
        }
    }

    pub fn export(&self) -> Vec<(CanisterType, CanisterDirectoryEntry)> {
        self.map.to_vec()
    }
}
