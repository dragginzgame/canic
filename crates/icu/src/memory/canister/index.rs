use crate::ic::structures::{BTreeMap, DefaultMemory};
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

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

#[derive(Deref, DerefMut)]
pub struct ChildIndex(BTreeMap<Principal, String>);

impl ChildIndex {
    // init
    #[must_use]
    pub fn init(memory: DefaultMemory) -> Self {
        Self(BTreeMap::init(memory))
    }

    // get_data
    #[must_use]
    pub fn get_data(&self) -> ChildIndexData {
        ChildIndexData(self.iter().collect())
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
    pub fn insert_canister(&mut self, pid: Principal, ty: &str) {
        self.insert(pid, ty.to_string());
    }
}

///
/// ChildIndexData
///

#[derive(Clone, Debug, Deref, DerefMut, CandidType, Deserialize, Serialize)]
pub struct ChildIndexData(pub Vec<(Principal, String)>);

impl ChildIndexData {
    #[must_use]
    pub fn get(&self, pid: &Principal) -> Option<&str> {
        self.0
            .iter()
            .find_map(|(p, ty)| if p == pid { Some(ty.as_str()) } else { None })
    }

    #[must_use]
    pub fn get_by_type(&self, ty: &str) -> Vec<Principal> {
        self.0
            .iter()
            .filter_map(|(p, t)| if t == ty { Some(*p) } else { None })
            .collect()
    }
}

impl IntoIterator for ChildIndexData {
    type Item = (Principal, String);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
