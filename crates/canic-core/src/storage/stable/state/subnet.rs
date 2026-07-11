use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    role_contract::allocation::memory::env::SUBNET_STATE_ID,
    storage::prelude::*,
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

impl SubnetStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "SubnetStateRecord";
}

///
/// SubnetStateData
///
/// Canonical subnet-state import/export snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SubnetStateData {
    pub record: SubnetStateRecord,
}

impl SubnetStateData {
    pub const STATE_CONTRACT_NAME: &'static str = "SubnetStateData";
}

///
/// SubnetState
///

pub struct SubnetState;

impl SubnetState {
    pub(crate) fn import(data: SubnetStateData) {
        SUBNET_STATE.with_borrow_mut(|cell| cell.set(data.record));
    }

    #[must_use]
    pub(crate) fn export() -> SubnetStateData {
        SubnetStateData {
            record: SUBNET_STATE.with_borrow(|cell| cell.get().clone()),
        }
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subnet_state_round_trip() {
        let record = SubnetStateRecord {
            auth: SubnetAuthStateRecord {},
        };

        SubnetState::import(SubnetStateData {
            record: record.clone(),
        });
        assert_eq!(SubnetState::export(), SubnetStateData { record });
    }
}
