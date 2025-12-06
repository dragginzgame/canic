use crate::model::memory::state::{SubnetState, SubnetStateData};

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
