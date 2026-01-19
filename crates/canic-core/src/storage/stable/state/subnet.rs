use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    storage::{prelude::*, stable::memory::state::SUBNET_STATE_ID},
};
use std::cell::RefCell;

eager_static! {
    //
    // SUBNET_STATE
    // EMPTY FOR NOW - if we ever want to store subnet-specific state it's here
    //
    static SUBNET_STATE: RefCell<Cell<SubnetStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(SubnetState, SUBNET_STATE_ID),
            SubnetStateRecord::default(),
        ));
}

///
/// SubnetStateRecord
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetStateRecord {}

impl_storable_bounded!(SubnetStateRecord, 32, true);

///
/// SubnetState
///

pub struct SubnetState;

impl SubnetState {
    pub(crate) fn import(data: SubnetStateRecord) {
        SUBNET_STATE.with_borrow_mut(|cell| cell.set(data));
    }

    #[must_use]
    pub(crate) fn export() -> SubnetStateRecord {
        SUBNET_STATE.with_borrow(|cell| *cell.get())
    }
}
