use crate::{
    ic::structures::{BTreeMap, DefaultMemory},
    impl_storable_unbounded,
};
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, thread::LocalKey};
use thiserror::Error as ThisError;

///
/// SharderError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum SharderError {
    #[error("principal '{0}' already exists")]
    PrincipalExists(Principal),
}

///
/// CanisterShard
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterShard {
    pub pid: Principal,
    pub users: u16,
}

impl CanisterShard {
    #[must_use]
    pub const fn new(pid: Principal) -> Self {
        Self { pid, users: 0 }
    }
}

impl_storable_unbounded!(CanisterShard);

///
/// Sharder
///

#[derive(Deref, DerefMut)]
pub struct Sharder(BTreeMap<Principal, CanisterShard>);

impl Sharder {
    // init
    #[must_use]
    pub fn init(memory: DefaultMemory) -> Self {
        Self(BTreeMap::init(memory))
    }

    // get_data
    #[must_use]
    pub fn get_data(&self) -> SharderData {
        self.iter().collect()
    }

    //
    // get_next_canister_pid
    //
    // Some(principal) will assign the Player to an existing canister
    // None means a new game canister must be created
    //
    #[must_use]
    pub fn get_next_canister_pid(&self) -> Option<Principal> {
        self.first_key_value().map(|(k, _)| k)
    }

    // register_shard
    pub fn register_shard(
        &mut self,
        pid: Principal,
        shard: CanisterShard,
    ) -> Result<(), SharderError> {
        self.clear();
        if self.contains_key(&pid) {
            Err(SharderError::PrincipalExists(pid))?;
        }
        self.insert(pid, shard);

        Ok(())
    }
}

///
/// SharderData
///

pub type SharderData = Vec<(Principal, CanisterShard)>;

///
/// SharderLocal
///

pub type SharderLocal = &'static LocalKey<RefCell<Sharder>>;
