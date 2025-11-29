use crate::{
    Error,
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, ic_memory, impl_storable_bounded,
    model::{
        ModelError,
        memory::{MemoryError, id::state::SUBNET_STATE_ID, state::StateError},
    },
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// SUBNET_STATE
//
// EMPTY FOR NOW - if we ever want to store subnet-specific state it's here
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
pub enum SubnetStateError {}

impl From<SubnetStateError> for Error {
    fn from(err: SubnetStateError) -> Self {
        ModelError::MemoryError(MemoryError::from(StateError::from(err))).into()
    }
}

///
/// SubnetStateData
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetStateData {}

impl_storable_bounded!(SubnetStateData, 32, true);

///
/// SubnetState (public API)
///

pub struct SubnetState;

impl SubnetState {
    pub fn import(data: SubnetStateData) {
        SUBNET_STATE.with_borrow_mut(|cell| cell.set(data));
    }

    #[must_use]
    pub fn export() -> SubnetStateData {
        SUBNET_STATE.with_borrow(|cell| *cell.get())
    }
}
