use crate::{
    Error,
    ic::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_unbounded,
    memory::{CANISTER_DIRECTORY_MEMORY_ID, MemoryError},
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

pub type CanisterDirectoryView = Vec<(CanisterType, CanisterDirectoryEntry)>;

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

    pub fn insert(ty: CanisterType, id: Principal) -> Result<(), Error> {
        CANISTER_DIRECTORY.with_borrow_mut(|core| core.insert(ty, id))
    }

    pub fn remove(ty: &CanisterType, id: Principal) -> Result<(), Error> {
        CANISTER_DIRECTORY.with_borrow_mut(|core| core.remove(ty, id))
    }

    pub fn import(view: CanisterDirectoryView) {
        CANISTER_DIRECTORY.with_borrow_mut(|core| core.import(view));
    }

    #[must_use]
    pub fn export() -> CanisterDirectoryView {
        CANISTER_DIRECTORY.with_borrow(CanisterDirectoryCore::export)
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

    pub fn import(&mut self, view: CanisterDirectoryView) {
        self.map.clear();
        for (k, v) in view {
            self.map.insert(k.clone(), v);
        }
    }

    pub fn export(&self) -> CanisterDirectoryView {
        self.map.iter_pairs().collect()
    }
}
