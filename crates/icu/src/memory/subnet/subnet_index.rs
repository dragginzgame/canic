use crate::{
    Error,
    canister::{Canister, CanisterIndexable},
    ic::structures::BTreeMap,
    icu_register_memory, impl_storable_unbounded,
    memory::{MemoryError, SUBNET_INDEX_MEMORY_ID},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// SUBNET_INDEX
//

thread_local! {
    pub static SUBNET_INDEX: RefCell<BTreeMap<String, SubnetIndexEntry>> = RefCell::new(BTreeMap::init(
        icu_register_memory!(SubnetIndexData, SUBNET_INDEX_MEMORY_ID),
    ));
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
/// SubnetIndexData
///

pub type SubnetIndexData = HashMap<String, SubnetIndexEntry>;

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

pub struct SubnetIndex {}

impl SubnetIndex {
    //
    // INTERNAL ACCESSORS
    //

    pub fn with<R>(f: impl FnOnce(&BTreeMap<String, SubnetIndexEntry>) -> R) -> R {
        SUBNET_INDEX.with_borrow(|cell| f(cell))
    }

    pub fn with_mut<R>(f: impl FnOnce(&mut BTreeMap<String, SubnetIndexEntry>) -> R) -> R {
        SUBNET_INDEX.with_borrow_mut(|cell| f(cell))
    }

    //
    // METHODS
    //

    #[must_use]
    pub fn get(kind: &str) -> Option<SubnetIndexEntry> {
        Self::with(|map| map.get(&kind.to_string()))
    }

    pub fn try_get(kind: &str) -> Result<SubnetIndexEntry, Error> {
        if let Some(entry) = Self::get(kind) {
            Ok(entry)
        } else {
            Err(MemoryError::from(SubnetIndexError::NotFound(
                kind.to_string(),
            )))?
        }
    }

    pub fn try_get_singleton(kind: &str) -> Result<Principal, Error> {
        let entry = Self::try_get(kind)?;

        if entry.canisters.len() == 1 {
            Ok(entry.canisters[0])
        } else {
            Err(MemoryError::from(SubnetIndexError::NotSingleton(
                kind.to_string(),
            )))?
        }
    }

    // can_insert
    pub fn can_insert(canister: &Canister) -> Result<(), Error> {
        let kind = canister.kind.to_string();
        let attrs = &canister.attributes;
        let entry = Self::with(|map| map.get(&kind)).unwrap_or_default();

        match attrs.indexable {
            None => Err(MemoryError::from(SubnetIndexError::NotIndexable(kind))),

            Some(CanisterIndexable::Limited(cap)) if (entry.canisters.len() as u16) >= cap => {
                Err(MemoryError::from(SubnetIndexError::CapacityReached {
                    kind,
                    cap,
                }))
            }
            _ => Ok(()),
        }?;

        Ok(())
    }

    // insert
    // canister gets passed in (from canister registry), because it contains
    // subnet-specific logic
    pub fn insert(canister: &Canister, id: Principal) -> Result<(), Error> {
        Self::can_insert(canister)?;

        Self::with_mut(|map| {
            let kind = canister.kind.to_string();
            let mut entry = map.get(&kind).unwrap_or_default();

            // add if its not there
            if !entry.canisters.contains(&id) {
                entry.canisters.push(id);
                map.insert(kind, entry);
            }
        });

        Ok(())
    }

    // remove
    // Make insert/remove return Result<(), Error> even if children won’t
    // hit the policy—useful for root callers and consistent API.
    pub fn remove(kind: &str, id: Principal) -> Result<(), Error> {
        Self::with_mut(|map| {
            let key = kind.to_string();

            if let Some(mut entry) = map.get(&key) {
                entry.canisters.retain(|p| p != &id);

                if entry.canisters.is_empty() {
                    map.remove(&key);
                } else {
                    map.insert(key, entry);
                }
            }
        });

        Ok(())
    }

    //
    // IMPORT & EXPORT
    //

    pub fn import(data: SubnetIndexData) {
        Self::with_mut(|map| {
            map.clear();
            for (k, v) in data {
                map.insert(k.clone(), v);
            }
        });
    }

    #[must_use]
    pub fn export() -> SubnetIndexData {
        Self::with(|map| map.iter_pairs().collect())
    }
}
