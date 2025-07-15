use crate::ic::structures::{BTreeMap, DefaultMemory};
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error as ThisError;

///
/// SubnetIndexError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum SubnetIndexError {
    #[error("canister not found: {0}")]
    CanisterNotFound(String),
}

///
/// SubnetIndex
///

#[derive(Deref, DerefMut)]
pub struct SubnetIndex(BTreeMap<String, Principal>);

impl SubnetIndex {
    #[must_use]
    pub fn init(memory: DefaultMemory) -> Self {
        Self(BTreeMap::init(memory))
    }

    #[must_use]
    pub fn get_data(&self) -> SubnetIndexData {
        SubnetIndexData(self.iter().collect())
    }

    pub fn set_data(&mut self, data: SubnetIndexData) {
        self.clear();
        for (k, v) in data.iter() {
            self.insert(k.to_string(), *v);
        }
    }

    #[must_use]
    pub fn get_canister(&self, path: &str) -> Option<Principal> {
        self.get(&path.to_string())
    }

    pub fn try_get_canister(&self, path: &str) -> Result<Principal, SubnetIndexError> {
        self.get_canister(path)
            .ok_or_else(|| SubnetIndexError::CanisterNotFound(path.to_string()))
    }

    pub fn set_canister(&mut self, path: &str, id: Principal) {
        self.insert(path.to_string(), id);
    }

    pub fn remove_canister(&mut self, path: &str) {
        self.remove(&path.to_string());
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
