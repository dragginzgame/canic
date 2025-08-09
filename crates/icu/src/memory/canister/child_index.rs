use crate::{
    Error,
    ic::structures::BTreeMap,
    icu_register_memory,
    memory::{CHILD_INDEX_MEMORY_ID, MemoryError},
};
use candid::Principal;
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// CHILD_INDEX
//

thread_local! {
    pub static CHILD_INDEX: RefCell<BTreeMap<Principal, String>> = RefCell::new(BTreeMap::init(
        icu_register_memory!(ChildIndexData, CHILD_INDEX_MEMORY_ID),
    ));
}

///
/// ChildIndexError
///

#[derive(Debug, ThisError)]
pub enum ChildIndexError {
    #[error("canister not found: {0}")]
    CanisterNotFound(Principal),
}

///
/// ChildIndexData
///

pub type ChildIndexData = HashMap<Principal, String>;

///
/// ChildIndex
///

pub struct ChildIndex {}

impl ChildIndex {
    //
    // INTERNAL ACCESSORS
    //

    pub fn with<R>(f: impl FnOnce(&BTreeMap<Principal, String>) -> R) -> R {
        CHILD_INDEX.with_borrow(|cell| f(cell))
    }

    pub fn with_mut<R>(f: impl FnOnce(&mut BTreeMap<Principal, String>) -> R) -> R {
        CHILD_INDEX.with_borrow_mut(|cell| f(cell))
    }

    //
    // METHODS
    //

    #[must_use]
    pub fn is_empty() -> bool {
        Self::with(|map| map.is_empty())
    }

    #[must_use]
    pub fn get(pid: &Principal) -> Option<String> {
        Self::with(|map| map.get(pid))
    }

    pub fn try_get(pid: &Principal) -> Result<String, Error> {
        if let Some(kind) = Self::get(pid) {
            Ok(kind)
        } else {
            Err(MemoryError::from(ChildIndexError::CanisterNotFound(*pid)))?
        }
    }

    #[must_use]
    pub fn get_by_kind(kind: &str) -> Vec<Principal> {
        Self::with(|map| {
            map.iter_pairs()
                .filter_map(|(p, k)| if k == kind { Some(p) } else { None })
                .collect()
        })
    }

    pub fn insert(pid: Principal, kind: &str) {
        Self::with_mut(|map| {
            map.insert(pid, kind.to_string());
        });
    }

    pub fn remove(pid: &Principal) {
        Self::with_mut(|map| {
            map.remove(pid);
        });
    }

    pub fn clear() {
        Self::with_mut(|map| {
            map.clear();
        });
    }

    //
    // EXPORT
    //

    #[must_use]
    pub fn export() -> ChildIndexData {
        Self::with(|map| map.iter_pairs().collect())
    }
}
