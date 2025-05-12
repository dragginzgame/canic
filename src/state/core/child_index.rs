use crate::{
    ic::structures::{BTreeMap, DefaultMemory, memory::MemoryId},
    state::{CHILD_INDEX_MEMORY_ID, MEMORY_MANAGER},
};
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

///
/// ChildIndexError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum ChildIndexError {
    #[error("canister not found: {0}")]
    CanisterNotFound(Principal),
}

//
// CHILD_INDEX
//

thread_local! {
    pub static CHILD_INDEX: RefCell<ChildIndex> = RefCell::new(ChildIndex::init(
        MEMORY_MANAGER.with_borrow(|this| this.get(MemoryId::new(CHILD_INDEX_MEMORY_ID))),
    ));
}

///
/// ChildIndex
///

#[derive(Deref, DerefMut)]
pub struct ChildIndex(BTreeMap<Principal, String>);

impl ChildIndex {
    // init
    #[must_use]
    pub fn init(memory: DefaultMemory) -> Self {
        Self(BTreeMap::init(memory))
    }

    // get_state
    #[must_use]
    pub fn get_data(&self) -> ChildIndexData {
        self.iter().collect()
    }

    // get_canister
    #[must_use]
    pub fn get_canister(&self, pid: &Principal) -> Option<String> {
        self.get(pid)
    }

    // try_get_canister
    pub fn try_get_canister(&self, pid: &Principal) -> Result<String, ChildIndexError> {
        let canister = self
            .get_canister(pid)
            .ok_or(ChildIndexError::CanisterNotFound(*pid))?;

        Ok(canister)
    }

    // insert_canister
    pub fn insert_canister(&mut self, pid: Principal, path: &str) {
        self.insert(pid, path.to_string());
    }
}

///
/// ChildIndexData
///

pub type ChildIndexData = Vec<(Principal, String)>;
