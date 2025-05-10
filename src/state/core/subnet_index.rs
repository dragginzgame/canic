use crate::CanisterType;
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use mimic::ic::structures::{BTreeMap, DefaultMemory};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// SubnetIndexError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum SubnetIndexError {
    #[error("canister type not found: {0}")]
    CanisterTypeNotFound(CanisterType),
}

///
/// SubnetIndex
/// a map of CanisterType to canister principal
///

#[derive(Deref, DerefMut)]
pub struct SubnetIndex(BTreeMap<CanisterType, Principal>);

impl SubnetIndex {
    // init
    #[must_use]
    pub fn init(memory: DefaultMemory) -> Self {
        Self(BTreeMap::init(memory))
    }

    // get_data
    #[must_use]
    pub fn get_data(&self) -> SubnetIndexData {
        self.iter().collect()
    }

    // set_data
    pub fn set_data(&mut self, data: SubnetIndexData) {
        self.clear();
        for (k, v) in data {
            self.insert(k, v);
        }
    }

    // try_get_canister
    pub fn try_get_canister(&self, canister: &CanisterType) -> Result<Principal, SubnetIndexError> {
        let canister = self
            .get_canister(canister)
            .ok_or_else(|| SubnetIndexError::CanisterTypeNotFound(canister.clone()))?;

        Ok(canister)
    }

    // get_canister
    #[must_use]
    pub fn get_canister(&self, canister: &CanisterType) -> Option<Principal> {
        self.get(canister)
    }

    // set_canister
    pub fn set_canister(&mut self, canister: CanisterType, id: Principal) {
        self.insert(canister, id);
    }
}

///
/// SubnetIndexData
///

pub type SubnetIndexData = Vec<(CanisterType, Principal)>;
