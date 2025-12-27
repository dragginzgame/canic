use crate::{
    dto::state::SubnetStateView,
    model::memory::state::{SubnetState, SubnetStateData},
};

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

///
/// Adapter
///

impl From<SubnetStateData> for SubnetStateView {
    fn from(_: SubnetStateData) -> Self {
        Self {}
    }
}

impl From<SubnetStateView> for SubnetStateData {
    fn from(_: SubnetStateView) -> Self {
        Self {}
    }
}
