use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    storage::{prelude::*, stable::memory::env::SUBNET_STATE_ID},
};
use std::cell::RefCell;

//
// SUBNET_STATE
//

eager_static! {
    static SUBNET_STATE: RefCell<Cell<SubnetStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(SubnetState, SUBNET_STATE_ID),
            SubnetStateRecord,
        ));
}

///
/// SubnetStateRecord
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetStateRecord;

impl_storable_bounded!(SubnetStateRecord, 8, true);

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

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subnet_state_round_trip() {
        SubnetState::import(SubnetStateRecord);
        assert_eq!(SubnetState::export(), SubnetStateRecord);
    }
}
