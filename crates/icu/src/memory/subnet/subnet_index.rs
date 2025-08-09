use crate::{
    Error,
    ic::structures::BTreeMap,
    icu_register_memory,
    memory::{MemoryError, SUBNET_INDEX_MEMORY_ID},
};
use candid::{CandidType, Principal};
use derive_more::IntoIterator;
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// SUBNET_INDEX
//

thread_local! {
    pub static SUBNET_INDEX: RefCell<BTreeMap<String, Principal>> = RefCell::new(BTreeMap::init(
        icu_register_memory!(SubnetIndexData, SUBNET_INDEX_MEMORY_ID),
    ));
}

///
/// SubnetIndexError
///

#[derive(Debug, ThisError)]
pub enum SubnetIndexError {
    #[error("canister not found: {0}")]
    CanisterNotFound(String),
}

///
/// SubnetIndex
///

pub struct SubnetIndex {}

impl SubnetIndex {
    //
    // INTERNAL ACCESSORS
    //

    pub fn with<R>(f: impl FnOnce(&BTreeMap<String, Principal>) -> R) -> R {
        SUBNET_INDEX.with(|cell| f(&cell.borrow()))
    }

    pub fn with_mut<R>(f: impl FnOnce(&mut BTreeMap<String, Principal>) -> R) -> R {
        SUBNET_INDEX.with(|cell| f(&mut cell.borrow_mut()))
    }

    //
    // METHODS
    //

    #[must_use]
    pub fn get(kind: &str) -> Option<Principal> {
        Self::with(|map| map.get(&kind.to_string()))
    }

    pub fn try_get(kind: &str) -> Result<Principal, Error> {
        if let Some(pid) = Self::get(kind) {
            Ok(pid)
        } else {
            Err(MemoryError::from(SubnetIndexError::CanisterNotFound(
                kind.to_string(),
            )))?
        }
    }

    pub fn insert(kind: &str, id: Principal) {
        Self::with_mut(|map| {
            map.insert(kind.to_string(), id);
        });
    }

    pub fn remove(kind: &str) {
        Self::with_mut(|map| {
            map.remove(&kind.to_string());
        });
    }

    //
    // IMPORT & EXPORT
    //

    pub fn import(data: SubnetIndexData) {
        Self::with_mut(|map| {
            map.clear();
            for (k, v) in data.into_iter() {
                map.insert(k.clone(), v);
            }
        });
    }

    #[must_use]
    pub fn export() -> SubnetIndexData {
        Self::with(|map| SubnetIndexData(map.iter_pairs().collect()))
    }
}

///
/// SubnetIndexData
///

#[derive(CandidType, Clone, Debug, IntoIterator, Deserialize)]
pub struct SubnetIndexData(HashMap<String, Principal>);
