use crate::{
    Error,
    ic::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_unbounded,
    memory::{MemoryError, SUBNET_DIRECTORY_MEMORY_ID},
    state::canister::Canister,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// SUBNET_DIRECTORY
//

thread_local! {
    pub static SUBNET_DIRECTORY: RefCell<SubnetDirectoryCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(SubnetDirectoryCore::new(BTreeMap::init(
            icu_register_memory!(SUBNET_DIRECTORY_MEMORY_ID),
        )));
}

///
/// SubnetDirectoryError
///

#[derive(Debug, ThisError)]
pub enum SubnetDirectoryError {
    #[error("canister not found: {0}")]
    NotFound(String),

    #[error("canister kind '{0}' is not a singleton")]
    NotSingleton(String),

    #[error("canister kind '{0}' is not indexable")]
    NotIndexable(String),

    #[error("subnet directory capacity reached for kind '{kind}' (cap {cap})")]
    CapacityReached { kind: String, cap: u16 },
}

///
/// SubnetDirectoryEntry
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct SubnetDirectoryEntry {
    pub canisters: Vec<Principal>,
}

impl_storable_unbounded!(SubnetDirectoryEntry);

///
/// SubnetDirectory
///

pub type SubnetDirectoryView = Vec<(String, SubnetDirectoryEntry)>;

pub struct SubnetDirectory;

impl SubnetDirectory {
    #[must_use]
    pub fn get(kind: &str) -> Option<SubnetDirectoryEntry> {
        SUBNET_DIRECTORY.with_borrow(|core| core.get(kind))
    }

    pub fn try_get(kind: &str) -> Result<SubnetDirectoryEntry, Error> {
        SUBNET_DIRECTORY.with_borrow(|core| core.try_get(kind))
    }

    pub fn try_get_singleton(kind: &str) -> Result<Principal, Error> {
        SUBNET_DIRECTORY.with_borrow(|core| core.try_get_singleton(kind))
    }

    pub fn can_insert(canister: &Canister) -> Result<(), Error> {
        SUBNET_DIRECTORY.with_borrow(|core| core.can_insert(canister))
    }

    pub fn insert(canister: &Canister, id: Principal) -> Result<(), Error> {
        SUBNET_DIRECTORY.with_borrow_mut(|core| core.insert(canister, id))
    }

    pub fn remove(kind: &str, id: Principal) -> Result<(), Error> {
        SUBNET_DIRECTORY.with_borrow_mut(|core| core.remove(kind, id))
    }

    pub fn import(view: SubnetDirectoryView) {
        SUBNET_DIRECTORY.with_borrow_mut(|core| core.import(view));
    }

    #[must_use]
    pub fn export() -> SubnetDirectoryView {
        SUBNET_DIRECTORY.with_borrow(SubnetDirectoryCore::export)
    }
}

///
/// SubnetDirectoryCore
///

pub struct SubnetDirectoryCore<M: Memory> {
    map: BTreeMap<String, SubnetDirectoryEntry, M>,
}

impl<M: Memory> SubnetDirectoryCore<M> {
    pub const fn new(map: BTreeMap<String, SubnetDirectoryEntry, M>) -> Self {
        Self { map }
    }

    pub fn get(&self, kind: &str) -> Option<SubnetDirectoryEntry> {
        self.map.get(&kind.to_string())
    }

    pub fn try_get(&self, kind: &str) -> Result<SubnetDirectoryEntry, Error> {
        if let Some(entry) = self.get(kind) {
            Ok(entry)
        } else {
            Err(MemoryError::from(SubnetDirectoryError::NotFound(
                kind.to_string(),
            )))?
        }
    }

    pub fn try_get_singleton(&self, kind: &str) -> Result<Principal, Error> {
        let entry = self.try_get(kind)?;

        if entry.canisters.len() == 1 {
            Ok(entry.canisters[0])
        } else {
            Err(MemoryError::from(SubnetDirectoryError::NotSingleton(
                kind.to_string(),
            )))?
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn can_insert(&self, canister: &Canister) -> Result<(), Error> {
        let kind = canister.kind.to_string();
        let attrs = &canister.attributes;
        let entry = self.get(&kind).unwrap_or_default();

        match attrs.directory.limit() {
            None => Err(MemoryError::from(SubnetDirectoryError::NotIndexable(kind))),
            Some(cap) if (entry.canisters.len() as u16) >= cap => {
                Err(MemoryError::from(SubnetDirectoryError::CapacityReached {
                    kind,
                    cap,
                }))
            }
            _ => Ok(()),
        }?; // propagate

        Ok(())
    }

    pub fn insert(&mut self, canister: &Canister, id: Principal) -> Result<(), Error> {
        self.can_insert(canister)?;

        let kind = canister.kind.to_string();
        let mut entry = self.get(&kind).unwrap_or_default();

        if !entry.canisters.contains(&id) {
            entry.canisters.push(id);
            self.map.insert(kind, entry);
        }

        Ok(())
    }

    pub fn remove(&mut self, kind: &str, id: Principal) -> Result<(), Error> {
        let key = kind.to_string();

        if let Some(mut entry) = self.get(&key) {
            entry.canisters.retain(|p| p != &id);

            if entry.canisters.is_empty() {
                self.map.remove(&key);
            } else {
                self.map.insert(key, entry);
            }
        }

        Ok(())
    }

    pub fn import(&mut self, view: SubnetDirectoryView) {
        self.map.clear();
        for (k, v) in view {
            self.map.insert(k.clone(), v);
        }
    }

    pub fn export(&self) -> SubnetDirectoryView {
        self.map.iter_pairs().collect()
    }
}
