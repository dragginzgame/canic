use crate::model::memory::state::{SubnetState, SubnetStateData, SubnetStateView};

///
/// SubnetStateOps
///

pub struct SubnetStateOps;

impl SubnetStateOps {
    pub fn import(view: SubnetStateView) {
        let data: SubnetStateData = view.into();
        SubnetState::import(data);
    }

    #[must_use]
    pub fn export() -> SubnetStateView {
        let data: SubnetStateData = SubnetState::export();
        data.into()
    }
}
