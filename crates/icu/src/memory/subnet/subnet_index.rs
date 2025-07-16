use crate::{ic::structures::BTreeMap, icu_register_memory, memory::SUBNET_INDEX_MEMORY_ID};
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// SUBNET_INDEX
//

thread_local! {
    pub static SUBNET_INDEX: RefCell<BTreeMap<String, Principal>> = RefCell::new(BTreeMap::init(
        icu_register_memory!(AppStateData, SUBNET_INDEX_MEMORY_ID),
    ));
}

///
/// SubnetIndexError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum SubnetIndexError {
    #[error("canister not found: {0}")]
    CanisterNotFound(String),
}

pub struct SubnetIndex {}

impl SubnetIndex {
    pub fn with<R>(f: impl FnOnce(&BTreeMap<String, Principal>) -> R) -> R {
        SUBNET_INDEX.with(|cell| f(&cell.borrow()))
    }

    pub fn with_mut<R>(f: impl FnOnce(&mut BTreeMap<String, Principal>) -> R) -> R {
        SUBNET_INDEX.with(|cell| f(&mut cell.borrow_mut()))
    }

    #[must_use]
    pub fn get_data() -> SubnetIndexData {
        Self::with(|map| SubnetIndexData(map.iter().collect()))
    }

    pub fn set_data(data: SubnetIndexData) {
        Self::with_mut(|map| {
            map.clear();
            for (k, v) in data.iter() {
                map.insert(k.clone(), *v);
            }
        });
    }

    #[must_use]
    pub fn get_canister(kind: &str) -> Option<Principal> {
        Self::with(|map| map.get(&kind.to_string()))
    }

    pub fn try_get_canister(kind: &str) -> Result<Principal, SubnetIndexError> {
        Self::get_canister(kind).ok_or_else(|| SubnetIndexError::CanisterNotFound(kind.to_string()))
    }

    pub fn set_canister(kind: &str, id: Principal) {
        Self::with_mut(|map| {
            map.insert(kind.to_string(), id);
        });
    }

    pub fn remove_canister(kind: &str) {
        Self::with_mut(|map| {
            map.remove(&kind.to_string());
        });
    }
}

///
/// SubnetIndexData
///

#[derive(Clone, Debug, Deref, DerefMut, CandidType, Deserialize, Serialize)]
pub struct SubnetIndexData(HashMap<String, Principal>);

impl IntoIterator for SubnetIndexData {
    type Item = (String, Principal);
    type IntoIter = std::collections::hash_map::IntoIter<String, Principal>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
