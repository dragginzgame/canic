use crate::{
    ic::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_unbounded,
    memory::CANISTER_POOL_MEMORY_ID,
    utils::time::now_secs,
};
use candid::{CandidType, Nat, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// CANISTER_POOL
//

thread_local! {
    pub static CANISTER_POOL: RefCell<CanisterPoolCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(CanisterPoolCore::new(BTreeMap::init(
            icu_register_memory!(CANISTER_POOL_MEMORY_ID),
        )));
}

///
/// CanisterPoolError
///

#[derive(Debug, ThisError)]
pub enum CanisterPoolError {}

///
/// CanisterPoolEntry
/// cycles stored as a candid Nat because minicbor doesn't support u128
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterPoolEntry {
    pub created_at: u64,
    pub cycles: Nat,
}

impl_storable_unbounded!(CanisterPoolEntry);

///
/// CanisterPool
///

pub type CanisterPoolView = Vec<(Principal, CanisterPoolEntry)>;

pub struct CanisterPool;

impl CanisterPool {
    pub fn register(pid: Principal, cycles: u128) {
        let entry = CanisterPoolEntry {
            created_at: now_secs(),
            cycles: cycles.into(),
        };

        CANISTER_POOL.with_borrow_mut(|core| core.insert(pid, entry));
    }

    #[must_use]
    pub fn export() -> CanisterPoolView {
        CANISTER_POOL.with_borrow(CanisterPoolCore::export)
    }
}

///
/// CanisterPoolCore
///

pub struct CanisterPoolCore<M: Memory> {
    map: BTreeMap<Principal, CanisterPoolEntry, M>,
}

impl<M: Memory> CanisterPoolCore<M> {
    pub const fn new(map: BTreeMap<Principal, CanisterPoolEntry, M>) -> Self {
        Self { map }
    }

    pub fn insert(&mut self, pid: Principal, entry: CanisterPoolEntry) {
        self.map.insert(pid, entry);
    }

    pub fn export(&self) -> CanisterPoolView {
        self.map.iter_pairs().collect()
    }
}
