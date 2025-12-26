use crate::model::memory::state::{SubnetState, SubnetStateData};

//
// Stable-memory adapter
//

///
/// SubnetStateOps
///

pub struct SubnetStateOps;

impl SubnetStateOps {
    pub fn import(data: SubnetStateData) {
        SubnetState::import(data);
    }

    #[must_use]
    pub fn export() -> SubnetStateData {
        SubnetState::export()
    }
}
