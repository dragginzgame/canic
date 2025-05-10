use crate::{
    ic::structures::{BTreeMap, DefaultMemory, memory::MemoryId},
    state::{MEMORY_MANAGER, SUBNET_INDEX_MEMORY_ID},
};
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

///
/// SubnetIndexError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum SubnetIndexError {
    #[error("canister type not found: {0}")]
    CanisterTypeNotFound(String),
}

//
// SUBNET_INDEX
//

thread_local! {
    pub static SUBNET_INDEX: RefCell<SubnetIndex> = RefCell::new(SubnetIndex::init(
        MEMORY_MANAGER.with_borrow(|this| this.get(MemoryId::new(SUBNET_INDEX_MEMORY_ID))),
    ));
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

    #[must_use]
    pub fn get_canister<S: ToString>(&self, key: &S) -> Option<Principal> {
        self.get(&key.to_string())
    }

    pub fn set_canister<S: ToString>(&mut self, key: S, id: Principal) {
        self.insert(key.to_string(), id);
    }
}

///
/// SubnetIndexData
///

pub type SubnetIndexData = Vec<(String, Principal)>;
