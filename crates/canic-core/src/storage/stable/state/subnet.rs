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
            crate::ic_memory_key!("canic.core.subnet_state.v1", SubnetState, SUBNET_STATE_ID),
            SubnetStateRecord::default(),
        ));
}

///
/// SubnetAuthStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetAuthStateRecord {}

///
/// SubnetStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetStateRecord {
    pub auth: SubnetAuthStateRecord,
}

impl_storable_bounded!(SubnetStateRecord, 1024, true);

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
        SUBNET_STATE.with_borrow(|cell| cell.get().clone())
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
        let record = SubnetStateRecord {
            auth: SubnetAuthStateRecord {},
        };

        SubnetState::import(record.clone());
        assert_eq!(SubnetState::export(), record);
    }
}
