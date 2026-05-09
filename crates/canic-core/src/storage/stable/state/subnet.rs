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
            SubnetStateRecord::default(),
        ));
}

///
/// RootPublicKeyRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootPublicKeyRecord {
    pub public_key_sec1: Vec<u8>,
    pub key_name: String,
    pub key_hash: [u8; 32],
}

///
/// SubnetAuthStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetAuthStateRecord {
    pub delegated_root_public_key: Option<RootPublicKeyRecord>,
}

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
            auth: SubnetAuthStateRecord {
                delegated_root_public_key: Some(RootPublicKeyRecord {
                    public_key_sec1: vec![1, 2, 3],
                    key_name: "key_1".to_string(),
                    key_hash: [7; 32],
                }),
            },
        };

        SubnetState::import(record.clone());
        assert_eq!(SubnetState::export(), record);
    }
}
