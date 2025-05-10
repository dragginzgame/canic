use crate::ic::structures::{BTreeMap, DefaultMemory};
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// SubnetIndexError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum SubnetIndexError {
    #[error("canister type not found: {0}")]
    CanisterTypeNotFound(String),
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
    pub fn get_data(&self) -> Vec<(String, Principal)> {
        self.iter().collect()
    }

    pub fn set_data(&mut self, data: Vec<(String, Principal)>) {
        self.clear();
        for (k, v) in data {
            self.insert(k, v);
        }
    }

    pub fn try_get_canister(&self, key: &str) -> Result<Principal, SubnetIndexError> {
        self.get_canister(key)
            .ok_or_else(|| SubnetIndexError::CanisterTypeNotFound(key.to_string()))
    }

    #[must_use]
    pub fn get_canister(&self, key: &str) -> Option<Principal> {
        self.get(&key.to_string())
    }

    pub fn set_canister(&mut self, key: String, id: Principal) {
        self.insert(key, id);
    }
}

///
/// SubnetIndexData
///

pub type SubnetIndexData = Vec<(String, Principal)>;
