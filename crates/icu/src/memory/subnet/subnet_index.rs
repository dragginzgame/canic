use crate::{
    Error,
    canister::Canister,
    ic::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_unbounded,
    memory::{MemoryError, SUBNET_INDEX_MEMORY_ID},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// SUBNET_INDEX
//

thread_local! {
    pub static SUBNET_INDEX: RefCell<SubnetIndexCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(SubnetIndexCore::new(BTreeMap::init(
            icu_register_memory!(SUBNET_INDEX_MEMORY_ID),
        )));
}

///
/// SubnetIndexError
///

#[derive(Debug, ThisError)]
pub enum SubnetIndexError {
    #[error("canister not found: {0}")]
    NotFound(String),

    #[error("canister kind '{0}' is not a singleton")]
    NotSingleton(String),

    #[error("canister kind '{0}' is not indexable")]
    NotIndexable(String),

    #[error("subnet index capacity reached for kind '{kind}' (cap {cap})")]
    CapacityReached { kind: String, cap: u16 },
}

///
/// SubnetIndexEntry
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct SubnetIndexEntry {
    pub canisters: Vec<Principal>,
}

impl_storable_unbounded!(SubnetIndexEntry);

///
/// SubnetIndex
///

pub type SubnetIndexView = Vec<(String, SubnetIndexEntry)>;

pub struct SubnetIndex;

impl SubnetIndex {
    #[must_use]
    pub fn get(kind: &str) -> Option<SubnetIndexEntry> {
        SUBNET_INDEX.with_borrow(|core| core.get(kind))
    }

    pub fn try_get(kind: &str) -> Result<SubnetIndexEntry, Error> {
        SUBNET_INDEX.with_borrow(|core| core.try_get(kind))
    }

    pub fn try_get_singleton(kind: &str) -> Result<Principal, Error> {
        SUBNET_INDEX.with_borrow(|core| core.try_get_singleton(kind))
    }

    pub fn can_insert(canister: &Canister) -> Result<(), Error> {
        SUBNET_INDEX.with_borrow(|core| core.can_insert(canister))
    }

    pub fn insert(canister: &Canister, id: Principal) -> Result<(), Error> {
        SUBNET_INDEX.with_borrow_mut(|core| core.insert(canister, id))
    }

    pub fn remove(kind: &str, id: Principal) -> Result<(), Error> {
        SUBNET_INDEX.with_borrow_mut(|core| core.remove(kind, id))
    }

    pub fn import(view: SubnetIndexView) {
        SUBNET_INDEX.with_borrow_mut(|core| core.import(view));
    }

    #[must_use]
    pub fn export() -> SubnetIndexView {
        SUBNET_INDEX.with_borrow(SubnetIndexCore::export)
    }
}

///
/// SubnetIndexCore
///

pub struct SubnetIndexCore<M: Memory> {
    map: BTreeMap<String, SubnetIndexEntry, M>,
}

impl<M: Memory> SubnetIndexCore<M> {
    pub const fn new(map: BTreeMap<String, SubnetIndexEntry, M>) -> Self {
        Self { map }
    }

    pub fn get(&self, kind: &str) -> Option<SubnetIndexEntry> {
        self.map.get(&kind.to_string())
    }

    pub fn try_get(&self, kind: &str) -> Result<SubnetIndexEntry, Error> {
        if let Some(entry) = self.get(kind) {
            Ok(entry)
        } else {
            Err(MemoryError::from(SubnetIndexError::NotFound(
                kind.to_string(),
            )))?
        }
    }

    pub fn try_get_singleton(&self, kind: &str) -> Result<Principal, Error> {
        let entry = self.try_get(kind)?;

        if entry.canisters.len() == 1 {
            Ok(entry.canisters[0])
        } else {
            Err(MemoryError::from(SubnetIndexError::NotSingleton(
                kind.to_string(),
            )))?
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn can_insert(&self, canister: &Canister) -> Result<(), Error> {
        let kind = canister.kind.to_string();
        let attrs = &canister.attributes;
        let entry = self.get(&kind).unwrap_or_default();

        match attrs.indexing.limit() {
            None => Err(MemoryError::from(SubnetIndexError::NotIndexable(kind))),
            Some(cap) if (entry.canisters.len() as u16) >= cap => {
                Err(MemoryError::from(SubnetIndexError::CapacityReached {
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

    pub fn import(&mut self, view: SubnetIndexView) {
        self.map.clear();
        for (k, v) in view {
            self.map.insert(k.clone(), v);
        }
    }

    pub fn export(&self) -> SubnetIndexView {
        self.map.iter_pairs().collect()
    }
}
