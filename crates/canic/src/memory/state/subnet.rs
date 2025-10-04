use crate::{
    Error,
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, ic_memory, impl_storable_bounded,
    memory::{MemoryError, id::state::SUBNET_STATE_ID, state::StateError},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// SUBNET_STATE
//

eager_static! {
    static SUBNET_STATE: RefCell<Cell<SubnetStateData, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(SubnetState, SUBNET_STATE_ID),
            SubnetStateData::default(),
        ));
}

///
/// SubnetStateError
///

#[derive(Debug, ThisError)]
pub enum SubnetStateError {
    #[error("subnet pid has not been set")]
    SubnetNotSet,

    #[error("root pid has not been set")]
    RootNotSet,
}

impl From<SubnetStateError> for Error {
    fn from(err: SubnetStateError) -> Self {
        MemoryError::from(StateError::from(err)).into()
    }
}

///
/// SubnetStateData
/// (identity of this subnet, shared across all canisters in it)
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetStateData {
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,
}

impl_storable_bounded!(SubnetStateData, 32, true);

///
/// SubnetState (public API)
///

pub struct SubnetState;

impl SubnetState {
    // ---- Subnet PID ----

    #[must_use]
    pub fn get_subnet_pid() -> Option<Principal> {
        SUBNET_STATE.with_borrow(|cell| cell.get().subnet_pid)
    }

    pub fn try_get_subnet_pid() -> Result<Principal, Error> {
        Self::get_subnet_pid().ok_or_else(|| SubnetStateError::SubnetNotSet.into())
    }

    pub fn set_subnet_pid(pid: Principal) {
        SUBNET_STATE.with_borrow_mut(|cell| {
            let mut data = *cell.get();
            data.subnet_pid = Some(pid);
            cell.set(data);
        });
    }

    // ---- Root PID ----

    #[must_use]
    pub fn get_root_pid() -> Option<Principal> {
        SUBNET_STATE.with_borrow(|cell| cell.get().root_pid)
    }

    pub fn try_get_root_pid() -> Result<Principal, Error> {
        Self::get_root_pid().ok_or_else(|| SubnetStateError::RootNotSet.into())
    }

    pub fn set_root_pid(pid: Principal) {
        SUBNET_STATE.with_borrow_mut(|cell| {
            let mut data = *cell.get();
            data.root_pid = Some(pid);
            cell.set(data);
        });
    }

    // ---- Import / Export ----

    pub fn import(data: SubnetStateData) {
        SUBNET_STATE.with_borrow_mut(|cell| cell.set(data));
    }

    #[must_use]
    pub fn export() -> SubnetStateData {
        SUBNET_STATE.with_borrow(|cell| *cell.get())
    }
}
