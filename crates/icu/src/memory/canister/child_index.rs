use crate::{ic::structures::BTreeMap, icu_register_memory, memory::CHILD_INDEX_MEMORY_ID};
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// CHILD_INDEX
//

thread_local! {
    pub static CHILD_INDEX: RefCell<BTreeMap<Principal, String>> = RefCell::new(BTreeMap::init(
        icu_register_memory!(AppStateData, CHILD_INDEX_MEMORY_ID),
    ));
}

///
/// ChildIndexError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum ChildIndexError {
    #[error("canister not found: {0}")]
    CanisterNotFound(Principal),
}

///
/// ChildIndex
///

pub struct ChildIndex;

impl ChildIndex {
    pub fn with<R>(f: impl FnOnce(&BTreeMap<Principal, String>) -> R) -> R {
        CHILD_INDEX.with(|cell| f(&cell.borrow()))
    }

    pub fn with_mut<R>(f: impl FnOnce(&mut BTreeMap<Principal, String>) -> R) -> R {
        CHILD_INDEX.with(|cell| f(&mut cell.borrow_mut()))
    }

    #[must_use]
    pub fn get_data() -> ChildIndexData {
        Self::with(|map| ChildIndexData(map.iter().collect()))
    }

    pub fn insert_canister(pid: Principal, ty: &str) {
        Self::with_mut(|map| {
            map.insert(pid, ty.to_string());
        });
    }

    pub fn remove_canister(pid: &Principal) {
        Self::with_mut(|map| {
            map.remove(pid);
        });
    }

    pub fn clear() {
        Self::with_mut(|map| {
            map.clear();
        });
    }

    // get_by_type
    #[must_use]
    pub fn get_by_type(ty: &str) -> Vec<Principal> {
        Self::with(|map| {
            map.iter()
                .filter_map(|(p, t)| if t == ty { Some(p) } else { None })
                .collect()
        })
    }

    // get_canister
    #[must_use]
    pub fn get_canister(pid: &Principal) -> Option<String> {
        Self::with(|map| map.get(pid))
    }

    // try_get_canister
    pub fn try_get_canister(pid: &Principal) -> Result<String, ChildIndexError> {
        let canister = Self::get_canister(pid).ok_or(ChildIndexError::CanisterNotFound(*pid))?;

        Ok(canister)
    }
}

///
/// ChildIndexData
///

#[derive(Clone, Debug, Deref, DerefMut, CandidType, Deserialize, Serialize)]
pub struct ChildIndexData(HashMap<Principal, String>);

impl IntoIterator for ChildIndexData {
    type Item = (Principal, String);
    type IntoIter = std::collections::hash_map::IntoIter<Principal, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
