use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    icu_memory, impl_storable_bounded,
    memory::id::state::SUBNET_STATE_ID,
    thread_local_memory,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

// SUBNET_STATE
thread_local_memory! {
    static SUBNET_STATE: RefCell<Cell<SubnetStateData, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            icu_memory!(SubnetState, SUBNET_STATE_ID),
            SubnetStateData::default(),
        ));
}

///
/// SubnetStateError
///

#[derive(Debug, ThisError)]
pub enum SubnetStateError {
    #[error("subnet pid has not been set")]
    NotSet,
}

impl_storable_bounded!(SubnetStateData, 32, true);

///
/// SubnetStateData
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetStateData {
    pub subnet_pid: Option<Principal>,
}

///
/// SubnetState
///

pub struct SubnetState;

impl SubnetState {
    pub fn set_subnet_pid(pid: Principal) {
        SUBNET_STATE.with_borrow_mut(|cell| {
            let mut data = *cell.get();
            data.subnet_pid = Some(pid);
            cell.set(data);
        });
    }

    pub fn import(data: SubnetStateData) {
        SUBNET_STATE.with_borrow_mut(|cell| cell.set(data));
    }

    #[must_use]
    pub fn export() -> SubnetStateData {
        SUBNET_STATE.with_borrow(|cell| *cell.get())
    }
}
